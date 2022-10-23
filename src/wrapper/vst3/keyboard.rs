use keyboard_types::*;

pub fn create_vst_keyboard_event(vst_character: vst3_sys::base::char16, vst_key_code: i16, vst_modifiers: i16, state: KeyState) -> Result<KeyboardEvent, ()> {
    let modifiers: Modifiers = VstKeyModifier::from_bits(vst_modifiers as usize).ok_or(())?.into();
    let vst_key_code= VstKeyCode::try_from(vst_key_code).ok();
    let (code, key) = translate_vst_key_code_and_character(vst_character, vst_key_code).ok_or(())?;
    let location = code_to_location(code);
    Ok(KeyboardEvent { code, key, location, modifiers, state, is_composing: false, repeat: false })
}

fn vst_character_to_char(vst_character: vst3_sys::base::char16) -> Option<char> {
    if vst_character != 0 {
        if let Some(Ok(ch)) = char::decode_utf16([vst_character as u16]).next() {
            if ch != '\0' { // NB: should already be covered by the first check, but we want to be sure
                return Some(ch)
            }
        }
    }
    None
}

fn translate_vst_key_code_and_character(vst_character: vst3_sys::base::char16, vst_key_code: Option<VstKeyCode>) -> Option<(Code, Key)> {
    let result = if let Some((code, mut key)) = translate_vst_key_code(vst_key_code) {
        let key = key.or_else(|| vst_character_to_char(vst_character).map(|x| Key::Character(x.to_string())))?;
        (code, key)
    } else {
        (Code::Unidentified, vst_character_to_char(vst_character).map(|x| Key::Character(x.to_string()))?)
    };
    Some(result)
}

fn translate_vst_key_code(vst_key_code: Option<VstKeyCode>) -> Option<(Code, Option<Key>)> {
    let key_code = vst_key_code?;
    let code = vst_code_to_code(key_code)?;
    let key = vst_code_to_key(key_code);
    Some((code, key))
}

