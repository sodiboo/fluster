use crate::{sys, Engine};

pub struct Task {
    runner: sys::FlutterTaskRunner,
    task: u64,
}

impl Task {
    pub fn task(&self) -> u64 {
        self.task
    }
}

impl From<Task> for sys::FlutterTask {
    fn from(task: Task) -> Self {
        sys::FlutterTask {
            runner: task.runner,
            task: task.task,
        }
    }
}

impl Engine {
    /// Inform the engine to run the specified task.
    /// This task has been given to the engine via the [TaskRunnerHandler::post_task].
    /// This call must only be made at the target time specified in that callback.
    /// Running the task before that time is undefined behavior.
    pub fn run_task(&mut self, task: Task) -> crate::Result<()> {
        let task = task.into();
        unsafe { sys::RunTask(self.inner.engine, &raw const task) }.to_result()
    }
}

/// An interface used by the Flutter engine to execute tasks at the target time on a specified thread.
/// There should be a 1-1 relationship between a thread and a task runner.
/// It is undefined behavior to run a task on a thread that is not associated with its task runner.
pub trait TaskRunnerHandler {
    /// May be called from any thread.
    /// Should return true if tasks posted on the calling thread will be run on that same thread.
    fn runs_task_on_current_thread(&mut self) -> bool;

    /// May be called from any thread.
    /// The given task should be executed by the embedder on the thread associated
    /// with that task runner by calling [crate::Engine::run_task] at the given target time.
    /// The system monotonic clock should be used for the target time.
    /// The target time is the absolute time from epoch (NOT a delta) at which the task
    /// must be returned back to the engine on the correct thread.
    /// If the embedder needs to calculate a delta, [crate::Engine::get_current_time]
    /// may be called and the difference used as the delta.
    fn post_task(&mut self, target_time_nanos: u64, task: Task);
}

/// An interface used by the Flutter engine to execute tasks at the target time on a specified thread.
/// There should be a 1-1 relationship between a thread and a task runner.
/// It is undefined behavior to run a task on a thread that is not associated with its task runner.
pub struct TaskRunnerDescription {
    /// A unique identifier for the task runner. If multiple task runners service
    /// tasks on the same thread, their identifiers must match.
    pub identifier: usize,
    pub handler: Box<dyn TaskRunnerHandler>,
}

pub(crate) struct TaskRunnerUserData {
    handler: Box<dyn TaskRunnerHandler>,
}

extern "C" fn runs_task_on_current_thread(user_data: *mut std::ffi::c_void) -> bool {
    let user_data = user_data as *mut TaskRunnerUserData;
    let user_data = unsafe { &mut *user_data };

    user_data.handler.runs_task_on_current_thread()
}

extern "C" fn post_task(
    task: sys::FlutterTask,
    target_time_nanos: u64,
    user_data: *mut std::ffi::c_void,
) {
    let user_data = user_data as *mut TaskRunnerUserData;
    let user_data = unsafe { &mut *user_data };

    user_data.handler.post_task(
        target_time_nanos,
        Task {
            runner: task.runner,
            task: task.task,
        },
    )
}

const _: sys::BoolCallback = Some(runs_task_on_current_thread);
const _: sys::FlutterTaskRunnerPostTaskCallback = Some(post_task);

impl From<TaskRunnerDescription> for (*mut TaskRunnerUserData, sys::FlutterTaskRunnerDescription) {
    fn from(description: TaskRunnerDescription) -> Self {
        let user_data = Box::new(TaskRunnerUserData {
            handler: description.handler,
        });

        let user_data = Box::into_raw(user_data);

        (
            user_data,
            sys::FlutterTaskRunnerDescription {
                struct_size: std::mem::size_of::<sys::FlutterTaskRunnerDescription>(),
                user_data: user_data as *mut std::ffi::c_void,

                runs_task_on_current_thread_callback: Some(runs_task_on_current_thread),
                post_task_callback: Some(post_task),

                identifier: description.identifier,
            },
        )
    }
}

pub struct CustomTaskRunners {
    /// Specify the task runner for the thread on which the `FlutterEngineRun`
    /// call is made. The same task runner description can be specified for both
    /// the render and platform task runners. This makes the Flutter engine use
    /// the same thread for both task runners.
    pub platform_task_runner: TaskRunnerDescription,

    /// Specify the task runner for the thread on which the render tasks will be
    /// run. The same task runner description can be specified for both the render
    /// and platform task runners. This makes the Flutter engine use the same
    /// thread for both task runners.
    pub render_task_runner: TaskRunnerDescription,

    /// Specify a callback that is used to set the thread priority for embedder
    /// task runners.
    pub set_thread_priority: extern "C" fn(sys::FlutterThreadPriority),
}

