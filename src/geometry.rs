use std::{fmt, ops};

use crate::sys;

// Really, everything in this file just needs to be f64, except specifically FlutterUIntSize, which is u32.
// But i may as well make it generic. It's some upfront boilerplate, which i just copy-pasted from Smithay.
// Makes it easier to add arbitrary conversions to other geometry types.

pub trait Coordinate:
    Sized
    + ops::Add<Self, Output = Self>
    + ops::Sub<Self, Output = Self>
    + PartialOrd
    + Default
    + Copy
    + fmt::Debug
{
    /// Downscale the coordinate
    fn downscale(self, scale: Self) -> Self;
    /// Upscale the coordinate
    fn upscale(self, scale: Self) -> Self;
    /// Convert the coordinate to a f64
    fn to_f64(self) -> f64;
    /// Convert to this coordinate from a f64
    fn from_f64(v: f64) -> Self;
    /// Compare and return the smaller one
    fn min(self, other: Self) -> Self {
        if self < other {
            self
        } else {
            other
        }
    }
    /// Compare and return the larger one
    fn max(self, other: Self) -> Self {
        if self > other {
            self
        } else {
            other
        }
    }
    /// Test if the coordinate is not negative
    fn non_negative(self) -> bool;
    /// Returns the absolute value of this coordinate
    fn abs(self) -> Self;

    /// Saturating integer addition. Computes self + other, saturating at the numeric bounds instead of overflowing.
    fn saturating_add(self, other: Self) -> Self;
    /// Saturating integer subtraction. Computes self - other, saturating at the numeric bounds instead of overflowing.
    fn saturating_sub(self, other: Self) -> Self;
    /// Saturating integer multiplication. Computes self * other, saturating at the numeric bounds instead of overflowing.
    fn saturating_mul(self, other: Self) -> Self;

    /// Multiplicative identity
    fn one() -> Self;
    /// Additive identity
    fn zero() -> Self;
}

/// Implements Coordinate for an unsigned numerical type.
macro_rules! unsigned_coordinate_impl {
    ($ty:ty, $ ($tys:ty),* ) => {
        unsigned_coordinate_impl!($ty);
        $(
            unsigned_coordinate_impl!($tys);
        )*
    };

    ($ty:ty) => {
        impl Coordinate for $ty {
            #[inline]
            fn downscale(self, scale: Self) -> Self {
                self / scale
            }

            #[inline]
            fn upscale(self, scale: Self) -> Self {
                self.saturating_mul(scale)
            }

            #[inline]
            fn to_f64(self) -> f64 {
                self as f64
            }

            #[inline]
            fn from_f64(v: f64) -> Self {
                v as Self
            }

            #[inline]
            fn non_negative(self) -> bool {
                true
            }

            #[inline]
            fn abs(self) -> Self {
                self
            }

            #[inline]
            fn saturating_add(self, other: Self) -> Self {
                self.saturating_add(other)
            }
            #[inline]
            fn saturating_sub(self, other: Self) -> Self {
                self.saturating_sub(other)
            }
            #[inline]
            fn saturating_mul(self, other: Self) -> Self {
                self.saturating_mul(other)
            }

            #[inline]
            fn one() -> Self {
                1
            }
            #[inline]
            fn zero() -> Self {
                0
            }
        }
    };
}

unsigned_coordinate_impl! {
    u8,
    u16,
    u32,
    u64,
    u128
}

