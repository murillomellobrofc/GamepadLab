//! Small numeric helpers.

pub fn mean(v: &[f32]) -> f32 {
    if v.is_empty() {
        0.0
    } else {
        (v.iter().map(|&x| x as f64).sum::<f64>() / v.len() as f64) as f32
    }
}

pub fn std_dev(v: &[f32]) -> f32 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = mean(v) as f64;
    let var = v.iter().map(|&x| (x as f64 - m).powi(2)).sum::<f64>() / v.len() as f64;
    var.sqrt() as f32
}

/// Linear-interpolated percentile, `p` in 0..=1.
pub fn percentile(v: &[f32], p: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s: Vec<f32> = v.iter().copied().filter(|x| x.is_finite()).collect();
    if s.is_empty() {
        return 0.0;
    }
    s.sort_by(f32::total_cmp);
    let pos = p.clamp(0.0, 1.0) * (s.len() - 1) as f32;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    s[lo] + (s[hi] - s[lo]) * (pos - lo as f32)
}

pub fn median(v: &[f32]) -> f32 {
    percentile(v, 0.5)
}

/// Least-squares slope of `y` against `x`.
pub fn slope(x: &[f32], y: &[f32]) -> f32 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }
    let mx = mean(&x[..n]) as f64;
    let my = mean(&y[..n]) as f64;
    let mut num = 0.0f64;
    let mut den = 0.0f64;
    for i in 0..n {
        let dx = x[i] as f64 - mx;
        num += dx * (y[i] as f64 - my);
        den += dx * dx;
    }
    if den == 0.0 {
        0.0
    } else {
        (num / den) as f32
    }
}

pub fn round_to(v: f32, step: f32) -> f32 {
    (v / step).round() * step
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        assert_eq!(mean(&[1.0, 2.0, 3.0]), 2.0);
        assert!((std_dev(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]) - 2.0).abs() < 1e-6);
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(percentile(&[0.0, 10.0], 0.25), 2.5);
        assert!((slope(&[0.0, 1.0, 2.0], &[1.0, 3.0, 5.0]) - 2.0).abs() < 1e-6);
        assert_eq!(slope(&[1.0, 1.0], &[0.0, 5.0]), 0.0);
        assert_eq!(mean(&[]), 0.0);
        assert!((round_to(0.0437, 0.005) - 0.045).abs() < 1e-6);
    }
}
