use keyboard_types::*;
use vst3_sys::base::char16;

pub fn create_vst_keyboard_event(key_char: vst3_sys::base::char16, virtual_key_code: i16, vst_modifiers: i16, state: KeyState) -> Result<KeyboardEvent, ()> {

    let key_code = VstKeyCode::try_from(virtual_key_code).ok();

    let virtual_keycode_to_char = if key_char != 0 {
        convert_char16(key_char)
    } else {
        if virtual_key_code >= VKEY_FIRST_ASCII {
            convert_char16(virtual_key_code - VKEY_FIRST_ASCII + 0x30)
        } else if key_code == Some(VstKeyCode::KEY_SPACE) {
            Some(' ')
        } else {
            None
        }
    };

    // NOTE: KEY_EQUALS is broken on windows? Test if this workaround is really necessary!
    let (result_key, result_code) = if let Some(key_code) = key_code {
        if key_code != VstKeyCode::KEY_EQUALS {
            (vst_code_to_key(key_code), vst_code_to_code(key_code))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };
    let key = result_key.or_else(|| {
        virtual_keycode_to_char.map(|ch| Key::Character(ch.to_string()))
    }).unwrap_or(Key::Unidentified);

    let code = result_code.or_else(|| {
        virtual_keycode_to_char.and_then(|ch| char_to_code(ch))
    }).unwrap_or(Code::Unidentified);

    let modifiers: Modifiers = VstKeyModifier::from_bits(vst_modifiers as usize).ok_or(())?.into();
    let location = code_to_location(code);

    Ok(KeyboardEvent { code, key, location, modifiers, state, is_composing: false, repeat: false })
}

fn convert_char16(key: char16) -> Option<char> {
    char::decode_utf16([key as u16]).next().and_then(|x| x.ok())
}

fn char_to_code(ch: char) -> Option<Code> {
    Some(match ch {
        'a' | 'A' => Code::KeyA,
        'b' | 'B' => Code::KeyB,
        'c' | 'C' => Code::KeyC,
        'd' | 'D' => Code::KeyD,
        'e' | 'E' => Code::KeyE,
        'f' | 'F' => Code::KeyF,
        'g' | 'G' => Code::KeyG,
        'h' | 'H' => Code::KeyH,
        'i' | 'I' => Code::KeyI,
        'j' | 'J' => Code::KeyJ,
        'k' | 'K' => Code::KeyK,
        'l' | 'L' => Code::KeyL,
        'm' | 'M' => Code::KeyM,
        'n' | 'N' => Code::KeyN,
        'o' | 'O' => Code::KeyO,
        'p' | 'P' => Code::KeyP,
        'q' | 'Q' => Code::KeyQ,
        'r' | 'R' => Code::KeyR,
        's' | 'S' => Code::KeyS,
        't' | 'T' => Code::KeyT,
        'u' | 'U' => Code::KeyU,
        'v' | 'V' => Code::KeyV,
        'w' | 'W' => Code::KeyW,
        'x' | 'X' => Code::KeyX,
        'y' | 'Y' => Code::KeyY,
        'z' | 'Z' => Code::KeyZ,
        '0' => Code::Digit0,
        '1' => Code::Digit1,
        '2' => Code::Digit2,
        '3' => Code::Digit3,
        '4' => Code::Digit4,
        '5' => Code::Digit5,
        '6' => Code::Digit6,
        '7' => Code::Digit7,
        '8' => Code::Digit8,
        '9' => Code::Digit9,
        
        _ => {
            // TODO: can we do more here?
            return None;
        }
    })
}

fn vst_code_to_key(key_code: VstKeyCode) -> Option<Key> {
    Some(match key_code {
        VstKeyCode::KEY_BACK => Key::Backspace,
        VstKeyCode::KEY_TAB => Key::Tab,
        VstKeyCode::KEY_CLEAR => Key::Clear,
        VstKeyCode::KEY_RETURN => Key::Enter,
        VstKeyCode::KEY_PAUSE => Key::Pause,
        VstKeyCode::KEY_ESCAPE => Key::Escape,
        VstKeyCode::KEY_SPACE => Key::Character(' '.to_string()),
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

        VstKeyCode::KEY_NUMPAD0 => Key::Character('0'.to_string()),
        VstKeyCode::KEY_NUMPAD1 => Key::Character('1'.to_string()),
        VstKeyCode::KEY_NUMPAD2 => Key::Character('2'.to_string()),
        VstKeyCode::KEY_NUMPAD3 => Key::Character('0'.to_string()),
        VstKeyCode::KEY_NUMPAD4 => Key::Character('0'.to_string()),
        VstKeyCode::KEY_NUMPAD5 => Key::Character('0'.to_string()),
        VstKeyCode::KEY_NUMPAD6 => Key::Character('0'.to_string()),
        VstKeyCode::KEY_NUMPAD7 => Key::Character('0'.to_string()),
        VstKeyCode::KEY_NUMPAD8 => Key::Character('0'.to_string()),
        VstKeyCode::KEY_NUMPAD9 => Key::Character('0'.to_string()),
        VstKeyCode::KEY_MULTIPLY => Key::Character('*'.to_string()),
        VstKeyCode::KEY_ADD => Key::Character('+'.to_string()),
        VstKeyCode::KEY_SEPARATOR => return None, // Not sure which one this is...
        VstKeyCode::KEY_SUBTRACT => Key::Character('-'.to_string()),
        VstKeyCode::KEY_DECIMAL => Key::Character('.'.to_string()),
        VstKeyCode::KEY_DIVIDE => Key::Character('/'.to_string()),

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
        VstKeyCode::KEY_EQUALS => Key::Character('='.to_string()),
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
        VstKeyCode::KEY_SEPARATOR => return None, // Not sure what to do here
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

// translated from "VirtualKeyCodes" data structure in vst3 api - see https://steinbergmedia.github.io/vst3_doc/base/namespaceSteinberg.html
#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq)]
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
    KEY_F17, KEY_F18, KEY_F19
}
const VKEY_FIRST_CODE: i16 = VstKeyCode::KEY_BACK as i16;
const VKEY_LAST_CODE: i16 = VstKeyCode::KEY_F19 as i16;
const VKEY_FIRST_ASCII: i16 = 128;

// TODO: Use macros on the enum to make this code safer? Are there crates that help with this?
impl TryFrom<i16> for VstKeyCode {
    type Error = ();
    fn try_from(key_code: i16) -> Result<Self, ()> {
        if key_code >= VKEY_FIRST_CODE && key_code <= VKEY_LAST_CODE {
            return Ok(unsafe { std::mem::transmute(key_code) })
        }
        Err(())
    }
}

// translated from "KeyModifier" data structure in vst3 api - see https://steinbergmedia.github.io/vst3_doc/base/namespaceSteinberg.html
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
