//! [egui](https://github.com/emilk/egui) editor support for NIH plug.
//!
//! TODO: Proper usage example, for now check out the gain_gui example

// See the comment in the main `nih_plug` crate
#![allow(clippy::type_complexity)]

use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use baseview::gl::GlConfig;
use baseview::{Size, WindowHandle, WindowOpenOptions, WindowScalePolicy};
use crossbeam::atomic::AtomicCell;
use egui::{ClipboardData, ClipboardMime, Context, Event, Key, Modifiers, RawInput, Vec2};
use egui_baseview::{EguiWindow, is_copy_command, is_cut_command, is_paste_command, translate_virtual_key_code};
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{Editor, GuiContext, ParamSetter, ParentWindowHandle};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use copypasta::ClipboardProvider;

#[cfg(not(feature = "opengl"))]
compile_error!("There's currently no software rendering support for egui");

/// Re-export for convenience.
pub use egui;
use keyboard_types::KeyboardEvent;
use nih_plug::editor::SpawnedWindow;

pub mod widgets;

/// Create an [`Editor`] instance using an [`egui`][::egui] GUI. Using the user state parameter is
/// optional, but it can be useful for keeping track of some temporary GUI-only settings. See the
/// `gui_gain` example for more information on how to use this. The [`EguiState`] passed to this
/// function contains the GUI's intitial size, and this is kept in sync whenever the GUI gets
/// resized. You can also use this to know if the GUI is open, so you can avoid performing
/// potentially expensive calculations while the GUI is not open. If you want this size to be
/// persisted when restoring a plugin instance, then you can store it in a `#[persist = "key"]`
/// field on your parameters struct.
///
/// See [`EguiState::from_size()`].
pub fn create_egui_editor<T, B, U>(
    egui_state: Arc<EguiState>,
    user_state: T,
    build: B,
    update: U,
) -> Option<Box<dyn Editor>>
where
    T: 'static + Send + Sync,
    B: Fn(&Context, &mut T) + 'static + Send + Sync,
    U: Fn(&Context, &ParamSetter, &mut T) + 'static + Send + Sync,
{
    Some(Box::new(EguiEditor {
        egui_state,
        user_state: Arc::new(RwLock::new(user_state)),
        build: Arc::new(build),
        update: Arc::new(update),

        // TODO: We can't get the size of the window when baseview does its own scaling, so if the
        //       host does not set a scale factor on Windows or Linux we should just use a factor of
        //       1. That may make the GUI tiny but it also prevents it from getting cut off.
        #[cfg(target_os = "macos")]
        scaling_factor: AtomicCell::new(None),
        #[cfg(not(target_os = "macos"))]
        scaling_factor: AtomicCell::new(Some(1.0)),
        plugin_keyboard_events: Arc::new(Mutex::new(vec![])),

        clipboard_ctx: Arc::new(Mutex::new(match copypasta::ClipboardContext::new() {
            Ok(clipboard_ctx) => Some(clipboard_ctx),
            Err(e) => {
                eprintln!("Failed to initialize clipboard: {}", e);
                None
            }
        })),
    }))
}

#[derive(Clone)]
pub enum AcceptableKeys {
    All,
    Specific(Vec<(egui::Modifiers, egui::Key)>),
}
impl Default for AcceptableKeys { fn default() -> Self { AcceptableKeys::none() } }
impl AcceptableKeys {
    pub fn none() -> AcceptableKeys {
        AcceptableKeys::Specific(Default::default())
    }
    pub fn specific(specific: Vec<(egui::Modifiers, egui::Key)>) -> AcceptableKeys {
        AcceptableKeys::Specific(specific)
    }
    pub fn accepts(&self, modifiers: egui::Modifiers, key: &egui::Key) -> bool {
        match self {
            AcceptableKeys::All => {
                true
            }
            AcceptableKeys::Specific(specific) => {
                specific.into_iter().any(|(required_modifiers, specific_key)| specific_key == key && match_modifiers_at_least(modifiers, *required_modifiers))
            }
        }
    }
}

fn match_modifiers_at_least(current_modifiers: Modifiers, required_modifiers: Modifiers) -> bool {
    if required_modifiers.ctrl && !current_modifiers.ctrl { return false; }
    if required_modifiers.alt && !current_modifiers.alt { return false; }
    if required_modifiers.command && !current_modifiers.command { return false; }
    if required_modifiers.mac_cmd && !current_modifiers.mac_cmd { return false; }
    if required_modifiers.shift && !current_modifiers.shift { return false; }
    true
}

