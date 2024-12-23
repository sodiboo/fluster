use crate::sys;

macro_rules! gen {
    (
        $(
            fn $fn:ident($($arg:ident: $arg_ty:ty),* $(,)?) $(-> $ret:ty)?;
        )*
        $(@ $($t:tt)*)?
    ) => {
        #[allow(non_snake_case, clippy::missing_safety_doc)]
        pub trait FlutterProcTable {
            $(
                unsafe fn $fn(&self, $($arg: $arg_ty),*) $(-> $ret)?;
            )*
        }

        pub struct StaticProcTable;

        impl FlutterProcTable for StaticProcTable {
            $(
                unsafe fn $fn(&self, $($arg: $arg_ty),*) $(-> $ret)? {
                    unsafe { sys::$fn($($arg),*) }
                }
            )*
        }

        #[allow(non_snake_case)]
        pub struct DynamicProcTable {
            $(
                pub $fn: unsafe extern "C" fn($($arg_ty),*) $(-> $ret)?,
            )*
        }

        impl FlutterProcTable for DynamicProcTable {
            $(
                unsafe fn $fn(&self, $($arg: $arg_ty),*) $(-> $ret)? {
                    unsafe { (self.$fn)($($arg),*) }
                }
            )*
        }

        impl DynamicProcTable {
            #[allow(clippy::missing_safety_doc, non_snake_case)]
            pub unsafe fn with_dynamic(
                GetProcAddresses: unsafe extern "C" fn(
                    table_out: *mut sys::FlutterEngineProcTable,
                ) -> sys::FlutterEngineResult,
            ) -> crate::Result<Self> {
                let mut table: sys::FlutterEngineProcTable = unsafe { std::mem::zeroed() };
                table.struct_size = std::mem::size_of::<sys::FlutterEngineProcTable>();
                unsafe { GetProcAddresses(&mut table) }
                    .to_result()
                    .map(|()| {
                        $(let $fn = table.$fn.expect(concat!("missing proc table entry for ", stringify!($fn)));)*

                        Self { $($fn),* }
                    })
            }
        }

        impl Default for DynamicProcTable {
            fn default() -> Self {
                unsafe { Self::with_dynamic(sys::GetProcAddresses) }.expect("failed to get proc table")
            }
        }

        impl From<StaticProcTable> for DynamicProcTable {
            fn from(StaticProcTable: StaticProcTable) -> Self {
                Self {
                    $(
                        $fn: sys::$fn,
                    )*
                }
            }
        }

        // This is essentially just here to check the exhaustiveness of the table.
        impl From<DynamicProcTable> for sys::FlutterEngineProcTable {
            fn from(table: DynamicProcTable) -> Self {
                Self {
                    struct_size: std::mem::size_of::<sys::FlutterEngineProcTable>(),
                    $(
                        $fn: Some(table.$fn),
                    )*
                }
            }
        }
    }
}

