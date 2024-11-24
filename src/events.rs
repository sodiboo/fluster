use std::ffi::CString;

use crate::{sys, ViewId};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowMetricsEvent {
    /// The view that this event is describing.
    pub view_id: ViewId,
    /// Physical width of the window.
    pub width: usize,
    /// Physical height of the window.
    pub height: usize,
    /// Scale factor for the physical screen.
    pub pixel_ratio: f64,
    /// Horizontal physical location of the left side of the window on the screen.
    pub left: usize,
    /// Vertical physical location of the top of the window on the screen.
    pub top: usize,
    /// Top inset of window.
    pub physical_view_inset_top: f64,
    /// Right inset of window.
    pub physical_view_inset_right: f64,
    /// Bottom inset of window.
    pub physical_view_inset_bottom: f64,
    /// Left inset of window.
    pub physical_view_inset_left: f64,
    /// The identifier of the display the view is rendering on.
    pub display_id: sys::FlutterEngineDisplayId,
}
impl From<WindowMetricsEvent> for sys::FlutterWindowMetricsEvent {
    fn from(event: WindowMetricsEvent) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>(),
            view_id: event.view_id.0,
            width: event.width,
            height: event.height,
            pixel_ratio: event.pixel_ratio,
            left: event.left,
            top: event.top,
            physical_view_inset_top: event.physical_view_inset_top,
            physical_view_inset_right: event.physical_view_inset_right,
            physical_view_inset_bottom: event.physical_view_inset_bottom,
            physical_view_inset_left: event.physical_view_inset_left,
            display_id: event.display_id,
        }
    }
}

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

bitfield! {
    /// Flags for the `buttons` field of `FlutterPointerEvent` when `device_kind`
    /// is [FlutterPointerDeviceKind::Mouse].
    pub struct PointerMouseButtons(sys::FlutterPointerMouseButtons) {
        Primary,
        Secondary,
        Middle,
        Back,
        Forward,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    /// The identifier of the view that received the pointer event.
    pub view_id: ViewId,
    /// The phase of the pointer event.
    pub phase: PointerPhase,
    /// The timestamp at which the pointer event was generated. The timestamp
    /// should be specified in microseconds and the clock should be the same as
    /// that used by `FlutterEngineGetCurrentTime`.
    pub timestamp: usize,
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
    pub buttons: i64,
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
            timestamp: event.timestamp,
            x: event.x,
            y: event.y,
            device: event.device,
            signal_kind: event.signal_kind.into(),
            scroll_delta_x: event.scroll_delta_x,
            scroll_delta_y: event.scroll_delta_y,
            device_kind: event.device_kind.into(),
            buttons: event.buttons,
            pan_x: event.pan_x,
            pan_y: event.pan_y,
            scale: event.scale,
            rotation: event.rotation,
            view_id: event.view_id.0,
        }
    }
}

simple_enum! {
    pub enum KeyPhase(sys::FlutterKeyEventType) {
        Up,
        Down,
        Repeat,
    }

    pub enum KeyEventDeviceType(sys::FlutterKeyEventDeviceType) {
        Keyboard,
        DirectionalPad,
        Gamepad,
        Joystick,
        Hdmi,
    }
}

/// A structure to represent a key event.
///
/// Sending `FlutterKeyEvent` via `FlutterEngineSendKeyEvent` results in a
/// corresponding `FlutterKeyEvent` to be dispatched in the framework. It is
/// embedder's responsibility to ensure the regularity of sent events, since the
/// framework only performs simple one-to-one mapping. The events must conform
/// the following rules:
///
///  * Each key press sequence shall consist of one key down event (`kind` being
///    `kFlutterKeyEventTypeDown`), zero or more repeat events, and one key up
///    event, representing a physical key button being pressed, held, and
///    released.
///  * All events throughout a key press sequence shall have the same `physical`
///    and `logical`. Having different `character`s is allowed.
///
/// A `FlutterKeyEvent` with `physical` 0 and `logical` 0 is an empty event.
/// This is the only case either `physical` or `logical` can be 0. An empty
/// event must be sent if a key message should be converted to no
/// `FlutterKeyEvent`s, for example, when a key down message is received for a
/// key that has already been pressed according to the record. This is to ensure
/// some `FlutterKeyEvent` arrives at the framework before raw key message.
/// See https://github.com/flutter/flutter/issues/87230.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyEvent {
    /// The timestamp at which the key event was generated. The timestamp should
    /// be specified in microseconds and the clock should be the same as that used
    /// by `FlutterEngineGetCurrentTime`.
    pub timestamp: f64,
    /// The phase of this key.
    // KeyEventType called KeyPhase in this library because it matches PointerPhase
    // and `type: KeyEventType` is a reserved keyword in Rust.
    pub phase: KeyPhase,
    /// The USB HID code for the physical key of the event.
    ///
    /// For the full definition and list of pre-defined physical keys, see
    /// `PhysicalKeyboardKey` from the framework.
    ///
    /// The only case that `physical` might be 0 is when this is an empty event.
    /// See `FlutterKeyEvent` for introduction.
    pub physical: u64,
    /// The key ID for the logical key of this event.
    ///
    /// For the full definition and a list of pre-defined logical keys, see
    /// `LogicalKeyboardKey` from the framework.
    ///
    /// The only case that `logical` might be 0 is when this is an empty event.
    /// See `FlutterKeyEvent` for introduction.
    pub logical: u64,
    /// Character input from the event. Can be [None]. Ignored for [KeyEventKind::Up].
    pub character: Option<CString>,
    /// True if this event does not correspond to a native event.
    ///
    /// The embedder is likely to skip events and/or construct new events that do
    /// not correspond to any native events in order to conform the regularity
    /// of events (as documented in `FlutterKeyEvent`). An example is when a key
    /// up is missed due to loss of window focus, on a platform that provides
    /// query to key pressing status, the embedder might realize that the key has
    /// been released at the next key event, and should construct a synthesized up
    /// event immediately before the actual event.
    ///
    /// An event being synthesized means that the `timestamp` might greatly
    /// deviate from the actual time when the event occurs physically.
    pub synthesized: bool,
    /// The source device for the key event.
    pub device_type: KeyEventDeviceType,
}
impl From<KeyEvent> for (Option<*mut std::ffi::c_char>, sys::FlutterKeyEvent) {
    fn from(event: KeyEvent) -> Self {
        let character = event.character.map(CString::into_raw);
        (
            character,
            sys::FlutterKeyEvent {
                struct_size: std::mem::size_of::<sys::FlutterKeyEvent>(),
                timestamp: event.timestamp,
                type_: event.phase.into(),
                physical: event.physical,
                logical: event.logical,
                character: character.unwrap_or_else(std::ptr::null_mut),
                synthesized: event.synthesized,
                device_type: event.device_type.into(),
            },
        )
    }
}
