// SPDX-License-Identifier: Apache-2.0
// Adapted from The Alacritty Project contributors' input/keyboard.rs at
// 542147067dd016fc5f49120ef0ba605a98a4e3a7 (Apache-2.0).
// modified by bboxy450.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Pure DEC/xterm and Kitty key encoding, independent of UI toolkits and IO.
//! SequenceBuilder/build_sequence are adapted from Alacritty 5421470.
//! The upstream license text is retained in this crate's LICENSE-APACHE.

use crate::term::TermMode;
use std::borrow::Cow;

/// Logical key independent of a window system. Character keys use Unicode text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key<S = String> {
    Character(S),
    Named(NamedKey),
    Unidentified,
}
impl<S: AsRef<str>> Key<S> {
    fn as_ref(&self) -> Key<&str> {
        match self {
            Self::Character(s) => Key::Character(s.as_ref()),
            Self::Named(n) => Key::Named(*n),
            Self::Unidentified => Key::Unidentified,
        }
    }
}
/// Functional key names used by DEC/xterm and the Kitty keyboard protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamedKey {
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
    Tab,
}
/// Physical US key positions needed for base-layout and application keypad keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
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
    Space,
}
/// A physical position; unsupported positions can retain logical key/text data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalKey {
    Code(KeyCode),
    Unidentified,
}
/// Location distinguishes the keypad and the two sides of modifier keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyLocation {
    Standard,
    Left,
    Right,
    Numpad,
}
/// Key transition. Repeats are pressed transitions with Input::repeat set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementState {
    Pressed,
    Released,
}
impl ElementState {
    pub fn is_pressed(self) -> bool {
        self == Self::Pressed
    }
}
bitflags::bitflags! {
    /// Toolkit-neutral depressed modifiers. Caps/Num toggles are separate Locks.
    #[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
    pub struct ModifiersState: u16 {
        const SHIFT=1; const ALT=2; const CONTROL=4; const SUPER=8; const HYPER=16; const META=32;
    }
}
impl ModifiersState {
    pub fn shift_key(self) -> bool {
        self.contains(Self::SHIFT)
    }
    pub fn alt_key(self) -> bool {
        self.contains(Self::ALT)
    }
    pub fn control_key(self) -> bool {
        self.contains(Self::CONTROL)
    }
    pub fn super_key(self) -> bool {
        self.contains(Self::SUPER)
    }
    pub fn hyper_key(self) -> bool {
        self.contains(Self::HYPER)
    }
    pub fn meta_key(self) -> bool {
        self.contains(Self::META)
    }
}

/// Lock state supplied by the platform, independent of keypad key location.
#[derive(Clone, Copy, Debug, Default)]
pub struct Locks {
    pub caps: bool,
    pub num: bool,
}

/// All modifiers consumed by the pure encoder. Command is reserved by the app.
#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub state: ModifiersState,
    pub locks: Locks,
}

/// Toolkit-neutral key data. The producer supplies active-layout logical and
/// unmodified keys, physical position, modifier state and optional generated text.
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
impl Input<'_> {
    fn key_without_modifiers(&self) -> Key {
        self.unmodified_key.clone()
    }
    fn text_with_all_modifiers(&self) -> Option<&str> {
        self.text_all_modifiers
    }
}

/// Encode one key using only the supplied event and focused Screen.modes.
/// Releases require REPORT_EVENT_TYPES. Shortcut policy belongs to the caller.
pub fn encode(key: &Input<'_>, modes: u64) -> Option<Vec<u8>> {
    let mode = TermMode::from_bits_retain(modes as u32);
    if key.state == ElementState::Released {
        if !mode.contains(TermMode::REPORT_EVENT_TYPES)
            || (!mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC)
                && matches!(
                    key.logical_key,
                    Key::Named(NamedKey::Enter | NamedKey::Tab | NamedKey::Backspace)
                ))
        {
            return None;
        }
        return nonempty(build_sequence(key, key.modifiers, mode));
    }
    let mods = key.modifiers.state;
    let disambiguate = mode.contains(TermMode::DISAMBIGUATE_ESC_CODES)
        && (key.logical_key == Key::Named(NamedKey::Escape)
            || key.location == KeyLocation::Numpad
            || (!mods.is_empty()
                && (mods != ModifiersState::SHIFT
                    || matches!(
                        key.logical_key,
                        Key::Named(NamedKey::Tab | NamedKey::Enter | NamedKey::Backspace)
                    ))));
    if mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC) || disambiguate {
        return nonempty(build_sequence(key, key.modifiers, mode));
    }
    let kitty = mode.intersects(
        TermMode::REPORT_ALL_KEYS_AS_ESC
            | TermMode::DISAMBIGUATE_ESC_CODES
            | TermMode::REPORT_EVENT_TYPES,
    );
    let mut legacy_mods = key.modifiers;
    // Preserve the existing app's legacy Alt-as-ESC behavior, including arrows.
    // Kitty-enhanced events encode Alt in the parameter field instead.
    if !kitty {
        legacy_mods.state.remove(ModifiersState::ALT);
    }
    let legacy = if !kitty {
        application_key(key, legacy_mods.state, mode)
    } else {
        None
    }
    .or_else(|| legacy_control_or_text(key, legacy_mods.state));
    // A text/control key can still use its legacy bytes with only event or
    // alternate reporting enabled. Alt must prefix those bytes too.
    let prefix_alt = mods.alt_key() && (legacy.is_some() || !kitty);
    let mut bytes = legacy.unwrap_or_else(|| build_sequence(key, legacy_mods, mode));
    if prefix_alt && !bytes.is_empty() {
        bytes.insert(0, 0x1b);
    }
    nonempty(bytes)
}
fn nonempty(bytes: Vec<u8>) -> Option<Vec<u8>> {
    (!bytes.is_empty()).then_some(bytes)
}

