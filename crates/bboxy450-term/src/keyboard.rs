// SPDX-License-Identifier: MIT OR Apache-2.0
//! The app's winit boundary: normalize input, read native locks and call bbt-core.

use bbt_core::keyboard as protocol;
pub use protocol::Locks;
use std::collections::BTreeSet;
use winit::{
    event::{ElementState, KeyEvent, Modifiers as WinitModifiers},
    keyboard::{
        Key, KeyCode, KeyLocation, ModifiersKeyState, ModifiersState, NamedKey, PhysicalKey,
    },
    platform::modifier_supplement::KeyEventExtModifierSupplement,
};

/// All modifiers consumed by the pure encoder. Command is reserved by the app.
#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub state: ModifiersState,
    pub locks: Locks,
}

/// Public winit fields plus its modifier-supplement values, without a private
/// platform-specific KeyEvent constructor in tests.
#[derive(Clone, Debug)]
pub struct Input<'a> {
    pub logical_key: Key,
    pub physical_key: PhysicalKey,
    pub unmodified_key: Key,
    pub text: Option<&'a str>,
    pub text_all_modifiers: Option<&'a str>,
    pub location: KeyLocation,
    pub state: ElementState,
    pub repeat: bool,
    pub modifiers: Modifiers,
}

/// Character bindings follow the active layout, ignoring Shift/Option/Caps Lock.
/// Physical keys are reserved for press/release ownership and protocol metadata.
pub fn shortcut_key(input: &Input<'_>) -> Option<char> {
    fn single(text: &str) -> Option<char> {
        let mut chars = text.chars();
        let character = chars.next()?;
        chars.next().is_none().then_some(character)
    }
    let Key::Character(text) = &input.unmodified_key else {
        return None;
    };
    let base = single(text)?;
    #[cfg(target_os = "macos")]
    if input.modifiers.state.super_key() && !base.is_ascii() {
        // macOS non-Latin layouts have an ASCII Command layer (Russian е -> t).
        // Winit's text_with_all_modifiers preserves NSEvent.characters; its
        // modifierless key deliberately removes that Command translation.
        let command = input
            .text_all_modifiers
            .and_then(single)
            .filter(char::is_ascii_graphic)
            .or_else(|| match &input.logical_key {
                Key::Character(text) => single(text).filter(char::is_ascii_graphic),
                _ => None,
            });
        if let Some(command) = command {
            return Some(command.to_ascii_lowercase());
        }
    }
    Some(base.to_ascii_lowercase())
}

/// UI-owned adaptation state, not terminal state. It keeps a locally consumed
/// chord's repeat/release out of the PTY even if Command is released first.
#[derive(Default)]
pub struct State {
    held_modifiers: u8,
    local_keys: BTreeSet<PhysicalKey>,
}
impl State {
    pub fn modifiers_changed(&mut self, modifiers: WinitModifiers) {
        let states = [
            modifiers.lshift_state(),
            modifiers.rshift_state(),
            modifiers.lalt_state(),
            modifiers.ralt_state(),
            modifiers.lcontrol_state(),
            modifiers.rcontrol_state(),
            modifiers.lsuper_state(),
            modifiers.rsuper_state(),
        ];
        self.held_modifiers = states.iter().enumerate().fold(0, |bits, (i, state)| {
            bits | (u8::from(*state == ModifiersKeyState::Pressed) << i)
        });
    }

    fn event_modifiers(
        &mut self,
        physical: PhysicalKey,
        state: ElementState,
        mut mods: ModifiersState,
    ) -> ModifiersState {
        let (index, flag) = match physical {
            PhysicalKey::Code(KeyCode::ShiftLeft) => (0, ModifiersState::SHIFT),
            PhysicalKey::Code(KeyCode::ShiftRight) => (1, ModifiersState::SHIFT),
            PhysicalKey::Code(KeyCode::AltLeft) => (2, ModifiersState::ALT),
            PhysicalKey::Code(KeyCode::AltRight) => (3, ModifiersState::ALT),
            PhysicalKey::Code(KeyCode::ControlLeft) => (4, ModifiersState::CONTROL),
            PhysicalKey::Code(KeyCode::ControlRight) => (5, ModifiersState::CONTROL),
            PhysicalKey::Code(KeyCode::SuperLeft) => (6, ModifiersState::SUPER),
            PhysicalKey::Code(KeyCode::SuperRight) => (7, ModifiersState::SUPER),
            _ => return mods,
        };
        if state.is_pressed() {
            self.held_modifiers |= 1 << index;
        } else {
            self.held_modifiers &= !(1 << index);
        }
        mods.set(flag, self.held_modifiers & (3 << (index & !1)) != 0);
        mods
    }

