use std::time::Duration;

use crate::{sys, ViewId};

simple_enum! {
    /// The phase of the pointer event.
    pub enum PointerPhase(sys::FlutterPointerPhase) {
        Cancel,
        /// The pointer, which must have been down (see Down), is now up.
        ///
        /// For touch, this means that the pointer is no longer in contact with the
        /// screen. For a mouse, it means the last button was released. Note that if
        /// any other buttons are still pressed when one button is released, that
        /// should be sent as a Move rather than a Up.
        Up,
        /// The pointer, which must have been up, is now down.
        ///
        /// For touch, this means that the pointer has come into contact with the
        /// screen. For a mouse, it means a button is now pressed. Note that if any
        /// other buttons are already pressed when a new button is pressed, that
        /// should be sent as a Move rather than a Down.
        Down,
        /// The pointer moved while down.
        ///
        /// This is also used for changes in button state that don't cause a Down or
        /// Up, such as releasing one of two pressed buttons.
        Move,
        /// The pointer is now sending input to Flutter. For instance, a mouse has
        /// entered the area where the Flutter content is displayed.
        ///
        /// A pointer should always be added before sending any other events.
        Add,
        /// The pointer is no longer sending input to Flutter. For instance, a mouse
        /// has left the area where the Flutter content is displayed.
        ///
        /// A removed pointer should no longer send events until sending a new Add.
        Remove,
        /// The pointer moved while up.
        Hover,
        /// A pan/zoom started on this pointer.
        PanZoomStart,
        /// The pan/zoom updated.
        PanZoomUpdate,
        /// The pan/zoom ended.
        PanZoomEnd,
    }

    pub enum PointerDeviceKind(sys::FlutterPointerDeviceKind) {
        Mouse,
        Touch,
        Stylus,
        Trackpad,
    }

    /// The type of a pointer signal.
    pub enum PointerSignalKind(sys::FlutterPointerSignalKind) {
        None,
        Scroll,
        ScrollInertiaCancel,
        Scale,
    }
}

// these values are taken from this file:
// https://github.com/flutter/engine/blob/1f6312df6d75cdfc72f47cc79acf8d2adb86c922/lib/ui/window/pointer_data.h#L12-L29
bitfield! {
    /// Flags for the `buttons` field of [`PointerEvent`]
    #[derive(Default)]
    pub struct PointerButtons(i64) {
        MousePrimary = 1 << 0,
        MouseSecondary = 1 << 1,
        MouseMiddle = 1 << 2,
        MouseBack = 1 << 3,
        MouseForward = 1 << 4,

        TouchContact = 1 << 0,

        StylusContact = 1 << 0,
        StylusPrimary = 1 << 1,
        StylusSecondary = 1 << 2,
    }
}

impl PointerButtons {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::empty()
    }

    pub fn press(&mut self, button: PointerButtons) {
        self.0 |= button.0;
    }

    pub fn release(&mut self, button: PointerButtons) {
        self.0 &= !button.0;
    }
}

#[allow(dead_code)] // <-- this supresses warnings that sys::FlutterPointerMouseButtons is never used
const _: () = {
    // embedder.h exports a `FlutterPointerMouseButtons` enum, so let's statically assert that the values match
    assert!(PointerButtons::MousePrimary.0 == sys::FlutterPointerMouseButtons::Primary.0 as _);
    assert!(PointerButtons::MouseSecondary.0 == sys::FlutterPointerMouseButtons::Secondary.0 as _);
    assert!(PointerButtons::MouseMiddle.0 == sys::FlutterPointerMouseButtons::Middle.0 as _);
    assert!(PointerButtons::MouseBack.0 == sys::FlutterPointerMouseButtons::Back.0 as _);
    assert!(PointerButtons::MouseForward.0 == sys::FlutterPointerMouseButtons::Forward.0 as _);

    // but it doesn't give us the values for the other device kinds, so there's nothing to assert (though they are correct)
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    /// The identifier of the view that received the pointer event.
    pub view_id: ViewId,
    /// The phase of the pointer event.
    pub phase: PointerPhase,
    /// The timestamp at which the pointer event was generated.
    /// The clock should be the same as that used by [`crate::Engine::get_current_time`].
    pub timestamp: Duration,
    /// The x coordinate of the pointer event in physical pixels.
    pub x: f64,
    /// The y coordinate of the pointer event in physical pixels.
    pub y: f64,
    /// An optional device identifier. If this is not specified, it is assumed
    /// that the embedder has no multi-touch capability.
    pub device: i32,
    pub signal_kind: PointerSignalKind,
    /// The x offset of the scroll in physical pixels.
    pub scroll_delta_x: f64,
    /// The y offset of the scroll in physical pixels.
    pub scroll_delta_y: f64,
    /// The type of the device generating this event.
    //
    // we don't support not setting this field
    // but if we did it'd be Option<FlutterPointerDeviceKind> with device_kind.map_or(0, Into::into)
    // and flutter has a doc comment for such a use case:
    //
    // Backwards compatibility note: If this is not set, the device will be
    // treated as a mouse, with the primary button set for `kDown` and `kMove`.
    // If set explicitly to `kFlutterPointerDeviceKindMouse`, you must set the correct buttons.
    pub device_kind: PointerDeviceKind,
    /// The buttons currently pressed, if any.
    pub buttons: PointerButtons,
    /// The x offset of the pan/zoom in physical pixels.
    pub pan_x: f64,
    /// The y offset of the pan/zoom in physical pixels.
    pub pan_y: f64,
    /// The scale of the pan/zoom, where 1.0 is the initial scale.
    pub scale: f64,
    /// The rotation of the pan/zoom in radians, where 0.0 is the initial angle.
    pub rotation: f64,
}
impl From<PointerEvent> for sys::FlutterPointerEvent {
    fn from(event: PointerEvent) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            phase: event.phase.into(),
            // what?? this overflows at 1.9hrs if usize is 32-bit
            timestamp: event.timestamp.as_micros() as usize,
            x: event.x,
            y: event.y,
            device: event.device,
            signal_kind: event.signal_kind.into(),
            scroll_delta_x: event.scroll_delta_x,
            scroll_delta_y: event.scroll_delta_y,
            device_kind: event.device_kind.into(),
            buttons: event.buttons.0,
            pan_x: event.pan_x,
            pan_y: event.pan_y,
            scale: event.scale,
            rotation: event.rotation,
            view_id: event.view_id.0,
        }
    }
}
