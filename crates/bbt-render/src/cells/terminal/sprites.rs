//! Cell-size sprites: font-independent edges join without missing glyphs or clipping.
use super::{CursorShape, Underline};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sprite {
    Character(char),
    Underline(Underline),
    Strike,
    Cursor(CursorShape, bool),
}

pub fn character(ch: char) -> bool {
    matches!(ch as u32, 0x2500..=0x259f | 0x2800..=0x28ff | 0xe0a0 | 0xe0b0..=0xe0b3)
}

// Up, right, down, left; 1=light, 2=heavy, 3=double.
fn arms(ch: char) -> Option<[u8; 4]> {
    let cp = ch as u32;
    if !(0x2500..=0x257f).contains(&cp) {
        return None;
    }
    let arms = BOX_ARMS[(cp - 0x2500) as usize];
    (arms != [0; 4]).then_some(arms)
}

// Generated from Unicode character names: directions and light/heavy/double weights.
const BOX_ARMS: [[u8; 4]; 128] = [
    [0, 1, 0, 1], // ─ LIGHT HORIZONTAL
    [0, 2, 0, 2], // ━ HEAVY HORIZONTAL
    [1, 0, 1, 0], // │ LIGHT VERTICAL
    [2, 0, 2, 0], // ┃ HEAVY VERTICAL
    [0, 1, 0, 1], // ┄ LIGHT TRIPLE DASH HORIZONTAL
    [0, 2, 0, 2], // ┅ HEAVY TRIPLE DASH HORIZONTAL
    [1, 0, 1, 0], // ┆ LIGHT TRIPLE DASH VERTICAL
    [2, 0, 2, 0], // ┇ HEAVY TRIPLE DASH VERTICAL
    [0, 1, 0, 1], // ┈ LIGHT QUADRUPLE DASH HORIZONTAL
    [0, 2, 0, 2], // ┉ HEAVY QUADRUPLE DASH HORIZONTAL
    [1, 0, 1, 0], // ┊ LIGHT QUADRUPLE DASH VERTICAL
    [2, 0, 2, 0], // ┋ HEAVY QUADRUPLE DASH VERTICAL
    [0, 1, 1, 0], // ┌ LIGHT DOWN AND RIGHT
    [0, 2, 1, 0], // ┍ DOWN LIGHT AND RIGHT HEAVY
    [0, 1, 2, 0], // ┎ DOWN HEAVY AND RIGHT LIGHT
    [0, 2, 2, 0], // ┏ HEAVY DOWN AND RIGHT
    [0, 0, 1, 1], // ┐ LIGHT DOWN AND LEFT
    [0, 0, 1, 2], // ┑ DOWN LIGHT AND LEFT HEAVY
    [0, 0, 2, 1], // ┒ DOWN HEAVY AND LEFT LIGHT
    [0, 0, 2, 2], // ┓ HEAVY DOWN AND LEFT
    [1, 1, 0, 0], // └ LIGHT UP AND RIGHT
    [1, 2, 0, 0], // ┕ UP LIGHT AND RIGHT HEAVY
    [2, 1, 0, 0], // ┖ UP HEAVY AND RIGHT LIGHT
    [2, 2, 0, 0], // ┗ HEAVY UP AND RIGHT
    [1, 0, 0, 1], // ┘ LIGHT UP AND LEFT
    [1, 0, 0, 2], // ┙ UP LIGHT AND LEFT HEAVY
    [2, 0, 0, 1], // ┚ UP HEAVY AND LEFT LIGHT
    [2, 0, 0, 2], // ┛ HEAVY UP AND LEFT
    [1, 1, 1, 0], // ├ LIGHT VERTICAL AND RIGHT
    [1, 2, 1, 0], // ┝ VERTICAL LIGHT AND RIGHT HEAVY
    [2, 1, 1, 0], // ┞ UP HEAVY AND RIGHT DOWN LIGHT
    [1, 1, 2, 0], // ┟ DOWN HEAVY AND RIGHT UP LIGHT
    [2, 1, 2, 0], // ┠ VERTICAL HEAVY AND RIGHT LIGHT
    [2, 2, 1, 0], // ┡ DOWN LIGHT AND RIGHT UP HEAVY
    [1, 2, 2, 0], // ┢ UP LIGHT AND RIGHT DOWN HEAVY
    [2, 2, 2, 0], // ┣ HEAVY VERTICAL AND RIGHT
    [1, 0, 1, 1], // ┤ LIGHT VERTICAL AND LEFT
    [1, 0, 1, 2], // ┥ VERTICAL LIGHT AND LEFT HEAVY
    [2, 0, 1, 1], // ┦ UP HEAVY AND LEFT DOWN LIGHT
    [1, 0, 2, 1], // ┧ DOWN HEAVY AND LEFT UP LIGHT
    [2, 0, 2, 1], // ┨ VERTICAL HEAVY AND LEFT LIGHT
    [2, 0, 1, 2], // ┩ DOWN LIGHT AND LEFT UP HEAVY
    [1, 0, 2, 2], // ┪ UP LIGHT AND LEFT DOWN HEAVY
    [2, 0, 2, 2], // ┫ HEAVY VERTICAL AND LEFT
    [0, 1, 1, 1], // ┬ LIGHT DOWN AND HORIZONTAL
    [0, 1, 1, 2], // ┭ LEFT HEAVY AND RIGHT DOWN LIGHT
    [0, 2, 1, 1], // ┮ RIGHT HEAVY AND LEFT DOWN LIGHT
    [0, 2, 1, 2], // ┯ DOWN LIGHT AND HORIZONTAL HEAVY
    [0, 1, 2, 1], // ┰ DOWN HEAVY AND HORIZONTAL LIGHT
    [0, 1, 2, 2], // ┱ RIGHT LIGHT AND LEFT DOWN HEAVY
    [0, 2, 2, 1], // ┲ LEFT LIGHT AND RIGHT DOWN HEAVY
    [0, 2, 2, 2], // ┳ HEAVY DOWN AND HORIZONTAL
    [1, 1, 0, 1], // ┴ LIGHT UP AND HORIZONTAL
    [1, 1, 0, 2], // ┵ LEFT HEAVY AND RIGHT UP LIGHT
    [1, 2, 0, 1], // ┶ RIGHT HEAVY AND LEFT UP LIGHT
    [1, 2, 0, 2], // ┷ UP LIGHT AND HORIZONTAL HEAVY
    [2, 1, 0, 1], // ┸ UP HEAVY AND HORIZONTAL LIGHT
    [2, 1, 0, 2], // ┹ RIGHT LIGHT AND LEFT UP HEAVY
    [2, 2, 0, 1], // ┺ LEFT LIGHT AND RIGHT UP HEAVY
    [2, 2, 0, 2], // ┻ HEAVY UP AND HORIZONTAL
    [1, 1, 1, 1], // ┼ LIGHT VERTICAL AND HORIZONTAL
    [1, 1, 1, 2], // ┽ LEFT HEAVY AND RIGHT VERTICAL LIGHT
    [1, 2, 1, 1], // ┾ RIGHT HEAVY AND LEFT VERTICAL LIGHT
    [1, 2, 1, 2], // ┿ VERTICAL LIGHT AND HORIZONTAL HEAVY
    [2, 1, 1, 1], // ╀ UP HEAVY AND DOWN HORIZONTAL LIGHT
    [1, 1, 2, 1], // ╁ DOWN HEAVY AND UP HORIZONTAL LIGHT
    [2, 1, 2, 1], // ╂ VERTICAL HEAVY AND HORIZONTAL LIGHT
    [2, 1, 1, 2], // ╃ LEFT UP HEAVY AND RIGHT DOWN LIGHT
    [2, 2, 1, 1], // ╄ RIGHT UP HEAVY AND LEFT DOWN LIGHT
    [1, 1, 2, 2], // ╅ LEFT DOWN HEAVY AND RIGHT UP LIGHT
    [1, 2, 2, 1], // ╆ RIGHT DOWN HEAVY AND LEFT UP LIGHT
    [2, 2, 1, 2], // ╇ DOWN LIGHT AND UP HORIZONTAL HEAVY
    [1, 2, 2, 2], // ╈ UP LIGHT AND DOWN HORIZONTAL HEAVY
    [2, 1, 2, 2], // ╉ RIGHT LIGHT AND LEFT VERTICAL HEAVY
    [2, 2, 2, 1], // ╊ LEFT LIGHT AND RIGHT VERTICAL HEAVY
    [2, 2, 2, 2], // ╋ HEAVY VERTICAL AND HORIZONTAL
    [0, 1, 0, 1], // ╌ LIGHT DOUBLE DASH HORIZONTAL
    [0, 2, 0, 2], // ╍ HEAVY DOUBLE DASH HORIZONTAL
    [1, 0, 1, 0], // ╎ LIGHT DOUBLE DASH VERTICAL
    [2, 0, 2, 0], // ╏ HEAVY DOUBLE DASH VERTICAL
    [0, 3, 0, 3], // ═ DOUBLE HORIZONTAL
    [3, 0, 3, 0], // ║ DOUBLE VERTICAL
    [0, 3, 1, 0], // ╒ DOWN SINGLE AND RIGHT DOUBLE
    [0, 1, 3, 0], // ╓ DOWN DOUBLE AND RIGHT SINGLE
    [0, 3, 3, 0], // ╔ DOUBLE DOWN AND RIGHT
    [0, 0, 1, 3], // ╕ DOWN SINGLE AND LEFT DOUBLE
    [0, 0, 3, 1], // ╖ DOWN DOUBLE AND LEFT SINGLE
    [0, 0, 3, 3], // ╗ DOUBLE DOWN AND LEFT
    [1, 3, 0, 0], // ╘ UP SINGLE AND RIGHT DOUBLE
    [3, 1, 0, 0], // ╙ UP DOUBLE AND RIGHT SINGLE
    [3, 3, 0, 0], // ╚ DOUBLE UP AND RIGHT
    [1, 0, 0, 3], // ╛ UP SINGLE AND LEFT DOUBLE
    [3, 0, 0, 1], // ╜ UP DOUBLE AND LEFT SINGLE
    [3, 0, 0, 3], // ╝ DOUBLE UP AND LEFT
    [1, 3, 1, 0], // ╞ VERTICAL SINGLE AND RIGHT DOUBLE
    [3, 1, 3, 0], // ╟ VERTICAL DOUBLE AND RIGHT SINGLE
    [3, 3, 3, 0], // ╠ DOUBLE VERTICAL AND RIGHT
    [1, 0, 1, 3], // ╡ VERTICAL SINGLE AND LEFT DOUBLE
    [3, 0, 3, 1], // ╢ VERTICAL DOUBLE AND LEFT SINGLE
    [3, 0, 3, 3], // ╣ DOUBLE VERTICAL AND LEFT
    [0, 3, 1, 3], // ╤ DOWN SINGLE AND HORIZONTAL DOUBLE
    [0, 1, 3, 1], // ╥ DOWN DOUBLE AND HORIZONTAL SINGLE
    [0, 3, 3, 3], // ╦ DOUBLE DOWN AND HORIZONTAL
    [1, 3, 0, 3], // ╧ UP SINGLE AND HORIZONTAL DOUBLE
    [3, 1, 0, 1], // ╨ UP DOUBLE AND HORIZONTAL SINGLE
    [3, 3, 0, 3], // ╩ DOUBLE UP AND HORIZONTAL
    [1, 3, 1, 3], // ╪ VERTICAL SINGLE AND HORIZONTAL DOUBLE
    [3, 1, 3, 1], // ╫ VERTICAL DOUBLE AND HORIZONTAL SINGLE
    [3, 3, 3, 3], // ╬ DOUBLE VERTICAL AND HORIZONTAL
    [0, 1, 1, 0], // ╭ LIGHT ARC DOWN AND RIGHT
    [0, 0, 1, 1], // ╮ LIGHT ARC DOWN AND LEFT
    [1, 0, 0, 1], // ╯ LIGHT ARC UP AND LEFT
    [1, 1, 0, 0], // ╰ LIGHT ARC UP AND RIGHT
    [0, 0, 0, 0], // ╱ LIGHT DIAGONAL UPPER RIGHT TO LOWER LEFT
    [0, 0, 0, 0], // ╲ LIGHT DIAGONAL UPPER LEFT TO LOWER RIGHT
    [0, 0, 0, 0], // ╳ LIGHT DIAGONAL CROSS
    [0, 0, 0, 1], // ╴ LIGHT LEFT
    [1, 0, 0, 0], // ╵ LIGHT UP
    [0, 1, 0, 0], // ╶ LIGHT RIGHT
    [0, 0, 1, 0], // ╷ LIGHT DOWN
    [0, 0, 0, 2], // ╸ HEAVY LEFT
    [2, 0, 0, 0], // ╹ HEAVY UP
    [0, 2, 0, 0], // ╺ HEAVY RIGHT
    [0, 0, 2, 0], // ╻ HEAVY DOWN
    [0, 2, 0, 1], // ╼ LIGHT LEFT AND HEAVY RIGHT
    [1, 0, 2, 0], // ╽ LIGHT UP AND HEAVY DOWN
    [0, 1, 0, 2], // ╾ HEAVY LEFT AND LIGHT RIGHT
    [2, 0, 1, 0], // ╿ HEAVY UP AND LIGHT DOWN
];

