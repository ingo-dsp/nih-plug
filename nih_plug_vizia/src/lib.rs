//! [VIZIA](https://github.com/vizia/vizia) editor support for NIH plug.

// See the comment in the main `nih_plug` crate
#![allow(clippy::type_complexity)]

use baseview::{WindowHandle, WindowScalePolicy};
use crossbeam::atomic::AtomicCell;
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{Editor, GuiContext, ParentWindowHandle};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vizia::prelude::*;

// Re-export for convenience
pub use vizia;

pub mod assets;
pub mod widgets;

/// Create an [`Editor`] instance using a [`vizia`][::vizia] GUI. The [`ViziaState`] passed to this
/// function contains the GUI's intitial size, and this is kept in sync whenever the GUI gets
/// resized. You can also use this to know if the GUI is open, so you can avoid performing
/// potentially expensive calculations while the GUI is not open. If you want this size to be
/// persisted when restoring a plugin instance, then you can store it in a `#[persist = "key"]`
/// field on your parameters struct.
///
/// The [`GuiContext`] is also passed to the app function. This is only meant for saving and
/// restoring state as part of your plugin's preset handling. You should not interact with this
/// directly to set parameters. Use the `ParamEvent`s instead.
///
/// See [VIZIA](https://github.com/vizia/vizia)'s repository for examples on how to use this.
pub fn create_vizia_editor<F>(vizia_state: Arc<ViziaState>, app: F) -> Option<Box<dyn Editor>>
where
    F: Fn(&mut Context, Arc<dyn GuiContext>) + 'static + Send + Sync,
{
    Some(Box::new(ViziaEditor {
        vizia_state,
        app: Arc::new(app),
        apply_theming: true,

        // TODO: We can't get the size of the window when baseview does its own scaling, so if the
        //       host does not set a scale factor on Windows or Linux we should just use a factor of
        //       1. That may make the GUI tiny but it also prevents it from getting cut off.
        #[cfg(target_os = "macos")]
        scaling_factor: AtomicCell::new(None),
        #[cfg(not(target_os = "macos"))]
        scaling_factor: AtomicCell::new(Some(1.0)),
    }))
}

/// The same as [`create_vizia_editor()`] but without changing VIZIA's default styling and font.
/// This also won't register the styling for any of the widgets that come with `nih_plug_vizia`, or
/// register the custom fonts. Event handlers for the [`ParamEvent`][widgets::ParamEvent]s are still
/// set up when using this function instead of [`create_vizia_editor()`].
pub fn create_vizia_editor_without_theme<F>(
    vizia_state: Arc<ViziaState>,
    app: F,
) -> Option<Box<dyn Editor>>
where
    F: Fn(&mut Context, Arc<dyn GuiContext>) + 'static + Send + Sync,
{
    Some(Box::new(ViziaEditor {
        vizia_state,
        app: Arc::new(app),
        apply_theming: false,

        #[cfg(target_os = "macos")]
        scaling_factor: AtomicCell::new(None),
        #[cfg(not(target_os = "macos"))]
        scaling_factor: AtomicCell::new(Some(1.0)),
    }))
}

/// State for an `nih_plug_vizia` editor. The scale factor can be manipulated at runtime by changing
/// `cx.user_scale_factor`.
#[derive(Serialize, Deserialize)]
pub struct ViziaState {
    /// The window's size in logical pixels before applying `scale_factor`.
    #[serde(with = "nih_plug::params::persist::serialize_atomic_cell")]
    size: AtomicCell<(u32, u32)>,
    /// A scale factor that should be applied to `size` separate from from any system HiDPI scaling.
    /// This can be used to allow GUIs to be scaled uniformly.
    #[serde(with = "nih_plug::params::persist::serialize_atomic_cell")]
    scale_factor: AtomicCell<f64>,
    /// Whether the editor's window is currently open.
    #[serde(skip)]
    open: AtomicBool,
}

impl<'a> PersistentField<'a, ViziaState> for Arc<ViziaState> {
    fn set(&self, new_value: ViziaState) {
        self.size.store(new_value.size.load());
        self.scale_factor.store(new_value.scale_factor.load());
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&ViziaState) -> R,
    {
        f(self)
    }
}

impl ViziaState {
    /// Initialize the GUI's state. This value can be passed to [`create_vizia_editor()`]. The
    /// window size is in logical pixels, so before it is multiplied by the DPI scaling factor.
    pub fn from_size(width: u32, height: u32) -> Arc<ViziaState> {
        Arc::new(ViziaState {
            size: AtomicCell::new((width, height)),
            scale_factor: AtomicCell::new(1.0),
            open: AtomicBool::new(false),
        })
    }

    /// The same as [`from_size()`][Self::from_size()], but with a separate initial scale factor.
    /// This scale factor gets applied on top of any HiDPI scaling, and it can be modified at
    /// runtime by changing `cx.user_scale_factor`.
    pub fn from_size_with_scale(width: u32, height: u32, scale_factor: f64) -> Arc<ViziaState> {
        Arc::new(ViziaState {
            size: AtomicCell::new((width, height)),
            scale_factor: AtomicCell::new(scale_factor),
            open: AtomicBool::new(false),
        })
    }

