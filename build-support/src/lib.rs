#![allow(clippy::print_stdout, clippy::missing_panics_doc)]

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
};

fn env(name: &str) -> Option<String> {
    println!("cargo::rerun-if-env-changed={name}");
    std::env::var(name).ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Debug,
    Release,
    Profile,
}

impl Default for Mode {
    fn default() -> Self {
        // standard cargo env var
        if env("DEBUG").unwrap().parse().unwrap() {
            Self::Debug
        } else {
            Self::Release
        }
    }
}

impl Mode {
    #[must_use]
    // but this one is intended to be used with custom env var, for the *Flutter* build profile
    pub fn from_env(env_var: impl AsRef<str>) -> Self {
        match env(env_var.as_ref()).as_deref() {
            Some("debug") => Self::Debug,
            Some("release") => Self::Release,
            Some("profile") => Self::Profile,
            Some(invalid) => {
                println!("cargo::error=invalid BUILD_PROFILE={invalid}");
                Self::default()
            }
            None => Self::default(),
        }
    }

    fn flag(self) -> &'static str {
        match self {
            Self::Debug => "--debug",
            Self::Release => "--release",
            Self::Profile => "--profile",
        }
    }
}

pub enum BuildError {
    FlutterNotFound,
    FlutterBundleBuildFailed(std::process::Output),
    FrontendServerNotFound,
    DartNotFound {
        /// whether the needed file is `dartaotruntime` or `dart`
        wanted_aot: bool,
    },
    KernelSnapshotBuildFailed(std::process::Output),
    GenSnapshotNotFound,
    DartAotBuildFailed(std::process::Output),
}

trait CommandExt {
    fn run_or_fail_as<T>(self, err: impl Fn(std::process::Output) -> T) -> Result<(), T>;
}

impl CommandExt for &mut Command {
    fn run_or_fail_as<T>(self, err: impl Fn(std::process::Output) -> T) -> Result<(), T> {
        let output = self.output().unwrap();
        if output.status.success() {
            Ok(())
        } else {
            Err(err(output))
        }
    }
}

pub struct FlutterApp {
    asset_dir: PathBuf,
    depfile: PathBuf,
    app_library: Option<PathBuf>,
}

impl FlutterApp {
    #[must_use]
    pub fn assets(&self) -> &Path {
        &self.asset_dir
    }

    #[must_use]
    pub fn app_library(&self) -> Option<&Path> {
        self.app_library.as_deref()
    }

    #[must_use]
    pub fn depfile(&self) -> &Path {
        &self.depfile
    }
}

pub struct FlutterAppBuilder {
    mode: Mode,
    project_root: PathBuf,
    entrypoint: PathBuf,
    experimental_features: Vec<String>,
}

impl FlutterApp {
    #[must_use]
    pub fn builder() -> FlutterAppBuilder {
        FlutterAppBuilder {
            mode: Mode::from_env("BUILD_PROFILE"),
            project_root: env("CARGO_MANIFEST_DIR").unwrap().into(),
            entrypoint: "lib/main.dart".into(),
            experimental_features: Vec::new(),
        }
    }
}

impl FlutterAppBuilder {
    pub fn mode(&mut self, mode: Mode) -> &mut Self {
        self.mode = mode;
        self
    }

    pub fn project_root(&mut self, project_root: impl AsRef<Path>) -> &mut Self {
        self.project_root = project_root.as_ref().to_path_buf();
        self
    }

    pub fn entrypoint(&mut self, entrypoint: impl AsRef<Path>) -> &mut Self {
        self.entrypoint = entrypoint.as_ref().to_path_buf();
        self
    }

    pub fn with_experimental_feature(&mut self, feature: impl AsRef<str>) -> &mut Self {
        self.experimental_features
            .push(feature.as_ref().to_string());
        self
    }

