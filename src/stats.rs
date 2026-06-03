//! Pure portfolio statistics over caller-supplied numeric series.
//!
//! These functions take whatever series the caller maintains (e.g. a portfolio
//! value history) and never fetch data. Statistical work uses `f64`; exact money
//! math elsewhere in the crate uses `Decimal`. Functions return `None` for
//! series too short to be meaningful (fewer than two points), or when a quantity
//! is undefined (e.g. Sharpe with zero volatility).

/// Period-over-period simple returns from a value series.
/// Returns an empty vec if fewer than two values.
///
/// # Example
/// ```
/// let r = coinbasis::stats::returns_from_values(&[100.0, 110.0]);
/// assert!((r[0] - 0.1).abs() < 1e-9);
/// ```
pub fn returns_from_values(values: &[f64]) -> Vec<f64> {
    values
        .windows(2)
        .filter(|w| w[0] != 0.0)
        .map(|w| (w[1] - w[0]) / w[0])
        .collect()
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Sample standard deviation of a returns series. `None` if fewer than two.
///
/// # Example
/// ```
/// let v = coinbasis::stats::volatility(&[0.1, -0.1]).unwrap();
/// assert!(v > 0.0);
/// assert!(coinbasis::stats::volatility(&[0.1]).is_none());
/// ```
pub fn volatility(returns: &[f64]) -> Option<f64> {
    if returns.len() < 2 {
        return None;
    }
    let m = mean(returns);
    let var = returns.iter().map(|r| (r - m).powi(2)).sum::<f64>() / (returns.len() as f64 - 1.0);
    Some(var.sqrt())
}

/// Sharpe ratio: (mean(returns) - risk_free) / volatility(returns).
/// `None` if fewer than two returns or volatility is zero.
///
/// # Example
/// ```
/// // Equal returns => zero volatility => undefined Sharpe.
/// assert!(coinbasis::stats::sharpe_ratio(&[0.05, 0.05, 0.05], 0.05).is_none());
/// ```
pub fn sharpe_ratio(returns: &[f64], risk_free: f64) -> Option<f64> {
    let vol = volatility(returns)?;
    if vol < f64::EPSILON {
        return None;
    }
    Some((mean(returns) - risk_free) / vol)
}

/// Worst peak-to-trough decline of a value series, as a fraction in `0.0..=1.0`.
/// `None` if fewer than two values.
///
/// # Example
/// ```
/// // Peak 120 -> trough 60 is a 0.5 drawdown.
/// let dd = coinbasis::stats::max_drawdown(&[100.0, 120.0, 60.0, 80.0]).unwrap();
/// assert!((dd - 0.5).abs() < 1e-9);
/// ```
pub fn max_drawdown(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mut peak = values[0];
    let mut worst = 0.0_f64;
    for &v in &values[1..] {
        if v > peak {
            peak = v;
        } else if peak > 0.0 {
            let dd = (peak - v) / peak;
            if dd > worst {
                worst = dd;
            }
        }
    }
    Some(worst)
}

/// Total return from first to last value. `None` if fewer than two values or the
/// first value is zero.
///
/// # Example
/// ```
/// let c = coinbasis::stats::cumulative_return(&[100.0, 150.0]).unwrap();
/// assert!((c - 0.5).abs() < 1e-9);
/// ```
pub fn cumulative_return(values: &[f64]) -> Option<f64> {
    if values.len() < 2 || values[0] == 0.0 {
        return None;
    }
    Some((values[values.len() - 1] - values[0]) / values[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    #[test]
    fn returns_from_values_basic() {
        let r = returns_from_values(&[100.0, 110.0, 99.0]);
        assert_eq!(r.len(), 2);
        approx(r[0], 0.1);
        approx(r[1], -0.1);
    }

    #[test]
    fn volatility_known_vector() {
        // sample std dev of [0.1, -0.1] = 0.14142135...
        let v = volatility(&[0.1, -0.1]).unwrap();
        approx(v, 0.1_f64.hypot(0.1)); // sqrt(0.02) = 0.141421356...
    }

    #[test]
    fn too_short_series_returns_none() {
        assert!(volatility(&[0.1]).is_none());
        assert!(max_drawdown(&[100.0]).is_none());
        assert!(cumulative_return(&[100.0]).is_none());
        assert!(sharpe_ratio(&[0.1], 0.0).is_none());
    }

    #[test]
    fn max_drawdown_peak_to_trough() {
        // peak 120 -> trough 60 = 0.5 drawdown
        let dd = max_drawdown(&[100.0, 120.0, 60.0, 80.0]).unwrap();
        approx(dd, 0.5);
    }

    #[test]
    fn cumulative_return_first_to_last() {
        approx(cumulative_return(&[100.0, 150.0]).unwrap(), 0.5);
    }

    #[test]
    fn sharpe_zero_when_no_excess() {
        // returns all equal -> zero volatility -> None (undefined)
        assert!(sharpe_ratio(&[0.05, 0.05, 0.05], 0.05).is_none());
    }

    #[test]
    fn returns_skips_zero_divisor_window() {
        let r = returns_from_values(&[0.0, 10.0, 20.0]);
        assert_eq!(r.len(), 1); // only the 10 -> 20 window survives
        approx(r[0], 1.0);
    }

    #[test]
    fn returns_empty_for_short_series() {
        assert!(returns_from_values(&[100.0]).is_empty());
    }

    #[test]
    fn cumulative_return_first_value_zero_is_none() {
        assert!(cumulative_return(&[0.0, 100.0]).is_none());
    }

    #[test]
    fn sharpe_with_nonzero_risk_free() {
        let s = sharpe_ratio(&[0.1, -0.1], 0.05).unwrap();
        assert!(s < 0.0);
    }

    #[test]
    fn max_drawdown_monotonic_increasing_is_zero() {
        approx(max_drawdown(&[100.0, 110.0, 130.0]).unwrap(), 0.0);
    }

    #[test]
    fn max_drawdown_non_positive_series_is_zero() {
        // All values are zero or negative so peak never rises above 0.
        // The `else if peak > 0.0` branch is never true; drawdown stays 0.
        approx(max_drawdown(&[0.0, -1.0, -2.0]).unwrap(), 0.0);
    }
}
