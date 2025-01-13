use std::path::PathBuf;

use bindgen::callbacks::{IntKind, ItemKind, MacroParsingBehavior};

#[derive(Debug)]
struct ParseCallbacks;

impl bindgen::callbacks::ParseCallbacks for ParseCallbacks {
    fn header_file(&self, filename: &str) {
        println!("cargo::rerun-if-changed={filename}");
    }

    fn include_file(&self, filename: &str) {
        println!("cargo::rerun-if-changed={filename}");
    }

    fn read_env_var(&self, key: &str) {
        println!("cargo::rerun-if-env-changed={key}");
    }

    fn generated_name_override(
        &self,
        item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        matches!(item_info.kind, ItemKind::Function)
            .then(|| {
                item_info
                    .name
                    .strip_prefix("FlutterEngine")
                    .or_else(|| match item_info.name {
                        "FlutterPlatformMessageCreateResponseHandle"
                        | "FlutterPlatformMessageReleaseResponseHandle" => {
                            Some(item_info.name.strip_prefix("Flutter").unwrap())
                        }
                        "__FlutterEngineFlushPendingTasksNow" => Some("__FlushPendingTasksNow"),
                        _ => {
                            println!(
                                "cargo:warning=embedder function with bad name: {}",
                                item_info.name
                            );
                            None
                        }
                    })
            })
            .and_then(std::convert::identity)
            .map(Into::into)
    }

    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        if name.contains("FLUTTER") {
            MacroParsingBehavior::Default
        } else {
            // buncha builtin stuff like integer constants.
            // so don't generate them; avoids dead code warnings.
            MacroParsingBehavior::Ignore
        }
    }

    fn int_macro(&self, name: &str, _value: i64) -> Option<IntKind> {
        if name == "FLUTTER_ENGINE_VERSION" {
            // This macro is specifically just the parameter to `FlutterEngine{Run,Initialize}`.
            // It has type `size_t`, which is `usize` in Rust.
            // There is no other use, so the macro clearly has that type.
            Some(IntKind::Custom {
                name: "usize",
                is_signed: false,
            })
        } else {
            None
        }
    }

    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        let Some(enum_name) = enum_name else {
            println!("cargo:warning=enum variant {original_variant_name} has no parent enum");
            return None;
        };
        let Some(variant) = original_variant_name.strip_prefix("k") else {
            println!("cargo:warning=enum variant {original_variant_name} not prefixed with 'k'");
            return None;
        };

        Some(if let Some(variant) = variant.strip_prefix(enum_name) {
            variant.to_string()
        } else {
            match enum_name {
                // has a weird prefix on variants
                "FlutterPointerMouseButtons" => variant
                    .strip_prefix("FlutterPointerButtonMouse")
                    .unwrap_or(variant)
                    .to_string(),
                // they have no prefix on variants (other than hungarian notation, ofc)
                "FlutterEngineResult"
                | "FlutterRendererType"
                | "FlutterThreadPriority"
                | "FlutterMetalExternalTexturePixelFormat"
                | "FlutterMetalExternalTextureYUVColorSpace"
                | "FlutterPointerPhase"
                | "FlutterStringAttributeType" => variant.to_string(),
                _ => {
                    println!(
                        "cargo:warning=enum variant {original_variant_name} has unpredictable name with parent {enum_name}"
                    );
                    variant.to_string()
                }
            }
        })
    }
}

fn generate_bindings(embedder_h: &str) {
    println!("cargo::rerun-if-changed={embedder_h}");
    bindgen::builder()
        .header(embedder_h)
        .parse_callbacks(Box::new(ParseCallbacks))
        // various non-flutter-related things that generate warnings idc about
        .blocklist_type("u?int_(f|le)ast(8|16|32|64)_t")
        .blocklist_type("(wchar|max_align|u?intmax)_t")
        .blocklist_type("__.*")
        // These are entirely unused. They're part of deprecated APIs.
        .blocklist_var("kFlutterSemantics(Node|CustomAction)IdBatchEnd")
        .default_enum_style(bindgen::EnumVariation::NewType {
            is_bitfield: false,
            is_global: false,
        })
        .bitfield_enum("FlutterAccessibilityFeature")
        .bitfield_enum("FlutterSemanticsAction")
        .bitfield_enum("FlutterSemanticsFlag")
        .bitfield_enum("FlutterPointerMouseButtons")
        .default_non_copy_union_style(bindgen::NonCopyUnionStyle::ManuallyDrop)
        .no_copy("FlutterOpenGL(Texture|Framebuffer|BackingStore)(__bindgen.*)?")
        .no_copy("FlutterSoftwareBackingStore2?(__bindgen.*)?")
        .no_copy("FlutterMetal(Texture|BackingStore)(__bindgen.*)?")
        .no_copy("FlutterVulkanBackingStore(__bindgen.*)?")
        .no_copy("Flutter(Damage|Region)")
        .layout_tests(false)
        .merge_extern_blocks(true)
        .sort_semantically(true)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(PathBuf::from(env("OUT_DIR").unwrap()).join("embedder.rs"))
        .expect("Couldn't write bindings");
}

