use std::{
    ffi::{CStr, CString, OsStr},
    mem::ManuallyDrop,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use tracing::error;

use crate::{
    sys, AOTData, Compositor, CompositorUserData, CustomTaskRunnerUserData, CustomTaskRunners,
    KeyEvent, PointerEvent, RendererConfig, RendererUserData, SemanticsUpdate, ViewId,
    WindowMetricsEvent,
};

#[repr(transparent)]
#[derive(Debug, Hash, PartialEq, Eq)] // HashSet?
pub struct VsyncBaton(pub isize);

pub struct PlatformMessageResponse {
    engine: sys::FlutterEngine,
    handle: *const sys::FlutterPlatformMessageResponseHandle,
}

// TODO: is this safe?
unsafe impl Send for PlatformMessageResponse {}

impl PlatformMessageResponse {
    pub fn send(self, response: &[u8]) -> crate::Result<()> {
        let this = ManuallyDrop::new(self);
        unsafe {
            // SAFETY: This is copied to a `std::vector` in the engine before the call returns.
            // https://github.com/flutter/engine/blob/94dac953a95fde7da32cfe092f6350aaa4f93c3d/shell/platform/embedder/embedder.cc#L3005-L3006C11
            // Therefore, we do not need to keep the slice alive.
            sys::SendPlatformMessageResponse(
                this.engine,
                this.handle,
                response.as_ptr(),
                response.len(),
            )
        }
        .to_result()
    }
}

impl Drop for PlatformMessageResponse {
    fn drop(&mut self) {
        error!(
            "PlatformMessageResponse dropped without sending a response. \
            This causes a memory leak."
        );
    }
}

pub trait EngineHandler {
    /// The callback invoked by the engine in order to give the embedder the
    /// chance to respond to platform messages from the Dart application.
    /// The callback will be invoked on the thread on which the `FlutterEngineRun`
    /// call is made.
    ///
    /// The `response` parameter is a callback that must be used to send a response.
    /// If the response is not sent, a memory leak will occur.
    ///
    /// It is presented as a callback to avoid introducing async/await into this library.
    /// This is the only function that requires this pattern.
    /// A more correct API would be `async fn on_platform_message(channel: &CStr, message: &[u8]) -> Vec<u8>`.
    ///
    /// It is not optional to send a response. If there is no meaningful response to send, send an empty response.
    /// Failure to do so will leak memory.
    fn platform_message(
        &mut self,
        channel: &CStr,
        message: &[u8],
        response: PlatformMessageResponse,
    );

    /// A callback that gets invoked by the engine when it attempts to wait for a
    /// platform vsync event. The engine will give the platform a baton that needs
    /// to be returned back to the engine via `FlutterEngineOnVsync`. All batons
    /// must be retured to the engine before initializing a
    /// `FlutterEngineShutdown`. Not doing the same will result in a memory leak.
    /// While the call to `FlutterEngineOnVsync` must occur on the thread that
    /// made the call to `FlutterEngineRun`, the engine will make this callback on
    /// an internal engine-managed thread. If the components accessed on the
    /// embedder are not thread safe, the appropriate re-threading must be done.
    fn vsync(&mut self, baton: VsyncBaton);

    /// The callback invoked by the engine in order to give the embedder the
    /// chance to respond to updates to semantics nodes and custom actions from
    /// the Dart application.
    ///
    /// The callback will be invoked on the thread on which the `FlutterEngineRun`
    /// call is made.
    fn update_semantics(&mut self, update: SemanticsUpdate);

    // Logging callback for Dart application messages.
    //
    // This callback is used by embedder to log print messages from the running
    // Flutter application. This callback is made on an internal engine managed
    // thread and embedders must re-thread if necessary. Performing blocking calls
    // in this callback may introduce application jank.
    fn log_message(&mut self, tag: &CStr, message: &CStr);

    // A callback that is invoked right before the engine is restarted.
    //
    // This optional callback is typically used to reset states to as if the
    // engine has just been started, and usually indicates the user has requested
    // a hot restart (Shift-R in the Flutter CLI.) It is not called the first time
    // the engine starts.
    //
    // The first argument is the `user_data` from `FlutterEngineInitialize`.
    fn on_pre_engine_restart(&mut self);

    /// The callback invoked by the engine in response to a channel listener
    /// being registered on the framework side. The callback is invoked from
    /// a task posted to the platform thread.
    ///
    /// The first parameter is the name of the channel. The second parameter is true if a listener has been set, false if one has been cleared.
    fn channel_update(&mut self, channel: &CStr, listening: bool);

    /// The callback invoked by the engine in root isolate scope.
    /// Called immediately after the root isolate has been created and marked runnable.
    fn root_isolate_created(&mut self);
}

pub(crate) struct InnerEngine {
    pub(crate) engine: sys::FlutterEngine,
    // Actually a Box<EngineUserData>, but kept as a raw pointer.
    // This ensures that it has no mutable references, which would be unsound.
    // It must still be dropped like a Box when the engine is dropped.
    user_data: *mut EngineUserData,
}

impl Drop for InnerEngine {
    fn drop(&mut self) {
        unsafe { sys::Shutdown(self.engine) };
        let user_data = unsafe { Box::from_raw(self.user_data) };
        drop(user_data);
    }
}

#[repr(transparent)]
pub struct Engine {
    pub(crate) inner: InnerEngine,
}

#[repr(transparent)]
pub struct InitializedEngine {
    inner: InnerEngine,
}

mod callbacks {
    use super::*;

    pub extern "C" fn platform_message(
        message: *const sys::FlutterPlatformMessage,
        user_data: *mut std::ffi::c_void,
    ) {
        tracing::trace!("platform_message_callback");
        let user_data = user_data.cast::<EngineUserData>();
        let user_data = unsafe { &mut *user_data };

        let message = unsafe { &*message };

        let channel = unsafe { CStr::from_ptr(message.channel) };
        let message_content =
            unsafe { std::slice::from_raw_parts(message.message, message.message_size) };

        let response = PlatformMessageResponse {
            engine: user_data.engine,
            handle: message.response_handle,
        };

        user_data
            .handler
            .platform_message(channel, message_content, response)
    }

    pub extern "C" fn vsync(user_data: *mut std::ffi::c_void, baton: isize) {
        tracing::trace!("vsync");
        let user_data = user_data.cast::<EngineUserData>();
        let user_data = unsafe { &mut *user_data };

        user_data.handler.vsync(VsyncBaton(baton))
    }

    pub extern "C" fn log_message(
        tag: *const std::os::raw::c_char,
        message: *const std::os::raw::c_char,
        user_data: *mut std::ffi::c_void,
    ) {
        let user_data = user_data.cast::<EngineUserData>();
        let user_data = unsafe { &mut *user_data };

        let tag = unsafe { CStr::from_ptr(tag) };
        let message = unsafe { CStr::from_ptr(message) };

        user_data.handler.log_message(tag, message)
    }

    pub extern "C" fn on_pre_engine_restart(user_data: *mut std::ffi::c_void) {
        let user_data = user_data.cast::<EngineUserData>();
        let user_data = unsafe { &mut *user_data };

        user_data.handler.on_pre_engine_restart()
    }

    pub extern "C" fn update_semantics(
        update: *const sys::FlutterSemanticsUpdate2,
        user_data: *mut std::ffi::c_void,
    ) {
        let user_data = user_data.cast::<EngineUserData>();
        let user_data = unsafe { &mut *user_data };

        let update = unsafe { &*update };

        user_data
            .handler
            .update_semantics(SemanticsUpdate::from_raw(update))
    }

    pub extern "C" fn channel_update_callback(
        channel_update: *const sys::FlutterChannelUpdate,
        user_data: *mut std::ffi::c_void,
    ) {
        let user_data = user_data.cast::<EngineUserData>();
        let user_data = unsafe { &mut *user_data };

        let channel_update = unsafe { &*channel_update };

        let channel = unsafe { CStr::from_ptr(channel_update.channel) };
        let listening = channel_update.listening;

        user_data.handler.channel_update(channel, listening)
    }

    pub extern "C" fn root_isolate_create(user_data: *mut std::ffi::c_void) {
        let user_data = user_data.cast::<EngineUserData>();
        let user_data = unsafe { &mut *user_data };

        user_data.handler.root_isolate_created();
    }

    /// This block type-checks the callbacks above. They must be the same type as defined in `embedder.h`.
    const _: sys::FlutterPlatformMessageCallback = Some(platform_message);
    const _: sys::VsyncCallback = Some(vsync);
    const _: sys::FlutterLogMessageCallback = Some(log_message);
    const _: sys::VoidCallback = Some(on_pre_engine_restart);
    const _: sys::FlutterUpdateSemanticsCallback2 = Some(update_semantics);
    const _: sys::FlutterChannelUpdateCallback = Some(channel_update_callback);
    const _: sys::VoidCallback = Some(root_isolate_create);
}

pub(crate) struct EngineUserData {
    engine: sys::FlutterEngine,
    pub(crate) renderer_user_data: RendererUserData,

    // Option<(Box<_>, Box<_>)>
    custom_task_runners: Option<(
        *mut CustomTaskRunnerUserData,
        *mut sys::FlutterCustomTaskRunners,
    )>,
    compositor: Option<(*mut CompositorUserData, *mut sys::FlutterCompositor)>,
    #[allow(dead_code)] // no custom drop glue, but must be kept alive.
    aot_data: Option<Arc<AOTData>>,

    handler: Box<dyn EngineHandler>,
}

impl Drop for EngineUserData {
    fn drop(&mut self) {
        if let Some((compositor_user_data, compositor)) = self.compositor {
            let compositor = unsafe { Box::from_raw(compositor) };
            drop(compositor);
            let compositor_user_data = unsafe { Box::from_raw(compositor_user_data) };
            drop(compositor_user_data);
        }

        if let Some((custom_task_runner_user_data, custom_task_runners)) = self.custom_task_runners
        {
            let custom_task_runners = unsafe { Box::from_raw(custom_task_runners) };
            drop(custom_task_runners);
            let custom_task_runner_user_data =
                unsafe { Box::from_raw(custom_task_runner_user_data) };
            drop(custom_task_runner_user_data);
        }
    }
}

pub struct ProjectArgs<'a> {
    /// The path to the Flutter assets directory containing project assets.
    pub assets_path: &'a Path,
    /// The path to the `icudtl.dat` file for the project.
    pub icu_data_path: &'a Path,
    /// The command line arguments used to initialize the project.
    ///
    /// @attention     The first item in the command line (if specified at all) is
    ///                interpreted as the executable name. So if an engine flag
    ///                needs to be passed into the same, it needs to not be the
    ///                very first item in the list.
    ///
    /// The set of engine flags are only meant to control
    /// unstable features in the engine. Deployed applications should not pass any
    /// command line arguments at all as they may affect engine stability at
    /// runtime in the presence of un-sanitized input. The list of currently
    /// recognized engine flags and their descriptions can be retrieved from the
    /// `switches.h` engine source file.
    pub command_line_argv: &'a [&'a OsStr],

    /// Path to a directory used to store data that is cached across runs of a
    /// Flutter application (such as compiled shader programs used by Skia).
    /// This is optional.  The string must be NULL terminated.
    ///
    // This is different from the cache-path-dir argument defined in switches.h,
    // which is used in `flutter::Settings` as `temp_directory_path`.
    pub persistent_cache_path: Option<PathBuf>,

    /// If true, the engine would only read the existing cache, but not write new
    /// ones.
    pub is_persistent_cache_read_only: bool,

    /// The name of a custom Dart entrypoint. This is optional and specifying a
    /// null or empty entrypoint makes the engine look for a method named "main"
    /// in the root library of the application.
    ///
    /// Care must be taken to ensure that the custom entrypoint is not tree-shaken
    /// away. Usually, this is done using the `@pragma('vm:entry-point')`
    /// decoration.
    pub custom_dart_entrypoint: Option<&'a str>,

    /// Typically the Flutter engine create and manages its internal threads.
    /// This optional argument allows for the specification of task runner
    /// interfaces to event loops managed by the embedder on threads it creates.
    pub custom_task_runners: Option<CustomTaskRunners>,

    /// All `FlutterEngine` instances in the process share the same Dart VM. When
    /// the first engine is launched, it starts the Dart VM as well. It used to be
    /// the case that it was not possible to shutdown the Dart VM cleanly and
    /// start it back up in the process in a safe manner. This issue has since
    /// been patched. Unfortunately, applications already began to make use of the
    /// fact that shutting down the Flutter engine instance left a running VM in
    /// the process. Since a Flutter engine could be launched on any thread,
    /// applications would "warm up" the VM on another thread by launching
    /// an engine with no isolates and then shutting it down immediately. The main
    /// Flutter application could then be started on the main thread without
    /// having to incur the Dart VM startup costs at that time. With the new
    /// behavior, this "optimization" immediately becomes massive performance
    /// pessimization as the VM would be started up in the "warm up" phase, shut
    /// down there and then started again on the main thread. Changing this
    /// behavior was deemed to be an unacceptable breaking change. Embedders that
    /// wish to shutdown the Dart VM when the last engine is terminated in the
    /// process should opt into this behavior by setting this flag to true.
    pub shutdown_dart_vm_when_done: bool,

    /// Typically, Flutter renders the layer hierarchy into a single root surface.
    /// However, when embedders need to interleave their own contents within the
    /// Flutter layer hierarchy, their applications can push platform views within
    /// the Flutter scene. This is done using the `SceneBuilder.addPlatformView`
    /// call. When this happens, the Flutter rasterizer divides the effective view
    /// hierarchy into multiple layers. Each layer gets its own backing store and
    /// Flutter renders into the same. Once the layers contents have been
    /// fulfilled, the embedder is asked to composite these layers on-screen. At
    /// this point, it can interleave its own contents within the effective
    /// hierarchy. The interface for the specification of these layer backing
    /// stores and the hooks to listen for the composition of layers on-screen can
    /// be controlled using this field. This field is completely optional. In its
    /// absence, platforms views in the scene are ignored and Flutter renders to
    /// the root surface as normal.
    pub compositor: Option<Compositor>,

    /// The command line arguments passed through to the Dart entrypoint.
    pub dart_entrypoint_argv: &'a [&'a str],

    // A tag string associated with application log messages.
    //
    // A log message tag string that can be used convey application, subsystem,
    // or component name to embedder's logger. This string will be passed to to
    // callbacks on `log_message_callback`. Defaults to "flutter" if unspecified.
    pub log_tag: CString,

    /// Max size of the old gen heap for the Dart VM in MB, or 0 for unlimited, -1
    /// for default value.
    ///
    /// See also:
    /// <https://github.com/dart-lang/sdk/blob/ca64509108b3e7219c50d6c52877c85ab6a35ff2/runtime/vm/flag_list.h#L150>
    pub dart_old_gen_heap_size: i64,

    /// The AOT data to be used in AOT operation.
    ///
    /// The AOT data can be created with [`AOTData::new`], and will be released when the object is dropped.
    /// The engine holds an `Arc` to the data, so it will not be dropped until the engine is dropped.
    /// If you pass `shutdown_dart_vm_when_done: true`, the AOT data will **not** be dropped when the engine is dropped.
    /// In fact, it won't *ever* be dropped, because the Dart VM will not shut down. It will cause a memory leak.
    pub aot_data: Option<Arc<AOTData>>,

    pub handler: Box<dyn EngineHandler>,

    /// A callback that computes the locale the platform would natively resolve
    /// to.
    ///
    /// Unfortunately, this callback receives no `user_data` parameter, so it cannot be implemented as a method on `EngineHandler`.
    /// This also means it cannot be wrapped in a safe manner. You must deal with raw pointers and unsafe code.
    /// You can also just set it to `None` if you don't need it.
    ///
    /// The input parameter is an array of [`sys::FlutterLocale`]s which represent the
    /// locales supported by the app. One of the input supported locales should
    /// be selected and returned to best match with the user/device's preferred
    /// locale. The implementation should produce a result that as closely
    /// matches what the platform would natively resolve to as possible.
    pub compute_platform_resolved_locale: sys::FlutterComputePlatformResolvedLocaleCallback,
}