fn vst_code_to_key(key_code: VstKeyCode) -> Option<Key> {
    Some(match key_code {
        VstKeyCode::KEY_BACK => Key::Backspace,
        VstKeyCode::KEY_TAB => Key::Tab,
        VstKeyCode::KEY_CLEAR => Key::Clear,
        VstKeyCode::KEY_RETURN => Key::Enter,
        VstKeyCode::KEY_PAUSE => Key::Pause,
        VstKeyCode::KEY_ESCAPE => Key::Escape,
        VstKeyCode::KEY_SPACE => return None,
        VstKeyCode::KEY_NEXT => Key::NavigateNext,
        VstKeyCode::KEY_END => Key::End,
        VstKeyCode::KEY_HOME => Key::Home,
        VstKeyCode::KEY_LEFT => Key::ArrowLeft,
        VstKeyCode::KEY_UP => Key::ArrowUp,
        VstKeyCode::KEY_RIGHT => Key::ArrowRight,
        VstKeyCode::KEY_DOWN => Key::ArrowDown,
        VstKeyCode::KEY_PAGEUP => Key::PageUp,
        VstKeyCode::KEY_PAGEDOWN => Key::PageDown,
        VstKeyCode::KEY_SELECT => Key::Select,
        VstKeyCode::KEY_PRINT => Key::Print,
        VstKeyCode::KEY_ENTER => Key::Enter,
        VstKeyCode::KEY_SNAPSHOT => Key::PrintScreen,
        VstKeyCode::KEY_INSERT => Key::Insert,
        VstKeyCode::KEY_DELETE => Key::Delete,
        VstKeyCode::KEY_HELP => Key::Help,

        VstKeyCode::KEY_NUMPAD0 | VstKeyCode::KEY_NUMPAD1 | VstKeyCode::KEY_NUMPAD2 |
        VstKeyCode::KEY_NUMPAD3 | VstKeyCode::KEY_NUMPAD4 | VstKeyCode::KEY_NUMPAD5 |
        VstKeyCode::KEY_NUMPAD6 | VstKeyCode::KEY_NUMPAD7 | VstKeyCode::KEY_NUMPAD8 |
        VstKeyCode::KEY_NUMPAD9 |
        VstKeyCode::KEY_MULTIPLY |
        VstKeyCode::KEY_ADD |

        VstKeyCode::KEY_SEPARATOR |
        VstKeyCode::KEY_SUBTRACT |
        VstKeyCode::KEY_DECIMAL |
        VstKeyCode::KEY_DIVIDE => return None,

        VstKeyCode::KEY_F1 => Key::F1,
        VstKeyCode::KEY_F2 => Key::F2,
        VstKeyCode::KEY_F3 => Key::F3,
        VstKeyCode::KEY_F4 => Key::F4,
        VstKeyCode::KEY_F5 => Key::F5,
        VstKeyCode::KEY_F6 => Key::F6,
        VstKeyCode::KEY_F7 => Key::F7,
        VstKeyCode::KEY_F8 => Key::F8,
        VstKeyCode::KEY_F9 => Key::F9,
        VstKeyCode::KEY_F10 => Key::F10,
        VstKeyCode::KEY_F11 => Key::F11,
        VstKeyCode::KEY_F12 => Key::F12,
        VstKeyCode::KEY_F13 => Key::F13,
        VstKeyCode::KEY_F14 => Key::F14,
        VstKeyCode::KEY_F15 => Key::F15,
        VstKeyCode::KEY_F16 => Key::F16,
        VstKeyCode::KEY_F17 => Key::F17,
        VstKeyCode::KEY_F18 => Key::F18,
        VstKeyCode::KEY_F19 => Key::F19,
        VstKeyCode::KEY_NUMLOCK => Key::NumLock,
        VstKeyCode::KEY_SCROLL => Key::ScrollLock,
        VstKeyCode::KEY_SHIFT => Key::Shift,
        VstKeyCode::KEY_CONTROL => Key::Control,
        VstKeyCode::KEY_ALT => Key::Alt,
        VstKeyCode::KEY_EQUALS => return None,
        VstKeyCode::KEY_CONTEXTMENU => Key::ContextMenu,
        VstKeyCode::KEY_MEDIA_PLAY => Key::MediaPlay,
        VstKeyCode::KEY_MEDIA_STOP => Key::MediaStop,
        VstKeyCode::KEY_MEDIA_PREV => Key::MediaTrackPrevious,
        VstKeyCode::KEY_MEDIA_NEXT => Key::MediaTrackNext,
        VstKeyCode::KEY_VOLUME_UP => Key::AudioVolumeUp,
        VstKeyCode::KEY_VOLUME_DOWN => Key::AudioVolumeDown,
    })
}

