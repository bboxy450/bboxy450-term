// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Embedded, synthetic PTY fixtures. Playback performs no input or filesystem reads.

pub mod options;

/// Every scene starts from a clean primary screen and ends with normal SGR,
/// full-height scrolling, synchronized output off and a visible cursor.
pub const START: &[u8] = b"\x1b[?2026l\x1b[?1049l\x1b[0 q\x1b[0m\x1b[r\x1b[2J\x1b[H";
pub const END: &[u8] = b"\x1b[0m\x1b[r\x1b[?2026l\x1b[?25h\x1b[24;1H";
pub const COLS: usize = 80;
pub const ROWS: usize = 24;

pub struct Scene {
    pub name: &'static str,
    pub description: &'static str,
    pub bytes: &'static [u8],
}

pub const SCENES: &[Scene] = &[
    Scene {
        name: "codex",
        description: "synthetic Codex tools, code and diffs",
        bytes: include_bytes!("../corpus/codex.ansi"),
    },
    Scene {
        name: "claude",
        description: "synthetic Claude tool calls and nested work",
        bytes: include_bytes!("../corpus/claude.ansi"),
    },
    Scene {
        name: "palette",
        description: "16/256/RGB colours and all text decorations",
        bytes: include_bytes!("../corpus/palette.ansi"),
    },
    Scene {
        name: "unicode",
        description: "CJK, ZWJ emoji, combining marks and procedural glyphs",
        bytes: include_bytes!("../corpus/unicode.ansi"),
    },
    Scene {
        name: "progress",
        description: "CR spinners, progress bars and cursor rewrites",
        bytes: include_bytes!("../corpus/progress.ansi"),
    },
    Scene {
        name: "alternate",
        description: "alternate-screen tool with primary-screen restoration",
        bytes: include_bytes!("../corpus/alternate.ansi"),
    },
    Scene {
        name: "scroll-links-sync",
        description: "scroll margins, OSC 8 links and mode 2026",
        bytes: include_bytes!("../corpus/scroll-links-sync.ansi"),
    },
    Scene {
        name: "osc7501-claude",
        description: "original Claude status capture (482 bytes) plus synthetic display",
        bytes: include_bytes!("../corpus/osc7501-claude.ansi"),
    },
    Scene {
        name: "osc7501-preflight",
        description: "original Claude preflight (303 bytes) plus synthetic display",
        bytes: include_bytes!("../corpus/osc7501-preflight.ansi"),
    },
    Scene {
        name: "osc7501-codex",
        description: "empty Codex captures, labelled synthetic protocol coverage",
        bytes: include_bytes!("../corpus/osc7501-codex.ansi"),
    },
    Scene {
        name: "cursor-beam",
        description: "steady focused beam cursor (DECSCUSR 6)",
        bytes: include_bytes!("../corpus/cursor-beam.ansi"),
    },
    Scene {
        name: "cursor-underline",
        description: "steady focused underline cursor (DECSCUSR 4)",
        bytes: include_bytes!("../corpus/cursor-underline.ansi"),
    },
];

/// Frozen expectations, never calculated by the terminal under test at runtime.
pub const SCREEN_HASHES: &[u8] = include_bytes!("../corpus/screen-hashes.txt");

pub fn expected_hash(name: &str) -> Option<&'static str> {
    std::str::from_utf8(SCREEN_HASHES)
        .ok()?
        .lines()
        .find_map(|line| {
            let (scene, hash) = line.split_once(' ')?;
            (scene == name).then_some(hash)
        })
}

pub fn scene_bytes(scene: &Scene) -> Vec<u8> {
    [START, scene.bytes, END].concat()
}

/// Seed changes scene order and timing; a named scene's byte stream is fixed.
pub fn ordered_scenes(name: Option<&str>, seed: u64) -> Vec<&'static Scene> {
    if let Some(name) = name {
        return SCENES.iter().filter(|scene| scene.name == name).collect();
    }
    let mut scenes: Vec<_> = SCENES.iter().collect();
    let mut rng = Rng(seed);
    for i in (1..scenes.len()).rev() {
        scenes.swap(i, rng.next_u64() as usize % (i + 1));
    }
    scenes
}

/// SplitMix64, with specified wrapping arithmetic for reproducible schedules.
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

/// Non-cryptographic fingerprint of the semantic screen schema in README.md.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}
