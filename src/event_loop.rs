//! An internal event loop for spooling tasks to the/a GUI thread.

use std::sync::Arc;

mod background_thread;

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
mod linux;
// For now, also use the Linux event loop on macOS so it at least compiles
#[cfg(target_os = "macos")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

pub(crate) use self::background_thread::BackgroundThread;

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
pub(crate) use self::linux::LinuxEventLoop as OsEventLoop;
// For now, also use the Linux event loop on macOS so it at least compiles
#[cfg(target_os = "macos")]
pub(crate) use self::linux::LinuxEventLoop as OsEventLoop;
#[cfg(target_os = "windows")]
pub(crate) use self::windows::WindowsEventLoop as OsEventLoop;

pub(crate) const TASK_QUEUE_CAPACITY: usize = 512;

/// A trait describing the functionality of a platform-specific event loop that can execute tasks of
/// type `T` in executor `E` on the operating system's main thread (if applicable). Posting a task
/// to the internal task queue should be realtime-safe. This event loop should be created during the
/// wrapper's initial initialization on the main thread.
///
/// Additionally, this trait also allows posting tasks to a background thread that's completely
/// detached from the GUI. This makes it possible for a plugin to execute long running jobs without
/// blocking GUI rendering.
///
/// This is never used generically, but having this as a trait will cause any missing functions on
/// an implementation to show up as compiler errors even when using a different platform. And since
/// the tasks and executor will be sent to a thread, they need to have static lifetimes.
///
/// TODO: At some point rethink the design to make it possible to have a singleton message queue for
///       all instances of a plugin.
pub(crate) trait EventLoop<T, E>
where
    T: Send + 'static,
    E: MainThreadExecutor<T> + 'static,
{
    /// Create and start a new event loop. The thread this is called on will be designated as the
    /// main thread, so this should be called when constructing the wrapper.
    fn new_and_spawn(executor: Arc<E>) -> Self;

    /// Either post the function to the task queue so it can be delegated to the main thread, or
    /// execute the task directly if this is the main thread. This function needs to be callable at
    /// any time without blocking.
    ///
    /// If the task queue is full, then this will return false.
    #[must_use]
    fn schedule_gui(&self, task: T) -> bool;

    /// Post a task to the background task queue so it can be run in a dedicated background thread
    /// without blocking the plugin's GUI. This function needs to be callable at any time without
    /// blocking.
    ///
    /// If the task queue is full, then this will return false.
    #[must_use]
    fn schedule_background(&self, task: T) -> bool;

    /// Whether the calling thread is the event loop's main thread. This is usually the thread the
    /// event loop instance was initialized on.
    fn is_main_thread(&self) -> bool;
}

/// Something that can execute tasks of type `T`.
pub(crate) trait MainThreadExecutor<T>: Send + Sync {
    /// Execute a task on the current thread. This is either called from the GUI thread or from
    /// another background thread, depending on how the task was scheduled in the [`EventContext`].
    fn execute(&self, task: T, is_gui_thread: bool);
}