fn vst_code_to_code(key_code: VstKeyCode) -> Option<Code> {
    Some(match key_code {
        VstKeyCode::KEY_BACK => Code::Backspace,
        VstKeyCode::KEY_TAB => Code::Tab,
        VstKeyCode::KEY_CLEAR => Code::NumpadClear,
        VstKeyCode::KEY_RETURN => Code::Enter,
        VstKeyCode::KEY_PAUSE => Code::Pause,
        VstKeyCode::KEY_ESCAPE => Code::Escape,
        VstKeyCode::KEY_SPACE => Code::Space,
        VstKeyCode::KEY_NEXT => return None,
        VstKeyCode::KEY_END => Code::End,
        VstKeyCode::KEY_HOME => Code::Home,
        VstKeyCode::KEY_LEFT => Code::ArrowLeft,
        VstKeyCode::KEY_UP => Code::ArrowUp,
        VstKeyCode::KEY_RIGHT => Code::ArrowRight,
        VstKeyCode::KEY_DOWN => Code::ArrowDown,
        VstKeyCode::KEY_PAGEUP => Code::PageUp,
        VstKeyCode::KEY_PAGEDOWN => Code::PageDown,
        VstKeyCode::KEY_SELECT => Code::Select,
        VstKeyCode::KEY_PRINT => return None,
        VstKeyCode::KEY_ENTER => Code::Enter,
        VstKeyCode::KEY_SNAPSHOT => Code::PrintScreen,
        VstKeyCode::KEY_INSERT => Code::Insert,
        VstKeyCode::KEY_DELETE => Code::Delete,
        VstKeyCode::KEY_HELP => Code::Help,
        VstKeyCode::KEY_NUMPAD0 => Code::Numpad0,
        VstKeyCode::KEY_NUMPAD1 => Code::Numpad1,
        VstKeyCode::KEY_NUMPAD2 => Code::Numpad2,
        VstKeyCode::KEY_NUMPAD3 => Code::Numpad3,
        VstKeyCode::KEY_NUMPAD4 => Code::Numpad4,
        VstKeyCode::KEY_NUMPAD5 => Code::Numpad5,
        VstKeyCode::KEY_NUMPAD6 => Code::Numpad6,
        VstKeyCode::KEY_NUMPAD7 => Code::Numpad7,
        VstKeyCode::KEY_NUMPAD8 => Code::Numpad8,
        VstKeyCode::KEY_NUMPAD9 => Code::Numpad9,
        VstKeyCode::KEY_MULTIPLY => Code::NumpadMultiply,
        VstKeyCode::KEY_ADD => Code::NumpadAdd,
        VstKeyCode::KEY_SEPARATOR => return None,
        VstKeyCode::KEY_SUBTRACT => Code::NumpadSubtract,
        VstKeyCode::KEY_DECIMAL => Code::NumpadDecimal,
        VstKeyCode::KEY_DIVIDE => Code::NumpadDivide,
        VstKeyCode::KEY_F1 => Code::F1,
        VstKeyCode::KEY_F2 => Code::F2,
        VstKeyCode::KEY_F3 => Code::F3,
        VstKeyCode::KEY_F4 => Code::F4,
        VstKeyCode::KEY_F5 => Code::F5,
        VstKeyCode::KEY_F6 => Code::F6,
        VstKeyCode::KEY_F7 => Code::F7,
        VstKeyCode::KEY_F8 => Code::F8,
        VstKeyCode::KEY_F9 => Code::F9,
        VstKeyCode::KEY_F10 => Code::F10,
        VstKeyCode::KEY_F11 => Code::F11,
        VstKeyCode::KEY_F12 => Code::F12,
        VstKeyCode::KEY_F13 => Code::F13,
        VstKeyCode::KEY_F14 => Code::F14,
        VstKeyCode::KEY_F15 => Code::F15,
        VstKeyCode::KEY_F16 => Code::F16,
        VstKeyCode::KEY_F17 => Code::F17,
        VstKeyCode::KEY_F18 => Code::F18,
        VstKeyCode::KEY_F19 => Code::F19,
        VstKeyCode::KEY_NUMLOCK => Code::NumLock,
        VstKeyCode::KEY_SCROLL => Code::ScrollLock,
        VstKeyCode::KEY_SHIFT => Code::ShiftLeft,
        VstKeyCode::KEY_CONTROL => Code::ControlLeft,
        VstKeyCode::KEY_ALT => Code::AltLeft,
        VstKeyCode::KEY_EQUALS => Code::Equal,
        VstKeyCode::KEY_CONTEXTMENU => Code::ContextMenu,
        VstKeyCode::KEY_MEDIA_PLAY => Code::MediaPlay,
        VstKeyCode::KEY_MEDIA_STOP => Code::MediaStop,
        VstKeyCode::KEY_MEDIA_PREV => Code::MediaTrackPrevious,
        VstKeyCode::KEY_MEDIA_NEXT => Code::MediaTrackNext,
        VstKeyCode::KEY_VOLUME_UP => Code::AudioVolumeUp,
        VstKeyCode::KEY_VOLUME_DOWN => Code::AudioVolumeDown,
    })
}