fn application_key(key: &Input<'_>, mods: ModifiersState, mode: TermMode) -> Option<Vec<u8>> {
    let keypad = mode
        .contains(TermMode::APP_KEYPAD)
        .then_some(match key.physical_key {
            PhysicalKey::Code(KeyCode::Numpad0) => Some('p'),
            PhysicalKey::Code(KeyCode::Numpad1) => Some('q'),
            PhysicalKey::Code(KeyCode::Numpad2) => Some('r'),
            PhysicalKey::Code(KeyCode::Numpad3) => Some('s'),
            PhysicalKey::Code(KeyCode::Numpad4) => Some('t'),
            PhysicalKey::Code(KeyCode::Numpad5) => Some('u'),
            PhysicalKey::Code(KeyCode::Numpad6) => Some('v'),
            PhysicalKey::Code(KeyCode::Numpad7) => Some('w'),
            PhysicalKey::Code(KeyCode::Numpad8) => Some('x'),
            PhysicalKey::Code(KeyCode::Numpad9) => Some('y'),
            PhysicalKey::Code(KeyCode::NumpadDecimal) => Some('n'),
            PhysicalKey::Code(KeyCode::NumpadDivide) => Some('o'),
            PhysicalKey::Code(KeyCode::NumpadMultiply) => Some('j'),
            PhysicalKey::Code(KeyCode::NumpadSubtract) => Some('m'),
            PhysicalKey::Code(KeyCode::NumpadAdd) => Some('k'),
            PhysicalKey::Code(KeyCode::NumpadEnter) => Some('M'),
            PhysicalKey::Code(KeyCode::NumpadEqual) => Some('X'),
            PhysicalKey::Code(KeyCode::NumpadComma) => Some('l'),
            _ => None,
        })
        .flatten();
    let ss3 = keypad.or_else(|| match key.logical_key {
        Key::Named(NamedKey::F1) => Some('P'),
        Key::Named(NamedKey::F2) => Some('Q'),
        Key::Named(NamedKey::F3) => Some('R'),
        Key::Named(NamedKey::F4) => Some('S'),
        Key::Named(named) if mode.contains(TermMode::APP_CURSOR) => match named {
            NamedKey::ArrowUp => Some('A'),
            NamedKey::ArrowDown => Some('B'),
            NamedKey::ArrowRight => Some('C'),
            NamedKey::ArrowLeft => Some('D'),
            NamedKey::Home => Some('H'),
            NamedKey::End => Some('F'),
            _ => None,
        },
        _ => None,
    })?;
    if mods.is_empty() {
        Some(format!("\x1bO{ss3}").into_bytes())
    } else if keypad.is_some() {
        Some(
            format!(
                "\x1b[1;{}{ss3}",
                SequenceModifiers::from(Modifiers {
                    state: mods,
                    ..Default::default()
                })
                .encode_esc_sequence()
            )
            .into_bytes(),
        )
    } else {
        None
    }
}
fn legacy_control_or_text(key: &Input<'_>, mods: ModifiersState) -> Option<Vec<u8>> {
    let bytes: &[u8] = match &key.logical_key {
        Key::Named(NamedKey::Enter) => b"\r",
        Key::Named(NamedKey::Space) if mods.control_key() => b"\0",
        Key::Named(NamedKey::Space) => b" ",
        Key::Named(NamedKey::Backspace) => b"\x7f",
        Key::Named(NamedKey::Tab) if mods.shift_key() => b"\x1b[Z",
        Key::Named(NamedKey::Tab) => b"\t",
        Key::Named(NamedKey::Escape) => b"\x1b",
        Key::Character(text) if mods.control_key() => {
            // Like Alacritty's key_input, preserve the platform's actual Ctrl
            // text. Infer C0 bytes only when no supplemental text is available.
            if let Some(text) = key.text_all_modifiers.filter(|text| !text.is_empty()) {
                return Some(text.as_bytes().to_vec());
            }
            let c = single_char(&key.unmodified_key).or_else(|| text.chars().next())?;
            let control = match c {
                '@' | ' ' | '2' => 0,
                'A'..='Z' | 'a'..='z' => (c as u8) & 0x1f,
                '[' | '3' => 27,
                '\\' | '4' => 28,
                ']' | '5' => 29,
                '^' | '6' => 30,
                '_' | '7' | '/' => 31,
                '?' | '8' => 127,
                _ => {
                    return key
                        .text_all_modifiers
                        .or(key.text)
                        .map(|text| text.as_bytes().to_vec());
                }
            };
            return Some(vec![control]);
        }
        Key::Character(text) => key.text.unwrap_or(text).as_bytes(),
        _ => return None,
    };
    Some(bytes.to_vec())
}
fn single_char(key: &Key) -> Option<char> {
    let Key::Character(text) = key else {
        return None;
    };
    let mut chars = text.chars();
    let first = chars.next()?;
    chars.next().is_none().then_some(first)
}

