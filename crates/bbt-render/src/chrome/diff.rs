//! Renderer comparison policy: per-channel tolerance plus changed-pixel budget.
//! This intentionally ranks review images; it does not bless a new golden.
pub const CHANNEL_TOLERANCE: u8 = 8;
pub const CHANGED_BUDGET: f64 = 0.001;
pub struct Difference {
    pub changed_fraction: f64,
    pub mean_error: f64,
    pub rgba: Vec<u8>,
}
pub fn compare(a: &[u8], b: &[u8]) -> Result<Difference, &'static str> {
    if a.len() != b.len() || a.is_empty() || !a.len().is_multiple_of(4) {
        return Err("comparison requires equal nonempty RGBA buffers");
    }
    let mut rgba = Vec::with_capacity(a.len());
    let mut changed = 0usize;
    let mut error = 0u64;
    for (a, b) in a.as_chunks::<4>().0.iter().zip(b.as_chunks::<4>().0) {
        let delta: [u8; 4] = std::array::from_fn(|i| a[i].abs_diff(b[i]));
        let hit = delta.iter().any(|n| *n > CHANNEL_TOLERANCE);
        changed += usize::from(hit);
        error += delta.iter().map(|n| u64::from(*n)).sum::<u64>();
        if hit {
            rgba.extend_from_slice(&[
                255,
                delta[1].saturating_mul(3),
                delta[2].saturating_mul(3),
                255,
            ]);
        } else {
            let gray = ((u16::from(a[0]) + u16::from(a[1]) + u16::from(a[2])) / 6) as u8;
            rgba.extend_from_slice(&[gray, gray, gray, 255]);
        }
    }
    Ok(Difference {
        changed_fraction: changed as f64 / (a.len() / 4) as f64,
        mean_error: error as f64 / (a.len() as f64 * 255.0),
        rgba,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tolerance_is_not_an_exact_byte_comparison() {
        let a = [100, 100, 100, 255];
        assert_eq!(
            compare(&a, &[108, 100, 100, 255]).unwrap().changed_fraction,
            0.0
        );
        assert_eq!(
            compare(&a, &[109, 100, 100, 255]).unwrap().changed_fraction,
            1.0
        );
        assert!(compare(&a, &[]).is_err());
    }
}