// translated from "VirtualKeyCodes" data structure in vst3 api
#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Copy, Clone)]
#[repr(i16)]
enum VstKeyCode {
    KEY_BACK = 1, KEY_TAB, KEY_CLEAR, KEY_RETURN,
    KEY_PAUSE, KEY_ESCAPE, KEY_SPACE, KEY_NEXT,
    KEY_END, KEY_HOME, KEY_LEFT, KEY_UP,
    KEY_RIGHT, KEY_DOWN, KEY_PAGEUP, KEY_PAGEDOWN,
    KEY_SELECT, KEY_PRINT, KEY_ENTER, KEY_SNAPSHOT,
    KEY_INSERT, KEY_DELETE, KEY_HELP, KEY_NUMPAD0,
    KEY_NUMPAD1, KEY_NUMPAD2, KEY_NUMPAD3, KEY_NUMPAD4,
    KEY_NUMPAD5, KEY_NUMPAD6, KEY_NUMPAD7, KEY_NUMPAD8,
    KEY_NUMPAD9, KEY_MULTIPLY, KEY_ADD, KEY_SEPARATOR,
    KEY_SUBTRACT, KEY_DECIMAL, KEY_DIVIDE, KEY_F1,
    KEY_F2, KEY_F3, KEY_F4, KEY_F5,
    KEY_F6, KEY_F7, KEY_F8, KEY_F9,
    KEY_F10, KEY_F11, KEY_F12, KEY_NUMLOCK,
    KEY_SCROLL, KEY_SHIFT, KEY_CONTROL, KEY_ALT,
    KEY_EQUALS, KEY_CONTEXTMENU, KEY_MEDIA_PLAY, KEY_MEDIA_STOP,
    KEY_MEDIA_PREV, KEY_MEDIA_NEXT, KEY_VOLUME_UP, KEY_VOLUME_DOWN,
    KEY_F13, KEY_F14, KEY_F15, KEY_F16,
    KEY_F17, KEY_F18, KEY_F19,
    //VKEY_FIRST_CODE = VirtualKeyCodes::KEY_BACK as i32,
    //VKEY_LAST_CODE = VirtualKeyCodes::KEY_F19 as i32,
    //VKEY_ANY_ASCII = 128
}

// TODO: Use external crate to make this code safer?
impl TryFrom<i16> for VstKeyCode {
    type Error = ();
    fn try_from(key_code: i16) -> Result<Self, ()> {
        if key_code >= VstKeyCode::KEY_BACK as i16 && key_code <= VstKeyCode::KEY_F19 as i16 {
            return Ok(unsafe { std::mem::transmute(key_code) })
        }
        Err(())
    }
}

// translated from "KeyModifier" data structure in vst3 api
bitflags::bitflags! {
    struct VstKeyModifier: usize {    
        const SHIFT_KEY = 1 << 0;
        const ALTERNATE_KEY = 1 << 1;
        const COMMAND_KEY = 1 << 2;
        const CONTROL_KEY = 1 << 3;
    }
}

impl Into<Modifiers> for VstKeyModifier {
    fn into(self) -> Modifiers {
        let mut result = Modifiers::empty();
        if self.contains(VstKeyModifier::SHIFT_KEY) { result |= Modifiers::SHIFT; }
        if self.contains(VstKeyModifier::ALTERNATE_KEY) { result |= Modifiers::ALT; }
        if self.contains(VstKeyModifier::COMMAND_KEY) { result |= Modifiers::META; }
        if self.contains(VstKeyModifier::CONTROL_KEY) { result |= Modifiers::CONTROL; }
        result
    }
}

// copied from baseview::keyboard to make it available here
pub fn code_to_location(code: Code) -> Location {
    match code {
        Code::MetaLeft | Code::ShiftLeft | Code::AltLeft | Code::ControlLeft => Location::Left,
        Code::MetaRight | Code::ShiftRight | Code::AltRight | Code::ControlRight => Location::Right,
        Code::Numpad0
        | Code::Numpad1
        | Code::Numpad2
        | Code::Numpad3
        | Code::Numpad4
        | Code::Numpad5
        | Code::Numpad6
        | Code::Numpad7
        | Code::Numpad8
        | Code::Numpad9
        | Code::NumpadAdd
        | Code::NumpadComma
        | Code::NumpadDecimal
        | Code::NumpadDivide
        | Code::NumpadEnter
        | Code::NumpadEqual
        | Code::NumpadMultiply
        | Code::NumpadSubtract => Location::Numpad,
        _ => Location::Standard,
    }
}
