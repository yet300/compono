//! Arithmetic that `core` lacks for `f32`.

/// Clamps `value` to `0..=1`. A NaN becomes 0.
pub(crate) fn clamp01(value: f32) -> f32 {
    if value > 0.0 {
        if value < 1.0 { value } else { 1.0 }
    } else {
        0.0
    }
}

pub(crate) fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

/// Square root by Newton's method. Gives 0 for zero, negative and NaN input.
pub(crate) fn sqrt(value: f32) -> f32 {
    if value.is_nan() || value <= 0.0 {
        return 0.0;
    }
    if value.is_infinite() {
        return value;
    }
    let mut root = if value > 1.0 { value * 0.5 } else { 1.0 };
    for _ in 0..40 {
        let next = 0.5 * (root + value / root);
        if next == root {
            break;
        }
        root = next;
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_roots() {
        assert_eq!(sqrt(0.0), 0.0);
        assert_eq!(sqrt(-4.0), 0.0);
        assert_eq!(sqrt(f32::NAN), 0.0);
        assert!((sqrt(700.0) - 26.457_513).abs() < 1e-4);
        assert!((sqrt(0.25) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn clamps() {
        assert_eq!(clamp01(-1.0), 0.0);
        assert_eq!(clamp01(2.0), 1.0);
        assert_eq!(clamp01(f32::NAN), 0.0);
        assert_eq!(clamp01(0.25), 0.25);
    }
}