/// State for an `nih_plug_egui` editor.
#[derive(Serialize, Deserialize)]
pub struct EguiState {
    /// The window's size in logical pixels before applying `scale_factor`.
    #[serde(with = "nih_plug::params::persist::serialize_atomic_cell")]
    size: AtomicCell<(u32, u32)>,
    /// Whether the editor's window is currently open.
    #[serde(skip)]
    open: AtomicBool,

    #[serde(skip)]
    acceptable_keys: Arc<Mutex<AcceptableKeys>>,
}

impl<'a> PersistentField<'a, EguiState> for Arc<EguiState> {
    fn set(&self, new_value: EguiState) {
        self.size.store(new_value.size.load());
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&EguiState) -> R,
    {
        f(self)
    }
}

impl EguiState {
    /// Initialize the GUI's state. This value can be passed to [`create_egui_editor()`]. The window
    /// size is in logical pixels, so before it is multiplied by the DPI scaling factor.
    pub fn from_size(width: u32, height: u32) -> Arc<EguiState> {
        Arc::new(EguiState {
            size: AtomicCell::new((width, height)),
            open: AtomicBool::new(false),
            acceptable_keys: Default::default()
        })
    }

    /// Returns a `(width, height)` pair for the current size of the GUI in logical pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size.load()
    }

    /// Whether the GUI is currently visible.
    // Called `is_open()` instead of `open()` to avoid the ambiguity.
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }

    pub fn set_acceptable_keys(&self, acceptable_keys: AcceptableKeys) -> Result<(), ()> {
        *self.acceptable_keys.try_lock().map_err(|_| ())? = acceptable_keys;
        Ok(())
    }
}

/// An [`Editor`] implementation that calls an egui draw loop.
struct EguiEditor<T> {
    egui_state: Arc<EguiState>,
    /// The plugin's state. This is kept in between editor openenings.
    user_state: Arc<RwLock<T>>,

    /// The user's build function. Applied once at the start of the application.
    build: Arc<dyn Fn(&Context, &mut T) + 'static + Send + Sync>,
    /// The user's update function.
    update: Arc<dyn Fn(&Context, &ParamSetter, &mut T) + 'static + Send + Sync>,

    /// The scaling factor reported by the host, if any. On macOS this will never be set and we
    /// should use the system scaling factor instead.
    scaling_factor: AtomicCell<Option<f32>>,

    plugin_keyboard_events: Arc<Mutex<Vec<EguiKeyboardInput>>>,

    clipboard_ctx:  Arc<Mutex<Option<copypasta::ClipboardContext>>>,

}