gen! {
    fn CreateAOTData(
        source: *const sys::FlutterEngineAOTDataSource,
        data_out: *mut sys::FlutterEngineAOTData,
    ) -> sys::FlutterEngineResult;
    fn CollectAOTData(data: sys::FlutterEngineAOTData) -> sys::FlutterEngineResult;
    fn Run(
        version: usize,
        config: *const sys::FlutterRendererConfig,
        args: *const sys::FlutterProjectArgs,
        user_data: *mut ::std::os::raw::c_void,
        engine_out: *mut sys::FlutterEngine,
    ) -> sys::FlutterEngineResult;
    fn Shutdown(engine: sys::FlutterEngine) -> sys::FlutterEngineResult;
    fn Initialize(
        version: usize,
        config: *const sys::FlutterRendererConfig,
        args: *const sys::FlutterProjectArgs,
        user_data: *mut ::std::os::raw::c_void,
        engine_out: *mut sys::FlutterEngine,
    ) -> sys::FlutterEngineResult;
    fn Deinitialize(engine: sys::FlutterEngine) -> sys::FlutterEngineResult;
    fn RunInitialized(engine: sys::FlutterEngine) -> sys::FlutterEngineResult;
    fn AddView(engine: sys::FlutterEngine, info: *const sys::FlutterAddViewInfo) -> sys::FlutterEngineResult;
    fn RemoveView(
        engine: sys::FlutterEngine,
        info: *const sys::FlutterRemoveViewInfo,
    ) -> sys::FlutterEngineResult;   fn SendWindowMetricsEvent(
        engine: sys::FlutterEngine,
        event: *const sys::FlutterWindowMetricsEvent,
    ) -> sys::FlutterEngineResult;   fn SendPointerEvent(
        engine: sys::FlutterEngine,
        events: *const sys::FlutterPointerEvent,
        events_count: usize,
    ) -> sys::FlutterEngineResult;
    fn SendKeyEvent(
        engine: sys::FlutterEngine,
        event: *const sys::FlutterKeyEvent,
        callback: sys::FlutterKeyEventCallback,
        user_data: *mut ::std::os::raw::c_void,
    ) -> sys::FlutterEngineResult;   fn SendPlatformMessage(
        engine: sys::FlutterEngine,
        message: *const sys::FlutterPlatformMessage,
    ) -> sys::FlutterEngineResult;
    fn PlatformMessageCreateResponseHandle(
        engine: sys::FlutterEngine,
        data_callback: sys::FlutterDataCallback,
        user_data: *mut ::std::os::raw::c_void,
        response_out: *mut *mut sys::FlutterPlatformMessageResponseHandle,
    ) -> sys::FlutterEngineResult;
    fn PlatformMessageReleaseResponseHandle(
        engine: sys::FlutterEngine,
        response: *mut sys::FlutterPlatformMessageResponseHandle,
    ) -> sys::FlutterEngineResult;
    fn SendPlatformMessageResponse(
        engine: sys::FlutterEngine,
        handle: *const sys::FlutterPlatformMessageResponseHandle,
        data: *const u8,
        data_length: usize,
    ) -> sys::FlutterEngineResult;
    fn RegisterExternalTexture(
        engine: sys::FlutterEngine,
        texture_identifier: i64,
    ) -> sys::FlutterEngineResult;
    fn UnregisterExternalTexture(
        engine: sys::FlutterEngine,
        texture_identifier: i64,
    ) -> sys::FlutterEngineResult;
    fn MarkExternalTextureFrameAvailable(
        engine: sys::FlutterEngine,
        texture_identifier: i64,
    ) -> sys::FlutterEngineResult;
    fn UpdateSemanticsEnabled(engine: sys::FlutterEngine, enabled: bool) -> sys::FlutterEngineResult;
    fn UpdateAccessibilityFeatures(
        engine: sys::FlutterEngine,
        features: sys::FlutterAccessibilityFeature,
    ) -> sys::FlutterEngineResult;
    fn DispatchSemanticsAction(
        engine: sys::FlutterEngine,
        node_id: u64,
        action: sys::FlutterSemanticsAction,
        data: *const u8,
        data_length: usize,
    ) -> sys::FlutterEngineResult;
    fn OnVsync(
        engine: sys::FlutterEngine,
        baton: isize,
        frame_start_time_nanos: u64,
        frame_target_time_nanos: u64,
    ) -> sys::FlutterEngineResult;
    fn ReloadSystemFonts(engine: sys::FlutterEngine) -> sys::FlutterEngineResult;
    fn TraceEventDurationBegin(name: *const ::std::os::raw::c_char);
    fn TraceEventDurationEnd(name: *const ::std::os::raw::c_char);
    fn TraceEventInstant(name: *const ::std::os::raw::c_char);
    fn PostRenderThreadTask(
        engine: sys::FlutterEngine,
        callback: sys::VoidCallback,
        callback_data: *mut ::std::os::raw::c_void,
    ) -> sys::FlutterEngineResult;
    fn GetCurrentTime() -> u64;
    fn RunTask(engine: sys::FlutterEngine, task: *const sys::FlutterTask) -> sys::FlutterEngineResult;
    fn UpdateLocales(
        engine: sys::FlutterEngine,
        locales: *mut *const sys::FlutterLocale,
        locales_count: usize,
    ) -> sys::FlutterEngineResult;
    fn RunsAOTCompiledDartCode() -> bool;
    fn PostDartObject(
        engine: sys::FlutterEngine,
        port: sys::FlutterEngineDartPort,
        object: *const sys::FlutterEngineDartObject,
    ) -> sys::FlutterEngineResult;
    fn NotifyLowMemoryWarning(engine: sys::FlutterEngine) -> sys::FlutterEngineResult;
    fn PostCallbackOnAllNativeThreads(
        engine: sys::FlutterEngine,
        callback: sys::FlutterNativeThreadCallback,
        user_data: *mut ::std::os::raw::c_void,
    ) -> sys::FlutterEngineResult;
    fn NotifyDisplayUpdate(
        engine: sys::FlutterEngine,
        update_type: sys::FlutterEngineDisplaysUpdateType,
        displays: *const sys::FlutterEngineDisplay,
        display_count: usize,
    ) -> sys::FlutterEngineResult;
    fn ScheduleFrame(engine: sys::FlutterEngine) -> sys::FlutterEngineResult;
    fn SetNextFrameCallback(
        engine: sys::FlutterEngine,
        callback: sys::VoidCallback,
        user_data: *mut ::std::os::raw::c_void,
    ) -> sys::FlutterEngineResult;
}
