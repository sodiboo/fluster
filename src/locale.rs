use std::ffi::{CStr, CString};

use crate::{sys, Engine};

pub struct Locale {
    /// The language code of the locale. For example, "en".
    pub language_code: CString,
    /// The country code of the locale. For example, "US".
    pub country_code: Option<CString>,
    /// The script code of the locale.
    pub script_code: Option<CString>,
    /// The variant code of the locale.
    pub variant_code: Option<CString>,
}

impl Engine {
    /// Notify a running engine instance that the locale has been updated.
    /// The preferred locale must be the first item in the list of locales supplied.
    /// The other entries will be used as a fallback.
    pub fn update_locales(&mut self, locales: &[Locale]) -> crate::Result<()> {
        let locales: Box<[sys::FlutterLocale]> = locales
            .iter()
            .map(|locale| sys::FlutterLocale {
                struct_size: std::mem::size_of::<sys::FlutterLocale>(),
                language_code: locale.language_code.as_ptr(),
                country_code: locale
                    .country_code
                    .as_deref()
                    .map(CStr::as_ptr)
                    .unwrap_or(std::ptr::null()),
                script_code: locale
                    .script_code
                    .as_deref()
                    .map(CStr::as_ptr)
                    .unwrap_or(std::ptr::null()),
                variant_code: locale
                    .variant_code
                    .as_deref()
                    .map(CStr::as_ptr)
                    .unwrap_or(std::ptr::null()),
            })
            .collect();

        let mut locales: Box<[*const sys::FlutterLocale]> =
            locales.iter().map(|locale| locale as _).collect();

        unsafe { sys::UpdateLocales(self.inner.engine, locales.as_mut_ptr(), locales.len()) }
            .to_result()
    }
}