    pub fn build(&self) -> Result<FlutterApp, BuildError> {
        let link_host = env("DEP_FLUTTER_ENGINE_LINK_HOST").unwrap();
        let link_host = link_host.as_str();
        let experimental_features = self
            .experimental_features
            .iter()
            .map(|f| format!("--enable-experiment={f}"))
            .collect::<Vec<_>>();
        let experimental_features = experimental_features.as_slice();

        let out_dir = PathBuf::from(env("OUT_DIR").unwrap());
        let flutter_engine_root = env("DEP_FLUTTER_ENGINE_ROOT").unwrap();
        let flutter_engine = PathBuf::from(env("DEP_FLUTTER_ENGINE_PATH").unwrap());

        let asset_dir = out_dir.join("assets");
        let depfile = out_dir.join("dependencies");

        let Ok(flutter) = which::which("flutter") else {
            return Err(BuildError::FlutterNotFound);
        };

        std::process::Command::new(flutter)
            .current_dir(&self.project_root)
            .args([
                format!("--local-engine-src-path={flutter_engine_root}"),
                format!("--local-engine={link_host}"),
                format!("--local-engine-host={link_host}"),
            ])
            .args(["build", "bundle", self.mode.flag()])
            .arg(format!(
                "--extra-front-end-options={}",
                experimental_features.join(",")
            ))
            .arg("--no-pub") // this is like `cargo update`
            .args(["--asset-dir".as_ref(), asset_dir.as_os_str()])
            .args(["--depfile".as_ref(), depfile.as_os_str()])
            .args(["--target".as_ref(), self.entrypoint.as_os_str()])
            .run_or_fail_as(BuildError::FlutterBundleBuildFailed)?;

        {
            let dependencies = std::fs::read_to_string(&depfile).unwrap();

            let mut watched_files = HashSet::new();

            for entry in dependencies.split_whitespace() {
                let p = Path::new(entry);
                watched_files.insert(p);
                if p.starts_with(&self.project_root) && !p.starts_with(&out_dir) {
                    println!("cargo::rerun-if-changed={entry}");
                }
            }

            watch_all_dart_files(&self.project_root, &watched_files);
        }

        if self.mode == Mode::Release {
            let dart_sdk = flutter_engine.join("flutter_patched_sdk");

            let regular_dart_runtime = flutter_engine.join("dart-sdk").join("bin").join("dart");
            let dart_aot_runtime = flutter_engine
                .join("dart-sdk")
                .join("bin")
                .join("dartaotruntime");

            let frontend_server_jit = flutter_engine
                .join("gen")
                .join("frontend_server.dart.snapshot");
            let frontend_server_aot = flutter_engine
                .join("gen")
                .join("frontend_server_aot.dart.snapshot");

            let (dart, frontend_server) = if frontend_server_jit.exists() {
                if !regular_dart_runtime.exists() {
                    return Err(BuildError::DartNotFound { wanted_aot: false });
                }
                (regular_dart_runtime, frontend_server_jit)
            } else if frontend_server_aot.exists() {
                if !dart_aot_runtime.exists() {
                    return Err(BuildError::DartNotFound { wanted_aot: true });
                }
                (dart_aot_runtime, frontend_server_aot)
            } else {
                return Err(BuildError::FrontendServerNotFound);
            };

            let kernel_snapshot = out_dir.join("app.dill");

            std::process::Command::new(dart)
                .current_dir(&self.project_root)
                .arg(frontend_server)
                .args(experimental_features)
                .args(["--sdk-root".as_ref(), dart_sdk.as_os_str()])
                .args(["--target=flutter", "--aot", "--tfa"])
                .arg("-Ddart.vm.product=true")
                // .args(["--packages", ".packages"])
                .args(["--output-dill".as_ref(), kernel_snapshot.as_os_str()])
                .arg(&self.entrypoint)
                .run_or_fail_as(BuildError::KernelSnapshotBuildFailed)?;

            let gen_snapshot = flutter_engine.join("gen_snapshot");

            if !gen_snapshot.exists() {
                return Err(BuildError::GenSnapshotNotFound);
            }

            let app_library = out_dir.join("app.so");

            std::process::Command::new(gen_snapshot)
                .current_dir(&self.project_root)
                .args([
                    // "--causal_async_stacks",
                    "--deterministic",
                    "--snapshot_kind=app-aot-elf",
                    "--strip",
                ])
                .arg(format!("--elf={}", app_library.display()))
                .arg(kernel_snapshot.as_path())
                .run_or_fail_as(BuildError::DartAotBuildFailed)?;
            // yay we built it

            Ok(FlutterApp {
                asset_dir,
                depfile,
                app_library: Some(app_library),
            })
        } else {
            Ok(FlutterApp {
                asset_dir,
                depfile,
                app_library: None,
            })
        }
    }
}

fn watch_all_dart_files(dir: &Path, watched_files: &HashSet<&Path>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            watch_all_dart_files(&path, watched_files);
        } else if !watched_files.contains(path.as_path())
            && path.extension() == Some("dart".as_ref())
        {
            println!("cargo::rerun-if-changed={}", path.display());
        }
    }
}
