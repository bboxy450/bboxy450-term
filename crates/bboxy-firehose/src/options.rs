// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rate {
    Max,
    Bytes(u64),
    Lines(u64),
}

impl Rate {
    pub fn parse(value: &str) -> Result<Self, String> {
        if value == "max" {
            return Ok(Self::Max);
        }
        let split = value
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(value.len());
        let number: u64 = value[..split]
            .parse()
            .map_err(|_| format!("invalid rate: {value}"))?;
        if number == 0 {
            return Err("rate must be positive".into());
        }
        let (multiplier, lines) = match &value[split..] {
            "" | "B/s" | "bytes/s" => (1, false),
            "kB/s" => (1_000, false),
            "MB/s" => (1_000_000, false),
            "KiB/s" => (1_024, false),
            "MiB/s" => (1_048_576, false),
            "l/s" | "lines/s" => (1, true),
            _ => return Err(format!("invalid rate unit: {value}")),
        };
        let rate = number.checked_mul(multiplier).ok_or("rate overflow")?;
        Ok(if lines {
            Self::Lines(rate)
        } else {
            Self::Bytes(rate)
        })
    }

    pub fn elapsed_for(self, bytes: u128, lines: u128) -> Duration {
        let (units, rate) = match self {
            Self::Max => return Duration::ZERO,
            Self::Bytes(rate) => (bytes, rate),
            Self::Lines(rate) => (lines, rate),
        };
        let seconds = units / u128::from(rate);
        Duration::new(
            seconds.min(u128::from(u64::MAX)) as u64,
            ((units % u128::from(rate)) * 1_000_000_000 / u128::from(rate)) as u32,
        )
    }
}

#[derive(Debug)]
pub struct Options {
    pub rate: Rate,
    pub real: bool,
    pub loops: Option<u64>,
    pub seed: u64,
    pub scene: Option<String>,
    pub check: bool,
    pub list: bool,
    pub help: bool,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            rate: Rate::Max,
            real: false,
            loops: Some(1),
            seed: 1,
            scene: None,
            check: false,
            list: false,
            help: false,
        };
        let mut args = args.into_iter().peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--rate" => {
                    options.rate = Rate::parse(&args.next().ok_or("--rate needs a value")?)?
                }
                "--pace" => {
                    options.real = match args.next().as_deref() {
                        Some("real") => true,
                        Some("flood") => false,
                        _ => return Err("--pace must be real or flood".into()),
                    }
                }
                "--loop" => {
                    options.loops = None;
                    if args.peek().is_some_and(|arg| !arg.starts_with('-')) {
                        let count = args
                            .next()
                            .unwrap()
                            .parse()
                            .map_err(|_| "invalid loop count")?;
                        if count == 0 {
                            return Err("loop count must be positive".into());
                        }
                        options.loops = Some(count);
                    }
                }
                "--seed" => {
                    let seed = args.next().ok_or("--seed needs a value")?;
                    options.seed = match seed.strip_prefix("0x") {
                        Some(hex) => u64::from_str_radix(hex, 16),
                        None => seed.parse(),
                    }
                    .map_err(|_| "invalid seed")?;
                }
                "--scene" => options.scene = Some(args.next().ok_or("--scene needs a value")?),
                "--check" => options.check = true,
                "--list" => options.list = true,
                "--help" | "-h" => options.help = true,
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }
        if let Some(name) = &options.scene
            && !crate::SCENES.iter().any(|scene| scene.name == name)
        {
            return Err(format!("unknown scene: {name}; use --list"));
        }
        Ok(options)
    }
}