// impl InitializedEngine {
//     pub fn run(self) -> crate::Result<Engine> {
//         unsafe { sys::FlutterEngineRunInitialized(self.inner.engine) }
//             .to_result()
//             .map(|()| Engine { inner: self.inner })
//     }
// }

impl Engine {
    pub fn run(
        renderer_config: impl Into<RendererConfig>,
        project_args: ProjectArgs,
    ) -> crate::Result<Self> {
        Self::_run(renderer_config.into(), project_args)
    }
    fn _run(renderer_config: RendererConfig, project_args: ProjectArgs) -> crate::Result<Self> {
        let (renderer_user_data, raw_renderer_config) = renderer_config.into();

        let compositor = project_args.compositor.map(|compositor| {
            let (compositor_user_data, compositor) = compositor.into();
            (compositor_user_data, Box::into_raw(Box::new(compositor)))
        });

        let custom_task_runners = project_args.custom_task_runners.map(|compositor| {
            let (compositor_user_data, compositor) = compositor.into();
            (
                Box::into_raw(Box::new(compositor_user_data)),
                Box::into_raw(Box::new(compositor)),
            )
        });

        // Intentionally cause a memory leak.
        // If the Dart VM is not shut down when the engine is, it is unsafe to drop the AOT data, ever.
        if project_args.shutdown_dart_vm_when_done {
            if let Some(ref aot_data) = project_args.aot_data {
                let _ = Arc::into_raw(aot_data.clone());
            }
        }

        let user_data = Box::new(EngineUserData {
            engine: std::ptr::null_mut(),
            renderer_user_data,
            compositor,
            custom_task_runners,
            aot_data: project_args.aot_data.clone(),
            handler: project_args.handler,
        });

        let compositor = compositor.map(|(_, c)| c);
        let custom_task_runners = custom_task_runners.map(|(_, c)| c);

        let assets_path = CString::new(project_args.assets_path.as_os_str().as_bytes())
            .expect("assets_path must be valid C string");
        let icu_data_path = CString::new(project_args.icu_data_path.as_os_str().as_bytes())
            .expect("icu_data_path must be valid C string");
        let persistent_cache_path = project_args.persistent_cache_path.map(|p| {
            CString::new(p.as_os_str().as_bytes())
                .expect("persistent_cache_path must be valid C string")
        });

        let custom_dart_entrypoint = project_args.custom_dart_entrypoint.as_ref().map(|s| {
            CString::new(s.as_bytes()).expect("custom_dart_entrypoint must be valid C string")
        });

        let dart_entrypoint_argv = project_args
            .dart_entrypoint_argv
            .iter()
            .map(|arg| {
                CString::new(arg.as_bytes()).expect("dart_entrypoint_argv contain valid C strings")
            })
            .collect::<Box<_>>();

        let command_line_argv = project_args
            .command_line_argv
            .iter()
            .map(|arg| {
                CString::new(arg.as_bytes()).expect("command_line_argv contain valid C strings")
            })
            .collect::<Box<_>>();

        // these go through an extra step because [CString] is not [*const c_char]
        // so we need to map them to [*const c_char] first, which points to the previously created CStrings
        // and that's easier than manually reclaiming the memory with CString::{into,from}_raw
        // because they can be dropped at the end of this scope. we're not using them after this.
        // it's not stored in a struct field, so it's okay to do that.

        let dart_entrypoint_argv = dart_entrypoint_argv
            .iter()
            .map(|arg| arg.as_ptr())
            .collect::<Box<_>>();

        let command_line_argv = command_line_argv
            .iter()
            .map(|arg| arg.as_ptr())
            .collect::<Box<_>>();

        let raw_project_args = sys::FlutterProjectArgs {
            struct_size: std::mem::size_of::<sys::FlutterProjectArgs>(),

            // these are required
            assets_path: assets_path.as_ptr(),
            icu_data_path: icu_data_path.as_ptr(),

            // this one is optional
            persistent_cache_path: persistent_cache_path
                .as_deref()
                .map_or_else(std::ptr::null, CStr::as_ptr),
            is_persistent_cache_read_only: project_args.is_persistent_cache_read_only,

            // the compositor is optional, so we set it to null if it doesn't exist
            compositor: compositor.unwrap_or_else(std::ptr::null_mut),
            // likewise, custom_task_runners is optional :3
            custom_task_runners: custom_task_runners.unwrap_or_else(std::ptr::null_mut),
            // and this AOT data wasn't allocated by us, so freeing it happens entirely differently
            // and that's why we extract the pointer here, instead of preparing it earlier
            aot_data: project_args
                .aot_data
                .as_deref()
                .map_or_else(std::ptr::null_mut, |aot| aot.data),

            // just dart vm params
            shutdown_dart_vm_when_done: project_args.shutdown_dart_vm_when_done,
            dart_old_gen_heap_size: project_args.dart_old_gen_heap_size,

            // these callbacks must be just dumb function pointers, so we keep Rust closures in the user data
            platform_message_callback: Some(callbacks::platform_message),
            vsync_callback: Some(callbacks::vsync),
            update_semantics_callback2: Some(callbacks::update_semantics),
            log_message_callback: Some(callbacks::log_message),
            on_pre_engine_restart_callback: Some(callbacks::on_pre_engine_restart),
            channel_update_callback: Some(callbacks::channel_update_callback),
            root_isolate_create_callback: Some(callbacks::root_isolate_create),

            // this callback is fucking stupid and doesn't take user data. but i will allow it to be used, still.
            compute_platform_resolved_locale_callback: project_args
                .compute_platform_resolved_locale,

            // these are all optional
            custom_dart_entrypoint: custom_dart_entrypoint
                .as_ref()
                .map_or_else(std::ptr::null, |s| s.as_ptr()),

            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                reason = "can't do anything about it"
            )]
            dart_entrypoint_argc: dart_entrypoint_argv.len() as std::ffi::c_int,
            dart_entrypoint_argv: dart_entrypoint_argv.as_ptr(),

            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                reason = "can't do anything about it"
            )]
            command_line_argc: command_line_argv.len() as std::ffi::c_int,
            command_line_argv: command_line_argv.as_ptr(),

            log_tag: project_args.log_tag.as_ptr(),

            // deprecated fields
            main_path__unused__: std::ptr::null(),
            packages_path__unused__: std::ptr::null(),
            update_semantics_node_callback: None,
            update_semantics_custom_action_callback: None,
            update_semantics_callback: None,

            // these are not necessarily deprecated, but they are *all* replaced by `aot_data`
            // and are mutually exclusive with it, so we never pass them ever.
            vm_snapshot_data: std::ptr::null(),
            vm_snapshot_data_size: 0,
            vm_snapshot_instructions: std::ptr::null(),
            vm_snapshot_instructions_size: 0,
            isolate_snapshot_data: std::ptr::null(),
            isolate_snapshot_data_size: 0,
            isolate_snapshot_instructions: std::ptr::null(),
            isolate_snapshot_instructions_size: 0,
        };

        // FlutterEngine* is just a pointer to a pointer, so we set the inner pointer to null
        // this effectively gets around annoying rust rules for uninit values because ptr::null_mut() is initialized
        // if it weren't publicly a pointer, we would have to use MaybeUninit
        let mut engine: sys::FlutterEngine = std::ptr::null_mut();

        let user_data = Box::into_raw(user_data);

        unsafe {
            sys::Run(
                sys::FLUTTER_ENGINE_VERSION,
                &raw const raw_renderer_config,
                &raw const raw_project_args,
                user_data.cast::<std::ffi::c_void>(),
                &raw mut engine,
            )
        }
        .to_result()
        .map(|()| {
            let inner = InnerEngine { engine, user_data };
            Self { inner }
        })
    }

    /// Adds a view.
    ///
    /// This is an asynchronous operation.
    /// The view should not be used until the `callback` is invoked with a value of true.
    /// The embedder should prepare resources in advance but be ready to clean up on failure.
    ///
    /// A frame is scheduled if the operation succeeds.
    ///
    /// The callback is invoked on a thread managed by the engine.
    /// The embedder should re-thread if needed.
    ///
    /// Attempting to add the implicit view will fail and will return
    /// [`crate::Error::InvalidArguments`]. Attempting to add a view with an already
    /// existing view ID will fail, and `callback` will be invoked with a value of false.
    ///
    /// Returns the result of *starting* the asynchronous operation.
    /// If [`Ok()`], the `callback` will be invoked.
    pub fn add_view(
        &mut self,
        view_id: ViewId,
        view_metrics: WindowMetricsEvent,
        callback: impl FnOnce(bool) + 'static,
    ) -> crate::Result<()> {
        struct UserData {
            callback: Box<dyn FnOnce(bool)>,
        }

        extern "C" fn add_view_callback(result: *const sys::FlutterAddViewResult) {
            let result = unsafe { &*result };
            let user_data = *unsafe { Box::from_raw(result.user_data.cast::<UserData>()) };
            (user_data.callback)(result.added);
        }

        const _: sys::FlutterAddViewCallback = Some(add_view_callback);

        let user_data = Box::new(UserData {
            callback: Box::new(callback),
        });

        let user_data = Box::into_raw(user_data);

        let view_metrics = view_metrics.into();

        let info = sys::FlutterAddViewInfo {
            struct_size: std::mem::size_of::<sys::FlutterAddViewInfo>(),
            view_id: view_id.0,
            view_metrics: &raw const view_metrics,
            user_data: user_data.cast::<std::ffi::c_void>(),
            add_view_callback: Some(add_view_callback),
        };

        let result = unsafe { sys::AddView(self.inner.engine, &raw const info) }.to_result();

        if result.is_err() {
            // the callback will never be invoked
            let user_data = unsafe { Box::from_raw(user_data) };
            drop(user_data);
        }

        result
    }

    /// Removes a view.
    ///
    /// This is an asynchronous operation. The view's resources must not
    /// be cleaned up until `callback` is invoked with a value of true.
    ///
    /// The callback is invoked on a thread managed by the engine.
    /// The embedder should re-thread if needed.
    ///
    /// Attempting to remove the implicit view will fail and will return
    /// [`crate::Error::InvalidArguments`]. Attempting to remove a view with a
    /// non-existent view ID will fail, and `callback` will be invoked with a value of false.
    ///
    /// Returns the result of *starting* the asynchronous operation.
    /// If [`Ok()`], the `callback` will be invoked.
    pub fn remove_view(
        &mut self,
        view_id: ViewId,
        callback: impl FnOnce(bool) + Send + 'static,
    ) -> crate::Result<()> {
        struct UserData {
            callback: Box<dyn FnOnce(bool)>,
        }

        extern "C" fn remove_view_callback(result: *const sys::FlutterRemoveViewResult) {
            let result = unsafe { &*result };
            let user_data = *unsafe { Box::from_raw(result.user_data.cast::<UserData>()) };
            (user_data.callback)(result.removed);
        }

        const _: sys::FlutterRemoveViewCallback = Some(remove_view_callback);

        let user_data = Box::new(UserData {
            callback: Box::new(callback),
        });

        let user_data = Box::into_raw(user_data);

        let info = sys::FlutterRemoveViewInfo {
            struct_size: std::mem::size_of::<sys::FlutterAddViewInfo>(),
            view_id: view_id.0,
            user_data: user_data.cast::<std::ffi::c_void>(),
            remove_view_callback: Some(remove_view_callback),
        };

        let result = unsafe { sys::RemoveView(self.inner.engine, &raw const info) }.to_result();

        if result.is_err() {
            // the callback will never be invoked
            let user_data = unsafe { Box::from_raw(user_data) };
            drop(user_data);
        }

        result
    }

    pub fn send_window_metrics_event(&mut self, event: WindowMetricsEvent) -> crate::Result<()> {
        let event = event.into();

        unsafe { sys::SendWindowMetricsEvent(self.inner.engine, &raw const event) }.to_result()
    }

    pub fn send_pointer_event(&mut self, events: &[PointerEvent]) -> crate::Result<()> {
        let events: Box<[sys::FlutterPointerEvent]> =
            events.iter().copied().map(Into::into).collect();

        unsafe { sys::SendPointerEvent(self.inner.engine, events.as_ptr(), events.len()) }
            .to_result()
    }

    /// Sends a key event to the engine. The framework will decide
    /// whether to handle this event in a synchronous fashion, although
    /// due to technical limitation, the result is always reported
    /// asynchronously. The `callback` is guaranteed to be called
    /// exactly once, if and only if this function returns [`Ok()`].
    ///
    /// The callback invoked by the engine when the Flutter application
    /// has decided whether it handles this event.
    pub fn send_key_event(
        &mut self,
        event: KeyEvent,
        callback: impl FnOnce(bool) + 'static,
    ) -> crate::Result<()> {
        struct UserData {
            callback: Box<dyn FnOnce(bool)>,
        }

        extern "C" fn key_event_callback(handled: bool, user_data: *mut std::ffi::c_void) {
            let user_data = *unsafe { Box::from_raw(user_data.cast::<UserData>()) };
            (user_data.callback)(handled);
        }

        const _: sys::FlutterKeyEventCallback = Some(key_event_callback);

        let user_data = Box::new(UserData {
            callback: Box::new(callback),
        });

        let user_data = Box::into_raw(user_data);

        let (character, key_event) = event.into();

        let result = unsafe {
            sys::SendKeyEvent(
                self.inner.engine,
                &raw const key_event,
                Some(key_event_callback),
                user_data.cast::<std::ffi::c_void>(),
            )
        }
        .to_result();

        if let Some(character) = character {
            // this CString is allocated in the conversion
            // and it's cloned into the engine, so we should drop it now
            let character = unsafe { CString::from_raw(character) };
            drop(character);
        }

        if result.is_err() {
            // the callback will never be invoked
            let user_data = unsafe { Box::from_raw(user_data) };
            drop(user_data);
        }

        result
    }

    pub fn send_platform_message(
        &mut self,
        channel: &CStr,
        message: &[u8],
        response: impl FnOnce(&[u8]) + 'static,
    ) -> crate::Result<()> {
        struct UserData {
            engine: sys::FlutterEngine,
            response: *mut sys::FlutterPlatformMessageResponseHandle,

            #[allow(clippy::type_complexity)] // not a complex type
            callback: Option<Box<dyn FnOnce(&[u8])>>,
        }

        impl Drop for UserData {
            fn drop(&mut self) {
                unsafe { sys::PlatformMessageReleaseResponseHandle(self.engine, self.response) }
                    .to_result()
                    .expect("releasing response handle never fails")
            }
        }

        extern "C" fn message_response(
            data: *const u8,
            size: usize,
            user_data: *mut std::ffi::c_void,
        ) {
            let mut user_data = *unsafe { Box::from_raw(user_data.cast::<UserData>()) };
            let data = unsafe { std::slice::from_raw_parts(data, size) };
            let callback = user_data
                .callback
                .take()
                .expect("callback is only called once");
            callback(data);
        }

        const _: sys::FlutterDataCallback = Some(message_response);

        let user_data = Box::new(UserData {
            engine: self.inner.engine,
            response: std::ptr::null_mut(),
            callback: Some(Box::new(response)),
        });

        let user_data = Box::into_raw(user_data);

        let mut response_handle: *mut sys::FlutterPlatformMessageResponseHandle =
            std::ptr::null_mut();

        if let Err(err) = unsafe {
            sys::PlatformMessageCreateResponseHandle(
                self.inner.engine,
                Some(message_response),
                user_data.cast::<std::ffi::c_void>(),
                &raw mut response_handle,
            )
        }
        .to_result()
        {
            // the callback will never be invoked
            let user_data = unsafe { Box::from_raw(user_data) };
            drop(user_data);
            return Err(err);
        }

        let message = sys::FlutterPlatformMessage {
            struct_size: std::mem::size_of::<sys::FlutterPlatformMessage>(),
            channel: channel.as_ptr(),
            message: message.as_ptr(),
            message_size: message.len(),
            response_handle,
        };

        unsafe { sys::SendPlatformMessage(self.inner.engine, &raw const message) }.to_result()
    }

    /// Notify the engine that a vsync event occurred.
    /// A baton passed to the platform via the vsync callback must be returned.
    /// This call must be made on the thread on which the call to [`Engine::run`] was made.
    ///
    /// Frame timepoints are in nanoseconds.
    ///
    /// The system monotonic clock is used as the timebase.
    ///
    /// `frame_start_time_nanos` is the point at which the vsync event occurred or will occur.
    /// If the time point is in the future, the engine will wait till that point to begin its frame workload.
    ///
    /// `frame_target_time_nanos` is the point at which the embedder anticipates the next vsync to occur.
    /// This is a hint the engine uses to schedule Dart VM garbage collection in periods in which
    /// the various threads are most likely to be idle.
    /// For example, for a 60Hz display, embedders should add 16.6 * 1e6 to the frame time field.
    #[allow(clippy::needless_pass_by_value)] // intentional to enforce the type semantics
    pub fn on_vsync(
        &mut self,
        baton: VsyncBaton,
        frame_start_time: Duration,
        frame_target_time: Duration,
    ) -> crate::Result<()> {
        unsafe {
            #[allow(clippy::cast_possible_truncation)] // that's just how the API do be
            sys::OnVsync(
                self.inner.engine,
                baton.0,
                // intentional to enforce the type semantics
                frame_start_time.as_nanos() as u64,
                frame_target_time.as_nanos() as u64,
            )
        }
        .to_result()
    }

    /// Reloads the system fonts in the engine.
    pub fn reload_system_fonts(&mut self) -> crate::Result<()> {
        unsafe { sys::ReloadSystemFonts(self.inner.engine) }.to_result()
    }

    /// Get the current time in nanoseconds from the clock used by the flutter engine.
    /// This is the system monotonic clock.
    #[must_use]
    pub fn get_current_time() -> Duration {
        Duration::from_nanos(unsafe { sys::GetCurrentTime() })
    }

    /// Register an external texture with a unique (per engine) identifier.
    /// Only rendering backends that support external textures accept external texture registrations.
    /// After the external texture is registered,
    /// the application can mark that a frame is available by calling [`Self::mark_external_texture_frame_available`].
    ///
    /// The parameter is the identifier of the texture to register  with the engine.
    /// The embedder may supply new frames to this texture using the same identifier.
    pub fn register_external_texture(&mut self, texture_identifier: i64) -> crate::Result<()> {
        unsafe { sys::RegisterExternalTexture(self.inner.engine, texture_identifier) }.to_result()
    }

    /// Unregister a previous texture registration.
    ///
    /// The parameter is the identifier of the texture for which new frame will not be available
    pub fn unregister_external_texture(&mut self, texture_identifier: i64) -> crate::Result<()> {
        unsafe { sys::UnregisterExternalTexture(self.inner.engine, texture_identifier) }.to_result()
    }

    /// Mark that a new texture frame is available for a given texture identifier.
    pub fn mark_external_texture_frame_available(
        &mut self,
        texture_identifier: i64,
    ) -> crate::Result<()> {
        unsafe { sys::MarkExternalTextureFrameAvailable(self.inner.engine, texture_identifier) }
            .to_result()
    }

    /// Posts a low memory notification to a running engine instance.
    /// The engine will do its best to release non-critical resources in response.
    /// It is not guaranteed that the resource would have been collected by the time this call returns.
    /// The notification is posted to engine subsystems that may be operating on other threads.
    ///
    /// Flutter applications can respond to these notifications by setting
    /// `WidgetsBindingObserver.didHaveMemoryPressure` observers.
    ///
    /// Returns if the low memory notification was sent to the running engine instance.
    ///
    /// Hint: combine this with something like <https://crates.io/crates/psi>
    pub fn notify_low_memory_warning(&mut self) -> crate::Result<()> {
        unsafe { sys::NotifyLowMemoryWarning(self.inner.engine) }.to_result()
    }

    /// Schedule a new frame to redraw the content.
    pub fn schedule_frame(&mut self) -> crate::Result<()> {
        unsafe { sys::ScheduleFrame(self.inner.engine) }.to_result()
    }

    /// Schedule a callback to be called after the next frame is drawn.
    /// This must be called from the platform thread.
    /// The callback is executed only once from the raster thread; embedders must re-thread if necessary.
    /// Performing blocking calls in this callback may introduce application jank.
    pub fn set_next_frame_callback(
        &mut self,
        callback: impl FnOnce() + 'static,
    ) -> crate::Result<()> {
        struct UserData {
            callback: Box<dyn FnOnce()>,
        }

        unsafe extern "C" fn next_frame_callback(user_data: *mut std::ffi::c_void) {
            let user_data = user_data.cast::<UserData>();
            let user_data = *unsafe { Box::from_raw(user_data) };
            (user_data.callback)();
        }
        const _: sys::VoidCallback = Some(next_frame_callback);

        let user_data = Box::new(UserData {
            callback: Box::new(callback),
        });
        let user_data = Box::into_raw(user_data);

        let result = unsafe {
            sys::SetNextFrameCallback(
                self.inner.engine,
                Some(next_frame_callback),
                user_data.cast::<std::ffi::c_void>(),
            )
        }
        .to_result();

        if result.is_err() {
            let user_data = unsafe { Box::from_raw(user_data) };
            drop(user_data);
        }

        result
    }
}

#[allow(path_statements)]
pub const _: () = {
    sys::Initialize;
    sys::RunInitialized;
    sys::Deinitialize;
};
