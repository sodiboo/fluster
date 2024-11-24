#![warn(clippy::todo)]
#![deny(unsafe_op_in_unsafe_fn)]

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
    (

        $(
            $(#[$meta:meta])*
            $v:vis struct $name:ident($c_type:ty) {
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
            $v struct $name($c_type);

            impl $name {
                $(
                    $(#[$variant_meta])*
                    #[allow(non_upper_case_globals)]
                    pub const $variant: Self = Self(<$c_type>::$variant);
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

// mod proc_table;
mod sys;

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