fn base_layout_key(physical: PhysicalKey) -> Option<char> {
    Some(match physical {
        PhysicalKey::Code(code) => match code {
            KeyCode::KeyA => 'a',
            KeyCode::KeyB => 'b',
            KeyCode::KeyC => 'c',
            KeyCode::KeyD => 'd',
            KeyCode::KeyE => 'e',
            KeyCode::KeyF => 'f',
            KeyCode::KeyG => 'g',
            KeyCode::KeyH => 'h',
            KeyCode::KeyI => 'i',
            KeyCode::KeyJ => 'j',
            KeyCode::KeyK => 'k',
            KeyCode::KeyL => 'l',
            KeyCode::KeyM => 'm',
            KeyCode::KeyN => 'n',
            KeyCode::KeyO => 'o',
            KeyCode::KeyP => 'p',
            KeyCode::KeyQ => 'q',
            KeyCode::KeyR => 'r',
            KeyCode::KeyS => 's',
            KeyCode::KeyT => 't',
            KeyCode::KeyU => 'u',
            KeyCode::KeyV => 'v',
            KeyCode::KeyW => 'w',
            KeyCode::KeyX => 'x',
            KeyCode::KeyY => 'y',
            KeyCode::KeyZ => 'z',
            KeyCode::Digit0 => '0',
            KeyCode::Digit1 => '1',
            KeyCode::Digit2 => '2',
            KeyCode::Digit3 => '3',
            KeyCode::Digit4 => '4',
            KeyCode::Digit5 => '5',
            KeyCode::Digit6 => '6',
            KeyCode::Digit7 => '7',
            KeyCode::Digit8 => '8',
            KeyCode::Digit9 => '9',
            KeyCode::Backquote => '`',
            KeyCode::Minus => '-',
            KeyCode::Equal => '=',
            KeyCode::BracketLeft => '[',
            KeyCode::BracketRight => ']',
            KeyCode::Backslash => '\\',
            KeyCode::Semicolon => ';',
            KeyCode::Quote => '\'',
            KeyCode::Comma => ',',
            KeyCode::Period => '.',
            KeyCode::Slash => '/',
            KeyCode::Space => ' ',
            _ => return None,
        },
        _ => return None,
    })
}

/// Build a key's keyboard escape sequence based on the given `key`, `mods`, and `mode`.
///
/// Legacy application cursor/keypad and SS3 sequences are handled above.
#[inline(never)]
fn build_sequence(key: &Input<'_>, mods: Modifiers, mode: TermMode) -> Vec<u8> {
    let modifiers = SequenceModifiers::from(mods);
    let modifiers = if mode.intersects(
        TermMode::REPORT_ALL_KEYS_AS_ESC
            | TermMode::DISAMBIGUATE_ESC_CODES
            | TermMode::REPORT_EVENT_TYPES,
    ) {
        modifiers
    } else {
        SequenceModifiers(modifiers.0 & 0x0f)
    };

    let kitty_seq = mode.intersects(
        TermMode::REPORT_ALL_KEYS_AS_ESC
            | TermMode::DISAMBIGUATE_ESC_CODES
            | TermMode::REPORT_EVENT_TYPES,
    );

    let kitty_encode_all = mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC);
    // The default parameter is 1, so we can omit it.
    let kitty_event_type = mode.contains(TermMode::REPORT_EVENT_TYPES)
        && (key.repeat || key.state == ElementState::Released);

    let context = SequenceBuilder {
        mode,
        modifiers,
        kitty_seq,
        kitty_encode_all,
        kitty_event_type,
    };

    let associated_text = key.text_with_all_modifiers().filter(|text| {
        mode.contains(TermMode::REPORT_ASSOCIATED_TEXT)
            && key.state != ElementState::Released
            && !text.is_empty()
            && !is_control_character(text)
    });

    let sequence_base = context
        .try_build_numpad(key)
        .or_else(|| context.try_build_named_kitty(key))
        .or_else(|| context.try_build_named_normal(key, associated_text.is_some()))
        .or_else(|| context.try_build_control_char_or_mod(key))
        .or_else(|| context.try_build_textual(key, associated_text));

    let (payload, terminator) = match sequence_base {
        Some(SequenceBase {
            payload,
            terminator,
        }) => (payload, terminator),
        _ => return Vec::new(),
    };

    let mut payload = format!("\x1b[{payload}");

    // Add modifiers information.
    if kitty_event_type || !modifiers.is_empty() || associated_text.is_some() {
        payload.push_str(&format!(";{}", modifiers.encode_esc_sequence()));
    }

    // Push event type.
    if kitty_event_type {
        payload.push(':');
        let event_type = match key.state {
            _ if key.repeat => '2',
            ElementState::Pressed => '1',
            ElementState::Released => '3',
        };
        payload.push(event_type);
    }

    if let Some(text) = associated_text {
        let mut codepoints = text.chars().map(u32::from);
        if let Some(codepoint) = codepoints.next() {
            payload.push_str(&format!(";{codepoint}"));
        }
        for codepoint in codepoints {
            payload.push_str(&format!(":{codepoint}"));
        }
    }

    payload.push(terminator.encode_esc_sequence());

    payload.into_bytes()
}

/// Helper to build escape sequence payloads from [`KeyEvent`].
struct SequenceBuilder {
    mode: TermMode,
    /// The emitted sequence should follow the kitty keyboard protocol.
    kitty_seq: bool,
    /// Encode all the keys according to the protocol.
    kitty_encode_all: bool,
    /// Report event types.
    kitty_event_type: bool,
    modifiers: SequenceModifiers,
}

impl SequenceBuilder {
    /// Try building sequence from the event's emitting text.
    fn try_build_textual(
        &self,
        key: &Input<'_>,
        associated_text: Option<&str>,
    ) -> Option<SequenceBase> {
        let character = match key.logical_key.as_ref() {
            Key::Character(character) if self.kitty_seq => character,
            _ => return None,
        };

        if character.chars().count() == 1 {
            let ch = character.chars().next().unwrap();
            // Winit's supplemental key ignores Shift, Caps Lock and Ctrl while
            // retaining the active layout. PhysicalKey supplies the PC-101 base.
            let unshifted = single_char(&key.key_without_modifiers()).unwrap_or(ch);
            let code = u32::from(unshifted);
            let mut payload = code.to_string();
            if self.mode.contains(TermMode::REPORT_ALTERNATE_KEYS) {
                let shifted = (self.modifiers.contains(SequenceModifiers::SHIFT)
                    && ch != unshifted)
                    .then_some(u32::from(ch));
                let base = base_layout_key(key.physical_key).filter(|base| *base != unshifted);
                if shifted.is_some() || base.is_some() {
                    payload.push(':');
                    if let Some(shifted) = shifted {
                        payload.push_str(&shifted.to_string());
                    }
                    if let Some(base) = base {
                        payload.push(':');
                        payload.push_str(&u32::from(base).to_string());
                    }
                }
            }
            Some(SequenceBase::new(payload.into(), SequenceTerminator::Kitty))
        } else if self.kitty_encode_all && associated_text.is_some() {
            // Fallback when need to report text, but we don't have any key associated with this
            // text.
            Some(SequenceBase::new("0".into(), SequenceTerminator::Kitty))
        } else {
            None
        }
    }