/// Implements Coordinate for an signed numerical type.
macro_rules! signed_coordinate_impl {
    ($ty:ty, $ ($tys:ty),* ) => {
        signed_coordinate_impl!($ty);
        $(
            signed_coordinate_impl!($tys);
        )*
    };

    ($ty:ty) => {
        impl Coordinate for $ty {
            #[inline]
            fn downscale(self, scale: Self) -> Self {
                self / scale
            }

            #[inline]
            fn upscale(self, scale: Self) -> Self {
                self.saturating_mul(scale)
            }

            #[inline]
            fn to_f64(self) -> f64 {
                self as f64
            }

            #[inline]
            fn from_f64(v: f64) -> Self {
                v as Self
            }

            #[inline]
            fn non_negative(self) -> bool {
                self >= 0
            }

            #[inline]
            fn abs(self) -> Self {
                self.abs()
            }

            #[inline]
            fn saturating_add(self, other: Self) -> Self {
                self.saturating_add(other)
            }
            #[inline]
            fn saturating_sub(self, other: Self) -> Self {
                self.saturating_sub(other)
            }
            #[inline]
            fn saturating_mul(self, other: Self) -> Self {
                self.saturating_mul(other)
            }

            #[inline]
            fn one() -> Self {
                1
            }
            #[inline]
            fn zero() -> Self {
                0
            }
        }
    };
}

signed_coordinate_impl! {
    i8,
    i16,
    i32,
    i64,
    i128
}

macro_rules! floating_point_coordinate_impl {
    ($ty:ty, $ ($tys:ty),* ) => {
        floating_point_coordinate_impl!($ty);
        $(
            floating_point_coordinate_impl!($tys);
        )*
    };

    ($ty:ty) => {
        impl Coordinate for $ty {
            #[inline]
            fn downscale(self, scale: Self) -> Self {
                self / scale
            }

            #[inline]
            fn upscale(self, scale: Self) -> Self {
                self * scale
            }

            #[inline]
            fn to_f64(self) -> f64 {
                self as f64
            }

            #[inline]
            fn from_f64(v: f64) -> Self {
                v as Self
            }

            #[inline]
            fn non_negative(self) -> bool {
                self >= 0.0
            }

            #[inline]
            fn abs(self) -> Self {
                self.abs()
            }

            #[inline]
            fn saturating_add(self, other: Self) -> Self {
                self + other
            }
            #[inline]
            fn saturating_sub(self, other: Self) -> Self {
                self - other
            }
            #[inline]
            fn saturating_mul(self, other: Self) -> Self {
                self * other
            }

            #[inline]
            fn one() -> Self {
                1.0
            }
            #[inline]
            fn zero() -> Self {
                0.0
            }
        }
    };
}

floating_point_coordinate_impl! {
    f32,
    f64
}

