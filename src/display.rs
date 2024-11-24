use crate::{sys, Engine};

simple_enum! {
    pub enum DisplaysUpdateType(sys::FlutterEngineDisplaysUpdateType) {
        /// `FlutterEngineDisplay`s that were active during start-up. A display is
        /// considered active if:
        ///     1. The frame buffer hardware is connected.
        ///     2. The display is drawable, e.g. it isn't being mirrored from another connected display or sleeping.
        Startup,
        Count,
    }
}

pub struct Display {
    pub display_id: sys::FlutterEngineDisplayId,

    /// This is set to true if the embedder only has one display.
    /// In cases where this is set to true, the value of display_id is ignored.
    /// In cases where this is not set to true, it is expected that a valid display_id be provided.
    pub single_display: bool,

    /// This represents the refresh period in frames per second.
    /// This value may be zero if the device is not running or unavailable or unknown.
    pub refresh_rate: f64,

    /// The width of the display, in physical pixels.
    pub width: usize,

    /// The height of the display, in physical pixels.
    pub height: usize,

    /// The pixel ratio of the display, which is used to convert physical pixels to logical pixels.
    pub device_pixel_ratio: f64,
}

impl From<&Display> for sys::FlutterEngineDisplay {
    fn from(display: &Display) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            display_id: display.display_id,
            single_display: display.single_display,
            refresh_rate: display.refresh_rate,
            width: display.width,
            height: display.height,
            device_pixel_ratio: display.device_pixel_ratio,
        }
    }
}

impl Engine {
    /// Posts updates corresponding to display changes to a running engine instance.
    ///
    /// There must be at least one display in the list of displays.
    pub fn notify_display_update(
        &mut self,
        update_type: DisplaysUpdateType,
        displays: &[Display],
    ) -> crate::Result<()> {
        let displays: Box<[sys::FlutterEngineDisplay]> =
            displays.iter().map(|display| display.into()).collect();

        unsafe {
            sys::NotifyDisplayUpdate(
                self.inner.engine,
                update_type.into(),
                displays.as_ptr(),
                displays.len(),
            )
        }
        .to_result()
    }
}