    /// Try building from numpad key.
    ///
    /// `None` is returned when the key is neither known nor numpad.
    fn try_build_numpad(&self, key: &Input<'_>) -> Option<SequenceBase> {
        if !self.kitty_seq || key.location != KeyLocation::Numpad {
            return None;
        }

        let base = match key.logical_key.as_ref() {
            Key::Character("0") => "57399",
            Key::Character("1") => "57400",
            Key::Character("2") => "57401",
            Key::Character("3") => "57402",
            Key::Character("4") => "57403",
            Key::Character("5") => "57404",
            Key::Character("6") => "57405",
            Key::Character("7") => "57406",
            Key::Character("8") => "57407",
            Key::Character("9") => "57408",
            Key::Character(".") => "57409",
            Key::Character("/") => "57410",
            Key::Character("*") => "57411",
            Key::Character("-") => "57412",
            Key::Character("+") => "57413",
            Key::Character("=") => "57415",
            Key::Character(",") => "57416",
            Key::Named(named) => match named {
                NamedKey::Enter => "57414",
                NamedKey::ArrowLeft => "57417",
                NamedKey::ArrowRight => "57418",
                NamedKey::ArrowUp => "57419",
                NamedKey::ArrowDown => "57420",
                NamedKey::PageUp => "57421",
                NamedKey::PageDown => "57422",
                NamedKey::Home => "57423",
                NamedKey::End => "57424",
                NamedKey::Insert => "57425",
                NamedKey::Delete => "57426",
                NamedKey::Clear => "57427",
                _ => return None,
            },
            _ => return None,
        };

        Some(SequenceBase::new(base.into(), SequenceTerminator::Kitty))
    }

    /// Try building from [`NamedKey`] using the kitty keyboard protocol encoding
    /// for functional keys.
    fn try_build_named_kitty(&self, key: &Input<'_>) -> Option<SequenceBase> {
        let named = match key.logical_key {
            Key::Named(named) if self.kitty_seq => named,
            _ => return None,
        };

        let (base, terminator) = match named {
            // F3 in kitty protocol diverges from alacritty's terminfo.
            NamedKey::F3 => ("13", SequenceTerminator::Normal('~')),
            NamedKey::F13 => ("57376", SequenceTerminator::Kitty),
            NamedKey::F14 => ("57377", SequenceTerminator::Kitty),
            NamedKey::F15 => ("57378", SequenceTerminator::Kitty),
            NamedKey::F16 => ("57379", SequenceTerminator::Kitty),
            NamedKey::F17 => ("57380", SequenceTerminator::Kitty),
            NamedKey::F18 => ("57381", SequenceTerminator::Kitty),
            NamedKey::F19 => ("57382", SequenceTerminator::Kitty),
            NamedKey::F20 => ("57383", SequenceTerminator::Kitty),
            NamedKey::F21 => ("57384", SequenceTerminator::Kitty),
            NamedKey::F22 => ("57385", SequenceTerminator::Kitty),
            NamedKey::F23 => ("57386", SequenceTerminator::Kitty),
            NamedKey::F24 => ("57387", SequenceTerminator::Kitty),
            NamedKey::F25 => ("57388", SequenceTerminator::Kitty),
            NamedKey::F26 => ("57389", SequenceTerminator::Kitty),
            NamedKey::F27 => ("57390", SequenceTerminator::Kitty),
            NamedKey::F28 => ("57391", SequenceTerminator::Kitty),
            NamedKey::F29 => ("57392", SequenceTerminator::Kitty),
            NamedKey::F30 => ("57393", SequenceTerminator::Kitty),
            NamedKey::F31 => ("57394", SequenceTerminator::Kitty),
            NamedKey::F32 => ("57395", SequenceTerminator::Kitty),
            NamedKey::F33 => ("57396", SequenceTerminator::Kitty),
            NamedKey::F34 => ("57397", SequenceTerminator::Kitty),
            NamedKey::F35 => ("57398", SequenceTerminator::Kitty),
            NamedKey::ScrollLock => ("57359", SequenceTerminator::Kitty),
            NamedKey::PrintScreen => ("57361", SequenceTerminator::Kitty),
            NamedKey::Pause => ("57362", SequenceTerminator::Kitty),
            NamedKey::ContextMenu => ("57363", SequenceTerminator::Kitty),
            NamedKey::MediaPlay => ("57428", SequenceTerminator::Kitty),
            NamedKey::MediaPause => ("57429", SequenceTerminator::Kitty),
            NamedKey::MediaPlayPause => ("57430", SequenceTerminator::Kitty),
            NamedKey::MediaStop => ("57432", SequenceTerminator::Kitty),
            NamedKey::MediaFastForward => ("57433", SequenceTerminator::Kitty),
            NamedKey::MediaRewind => ("57434", SequenceTerminator::Kitty),
            NamedKey::MediaTrackNext => ("57435", SequenceTerminator::Kitty),
            NamedKey::MediaTrackPrevious => ("57436", SequenceTerminator::Kitty),
            NamedKey::MediaRecord => ("57437", SequenceTerminator::Kitty),
            NamedKey::AudioVolumeDown => ("57438", SequenceTerminator::Kitty),
            NamedKey::AudioVolumeUp => ("57439", SequenceTerminator::Kitty),
            NamedKey::AudioVolumeMute => ("57440", SequenceTerminator::Kitty),
            _ => return None,
        };

        Some(SequenceBase::new(base.into(), terminator))
    }