    pub fn event<'a>(&mut self, event: &'a KeyEvent, modifiers: ModifiersState) -> Input<'a> {
        Input {
            logical_key: event.logical_key.clone(),
            physical_key: event.physical_key,
            unmodified_key: event.key_without_modifiers(),
            text: event.text.as_deref(),
            text_all_modifiers: event.text_with_all_modifiers(),
            location: event.location,
            state: event.state,
            repeat: event.repeat,
            modifiers: Modifiers {
                state: self.event_modifiers(event.physical_key, event.state, modifiers),
                locks: platform_locks(),
            },
        }
    }

    pub fn suppressed(&mut self, physical: PhysicalKey, state: ElementState, repeat: bool) -> bool {
        if state == ElementState::Released {
            return self.local_keys.remove(&physical);
        }
        if !repeat {
            self.local_keys.remove(&physical);
            return false;
        }
        self.local_keys.contains(&physical)
    }
    pub fn consumed(&mut self, physical: PhysicalKey, state: ElementState) {
        if state.is_pressed() {
            self.local_keys.insert(physical);
        }
    }
}

#[cfg(target_os = "macos")]
fn platform_locks() -> Locks {
    // SAFETY: NSEvent's class-level +modifierFlags returns NSUInteger without
    // arguments. AppKit is already linked; the caller is winit's UI callback.
    let flags: usize = unsafe { objc2::msg_send![objc2::class!(NSEvent), modifierFlags] };
    macos_locks(flags)
}
#[cfg(any(target_os = "macos", test))]
fn macos_locks(flags: usize) -> Locks {
    // macOS exposes Caps Lock (1<<16), but no Num Lock toggle. NumericPad
    // (1<<21) describes keypad/arrow origin and must never become Num Lock.
    Locks {
        caps: flags & (1 << 16) != 0,
        num: false,
    }
}
#[cfg(not(target_os = "macos"))]
fn platform_locks() -> Locks {
    Locks::default()
}

