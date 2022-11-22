//! Traits for working with plugin editors.

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::sync::Arc;

use crate::context::gui::GuiContext;

/// An editor for a [`Plugin`][crate::prelude::Plugin].
pub trait Editor: Send {
    /// Create an instance of the plugin's editor and embed it in the parent window. As explained in
    /// [`Plugin::editor()`][crate::prelude::Plugin::editor()], you can then read the parameter
    /// values directly from your [`Params`][crate::prelude::Params] object, and modifying the
    /// values can be done using the functions on the [`ParamSetter`][crate::prelude::ParamSetter].
    /// When you change a parameter value that way it will be broadcasted to the host and also
    /// updated in your [`Params`][crate::prelude::Params] struct.
    ///
    /// This function should return a handle to the editor, which will be dropped when the editor
    /// gets closed. Implement the [`Drop`] trait on the returned handle if you need to explicitly
    /// handle the editor's closing behavior.
    ///
    /// If [`set_scale_factor()`][Self::set_scale_factor()] has been called, then any created
    /// windows should have their sizes multiplied by that factor.
    ///
    /// The wrapper guarantees that a previous handle has been dropped before this function is
    /// called again.
    //
    // TODO: Think of how this would work with the event loop. On Linux the wrapper must provide a
    //       timer using VST3's `IRunLoop` interface, but on Window and macOS the window would
    //       normally register its own timer. Right now we just ignore this because it would
    //       otherwise be basically impossible to have this still be GUI-framework agnostic. Any
    //       callback that deos involve actual GUI operations will still be spooled to the IRunLoop
    //       instance.
    // TODO: This function should return an `Option` instead. Right now window opening failures are
    //       always fatal. This would need to be fixed in baseview first.
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
        request_keyboard_focus: bool,
    ) -> Box<dyn SpawnedWindow + Send>;

    /// Returns the (current) size of the editor in pixels as a `(width, height)` pair. This size
    /// must be reported in _logical pixels_, i.e. the size before being multiplied by the DPI
    /// scaling factor to get the actual physical screen pixels.
    fn size(&self) -> (u32, u32);

    /// Set the DPI scaling factor, if supported. The plugin APIs don't make any guarantees on when
    /// this is called, but for now just assume it will be the first function that gets called
    /// before creating the editor. If this is set, then any windows created by this editor should
    /// have their sizes multiplied by this scaling factor on Windows and Linux.
    ///
    /// Right now this is never called on macOS since DPI scaling is built into the operating system
    /// there.
    // fn set_scale_factor(&self, factor: f32) -> bool;

    /// A callback that will be called whenever the parameter values changed while the editor is
    /// open. You don't need to do anything with this, but this can be used to force a redraw when
    /// the host sends a new value for a parameter or when a parameter change sent to the host gets
    /// processed.
    ///
    /// This function will be called from the **audio thread**. It must thus be lock-free and may
    /// not allocate.
    fn param_values_changed(&self);

    /// Handle key presses.
    fn on_key_down(&self, keyboard_event: &keyboard_types::KeyboardEvent) -> bool;

    /// Handle key releases.
    fn on_key_up(&self,  keyboard_event: &keyboard_types::KeyboardEvent) -> bool;


    // TODO: Reconsider adding a tick function here for the Linux `IRunLoop`. To keep this platform
    //       and API agnostic, add a way to ask the GuiContext if the wrapper already provides a
    //       tick function. If it does not, then the Editor implementation must handle this by
    //       itself. This would also need an associated `PREFERRED_FRAME_RATE` constant.
    // TODO: Host->Plugin resizing
}

pub trait SpawnedWindow {
    fn resize(&self, logical_width: f32, logical_width: f32, scale_factor: f32);
}

/// A raw window handle for platform and GUI framework agnostic editors.
pub struct ParentWindowHandle {
    pub handle: RawWindowHandle,
}

unsafe impl HasRawWindowHandle for ParentWindowHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.handle
    }
}