    /// Try building from [`NamedKey`].
    fn try_build_named_normal(
        &self,
        key: &Input<'_>,
        has_associated_text: bool,
    ) -> Option<SequenceBase> {
        let named = match key.logical_key {
            Key::Named(named) => named,
            _ => return None,
        };

        // The default parameter is 1, so we can omit it.
        let one_based =
            if self.modifiers.is_empty() && !self.kitty_event_type && !has_associated_text {
                ""
            } else {
                "1"
            };
        let (base, terminator) = match named {
            NamedKey::PageUp => ("5", SequenceTerminator::Normal('~')),
            NamedKey::PageDown => ("6", SequenceTerminator::Normal('~')),
            NamedKey::Insert => ("2", SequenceTerminator::Normal('~')),
            NamedKey::Delete => ("3", SequenceTerminator::Normal('~')),
            NamedKey::Home => (one_based, SequenceTerminator::Normal('H')),
            NamedKey::End => (one_based, SequenceTerminator::Normal('F')),
            NamedKey::ArrowLeft => (one_based, SequenceTerminator::Normal('D')),
            NamedKey::ArrowRight => (one_based, SequenceTerminator::Normal('C')),
            NamedKey::ArrowUp => (one_based, SequenceTerminator::Normal('A')),
            NamedKey::ArrowDown => (one_based, SequenceTerminator::Normal('B')),
            NamedKey::F1 => (one_based, SequenceTerminator::Normal('P')),
            NamedKey::F2 => (one_based, SequenceTerminator::Normal('Q')),
            NamedKey::F3 => (one_based, SequenceTerminator::Normal('R')),
            NamedKey::F4 => (one_based, SequenceTerminator::Normal('S')),
            NamedKey::F5 => ("15", SequenceTerminator::Normal('~')),
            NamedKey::F6 => ("17", SequenceTerminator::Normal('~')),
            NamedKey::F7 => ("18", SequenceTerminator::Normal('~')),
            NamedKey::F8 => ("19", SequenceTerminator::Normal('~')),
            NamedKey::F9 => ("20", SequenceTerminator::Normal('~')),
            NamedKey::F10 => ("21", SequenceTerminator::Normal('~')),
            NamedKey::F11 => ("23", SequenceTerminator::Normal('~')),
            NamedKey::F12 => ("24", SequenceTerminator::Normal('~')),
            NamedKey::F13 => ("25", SequenceTerminator::Normal('~')),
            NamedKey::F14 => ("26", SequenceTerminator::Normal('~')),
            NamedKey::F15 => ("28", SequenceTerminator::Normal('~')),
            NamedKey::F16 => ("29", SequenceTerminator::Normal('~')),
            NamedKey::F17 => ("31", SequenceTerminator::Normal('~')),
            NamedKey::F18 => ("32", SequenceTerminator::Normal('~')),
            NamedKey::F19 => ("33", SequenceTerminator::Normal('~')),
            NamedKey::F20 => ("34", SequenceTerminator::Normal('~')),
            _ => return None,
        };

        Some(SequenceBase::new(base.into(), terminator))
    }

    /// Try building escape from control characters (e.g. Enter) and modifiers.
    fn try_build_control_char_or_mod(&self, key: &Input<'_>) -> Option<SequenceBase> {
        if !self.kitty_encode_all && !self.kitty_seq {
            return None;
        }

        let named = match key.logical_key {
            Key::Named(named) => named,
            _ => return None,
        };

        let base = match named {
            NamedKey::Tab => "9",
            NamedKey::Enter => "13",
            NamedKey::Escape => "27",
            NamedKey::Space => "32",
            NamedKey::Backspace => "127",
            _ => "",
        };

        // Fail when the key is not a named control character and the active mode prohibits us
        // from encoding modifier keys.
        if !self.kitty_encode_all && base.is_empty() {
            return None;
        }

        let base = match (named, key.location) {
            (NamedKey::Shift, KeyLocation::Left) => "57441",
            (NamedKey::Control, KeyLocation::Left) => "57442",
            (NamedKey::Alt, KeyLocation::Left) => "57443",
            (NamedKey::Super, KeyLocation::Left) => "57444",
            (NamedKey::Hyper, KeyLocation::Left) => "57445",
            (NamedKey::Meta, KeyLocation::Left) => "57446",
            (NamedKey::Shift, _) => "57447",
            (NamedKey::Control, _) => "57448",
            (NamedKey::Alt, _) => "57449",
            (NamedKey::Super, _) => "57450",
            (NamedKey::Hyper, _) => "57451",
            (NamedKey::Meta, _) => "57452",
            (NamedKey::CapsLock, _) => "57358",
            (NamedKey::NumLock, _) => "57360",
            _ => base,
        };

        // Producers supply the post-transition modifier state, including any
        // modifier still held on the other side of the keyboard.
        if base.is_empty() {
            None
        } else {
            Some(SequenceBase::new(base.into(), SequenceTerminator::Kitty))
        }
    }
}

struct SequenceBase {
    /// The base of the payload, which is the `number` and optionally an alt base from the kitty
    /// spec.
    payload: Cow<'static, str>,
    terminator: SequenceTerminator,
}

