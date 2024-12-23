use std::{ffi::CString, path::PathBuf};

use crate::sys;

/// This enum specifies one of the various locations the engine can look for AOT data sources.
#[derive(Debug, Clone)]
pub enum AOTDataSource {
    /// Absolute path to an ELF library file.
    ElfPath(PathBuf),
}

/// An opaque object that describes the AOT data that can be used to launch a Flutter [`crate::Engine`] instance in AOT mode.
#[must_use]
pub struct AOTData {
    pub(crate) data: sys::FlutterEngineAOTData,
}

impl AOTData {
    /// Indicates whether the Dart VM requires AOT data or JIT data to run.
    /// If this returns true, the Dart VM requires AOT data to run.
    /// If this returns false, the Dart VM requires JIT data to run.
    ///
    /// This function is static because it is a result of how the Dart VM was compiled,
    /// and not a property of any specific instance of the Dart VM.
    #[must_use]
    pub fn is_aot() -> bool {
        unsafe { sys::RunsAOTCompiledDartCode() }
    }

    /// Creates the necessary data structures to launch a Flutter Dart application in AOT mode.
    ///
    /// Returns [`Ok()`] if the AOT data could be successfully resolved.
    ///
    /// Always returns [`Err()`] if !([`Self::is_aot()`]).
    #[allow(clippy::missing_panics_doc)]
    pub fn new(source: &AOTDataSource) -> crate::Result<Self> {
        let mut data: sys::FlutterEngineAOTData = unsafe { std::mem::zeroed() };

        match source {
            AOTDataSource::ElfPath(path) => {
                let path = CString::new(path.as_os_str().as_encoded_bytes()).expect("invalid path");
                let source = sys::FlutterEngineAOTDataSource {
                    type_: sys::FlutterEngineAOTDataSourceType::ElfPath,
                    __bindgen_anon_1: sys::FlutterEngineAOTDataSource__bindgen_ty_1 {
                        elf_path: path.as_ptr(),
                    },
                };

                unsafe { sys::CreateAOTData(&raw const source, &raw mut data) }
            }
        }
        .to_result()
        .map(|()| Self { data })
    }
}

impl Drop for AOTData {
    fn drop(&mut self) {
        unsafe { sys::CollectAOTData(self.data) };
    }
}