macro_rules! geometry_structs {
    (
        $(
            $(#[$meta:meta])*
            struct $name:ident<$N:ident> {
                $(
                    $(#[$field_meta:meta])*
                    $field:ident: $ty:ty
                ),* $(,)?
            } where {
                $(
                    $coord:ident => $sys_ty:ty
                )*
            }
        )*
    ) => {
        $(
            $(#[$meta])*
            #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
            pub struct $name<$N> {
                $(
                    $(#[$field_meta])*
                    pub $field: $ty,
                )*
            }

            geometry_structs!(@ $name [$($field: $ty,)*] $($coord => $sys_ty,)*);
        )*
    };
    (@ $name:ident $fields:tt) => {};
    (@ $name:ident [$($field:ident: $ty:ty,)*] $coord:ident => $sys_ty:ty, $($t:tt)*) => {
        impl From<$sys_ty> for $name<$coord>
        {
            fn from(sys: $sys_ty) -> Self {
                Self {
                    $(
                        $field: sys.$field.into(),
                    )*
                }
            }
        }

        impl From<$name<$coord>> for $sys_ty
        {
            fn from(this: $name<$coord>) -> Self {
                Self {
                    $(
                        $field: this.$field.into(),
                    )*
                }
            }
        }

        geometry_structs!(@ $name [$($field: $ty,)*] $($t)*);
    };
}

geometry_structs! {
    #[allow(non_snake_case)]
    /// Represents a 2D transformation matrix.
    /// The matrix is defined as follows:
    ///
    /// | scaleX  skewX transX |
    /// |  skewY scaleY transY |
    /// |  pers0  pers1  pers2 |
    ///
    /// https://github.com/google/skia/blob/3333292a62c14231e67c6aac0940ad1243c0c081/include/core/SkMatrix.h#L162-L185
    struct Transformation<N>  {
        scaleX: N,
        skewX: N,
        transX: N,
        skewY: N,
        scaleY: N,
        transY: N,
        pers0: N,
        pers1: N,
        pers2: N,
    } where {
        f64 => sys::FlutterTransformation
    }
    struct Point<N> {
        x: N,
        y: N,
    } where {
        f64 => sys::FlutterPoint
    }

    struct Size<N> {
        width: N,
        height: N,
    } where {
        f64 => sys::FlutterSize
        u32 => sys::FlutterUIntSize
    }

    struct Rect<N> {
        left: N,
        top: N,
        right: N,
        bottom: N,
    } where {
        f64 => sys::FlutterRect
    }

    struct RoundedRect<N> {
        rect: Rect<N>,
        upper_left_corner_radius: Size<N>,
        upper_right_corner_radius: Size<N>,
        lower_right_corner_radius: Size<N>,
        lower_left_corner_radius: Size<N>,
    } where {
        f64 => sys::FlutterRoundedRect
    }
}

impl<N: Coordinate> Transformation<N> {
    pub fn identity() -> Self {
        Self {
            scaleX: N::one(),
            skewX: N::zero(),
            transX: N::zero(),
            skewY: N::zero(),
            scaleY: N::one(),
            transY: N::zero(),
            pers0: N::zero(),
            pers1: N::zero(),
            pers2: N::one(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlatformViewMutation {
    Opacity(f64),
    ClipRect(Rect<f64>),
    ClipRoundedRect(RoundedRect<f64>),
    Transformation(Transformation<f64>),
}

impl From<PlatformViewMutation> for sys::FlutterPlatformViewMutation {
    fn from(mutation: PlatformViewMutation) -> Self {
        match mutation {
            PlatformViewMutation::Opacity(opacity) => sys::FlutterPlatformViewMutation {
                type_: sys::FlutterPlatformViewMutationType::Opacity,
                __bindgen_anon_1: sys::FlutterPlatformViewMutation__bindgen_ty_1 { opacity },
            },
            PlatformViewMutation::ClipRect(rect) => sys::FlutterPlatformViewMutation {
                type_: sys::FlutterPlatformViewMutationType::ClipRect,
                __bindgen_anon_1: sys::FlutterPlatformViewMutation__bindgen_ty_1 {
                    clip_rect: rect.into(),
                },
            },
            PlatformViewMutation::ClipRoundedRect(rect) => sys::FlutterPlatformViewMutation {
                type_: sys::FlutterPlatformViewMutationType::ClipRoundedRect,
                __bindgen_anon_1: sys::FlutterPlatformViewMutation__bindgen_ty_1 {
                    clip_rounded_rect: rect.into(),
                },
            },
            PlatformViewMutation::Transformation(transformation) => {
                sys::FlutterPlatformViewMutation {
                    type_: sys::FlutterPlatformViewMutationType::Transformation,
                    __bindgen_anon_1: sys::FlutterPlatformViewMutation__bindgen_ty_1 {
                        transformation: transformation.into(),
                    },
                }
            }
        }
    }
}
impl From<sys::FlutterPlatformViewMutation> for PlatformViewMutation {
    fn from(sys: sys::FlutterPlatformViewMutation) -> Self {
        match sys.type_ {
            sys::FlutterPlatformViewMutationType::Opacity => {
                PlatformViewMutation::Opacity(unsafe { sys.__bindgen_anon_1.opacity })
            }
            sys::FlutterPlatformViewMutationType::ClipRect => {
                PlatformViewMutation::ClipRect(unsafe { sys.__bindgen_anon_1.clip_rect }.into())
            }
            sys::FlutterPlatformViewMutationType::ClipRoundedRect => {
                PlatformViewMutation::ClipRoundedRect(
                    unsafe { sys.__bindgen_anon_1.clip_rounded_rect }.into(),
                )
            }
            sys::FlutterPlatformViewMutationType::Transformation => {
                PlatformViewMutation::Transformation(
                    unsafe { sys.__bindgen_anon_1.transformation }.into(),
                )
            }
            _ => unreachable!("Unknown FlutterPlatformViewMutationType; cannot convert it."),
        }
    }
}