fn segment(x: f32, y: f32, ax: f32, ay: f32, bx: f32, by: f32, radius: f32) -> bool {
    let dx = bx - ax;
    let dy = by - ay;
    let u = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy).max(0.0001)).clamp(0., 1.);
    (x - ax - u * dx).powi(2) + (y - ay - u * dy).powi(2) <= radius * radius
}

pub fn raster(sprite: Sprite, w: u32, h: u32, scale: u32, baseline: i16) -> Vec<u8> {
    let mut mask = vec![0; (w * h) as usize];
    let (wf, hf, s) = (w as f32, h as f32, scale as f32);
    let (cx, cy) = ((wf * 0.5).floor() + 0.5, (hf * 0.5).floor() + 0.5);
    let test = |x: f32, y: f32| -> bool {
        match sprite {
            Sprite::Cursor(shape, focused) => match (shape, focused) {
                (_, false) => x < s || x >= wf - s || y < s || y >= hf - s,
                (CursorShape::Block, true) => true,
                (CursorShape::Bar, true) => x < 2. * s,
                (CursorShape::Underline, true) => y >= hf - 2. * s,
            },
            Sprite::Strike => (y - (f32::from(baseline) - hf * 0.22).round() - 0.5).abs() < s * 0.5,
            Sprite::Underline(style) => {
                let base = (hf - 2. * s).floor() + 0.5;
                match style {
                    Underline::None => false,
                    Underline::Single => (y - base).abs() < s * 0.5,
                    Underline::Double => {
                        (y - (hf - 3. * s + 0.5)).abs() < s * 0.5
                            || (y - (hf - s + 0.5)).abs() < s * 0.5
                    }
                    Underline::Dotted => {
                        ((x / s).rem_euclid(4.) - 1.).powi(2) + ((y - base) / s).powi(2) <= 0.7
                    }
                    Underline::Dashed => (y - base).abs() < s * 0.5 && (x / s).rem_euclid(6.) < 4.,
                    Underline::Curly => {
                        (y - (hf - 2. * s + (x / wf * std::f32::consts::TAU).sin() * s)).abs()
                            < s * 0.55
                    }
                }
            }
            Sprite::Character(ch) => {
                let cp = ch as u32;
                if (0x2800..=0x28ff).contains(&cp) {
                    let bits = cp - 0x2800;
                    return [
                        (0, 0, 0),
                        (1, 0, 1),
                        (2, 0, 2),
                        (3, 1, 0),
                        (4, 1, 1),
                        (5, 1, 2),
                        (6, 0, 3),
                        (7, 1, 3),
                    ]
                    .into_iter()
                    .any(|(bit, dx, dy)| {
                        let px = wf * (0.3 + dx as f32 * 0.4);
                        let py = hf * (0.125 + dy as f32 * 0.25);
                        bits & (1 << bit) != 0
                            && (x - px).powi(2) + (y - py).powi(2) <= (s * 0.7).powi(2)
                    });
                }
                if (0xe0b0..=0xe0b3).contains(&cp) {
                    let right = cp < 0xe0b2;
                    let dx = if right { x } else { wf - x };
                    if cp & 1 == 0 {
                        return dx <= wf * (1. - (y / hf * 2. - 1.).abs());
                    }
                    return segment(
                        x,
                        y,
                        if right { 0. } else { wf },
                        0.,
                        if right { wf } else { 0. },
                        hf * 0.5,
                        s * 0.55,
                    ) || segment(
                        x,
                        y,
                        if right { wf } else { 0. },
                        hf * 0.5,
                        if right { 0. } else { wf },
                        hf,
                        s * 0.55,
                    );
                }
                if cp == 0xe0a0 {
                    let r = s * 1.2;
                    return [
                        (wf * 0.3, hf * 0.2),
                        (wf * 0.7, hf * 0.4),
                        (wf * 0.3, hf * 0.8),
                    ]
                    .into_iter()
                    .any(|(a, b)| ((x - a).hypot(y - b) - r).abs() < s * 0.55)
                        || segment(x, y, wf * 0.3, hf * 0.3, wf * 0.3, hf * 0.7, s * 0.5)
                        || segment(x, y, wf * 0.3, hf * 0.65, wf * 0.7, hf * 0.5, s * 0.5);
                }
                if (0x2580..=0x259f).contains(&cp) {
                    return match cp {
                        0x2580 => y < hf * 0.5,
                        0x2581..=0x2588 => y >= hf * (1. - (cp - 0x2580) as f32 / 8.),
                        0x2589..=0x258f => x < wf * (0x2590 - cp) as f32 / 8.,
                        0x2590 => x >= wf * 0.5,
                        0x2591..=0x2593 => {
                            ((x.floor() as u32 + y.floor() as u32 * 3) % 4) < cp - 0x2590
                        }
                        0x2594 => y < hf / 8.,
                        0x2595 => x >= wf * 7. / 8.,
                        _ => {
                            let q = (if y < hf * 0.5 { 0 } else { 2 })
                                + (if x < wf * 0.5 { 0 } else { 1 });
                            let bits = [4, 8, 1, 13, 9, 7, 11, 2, 6, 14][(cp - 0x2596) as usize];
                            bits & (1 << q) != 0
                        }
                    };
                }
                if matches!(ch, '╱' | '╲' | '╳') {
                    return (ch != '╲' && segment(x, y, 0., hf, wf, 0., s * 0.5))
                        || (ch != '╱' && segment(x, y, 0., 0., wf, hf, s * 0.5));
                }
                if let Some(arms) = arms(ch) {
                    // Join double corners as two nested Ls. Independent half-lines
                    // would leave four short stubs inside the bend.
                    if matches!(ch, '╔' | '╗' | '╚' | '╝') {
                        let u = if arms[1] != 0 { x - cx } else { cx - x };
                        let v = if arms[2] != 0 { y - cy } else { cy - y };
                        return [-s, s].into_iter().any(|offset| {
                            ((v - offset).abs() < s * 0.5 && u >= offset)
                                || ((u - offset).abs() < s * 0.5 && v >= offset)
                        });
                    }
                    if matches!(ch, '╭' | '╮' | '╯' | '╰') {
                        let u = if matches!(ch, '╭' | '╰') {
                            x - cx
                        } else {
                            cx - x
                        };
                        let v = if matches!(ch, '╭' | '╮') {
                            y - cy
                        } else {
                            cy - y
                        };
                        let r = (wf.min(hf) * 0.4).max(s);
                        return (u >= r && v.abs() < s * 0.5)
                            || (v >= r && u.abs() < s * 0.5)
                            || (u >= 0.
                                && u <= r
                                && v >= 0.
                                && v <= r
                                && ((u - r).hypot(v - r) - r).abs() < s * 0.5);
                    }
                    if matches!(cp, 0x2504..=0x250b | 0x254c..=0x254f) {
                        let count = if cp >= 0x254c {
                            2.
                        } else if cp <= 0x2507 {
                            3.
                        } else {
                            4.
                        };
                        let position = if arms[0] == 0 { x / wf } else { y / hf };
                        if (position * count).fract() > 0.68 {
                            return false;
                        }
                    }
                    for (direction, weight) in arms.into_iter().enumerate() {
                        if weight == 0 {
                            continue;
                        }
                        let offsets: &[f32] = if weight == 3 { &[-1., 1.] } else { &[0.] };
                        let thickness = if weight == 2 { 2. * s } else { s };
                        for offset in offsets {
                            let offset = offset * s;
                            let hit = match direction {
                                0 => (x - cx - offset).abs() < thickness * 0.5 && y <= cy + s,
                                1 => (y - cy - offset).abs() < thickness * 0.5 && x >= cx - s,
                                2 => (x - cx - offset).abs() < thickness * 0.5 && y >= cy - s,
                                _ => (y - cy - offset).abs() < thickness * 0.5 && x <= cx + s,
                            };
                            if hit {
                                return true;
                            }
                        }
                    }
                }
                false
            }
        }
    };
    for y in 0..h {
        for x in 0..w {
            let mut n = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    if test(
                        x as f32 + (sx as f32 + 0.5) / 4.,
                        y as f32 + (sy as f32 + 0.5) / 4.,
                    ) {
                        n += 1;
                    }
                }
            }
            mask[(y * w + x) as usize] = (n * 255 / 16) as u8;
        }
    }
    mask
}
