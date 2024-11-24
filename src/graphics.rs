use core::slice;

use crate::{sys, Rect, Size};

#[derive(Debug, Clone, PartialEq)]
pub struct Damage {
    pub damage: Vec<Rect<f64>>,
}

impl Damage {
    fn from_raw(sys: &sys::FlutterDamage) -> Self {
        let damage = unsafe { slice::from_raw_parts(&raw const *sys.damage, sys.num_rects) };
        Self {
            damage: damage.iter().copied().map(Rect::from).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub regions: Vec<Rect<f64>>,
}

impl Region {
    pub fn from_raw(sys: &sys::FlutterRegion) -> Self {
        let regions = unsafe { slice::from_raw_parts(&raw const *sys.rects, sys.rects_count) };
        Self {
            regions: regions.iter().copied().map(Rect::from).collect(),
        }
    }
}

pub struct PresentInfo {
    /// Id of the fbo backing the surface that was presented.
    pub fbo_id: u32,
    /// Damage representing the area that the compositor needs to render.
    pub frame_damage: Damage,
    /// Damage used to set the buffer's damage region.
    pub buffer_damage: Damage,
}
impl PresentInfo {
    pub fn from_raw(raw: &sys::FlutterPresentInfo) -> Self {
        Self {
            fbo_id: raw.fbo_id,
            frame_damage: Damage::from_raw(&raw.frame_damage),
            buffer_damage: Damage::from_raw(&raw.buffer_damage),
        }
    }
}

pub struct FrameInfo {
    size: Size<u32>,
}
impl From<FrameInfo> for sys::FlutterFrameInfo {
    fn from(frame_info: FrameInfo) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            size: frame_info.size.into(),
        }
    }
}
impl From<sys::FlutterFrameInfo> for FrameInfo {
    fn from(frame_info: sys::FlutterFrameInfo) -> Self {
        Self {
            size: frame_info.size.into(),
        }
    }
}