    /// Returns a `(width, height)` pair for the current size of the GUI in logical pixels, after
    /// applying the user scale factor.
    pub fn scaled_logical_size(&self) -> (u32, u32) {
        let (logical_width, logical_height) = self.size.load();
        let scale_factor = self.scale_factor.load();

        (
            (logical_width as f64 * scale_factor).round() as u32,
            (logical_height as f64 * scale_factor).round() as u32,
        )
    }

    /// Returns a `(width, height)` pair for the current size of the GUI in logical pixels before
    /// applying the user scale factor.
    pub fn inner_logical_size(&self) -> (u32, u32) {
        self.size.load()
    }

    /// Get the non-DPI related uniform scaling factor the GUI's size will be multiplied with. This
    /// can be changed by changing `cx.user_scale_factor`.
    pub fn user_scale_factor(&self) -> f64 {
        self.scale_factor.load()
    }

    /// Whether the GUI is currently visible.
    // Called `is_open()` instead of `open()` to avoid the ambiguity.
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }
}

/// An [`Editor`] implementation that calls an vizia draw loop.
struct ViziaEditor {
    vizia_state: Arc<ViziaState>,
    /// The user's app function.
    app: Arc<dyn Fn(&mut Context, Arc<dyn GuiContext>) + 'static + Send + Sync>,
    /// Whether to apply `nih_plug_vizia`'s default theme. If this is disabled, then only the event
    /// handler for `ParamEvent`s is set up.
    apply_theming: bool,

    /// The scaling factor reported by the host, if any. On macOS this will never be set and we
    /// should use the system scaling factor instead.
    scaling_factor: AtomicCell<Option<f32>>,
}

impl Editor for ViziaEditor {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let app = self.app.clone();
        let vizia_state = self.vizia_state.clone();
        let apply_theming = self.apply_theming;

        let (unscaled_width, unscaled_height) = vizia_state.inner_logical_size();
        let system_scaling_factor = self.scaling_factor.load();
        let user_scale_factor = vizia_state.user_scale_factor();

        let window = Application::new(move |cx| {
            // Set some default styles to match the iced integration
            if apply_theming {
                // NOTE: vizia's font rendering looks way too dark and thick. Going one font weight
                //       lower seems to compensate for this.
                assets::register_fonts(cx);
                cx.set_default_font(assets::NOTO_SANS_LIGHT);
                cx.add_theme(include_str!("../assets/theme.css"));

                // There doesn't seem to be any way to bundle styles with a widget, so we'll always
                // include the style sheet for our custom widgets at context creation
                widgets::register_theme(cx);
            }

            // Any widget can change the parameters by emitting `ParamEvent` events. This model will
            // handle them automatically.
            widgets::ParamModel {
                context: context.clone(),
            }
            .build(cx);

            // And we'll link `WindowEvent::ResizeWindow` and `WindowEvent::SetScale` events to our
            // `ViziaState`. We'll notify the host when any of these change.
            widgets::WindowModel {
                context: context.clone(),
                vizia_state: vizia_state.clone(),
            }
            .build(cx);

            app(cx, context.clone())
        })
        .with_scale_policy(
            system_scaling_factor
                .map(|factor| WindowScalePolicy::ScaleFactor(factor as f64))
                .unwrap_or(WindowScalePolicy::SystemScaleFactor),
        )
        .inner_size((unscaled_width, unscaled_height))
        .user_scale_factor(user_scale_factor)
        .open_parented(&parent);

        self.vizia_state.open.store(true, Ordering::Release);
        Box::new(ViziaEditorHandle {
            vizia_state: self.vizia_state.clone(),
            window,
        })
    }

    fn size(&self) -> (u32, u32) {
        // This includes the user scale factor if set, but not any HiDPI scaling
        self.vizia_state.scaled_logical_size()
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        // We're making things a bit more complicated by having both a system scale factor, which is
        // used for HiDPI and also known to the host, and a user scale factor that the user can use
        // to arbitrarily resize the GUI
        self.scaling_factor.store(Some(factor));
        true
    }

    fn param_values_changed(&self) {
        // TODO: Update the GUI when this happens, right now this happens automatically as a result
        //       of of the reactivity
    }
}

/// The window handle used for [`ViziaEditor`].
struct ViziaEditorHandle {
    vizia_state: Arc<ViziaState>,
    window: WindowHandle,
}

/// The window handle enum stored within 'WindowHandle' contains raw pointers. Is there a way around
/// having this requirement?
unsafe impl Send for ViziaEditorHandle {}

impl Drop for ViziaEditorHandle {
    fn drop(&mut self) {
        self.vizia_state.open.store(false, Ordering::Release);
        // XXX: This should automatically happen when the handle gets dropped, but apparently not
        self.window.close();
    }
}