impl<T> Editor for EguiEditor<T>
where
    T: 'static + Send + Sync,
{
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
        request_keyboard_focus: bool
    ) -> Box<dyn SpawnedWindow + Send> {
        let build = self.build.clone();
        let update = self.update.clone();
        let state = self.user_state.clone();
        let plugin_keyboard_events = self.plugin_keyboard_events.clone();

        let (unscaled_width, unscaled_height) = self.egui_state.size();
        let scaling_factor = self.scaling_factor.load();
        let mut window = EguiWindow::open_parented(
            &parent,
            WindowOpenOptions {
                title: String::from("egui window"),
                // Baseview should be doing the DPI scaling for us
                size: Size::new(unscaled_width as f64, unscaled_height as f64),
                // NOTE: For some reason passing 1.0 here causes the UI to be scaled on macOS but
                //       not the mouse events.
                scale: scaling_factor
                    .map(|factor| WindowScalePolicy::ScaleFactor(factor as f64))
                    .unwrap_or(WindowScalePolicy::SystemScaleFactor),

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
                        event.apply(egui_ctx);
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

        if request_keyboard_focus {
            window.request_keyboard_focus();
        }

        self.egui_state.open.store(true, Ordering::Release);
        Box::new(EguiEditorHandle {
            egui_state: self.egui_state.clone(),
            window,
        })
    }

    fn size(&self) -> (u32, u32) {
        self.egui_state.size()
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        self.scaling_factor.store(Some(factor));
        true
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
        let acceptable_keys = self.egui_state.acceptable_keys.try_lock().map(|x| x.deref().clone()).unwrap_or_default();
        if let Some(translated_key) = translate_virtual_key_code(keyboard_event.code) {
            let translated_mods = translate_modifiers(&keyboard_event.modifiers);
            if acceptable_keys.accepts(translated_mods, &translated_key) {
                if let Ok(mut plugin_keyboard_events) = self.plugin_keyboard_events.try_lock() {
                    if let Ok(mut clipboard_ctx) = self.clipboard_ctx.try_lock() {
                        plugin_keyboard_events.push(EguiKeyboardInput::from_keyboard_event(keyboard_event, clipboard_ctx.as_mut()));
                        return true;
                    }
                }
            }
        }
        return false;
    }
}

struct EguiKeyboardInput {
    events: Vec<egui::Event>,
    modifiers: Modifiers,
}
impl EguiKeyboardInput {
    fn from_keyboard_event(event: &keyboard_types::KeyboardEvent, clipboard_ctx: Option<&mut copypasta::ClipboardContext>) -> EguiKeyboardInput {
        let mut events = vec![];
        let mut modifiers = translate_modifiers(&event.modifiers);

        use keyboard_types::Code;

        let pressed = event.state == keyboard_types::KeyState::Down;

        match event.code {
            Code::ShiftLeft | Code::ShiftRight => modifiers.shift = pressed,
            Code::ControlLeft | Code::ControlRight => {
                modifiers.ctrl = pressed;

                #[cfg(not(target_os = "macos"))]
                {
                    modifiers.command = pressed;
                }
            }
            Code::AltLeft | Code::AltRight => modifiers.alt = pressed,
            Code::MetaLeft | Code::MetaRight => {
                #[cfg(target_os = "macos")]
                {
                    modifiers.mac_cmd = pressed;
                    modifiers.command = pressed;
                }
                () // prevent `rustfmt` from breaking this
            }
            _ => (),
        }

        if let Some(key) = translate_virtual_key_code(event.code) {
            events.push(egui::Event::Key { key, pressed, modifiers });
        }

        if pressed {
            // VirtualKeyCode::Paste etc in winit are broken/untrustworthy,
            // so we detect these things manually:
            if is_cut_command(modifiers, event.code) {
                events.push(egui::Event::Cut);
            } else if is_copy_command(modifiers, event.code) {
                events.push(egui::Event::Copy);
            } else if is_paste_command(modifiers, event.code) {
                if let Some(clipboard_ctx) = clipboard_ctx {
                    if let Ok(contents) = clipboard_ctx.get_contents() {
                        events.push(egui::Event::Paste(contents));
                    }
                    if let Ok(data) = clipboard_ctx.get_mime_contents("application/dspstudio") {
                        events.push(
                            egui::Event::PasteMime(ClipboardData {
                                data,
                                mime: ClipboardMime::Specific("application/dspstudio".to_string())
                            })
                        );
                    }
                }
            } else if let keyboard_types::Key::Character(written) = &event.key {
                if !modifiers.ctrl && !modifiers.command {
                    events.push(egui::Event::Text(written.clone()));
                }
            }
        }
        EguiKeyboardInput {
            events,
            modifiers
        }
    }

    fn apply(self, ctx: &egui::Context) {
        let mut input_mut = ctx.input_mut();
        for event in self.events {
            if let Event::Key { key, pressed, .. } = &event {
                if *pressed {
                    input_mut.keys_down.insert(*key);
                } else {
                    input_mut.keys_down.remove(key);
                }
            }
            input_mut.raw.events.push(event.clone());
            input_mut.events.push(event);
        }
        input_mut.modifiers = self.modifiers;
    }
}
/// The window handle used for [`EguiEditor`].
struct EguiEditorHandle {
    egui_state: Arc<EguiState>,
    window: WindowHandle,
}
impl SpawnedWindow for EguiEditorHandle {
    fn resize(&self, logical_width: f32, logical_height: f32, scale_factor: f32) {

        // store new size in egui_state
        let unscaled_width = logical_width / scale_factor;
        let unscaled_height = logical_height / scale_factor;
        self.egui_state.size.store((unscaled_width as u32, unscaled_height as u32));

        // resize spawned window
        let logical_size = baseview::Size {
            width: logical_width as f64,
            height: logical_height as f64,
        };
        self.window.resize(logical_size, scale_factor);
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

pub fn translate_modifiers(modifiers: &keyboard_types::Modifiers) -> Modifiers {
    Modifiers {
        alt: modifiers.contains(keyboard_types::Modifiers::ALT),
        command: modifiers.contains(keyboard_types::Modifiers::META) || (!cfg!(target_os = "macos") && modifiers.contains(keyboard_types::Modifiers::CONTROL)),
        ctrl: modifiers.contains(keyboard_types::Modifiers::CONTROL) || (!cfg!(target_os = "macos") && modifiers.contains(keyboard_types::Modifiers::META)),
        mac_cmd: cfg!(target_os = "macos") && modifiers.contains(keyboard_types::Modifiers::META),
        shift: modifiers.contains(keyboard_types::Modifiers::SHIFT),
    }
}
