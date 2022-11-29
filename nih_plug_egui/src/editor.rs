//! An [`Editor`] implementation for egui.

use baseview::gl::GlConfig;
use baseview::{Size, WindowHandle, WindowOpenOptions};
use std::ops::DerefMut;
use egui::Context;
use egui_baseview::{EguiWindow, translate_virtual_key_code};
use egui_baseview::window::{EguiKeyboardInput, translate_modifiers};
use keyboard_types::Code;
use nih_plug::editor::SpawnedWindow;
use nih_plug::prelude::{Editor, GuiContext, ParamSetter, ParentWindowHandle};
use parking_lot::RwLock;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::ops::Deref;


use crate::EguiState;

/// An [`Editor`] implementation that calls an egui draw loop.
pub(crate) struct EguiEditor<T> {
    pub(crate) egui_state: Arc<EguiState>,
    /// The plugin's state. This is kept in between editor openenings.
    pub(crate) user_state: Arc<RwLock<T>>,

    /// The user's build function. Applied once at the start of the application.
    pub(crate) build: Arc<dyn Fn(&Context, &mut T) + 'static + Send + Sync>,
    /// The user's update function.
    pub(crate) update: Arc<dyn Fn(&Context, &ParamSetter, &mut T) + 'static + Send + Sync>,

    /// The scaling factor reported by the host, if any. On macOS this will never be set and we
    /// should use the system scaling factor instead.
    //pub(crate) scaling_factor: AtomicCell<Option<f32>>,

    pub(crate) plugin_keyboard_events: Arc<Mutex<Vec<EguiKeyboardInput>>>,

    pub(crate) clipboard_ctx:  Arc<Mutex<Option<copypasta::ClipboardContext>>>,
}

impl<T> Editor for EguiEditor<T>
where
    T: 'static + Send + Sync,
{
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
        _request_keyboard_focus: bool
    ) -> Box<dyn SpawnedWindow + Send> {
        let build = self.build.clone();
        let update = self.update.clone();
        let state = self.user_state.clone();
        let plugin_keyboard_events = self.plugin_keyboard_events.clone();

        let (physical_width, physical_height) = self.egui_state.size();
        let window = EguiWindow::open_parented(
            &parent,
            WindowOpenOptions {
                title: String::from("egui window"),
                size: Size::new(physical_width as f64, physical_height as f64),

                #[cfg(feature = "opengl")]
                gl_config: Some(GlConfig {
                    version: (3, 2),
                    red_bits: 8,
                    blue_bits: 8,
                    green_bits: 8,
                    alpha_bits: 8,
                    depth_bits: 24,
                    stencil_bits: 8,
                    samples: None,
                    srgb: true,
                    double_buffer: true,
                    vsync: true,
                    ..Default::default()
                }),
            },
            state,
            move |egui_ctx, _queue, state| build(egui_ctx, &mut state.write()),
            move |egui_ctx, _queue, state| {
                if let Ok(mut plugin_keyboard_events) = plugin_keyboard_events.try_lock() {
                    let mut events = vec![];
                    std::mem::swap(&mut *plugin_keyboard_events, &mut events);
                    for event in events.into_iter() {
                        let mut input_mut = egui_ctx.input_mut();
                        event.apply_on_input(input_mut.deref_mut());
                    }
                }

                let setter = ParamSetter::new(context.as_ref());

                // For now, just always redraw. Most plugin GUIs have meters, and those almost always
                // need a redraw. Later we can try to be a bit more sophisticated about this. Without
                // this we would also have a blank GUI when it gets first opened because most DAWs open
                // their GUI while the window is still unmapped.
                egui_ctx.request_repaint();
                (update)(egui_ctx, &setter, &mut state.write());
            },
        );

        // if request_keyboard_focus {
        //     window.request_keyboard_focus();
        // }

        self.egui_state.open.store(true, Ordering::Release);
        Box::new(EguiEditorHandle {
            egui_state: self.egui_state.clone(),
            window,
        })
    }

    fn size(&self) -> (u32, u32) {
        self.egui_state.size()
    }

    fn param_values_changed(&self) {
        // As mentioned above, for now we'll always force a redraw to allow meter widgets to work
        // correctly. In the future we can use an `Arc<AtomicBool>` and only force a redraw when
        // that boolean is set.
    }

    fn on_key_down(&self, keyboard_event: &keyboard_types::KeyboardEvent) -> bool {
        assert_eq!(keyboard_event.state, keyboard_types::KeyState::Down);
        self.handle_keyboard_event(keyboard_event)
    }

    fn on_key_up(&self, keyboard_event: &keyboard_types::KeyboardEvent) -> bool {
        assert_eq!(keyboard_event.state, keyboard_types::KeyState::Up);
        self.handle_keyboard_event(keyboard_event)
    }
}

impl<T> EguiEditor<T> where T: 'static + Send + Sync {
    fn handle_keyboard_event(&self, keyboard_event: &keyboard_types::KeyboardEvent) -> bool {
        let is_modifier_key = {
            match keyboard_event.code {
                Code::ShiftLeft | Code::ShiftRight |
                Code::ControlLeft | Code::ControlRight |
                Code::AltLeft | Code::AltRight |
                Code::MetaLeft | Code::MetaRight => true,
                _ => false,
            }
        };
        let translated_mods = translate_modifiers(&keyboard_event.modifiers);
        let is_acceptable_key = is_modifier_key || { // always accept modifiers, because we need to keep track of which are pressed.
            let acceptable_keys = self.egui_state.acceptable_keys.try_lock().map(|x| x.deref().clone());
            let acceptable_keys = acceptable_keys.unwrap_or_default();
            if let Some(translated_key) = translate_virtual_key_code(keyboard_event.code) {
                acceptable_keys.accepts(translated_mods, &translated_key)
            } else {
                acceptable_keys.accepts_all()
            }
        };
        if is_acceptable_key {
            if let Ok(mut plugin_keyboard_events) = self.plugin_keyboard_events.try_lock() {
                if let Ok(mut clipboard_ctx) = self.clipboard_ctx.try_lock() {
                    plugin_keyboard_events.push(EguiKeyboardInput::from_keyboard_event(keyboard_event, clipboard_ctx.as_mut()));
                    return true;
                }
            }
        }
        return false;
    }
}

/// The window handle used for [`EguiEditor`].
struct EguiEditorHandle {
    egui_state: Arc<EguiState>,
    window: WindowHandle,
}

impl SpawnedWindow for EguiEditorHandle {
    fn resize(&self, physical_width: f32, physical_height: f32, _ignored_host_reported_scale_factor: f32) {

        // TODO: Should we somehow honor the host-reported-scale-factor?

        // store new size in egui_state
        self.egui_state.size.store((physical_width as u32, physical_height as u32));

        // resize spawned window
        let physical_size = baseview::Size {
            width: physical_width as f64,
            height: physical_height as f64,
        };
        self.window.resize(physical_size);
    }
}
/// The window handle enum stored within 'WindowHandle' contains raw pointers. Is there a way around
/// having this requirement?
unsafe impl Send for EguiEditorHandle {}

impl Drop for EguiEditorHandle {
    fn drop(&mut self) {
        self.egui_state.open.store(false, Ordering::Release);
        // XXX: This should automatically happen when the handle gets dropped, but apparently not
        self.window.close();
    }
}