/// Adapt the event to the one neutral encoder. Command remains local to the app.
pub fn encode(input: &Input<'_>, modes: u64) -> Option<Vec<u8>> {
    if input.modifiers.state.super_key()
        || matches!(input.logical_key, Key::Named(NamedKey::Super))
        || matches!(
            input.physical_key,
            PhysicalKey::Code(KeyCode::SuperLeft | KeyCode::SuperRight)
        )
    {
        return None;
    }
    let mods = input.modifiers.state;
    let mut state = protocol::ModifiersState::empty();
    state.set(protocol::ModifiersState::SHIFT, mods.shift_key());
    state.set(protocol::ModifiersState::ALT, mods.alt_key());
    state.set(protocol::ModifiersState::CONTROL, mods.control_key());
    state.set(protocol::ModifiersState::SUPER, mods.super_key());
    let key = protocol::Input {
        logical_key: protocol_key(&input.logical_key),
        unmodified_key: protocol_key(&input.unmodified_key),
        physical_key: protocol_physical(input.physical_key),
        text: input.text,
        text_all_modifiers: input.text_all_modifiers,
        location: match input.location {
            KeyLocation::Standard => protocol::KeyLocation::Standard,
            KeyLocation::Left => protocol::KeyLocation::Left,
            KeyLocation::Right => protocol::KeyLocation::Right,
            KeyLocation::Numpad => protocol::KeyLocation::Numpad,
        },
        state: match input.state {
            ElementState::Pressed => protocol::ElementState::Pressed,
            ElementState::Released => protocol::ElementState::Released,
        },
        repeat: input.repeat,
        modifiers: protocol::Modifiers {
            state,
            locks: input.modifiers.locks,
        },
    };
    encode_protocol(&key, modes)
}
/// The adapter's one call into the neutral encoder, shared by the window's keys and the
/// operator's named keys (`ctl seat key`, #270).
fn encode_protocol(key: &protocol::Input<'_>, modes: u64) -> Option<Vec<u8>> {
    protocol::encode(key, modes)
}
/// A named key as the window would send it against these terminal modes: `enter`, `esc`,
/// `tab`, `shift-tab`, `backspace`, `space`, arrows, `home`, `end`, `pageup`, `pagedown`,
/// `delete`, `ctrl-<letter>`. None for a name it does not know.
pub fn named_key_bytes(name: &str, modes: u64) -> Option<Vec<u8>> {
    use protocol::{
        ElementState, Input, Key, KeyCode, KeyLocation, ModifiersState, NamedKey, PhysicalKey,
    };
    let (key, physical, mods) = if let Some(letter) = name.strip_prefix("ctrl-") {
        let c = letter
            .chars()
            .next()
            .filter(|c| letter.len() == 1 && c.is_ascii_lowercase())?;
        let code = [
            KeyCode::KeyA,
            KeyCode::KeyB,
            KeyCode::KeyC,
            KeyCode::KeyD,
            KeyCode::KeyE,
            KeyCode::KeyF,
            KeyCode::KeyG,
            KeyCode::KeyH,
            KeyCode::KeyI,
            KeyCode::KeyJ,
            KeyCode::KeyK,
            KeyCode::KeyL,
            KeyCode::KeyM,
            KeyCode::KeyN,
            KeyCode::KeyO,
            KeyCode::KeyP,
            KeyCode::KeyQ,
            KeyCode::KeyR,
            KeyCode::KeyS,
            KeyCode::KeyT,
            KeyCode::KeyU,
            KeyCode::KeyV,
            KeyCode::KeyW,
            KeyCode::KeyX,
            KeyCode::KeyY,
            KeyCode::KeyZ,
        ][(c as u8 - b'a') as usize];
        (
            Key::Character(c.to_string()),
            PhysicalKey::Code(code),
            ModifiersState::CONTROL,
        )
    } else {
        let empty = ModifiersState::empty();
        let (named, mods) = match name {
            "enter" | "return" => (NamedKey::Enter, empty),
            "esc" | "escape" => (NamedKey::Escape, empty),
            "tab" => (NamedKey::Tab, empty),
            "shift-tab" => (NamedKey::Tab, ModifiersState::SHIFT),
            "backspace" => (NamedKey::Backspace, empty),
            "space" => (NamedKey::Space, empty),
            "up" => (NamedKey::ArrowUp, empty),
            "down" => (NamedKey::ArrowDown, empty),
            "left" => (NamedKey::ArrowLeft, empty),
            "right" => (NamedKey::ArrowRight, empty),
            "home" => (NamedKey::Home, empty),
            "end" => (NamedKey::End, empty),
            "pageup" => (NamedKey::PageUp, empty),
            "pagedown" => (NamedKey::PageDown, empty),
            "delete" => (NamedKey::Delete, empty),
            _ => return None,
        };
        (Key::Named(named), PhysicalKey::Unidentified, mods)
    };
    let input = Input {
        unmodified_key: key.clone(),
        logical_key: key,
        physical_key: physical,
        text: None,
        text_all_modifiers: None,
        location: KeyLocation::Standard,
        state: ElementState::Pressed,
        repeat: false,
        modifiers: protocol::Modifiers {
            state: mods,
            ..Default::default()
        },
    };
    encode_protocol(&input, modes)
}
fn protocol_key(key: &Key) -> protocol::Key {
    let named = match key {
        Key::Character(text) => return protocol::Key::Character(text.to_string()),
        Key::Named(named) => named,
        _ => return protocol::Key::Unidentified,
    };
    macro_rules! names { ($($name:ident),* $(,)?) => { match named { $(NamedKey::$name=>protocol::NamedKey::$name,)* _=>return protocol::Key::Unidentified } }; }
    protocol::Key::Named(names!(
        Alt,
        ArrowDown,
        ArrowLeft,
        ArrowRight,
        ArrowUp,
        AudioVolumeDown,
        AudioVolumeMute,
        AudioVolumeUp,
        Backspace,
        CapsLock,
        Clear,
        ContextMenu,
        Control,
        Delete,
        End,
        Enter,
        Escape,
        F1,
        F10,
        F11,
        F12,
        F13,
        F14,
        F15,
        F16,
        F17,
        F18,
        F19,
        F2,
        F20,
        F21,
        F22,
        F23,
        F24,
        F25,
        F26,
        F27,
        F28,
        F29,
        F3,
        F30,
        F31,
        F32,
        F33,
        F34,
        F35,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        Home,
        Hyper,
        Insert,
        MediaFastForward,
        MediaPause,
        MediaPlay,
        MediaPlayPause,
        MediaRecord,
        MediaRewind,
        MediaStop,
        MediaTrackNext,
        MediaTrackPrevious,
        Meta,
        NumLock,
        PageDown,
        PageUp,
        Pause,
        PrintScreen,
        ScrollLock,
        Shift,
        Space,
        Super,
        Tab
    ))
}
fn protocol_physical(key: PhysicalKey) -> protocol::PhysicalKey {
    let PhysicalKey::Code(code) = key else {
        return protocol::PhysicalKey::Unidentified;
    };
    macro_rules! codes { ($($name:ident),* $(,)?) => { match code { $(KeyCode::$name=>protocol::KeyCode::$name,)* _=>return protocol::PhysicalKey::Unidentified } }; }
    protocol::PhysicalKey::Code(codes!(
        Backquote,
        Backslash,
        BracketLeft,
        BracketRight,
        Comma,
        ControlLeft,
        Digit0,
        Digit1,
        Digit2,
        Digit3,
        Digit4,
        Digit5,
        Digit6,
        Digit7,
        Digit8,
        Digit9,
        Equal,
        F5,
        KeyA,
        KeyB,
        KeyC,
        KeyD,
        KeyE,
        KeyF,
        KeyG,
        KeyH,
        KeyI,
        KeyJ,
        KeyK,
        KeyL,
        KeyM,
        KeyN,
        KeyO,
        KeyP,
        KeyQ,
        KeyR,
        KeyS,
        KeyT,
        KeyU,
        KeyV,
        KeyW,
        KeyX,
        KeyY,
        KeyZ,
        Minus,
        Numpad0,
        Numpad1,
        Numpad2,
        Numpad3,
        Numpad4,
        Numpad5,
        Numpad6,
        Numpad7,
        Numpad8,
        Numpad9,
        NumpadAdd,
        NumpadComma,
        NumpadDecimal,
        NumpadDivide,
        NumpadEnter,
        NumpadEqual,
        NumpadMultiply,
        NumpadSubtract,
        Period,
        Quote,
        Semicolon,
        Slash,
        Space
    ))
}