fn env(name: &str) -> Option<String> {
    println!("cargo::rerun-if-env-changed={name}");
    std::env::var(name).ok()
}

fn main() {
    const ENGINE: &str = "FLUTTER_ENGINE";
    let flutter_engine = match env(ENGINE) {
        Some(s) if !s.is_empty() => s,
        _ => {
            println!("cargo::error=The {ENGINE} environment variable is missing. It must point to a library path with libflutter_engine.so");
            return;
        }
    };
    println!("cargo::metadata=root={flutter_engine}");

    let flutter_engine = {
        let mut path = PathBuf::from(&flutter_engine);
        path.push("out");
        if !path.is_dir() {
            println!("cargo::error=The {ENGINE} environment variable must point to a flutter engine build directory, containing an 'out' directory");
            return;
        }

        let valid_link_hosts = path
            .read_dir()
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>();
        let valid_link_hosts = valid_link_hosts
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>();

        const LINK_HOST: &str = "VOLITO_LINK_HOST";

        fn auto_find_impl(
            path: &mut PathBuf,
            preferred: &str,
            sentinel: &str,
            valid_link_hosts: &[&str],
        ) -> bool {
            if valid_link_hosts.contains(&preferred) {
                path.push(preferred);
                assert!(path.is_dir());
                true
            } else {
                // apparently they are not named the same on all platforms?
                // https://github.com/flutter/engine/blob/a8054928cf3b18235ca5e32710eedcb012f96a9e/examples/glfw/run.sh#L4-L8
                println!("cargo::warning=there is no `{preferred}` directory in the engine build directory...");
                let v = if let Some(fallback) = valid_link_hosts
                    .iter()
                    .copied()
                    .find(|s| s.contains(sentinel))
                {
                    path.push(fallback);
                    assert!(path.is_dir());
                    println!(
                        "cargo::warning=...but `{fallback}` looks about right; using that one instead"
                    );
                    println!("cargo::warning=note: override this by setting the {LINK_HOST} environment variable");
                    true
                } else {
                    println!("cargo::error=...and i couldn't find anything that looks like a {sentinel} host.");
                    println!("cargo::error=please specify a host with the {LINK_HOST} environment variable.");
                    println!("cargo::error=valid hosts are:");
                    for host in valid_link_hosts {
                        println!("cargo::error=  - {host}");
                    }
                    false
                };

                println!("cargo::warning=note: open an issue. this is a bug.");
                v
            }
        }

        fn auto_find(path: &mut PathBuf, valid_link_hosts: &[&str]) -> bool {
            if cfg!(debug_assertions) {
                auto_find_impl(path, "host_debug_unopt", "debug", valid_link_hosts)
            } else {
                auto_find_impl(path, "host_release", "release", valid_link_hosts)
            }
        }

        if let Some(link_host) = env(LINK_HOST) {
            let (was_split, link_host) = link_host.split_once('/').map_or(
                (false, link_host.as_str()),
                |(release, debug)| {
                    (
                        true,
                        if cfg!(debug_assertions) {
                            debug
                        } else {
                            release
                        },
                    )
                },
            );
            path.push(link_host);
            if !path.is_dir() {
                println!("cargo::error={LINK_HOST} must contain a valid host identifier");
                println!(r#"cargo::error={LINK_HOST}="{link_host}"; valid hosts are:"#);
                for host in valid_link_hosts {
                    println!("cargo::error=  - {host}");
                }
                if !was_split {
                    println!("cargo::warning=note: you can specify a different release and debug host by separating them with a '/'");
                }

                return;
            }
        } else if !auto_find(&mut path, &valid_link_hosts) {
            return;
        }

        path
    };

    println!("cargo::metadata=path={}", flutter_engine.to_str().unwrap());
    println!(
        "cargo::rustc-link-search=native={}",
        flutter_engine.to_str().unwrap()
    );
    println!("cargo::rustc-link-lib=flutter_engine");

    let embedder_h = flutter_engine.join("flutter_embedder.h");
    let link_host = flutter_engine.file_name().unwrap();
    let icudtl_dat = flutter_engine.join("icudtl.dat");

    let embedder_h = embedder_h.to_str().unwrap();
    let link_host = link_host.to_str().unwrap();
    let icudtl_dat = icudtl_dat.to_str().unwrap();

    println!("cargo::metadata=link_host={link_host}");
    println!("cargo::metadata=icudtl_dat={icudtl_dat}");
    generate_bindings(embedder_h);
}
