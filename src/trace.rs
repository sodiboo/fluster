use std::ffi::CStr;

use crate::sys;

/// A profiling utility. Logs a trace duration begin event to the timeline.
/// If the timeline is unavailable or disabled, this has no effect.
/// Must be balanced with a duration end event
/// (via [`self::event_duration_end`]) with the same name on the same thread.
///
/// Can be called on any thread.
///
/// Strings passed into the function will NOT be copied when added to the timeline.
/// Therefore, only string literals may be passed in.
pub fn event_duration_begin(name: &'static CStr) {
    unsafe { sys::TraceEventDurationBegin(name.as_ptr()) }
}

/// A profiling utility. Logs a trace duration end event to the timeline.
/// If the timeline is unavailable or disabled, this has no effect.
/// This call must be preceded by a trace duration begin call
/// (via [`self::event_duration_begin`]) with the same name on the same thread.
///
/// Can be called on any thread.
///
/// Strings passed into the function will NOT be copied when added to the timeline.
/// Therefore, only string literals may be passed in.
pub fn event_duration_end(name: &'static CStr) {
    unsafe { sys::TraceEventDurationEnd(name.as_ptr()) }
}

/// A profiling utility. Logs a trace duration instant event to the timeline.
/// If the timeline is unavailable or disabled, this has no effect.
/// Can be called on any thread.
///
/// Strings passed into the function will NOT be copied when added to the timeline.
/// Therefore, only string literals may be passed in.
pub fn event_instant(name: &'static CStr) {
    unsafe { sys::TraceEventInstant(name.as_ptr()) }
}

/// A scope that logs a trace duration event to the timeline.
/// In [`Self::new`], a duration begin event is logged.
/// When it is dropped, a duration end event is logged.
pub struct DurationScope {
    name: &'static CStr,
}

impl DurationScope {
    #[must_use = "Must be bound to a variable to ensure the duration end event is logged"]
    pub fn new(name: &'static CStr) -> Self {
        event_duration_begin(name);
        Self { name }
    }
}

impl Drop for DurationScope {
    fn drop(&mut self) {
        event_duration_end(self.name);
    }
}
