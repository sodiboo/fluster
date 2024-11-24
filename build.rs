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

fn env(name: &str) -> Option<String> {
    println!("cargo::rerun-if-env-changed={name}");
    std::env::var(name).ok()
}

fn main() {
    let flutter_engine = match env("FLUTTER_ENGINE") {
        Some(s) if !s.is_empty() => s,
        _ => {
            println!("cargo::error=The FLUTTER_ENGINE environment variable is missing. It must point to a library path with libflutter_engine.so");
            return;
        }
    };

    {
        let mut path = PathBuf::from(&flutter_engine);
        path.push("libflutter_engine.so");
        if !path.exists() {
            println!("cargo::warning=The FLUTTER_ENGINE environment variable must point to a library path with libflutter_engine.so");
        }
    }

    println!("cargo::rustc-link-search=native={flutter_engine}");
    println!("cargo::rustc-link-lib=flutter_engine");

    let out_dir = PathBuf::from(env("OUT_DIR").unwrap());
    println!("cargo::rerun-if-changed=embedder.h");
    bindgen::builder()
        .header("embedder.h")
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
        // pointer
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
        .write_to_file(out_dir.join("embedder.rs"))
        .expect("Couldn't write bindings");
}
