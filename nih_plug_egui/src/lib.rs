//! [egui](https://github.com/emilk/egui) editor support for NIH plug.
//!
//! TODO: Proper usage example, for now check out the gain_gui example

// See the comment in the main `nih_plug` crate
#![allow(clippy::type_complexity)]

use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use baseview::gl::GlConfig;
use baseview::{Size, WindowHandle, WindowOpenOptions};
use crossbeam::atomic::AtomicCell;
use egui::{ClipboardData, ClipboardMime, Context, Event, Key, Modifiers, RawInput, Vec2};
use egui_baseview::{EguiWindow, is_copy_command, is_cut_command, is_paste_command, translate_virtual_key_code};
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{Editor, ParamSetter};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use copypasta::ClipboardProvider;

#[cfg(not(feature = "opengl"))]
compile_error!("There's currently no software rendering support for egui");

/// Re-export for convenience.
pub use egui;
use keyboard_types::{Code, KeyboardEvent};
use egui_baseview::window::{EguiKeyboardInput, translate_modifiers};
use nih_plug::editor::SpawnedWindow;

mod editor;
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
    Some(Box::new(editor::EguiEditor {
        egui_state,
        user_state: Arc::new(RwLock::new(user_state)),
        build: Arc::new(build),
        update: Arc::new(update),
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
    pub fn accepts_all(&self) -> bool {
        match self {
            AcceptableKeys::All => true,
            AcceptableKeys::Specific(specific) => false,
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
