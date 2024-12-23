#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::todo)]
#![deny(clippy::print_stderr, clippy::print_stdout)] // use tracing instead
// sure let's do pedantic lints
#![warn(clippy::pedantic)]
#![allow(clippy::wildcard_imports)] // useful when nesting a module in a file
#![allow(clippy::missing_errors_doc)] // currently no proper docs so yeah
#![allow(clippy::too_many_lines)] // ugh, fix it later
#![allow(clippy::semicolon_if_nothing_returned)] // i actually disagree with this one.

macro_rules! simple_enum {
    (
        $(
            $(#[$meta:meta])*
            $v:vis enum $name:ident($c_type:ty) {
                $(
                    $(#[$variant_meta:meta])*
                    $variant:ident
                ),* $(,)?
            }
        )*
    ) => {
        $(
            $(#[$meta])*
            #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
            $v enum $name {
                $(
                    $(#[$variant_meta])*
                    $variant,
                )*
            }
            impl ::std::convert::From<$name> for $c_type {
                fn from(value: $name) -> Self {
                    match value {
                        $(
                            $name::$variant => Self::$variant,
                        )*
                    }
                }
            }

            impl ::std::convert::TryFrom<$c_type> for $name {
                type Error = $c_type;

                fn try_from(value: $c_type) -> ::std::result::Result<Self, Self::Error> {
                    match value {
                        $(
                            <$c_type>::$variant => Ok(Self::$variant),
                        )*
                        unknown => Err(unknown),
                    }
                }
            }
        )*
    };
}

macro_rules! bitfield {
    (@ $value:expr$(; $otherwise:expr)?) => {
        $value
    };
    (
        $(
            $(#[$meta:meta])*
            $v:vis struct $name:ident($c_type:ty) {
                $(
                    $(#[$variant_meta:meta])*
                    $variant:ident$( = $value:expr)?
                ),* $(,)?
            }
        )*
    ) => {
        $(
            $(#[$meta])*
            #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
            $v struct $name($c_type);

            impl $name {
                $(
                    $(#[$variant_meta])*
                    #[allow(non_upper_case_globals)]
                    pub const $variant: Self = Self(bitfield!(@ $($value;)? <$c_type>::$variant));
                )*
            }

            impl ::std::convert::From<$name> for $c_type {
                fn from(value: $name) -> Self {
                    value.0
                }
            }

            impl ::std::convert::From<$c_type> for $name {
                fn from(value: $c_type) -> Self {
                    Self(value)
                }
            }

            impl ::std::ops::BitOr<Self> for $name {
                type Output = Self;
                #[inline]
                fn bitor(self, other: Self) -> Self {
                    Self(self.0 | other.0)
                }
            }
            impl ::std::ops::BitOrAssign for $name {
                #[inline]
                fn bitor_assign(&mut self, rhs: Self) {
                    self.0 |= rhs.0;
                }
            }
            impl ::std::ops::BitAnd<Self> for $name {
                type Output = Self;
                #[inline]
                fn bitand(self, other: Self) -> Self {
                    Self(self.0 & other.0)
                }
            }
            impl ::std::ops::BitAndAssign for $name {
                #[inline]
                fn bitand_assign(&mut self, rhs: Self) {
                    self.0 &= rhs.0;
                }
            }
        )*
    };
}

macro_rules! modules {
    (
        $(
            $module:ident,
        )*
    ) => {
        $(
            mod $module;
            pub use $module::*;
        )*
    };
}

pub mod proc_table;
mod sys;

const _CHECK_ENGINE_VERSION: () = {
    const TARGET_VERSION: usize = 1;
    use sys::FLUTTER_ENGINE_VERSION;

    // this is a macro so that the error span is less noisy
    macro_rules! flutter_version_mismatch {
        () => {
            panic!("{}", ::const_format::formatcp!(
                r"

                The `fluster` crate was authored against the stable Flutter API at version {TARGET_VERSION}.
                There has been a serious breakage in the API. It is now at version {FLUTTER_ENGINE_VERSION}.

                Please check for updates to the `fluster` crate, and consult the Flutter changelog for breaking changes.

                You can also try downgrading the Flutter engine to the version that `fluster` was authored against.
                There is no way to proceed with the current version of the Flutter engine.

                "
            ));
        };
    }

    if FLUTTER_ENGINE_VERSION != TARGET_VERSION {
        flutter_version_mismatch!();
    }
};

modules![
    aot,
    compositor,
    dart_object,
    display,
    engine,
    enums,
    events,
    geometry,
    graphics,
    locale,
    pointer,
    renderer,
    semantics,
    task_runners,
    util,
];
pub mod trace;

pub fn get_proc_table() -> crate::Result<sys::FlutterEngineProcTable> {
    let mut proc_table: sys::FlutterEngineProcTable = unsafe { std::mem::zeroed() };
    proc_table.struct_size = std::mem::size_of::<sys::FlutterEngineProcTable>();
    unsafe { sys::GetProcAddresses(&raw mut proc_table) }
        .to_result()
        .map(|()| proc_table)
}