pub(crate) struct CustomTaskRunnerUserData {
    platform_udata: *mut TaskRunnerUserData,
    platform_desc: *mut sys::FlutterTaskRunnerDescription,
    render_udata: *mut TaskRunnerUserData,
    render_desc: *mut sys::FlutterTaskRunnerDescription,
}

impl Drop for CustomTaskRunnerUserData {
    fn drop(&mut self) {
        unsafe {
            let platform_udata = Box::from_raw(self.platform_udata);
            let platform_desc = Box::from_raw(self.platform_desc);
            let render_udata = Box::from_raw(self.render_udata);
            let render_desc = Box::from_raw(self.render_desc);

            drop(platform_desc);
            drop(render_desc);

            drop(platform_udata);
            drop(render_udata);
        }
    }
}

impl From<CustomTaskRunners> for (CustomTaskRunnerUserData, sys::FlutterCustomTaskRunners) {
    fn from(runners: CustomTaskRunners) -> Self {
        let (platform_udata, platform_desc) = runners.platform_task_runner.into();
        let (render_udata, render_desc) = runners.render_task_runner.into();

        let platform_desc = Box::new(platform_desc);
        let render_desc = Box::new(render_desc);

        let platform_desc = Box::into_raw(platform_desc);
        let render_desc = Box::into_raw(render_desc);

        (
            CustomTaskRunnerUserData {
                platform_udata,
                platform_desc,
                render_udata,
                render_desc,
            },
            sys::FlutterCustomTaskRunners {
                struct_size: std::mem::size_of::<sys::FlutterCustomTaskRunners>(),
                platform_task_runner: platform_desc,
                render_task_runner: render_desc,
                thread_priority_setter: Some(runners.set_thread_priority),
            },
        )
    }
}

simple_enum!(
    pub enum NativeThreadType(sys::FlutterNativeThreadType) {
        /// The Flutter Engine considers the platform thread to be
        /// the thread on which the [Engine::run] call is made.
        /// There is only one such thread per engine instance.
        Platform,
        /// This is the thread the Flutter Engine uses to execute rendering commands
        /// based on the selected client rendering API.
        /// There is only one such thread per engine instance.
        Render,
        /// This is a dedicated thread on which the root Dart isolate is serviced.
        /// There is only one such thread per engine instance.
        UI,
        /// Multiple threads are used by the Flutter engine to perform long running background tasks.
        Worker,
    }
);

impl Engine {
    /// Posts a task onto the Flutter render thread.
    // Typically, this may be called from any thread as long as the specific engine has not already been dropped (shutdown).
    // (but we don't include that line in the doc comment because you can't call this method if the engine is dropped)
    pub fn post_render_thread_task(
        &mut self,
        callback: impl FnOnce() + 'static,
    ) -> crate::Result<()> {
        struct UserData {
            callback: Box<dyn FnOnce()>,
        }

        unsafe extern "C" fn task_callback(user_data: *mut std::ffi::c_void) {
            let user_data = user_data as *mut UserData;
            let user_data = *unsafe { Box::from_raw(user_data) };
            (user_data.callback)();
        }
        const _: sys::VoidCallback = Some(task_callback);

        let user_data = Box::new(UserData {
            callback: Box::new(callback),
        });
        let user_data = Box::into_raw(user_data);

        let result = unsafe {
            sys::PostRenderThreadTask(
                self.inner.engine,
                Some(task_callback),
                user_data as *mut std::ffi::c_void,
            )
        }
        .to_result();

        if result.is_err() {
            let user_data = unsafe { Box::from_raw(user_data) };
            drop(user_data);
        }

        result
    }

    /// Posts a task onto the Flutter render thread.
    // Typically, this may be called from any thread as long as the specific engine has not already been dropped (shutdown).
    // (but we don't include that line in the doc comment because you can't call this method if the engine is dropped)
    // TODO: what the fuck that looks like it causes memory leaks and is not at all threadsafe
    pub fn post_callback_on_all_native_threads(
        &mut self,
        callback: impl Fn(NativeThreadType) + 'static,
    ) -> crate::Result<()> {
        struct UserData {
            callback: Box<dyn Fn(NativeThreadType)>,
        }

        unsafe extern "C" fn thread_callback(
            kind: sys::FlutterNativeThreadType,
            user_data: *mut std::ffi::c_void,
        ) {
            let user_data = user_data as *mut UserData;
            let user_data = unsafe { &*user_data };
            match kind.try_into() {
                Ok(kind) => (user_data.callback)(kind),
                Err(kind) => {
                    eprintln!("Invalid FlutterNativeThreadType: {kind:?}");
                }
            }
        }
        const _: sys::FlutterNativeThreadCallback = Some(thread_callback);

        let user_data = Box::new(UserData {
            callback: Box::new(callback),
        });
        let user_data = Box::into_raw(user_data);

        let result = unsafe {
            sys::PostCallbackOnAllNativeThreads(
                self.inner.engine,
                Some(thread_callback),
                user_data as *mut std::ffi::c_void,
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