impl SequenceBase {
    fn new(payload: Cow<'static, str>, terminator: SequenceTerminator) -> Self {
        Self {
            payload,
            terminator,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SequenceTerminator {
    /// The normal key esc sequence terminator defined by xterm/dec.
    Normal(char),
    /// The terminator is for kitty escape sequence.
    Kitty,
}

impl SequenceTerminator {
    fn encode_esc_sequence(self) -> char {
        match self {
            SequenceTerminator::Normal(char) => char,
            SequenceTerminator::Kitty => 'u',
        }
    }
}

/// Kitty/xterm modifier bits; no new dependency is needed for this tiny mask.
#[derive(Clone, Copy)]
struct SequenceModifiers(u16);
impl SequenceModifiers {
    const SHIFT: Self = Self(1);
    const ALT: Self = Self(2);
    const CONTROL: Self = Self(4);
    const SUPER: Self = Self(8);
    const HYPER: Self = Self(16);
    const META: Self = Self(32);
    const CAPS: Self = Self(64);
    const NUM: Self = Self(128);
    fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
    fn set(&mut self, flag: Self, on: bool) {
        if on {
            self.0 |= flag.0;
        } else {
            self.0 &= !flag.0;
        }
    }
    fn is_empty(self) -> bool {
        self.0 == 0
    }
    fn encode_esc_sequence(self) -> u16 {
        self.0 + 1
    }
}
impl From<Modifiers> for SequenceModifiers {
    fn from(mods: Modifiers) -> Self {
        let mut bits = Self(0);
        bits.set(Self::SHIFT, mods.state.shift_key());
        bits.set(Self::ALT, mods.state.alt_key());
        bits.set(Self::CONTROL, mods.state.control_key());
        bits.set(Self::SUPER, mods.state.super_key());
        bits.set(Self::HYPER, mods.state.hyper_key());
        bits.set(Self::META, mods.state.meta_key());
        bits.set(Self::CAPS, mods.locks.caps);
        bits.set(Self::NUM, mods.locks.num);
        bits
    }
}
/// Associated text excludes every C0/C1/DEL scalar, including multi-byte C1.
fn is_control_character(text: &str) -> bool {
    text.chars().any(char::is_control)
}

#[cfg(test)]
pub(crate) fn assert_invariant() {
    tests::flag_matrix();
    tests::legacy_vectors();
    tests::extended_vectors();
    tests::terminal_stack();
    tests::modifier_keys();
    tests::source_guard();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(logical: Key, physical: KeyCode, mods: ModifiersState) -> Input<'static> {
        Input {
            unmodified_key: logical.clone(),
            logical_key: logical,
            physical_key: PhysicalKey::Code(physical),
            text: None,
            text_all_modifiers: None,
            location: KeyLocation::Standard,
            state: ElementState::Pressed,
            repeat: false,
            modifiers: Modifiers {
                state: mods,
                ..Default::default()
            },
        }
    }
    fn named(named: NamedKey) -> Input<'static> {
        key(Key::Named(named), KeyCode::F5, ModifiersState::empty())
    }
    fn character(text: &'static str, physical: KeyCode) -> Input<'static> {
        let mut input = key(
            Key::Character(text.into()),
            physical,
            ModifiersState::empty(),
        );
        input.text = Some(text);
        input.text_all_modifiers = Some(text);
        input
    }
    fn mode(flags: u32) -> u64 {
        u64::from(flags << 18)
    }
    fn check(input: &Input<'_>, flags: u32, expected: Option<&str>) {
        assert_eq!(
            encode(input, mode(flags)),
            expected.map(|s| s.as_bytes().to_vec()),
            "flags={flags}, key={input:?}"
        );
    }
    // Golden protocol vectors for every one of the 32 progressive flag sets.
    // Only the optional fields vary; expected codepoints/modifiers are literal.
    pub(super) fn flag_matrix() {
        let mut ctrl_shift_a = character("A", KeyCode::KeyA);
        ctrl_shift_a.unmodified_key = Key::Character("a".into());
        ctrl_shift_a.modifiers.state = ModifiersState::CONTROL | ModifiersState::SHIFT;
        ctrl_shift_a.text_all_modifiers = Some("\x01");
        let mut tab = named(NamedKey::Tab);
        tab.modifiers.state = ModifiersState::SHIFT;
        let mut keypad = character("1", KeyCode::Numpad1);
        keypad.location = KeyLocation::Numpad;
        for flags in 0..32 {
            let enhanced = flags & (1 | 8) != 0;
            let text = flags & (8 | 16) == (8 | 16);
            check(
                &named(NamedKey::Escape),
                flags,
                Some(if enhanced { "\x1b[27u" } else { "\x1b" }),
            );
            check(
                &ctrl_shift_a,
                flags,
                Some(if enhanced {
                    if flags & 4 != 0 {
                        "\x1b[97:65;6u"
                    } else {
                        "\x1b[97;6u"
                    }
                } else {
                    "\x01"
                }),
            );
            check(
                &tab,
                flags,
                Some(if enhanced { "\x1b[9;2u" } else { "\x1b[Z" }),
            );
            check(&named(NamedKey::F5), flags, Some("\x1b[15~"));
            check(
                &keypad,
                flags,
                Some(if enhanced {
                    // Like the pinned port, an already escaped key carries
                    // text with flag 16. Kitty leaves 16-without-8 unspecified.
                    if flags & 16 != 0 {
                        "\x1b[57400;1;49u"
                    } else {
                        "\x1b[57400u"
                    }
                } else {
                    "1"
                }),
            );
            let a = character("a", KeyCode::KeyA);
            let mut alt = a.clone();
            alt.modifiers.state = ModifiersState::ALT;
            check(
                &alt,
                flags,
                Some(if enhanced {
                    if flags & 16 != 0 {
                        "\x1b[97;3;97u"
                    } else {
                        "\x1b[97;3u"
                    }
                } else {
                    "\x1ba"
                }),
            );
            check(
                &a,
                flags,
                Some(if flags & 8 != 0 {
                    if text { "\x1b[97;1;97u" } else { "\x1b[97u" }
                } else {
                    "a"
                }),
            );
            let mut repeat = a.clone();
            repeat.repeat = true;
            check(
                &repeat,
                flags,
                Some(if flags & 8 != 0 {
                    match (flags & 2 != 0, text) {
                        (true, true) => "\x1b[97;1:2;97u",
                        (true, false) => "\x1b[97;1:2u",
                        (false, true) => "\x1b[97;1;97u",
                        (false, false) => "\x1b[97u",
                    }
                } else {
                    "a"
                }),
            );
            let mut release = a;
            release.state = ElementState::Released;
            check(&release, flags, (flags & 2 != 0).then_some("\x1b[97;1:3u"));
            let mut release_f5 = named(NamedKey::F5);
            release_f5.state = ElementState::Released;
            check(
                &release_f5,
                flags,
                (flags & 2 != 0).then_some("\x1b[15;1:3~"),
            );
        }
    }
    pub(super) fn legacy_vectors() {
        for (key, expected) in [
            (NamedKey::Enter, "\r"),
            (NamedKey::Tab, "\t"),
            (NamedKey::Escape, "\x1b"),
            (NamedKey::Backspace, "\x7f"),
            (NamedKey::ArrowUp, "\x1b[A"),
            (NamedKey::ArrowDown, "\x1b[B"),
            (NamedKey::ArrowRight, "\x1b[C"),
            (NamedKey::ArrowLeft, "\x1b[D"),
            (NamedKey::Home, "\x1b[H"),
            (NamedKey::End, "\x1b[F"),
            (NamedKey::Insert, "\x1b[2~"),
            (NamedKey::Delete, "\x1b[3~"),
            (NamedKey::PageUp, "\x1b[5~"),
            (NamedKey::PageDown, "\x1b[6~"),
            (NamedKey::F1, "\x1bOP"),
            (NamedKey::F2, "\x1bOQ"),
            (NamedKey::F3, "\x1bOR"),
            (NamedKey::F4, "\x1bOS"),
            (NamedKey::F5, "\x1b[15~"),
            (NamedKey::F6, "\x1b[17~"),
            (NamedKey::F7, "\x1b[18~"),
            (NamedKey::F8, "\x1b[19~"),
            (NamedKey::F9, "\x1b[20~"),
            (NamedKey::F10, "\x1b[21~"),
            (NamedKey::F11, "\x1b[23~"),
            (NamedKey::F12, "\x1b[24~"),
        ] {
            check(&named(key), 0, Some(expected));
        }
        for (key, expected) in [
            (NamedKey::ArrowUp, "\x1bOA"),
            (NamedKey::ArrowDown, "\x1bOB"),
            (NamedKey::ArrowRight, "\x1bOC"),
            (NamedKey::ArrowLeft, "\x1bOD"),
            (NamedKey::Home, "\x1bOH"),
            (NamedKey::End, "\x1bOF"),
        ] {
            assert_eq!(
                encode(&named(key), TermMode::APP_CURSOR.bits() as u64),
                Some(expected.as_bytes().to_vec())
            );
        }
        for (key, mods, expected) in [
            (NamedKey::ArrowUp, ModifiersState::CONTROL, "\x1b[1;5A"),
            (
                NamedKey::ArrowLeft,
                ModifiersState::SHIFT | ModifiersState::CONTROL,
                "\x1b[1;6D",
            ),
            (NamedKey::F5, ModifiersState::CONTROL, "\x1b[15;5~"),
            (NamedKey::Home, ModifiersState::SHIFT, "\x1b[1;2H"),
            (NamedKey::Tab, ModifiersState::SHIFT, "\x1b[Z"),
            (NamedKey::ArrowUp, ModifiersState::ALT, "\x1b\x1b[A"),
        ] {
            let mut input = named(key);
            input.modifiers.state = mods;
            check(&input, 0, Some(expected));
        }
        for (physical, value, expected) in [
            (KeyCode::Numpad1, "1", "\x1bOq"),
            (KeyCode::NumpadAdd, "+", "\x1bOk"),
            (KeyCode::NumpadDecimal, ".", "\x1bOn"),
        ] {
            let input = character(value, physical);
            assert_eq!(
                encode(&input, TermMode::APP_KEYPAD.bits() as u64),
                Some(expected.as_bytes().to_vec())
            );
        }
        for (value, byte) in [
            ("a", 1),
            ("c", 3),
            ("z", 26),
            ("[", 27),
            ("\\", 28),
            ("]", 29),
            ("^", 30),
            ("_", 31),
            ("?", 127),
            (" ", 0),
        ] {
            let mut input = character(value, KeyCode::KeyA);
            input.modifiers.state = ModifiersState::CONTROL;
            input.text_all_modifiers = None; // Exercise the missing-platform-text fallback.
            assert_eq!(encode(&input, 0), Some(vec![byte]));
        }
        check(&character("界", KeyCode::KeyA), 0, Some("界"));
        // The OS's actual Ctrl text is authoritative, including combinations
        // whose unmodified key alone would imply a different C0 byte.
        let mut control = character("?", KeyCode::Slash);
        control.unmodified_key = Key::Character("/".into());
        control.modifiers.state = ModifiersState::CONTROL | ModifiersState::SHIFT;
        control.text_all_modifiers = Some("\x7f");
        check(&control, 0, Some("\x7f"));
        control.text_all_modifiers = Some("?");
        check(&control, 0, Some("?"));
        control.text_all_modifiers = None;
        check(&control, 0, Some("\x1f"));
        let mut alt = character("a", KeyCode::KeyA);
        alt.modifiers.state = ModifiersState::ALT;
        check(&alt, 0, Some("\x1ba"));
    }
    pub(super) fn extended_vectors() {
        let mut punctuation = character("!", KeyCode::Digit1);
        punctuation.unmodified_key = Key::Character("1".into());
        punctuation.modifiers.state = ModifiersState::SHIFT;
        check(&punctuation, 8 | 4, Some("\x1b[49:33;2u"));
        let mut layout = character("Ф", KeyCode::KeyA);
        layout.unmodified_key = Key::Character("ф".into());
        layout.modifiers.state = ModifiersState::SHIFT;
        check(&layout, 8 | 4, Some("\x1b[1092:1060:97;2u"));
        layout.logical_key = Key::Character("ф".into());
        layout.modifiers.state = ModifiersState::empty();
        check(&layout, 8 | 4, Some("\x1b[1092::97u"));
        let mut a = character("A", KeyCode::KeyA);
        a.unmodified_key = Key::Character("a".into());
        a.modifiers.locks = Locks {
            caps: true,
            num: true,
        };
        check(&a, 8, Some("\x1b[97;193u"));
        check(&a, 0, Some("A"));
        let mut text = character("é", KeyCode::KeyE);
        text.text_all_modifiers = Some("e\u{301}");
        check(&text, 8 | 16, Some("\x1b[233;1;101:769u"));
        for control in ["\0", "\x7f", "\u{80}", "ok\u{9f}"] {
            text.text_all_modifiers = Some(control);
            check(&text, 8 | 16, Some("\x1b[233u"));
        }
        let mut composed = character("multi", KeyCode::KeyA);
        composed.text_all_modifiers = Some("😀");
        check(&composed, 8 | 16, Some("\x1b[0;1;128512u"));
        for (named_key, expected) in [
            (NamedKey::F3, "\x1b[13~"),
            (NamedKey::F13, "\x1b[57376u"),
            (NamedKey::F35, "\x1b[57398u"),
            (NamedKey::MediaPlay, "\x1b[57428u"),
            (NamedKey::AudioVolumeMute, "\x1b[57440u"),
            (NamedKey::CapsLock, "\x1b[57358u"),
        ] {
            check(&named(named_key), 8, Some(expected));
        }
        for (logical, physical, expected) in [
            (Key::Character("1".into()), KeyCode::Numpad1, "\x1b[57400u"),
            (
                Key::Named(NamedKey::Enter),
                KeyCode::NumpadEnter,
                "\x1b[57414u",
            ),
            (
                Key::Character(",".into()),
                KeyCode::NumpadComma,
                "\x1b[57416u",
            ),
            (
                Key::Named(NamedKey::ArrowLeft),
                KeyCode::Numpad4,
                "\x1b[57417u",
            ),
            (Key::Named(NamedKey::Clear), KeyCode::Numpad5, "\x1b[57427u"),
        ] {
            let mut input = key(logical, physical, ModifiersState::empty());
            input.location = KeyLocation::Numpad;
            check(&input, 1, Some(expected));
        }
        for named_key in [NamedKey::Enter, NamedKey::Tab, NamedKey::Backspace] {
            let mut input = named(named_key);
            input.state = ElementState::Released;
            check(&input, 2, None);
            assert!(encode(&input, mode(2 | 8)).is_some());
        }
    }
    pub(super) fn terminal_stack() {
        use crate::{Event, Term};
        let mut term = Term::new(10, 2, 0);
        for flags in 0..32 {
            term.advance(format!("\x1b[>{flags}u\x1b[?u").as_bytes());
            assert!(term.take_events().into_iter().any(
                |event| matches!(event,Event::PtyWrite(value) if value == format!("\x1b[?{flags}u"))
            ));
            let modes = term.mode().bits() as u64;
            assert_eq!(modes & mode(31), mode(flags));
            check(
                &named(NamedKey::Escape),
                flags,
                Some(if flags & (1 | 8) != 0 {
                    "\x1b[27u"
                } else {
                    "\x1b"
                }),
            );
            assert_eq!(
                encode(&named(NamedKey::Escape), modes),
                encode(&named(NamedKey::Escape), mode(flags))
            );
            term.advance(b"\x1b[>31u\x1b[<u\x1b[?u");
            assert!(term.take_events().into_iter().any(
                |event| matches!(event,Event::PtyWrite(value) if value == format!("\x1b[?{flags}u"))
            ));
            term.advance(b"\x1b[<u\x1b[?u");
            assert_eq!(term.mode().bits() as u64 & mode(31), 0);
            assert!(
                term.take_events()
                    .into_iter()
                    .any(|event| matches!(event,Event::PtyWrite(value) if value == "\x1b[?0u"))
            );
        }
        // The real CSI >31u path supplies every flag to the app's cached Screen.
        term.advance(b"\x1b[>31u");
        let modes = term.mode().bits() as u64;
        assert_eq!(
            encode(&character("a", KeyCode::KeyA), modes),
            Some(b"\x1b[97;1;97u".to_vec())
        );
        term.advance(b"\x1b[?1049h\x1b[?u");
        assert!(
            term.take_events()
                .into_iter()
                .any(|event| matches!(event,Event::PtyWrite(value) if value=="\x1b[?0u"))
        );
        term.advance(b"\x1b[>1u\x1b[?1049l\x1b[?u");
        assert!(
            term.take_events()
                .into_iter()
                .any(|event| matches!(event,Event::PtyWrite(value) if value=="\x1b[?31u"))
        );
    }
    pub(super) fn modifier_keys() {
        let mut modifier = named(NamedKey::Control);
        modifier.location = KeyLocation::Left;
        modifier.physical_key = PhysicalKey::Code(KeyCode::ControlLeft);
        modifier.modifiers.state = ModifiersState::CONTROL;
        check(&modifier, 8, Some("\x1b[57442;5u"));
        modifier.state = ElementState::Released;
        modifier.modifiers.state = ModifiersState::empty();
        check(&modifier, 8 | 2, Some("\x1b[57442;1:3u"));
        let mut super_key = named(NamedKey::Super);
        super_key.location = KeyLocation::Left;
        super_key.modifiers.state = ModifiersState::SUPER;
        check(&super_key, 8, Some("\x1b[57444;9u"));
        let mut all = character("a", KeyCode::KeyA);
        all.modifiers.state = ModifiersState::all();
        all.modifiers.locks = Locks {
            caps: true,
            num: true,
        };
        check(&all, 8, Some("\x1b[97;256u"));
    }
    #[allow(
        clippy::disallowed_methods,
        reason = "offline invariant scans app sources; this function is test-only, never a UI callback"
    )]
    pub(super) fn source_guard() {
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
        scan(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../bboxy450-term/src"));
        let adapter = include_str!("../../bboxy450-term/src/keyboard.rs");
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
        let source = include_str!("../../bboxy450-term/src/main.rs");
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
}