#[cfg(test)]
pub(crate) fn assert_adapter() {
    source_guard();
    let mut term = bbt_core::Term::new(10, 2, 0);
    let mut input = Input {
        logical_key: Key::Named(NamedKey::Escape),
        unmodified_key: Key::Named(NamedKey::Escape),
        physical_key: PhysicalKey::Code(KeyCode::Escape),
        text: None,
        text_all_modifiers: None,
        location: KeyLocation::Standard,
        state: ElementState::Pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    };
    for flags in 0..32 {
        term.advance(format!("\x1b[>{flags}u").as_bytes());
        let screen = bbt_proto::Screen::from_term(&term, String::new(), None, None);
        let esc = if flags & (1 | 8) != 0 {
            b"\x1b[27u".as_slice()
        } else {
            b"\x1b".as_slice()
        };
        assert_eq!(encode(&input, screen.modes), Some(esc.to_vec()));
        term.advance(b"\x1b[<u");
    }
    input.logical_key = Key::Character("?".into());
    input.unmodified_key = Key::Character("/".into());
    input.physical_key = PhysicalKey::Code(KeyCode::Slash);
    input.modifiers.state = ModifiersState::CONTROL | ModifiersState::SHIFT;
    input.text = Some("?");
    input.text_all_modifiers = Some("\x7f");
    assert_eq!(encode(&input, 0), Some(vec![127]));
    input.logical_key = Key::Character("Ф".into());
    input.unmodified_key = Key::Character("ф".into());
    input.physical_key = PhysicalKey::Code(KeyCode::KeyA);
    input.modifiers.state = ModifiersState::SHIFT;
    input.text = Some("Ф");
    input.text_all_modifiers = Some("Ф");
    assert_eq!(
        encode(&input, 31 << 18),
        Some(b"\x1b[1092:1060:97;2;1060u".to_vec())
    );
    input.repeat = true;
    assert_eq!(
        encode(&input, 31 << 18),
        Some(b"\x1b[1092:1060:97;2:2;1060u".to_vec())
    );
    input.repeat = false;
    input.state = ElementState::Released;
    assert_eq!(
        encode(&input, 31 << 18),
        Some(b"\x1b[1092:1060:97;2:3u".to_vec())
    );
    input.state = ElementState::Pressed;
    input.logical_key = Key::Named(NamedKey::Enter);
    input.unmodified_key = input.logical_key.clone();
    input.physical_key = PhysicalKey::Code(KeyCode::NumpadEnter);
    input.location = KeyLocation::Numpad;
    input.modifiers.state = ModifiersState::empty();
    assert_eq!(encode(&input, 1 << 18), Some(b"\x1b[57414u".to_vec()));
    input.logical_key = Key::Character("t".into());
    input.physical_key = PhysicalKey::Code(KeyCode::KeyT);
    input.modifiers.state = ModifiersState::SUPER;
    for event in [ElementState::Pressed, ElementState::Released] {
        input.state = event;
        assert_eq!(encode(&input, 31 << 18), None);
    }
    let physical = PhysicalKey::Code(KeyCode::KeyT);
    let mut state = State::default();
    state.consumed(physical, ElementState::Pressed);
    assert!(state.suppressed(physical, ElementState::Pressed, true));
    assert!(state.suppressed(physical, ElementState::Released, false));
    assert!(!state.suppressed(physical, ElementState::Pressed, false));
    let left = PhysicalKey::Code(KeyCode::ShiftLeft);
    let right = PhysicalKey::Code(KeyCode::ShiftRight);
    state.event_modifiers(left, ElementState::Pressed, ModifiersState::empty());
    state.event_modifiers(right, ElementState::Pressed, ModifiersState::SHIFT);
    assert!(
        state
            .event_modifiers(left, ElementState::Released, ModifiersState::SHIFT)
            .shift_key()
    );
    assert!(
        !state
            .event_modifiers(right, ElementState::Released, ModifiersState::SHIFT)
            .shift_key()
    );
    let caps = macos_locks((1 << 16) | (1 << 21));
    assert!(caps.caps && !caps.num);
    let keypad = macos_locks(1 << 21);
    assert!(!keypad.caps && !keypad.num);
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "offline invariant scans app sources; this function is test-only, never a UI callback"
)]
pub(crate) fn source_guard() {
    // KeyEvent/keysym interpretation is confined to the adapter and the
    // app's local shortcut router. Other modules receive opaque PTY bytes.
    fn scan(dir: &std::path::Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                scan(&path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            if matches!(
                path.file_name().unwrap().to_str().unwrap(),
                "keyboard.rs" | "main.rs"
            ) {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for token in [
                "KeyEvent",
                "NamedKey",
                "PhysicalKey",
                "logical_key",
                "KeyboardInput",
            ] {
                assert!(
                    !source.contains(token),
                    "second key interpretation: {}: {token}",
                    path.display()
                );
            }
        }
    }
    scan(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    let adapter = include_str!("keyboard.rs");
    let adapter = adapter.split("#[cfg(test)]").next().unwrap();
    assert_eq!(adapter.matches("protocol::encode(").count(), 1);
    for bypass in [
        "fn build_sequence",
        "struct SequenceBuilder",
        "fn legacy_control_or_text",
        "fn application_key",
        "as_bytes()",
        "into_bytes()",
        "\\x1b",
    ] {
        assert!(
            !adapter.contains(bypass),
            "app must only adapt keys: {bypass}"
        );
    }
    let source = include_str!("main.rs");
    assert_eq!(
        source.matches("WindowEvent::KeyboardInput {").count(),
        1,
        "second winit key path introduced"
    );
    let route = source
        .split_once("fn route_key(")
        .unwrap()
        .1
        .split_once("fn preedit_command(")
        .unwrap()
        .0;
    assert_eq!(route.matches("keyboard::encode(input, modes)").count(), 2);
    for bypass in [
        "\\x1b",
        "Vec::",
        "vec![",
        "to_vec()",
        "into_bytes()",
        "terminal_input(",
    ] {
        assert!(!route.contains(bypass), "key-to-bytes bypass: {bypass}");
    }
    let input = source
        .split_once("WindowEvent::KeyboardInput { event, .. } =>")
        .unwrap()
        .1
        .split_once("WindowEvent::RedrawRequested =>")
        .unwrap()
        .0;
    let compact: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    for required in [
        "self.keyboard.event(",
        "route_input(",
        "screen.modes",
        "modes.is_some()",
        "self.queue(live::UiMsg::Input{",
    ] {
        assert!(
            compact.contains(required),
            "live key route lost: {required}"
        );
    }
    for bypass in [
        "\\x1b",
        "as_bytes()",
        "into_bytes()",
        "vec![",
        "extend_from_slice",
    ] {
        assert!(
            !input.contains(bypass),
            "key callback encodes outside keyboard.rs: {bypass}"
        );
    }
    let local = source
        .split_once("fn route_input(")
        .unwrap()
        .1
        .split_once("fn preedit_command(")
        .unwrap()
        .0;
    for required in [
        "state.suppressed(",
        "route_key(input, modes, query)",
        "RoutedKey::Terminal(_)",
        "state.consumed(",
    ] {
        assert!(
            local.contains(required),
            "local lifecycle route lost: {required}"
        );
    }
    let old = source.split_once("fn terminal_input(").unwrap().0;
    assert!(
        old.trim_end().ends_with("#[cfg(test)]"),
        "legacy encoder reintroduced in production"
    );
}
