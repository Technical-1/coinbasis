//! Tax-liability estimation over a [`crate::CapitalGainsReport`].
//!
//! Cost-basis accounting (which lots, what gain, short/long term) is the rest of
//! the crate's job; this module turns a year's realized gains into an estimated
//! tax using a [`TaxConfig`] — a flat short-term rate plus progressive
//! long-term brackets, with a configurable holding-period threshold. Not tax advice.

use crate::report::{CapitalGainsReport, Term};
use chrono::Duration;
use rust_decimal::Decimal;

/// One long-term capital-gains bracket. `up_to` is the cumulative-gain ceiling
/// for this bracket; `None` marks the unbounded top bracket.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaxBracket {
    /// Upper gain bound for this bracket; `None` = unbounded.
    pub up_to: Option<Decimal>,
    /// Marginal rate applied within this bracket (e.g. `0.15` = 15%).
    pub rate: Decimal,
}

/// Tax-rate configuration: a flat short-term rate plus progressive long-term
/// brackets, with a configurable long-term holding threshold.
///
/// # Example
/// ```
/// use coinbasis::TaxConfig;
/// let c = TaxConfig::default();
/// assert_eq!(c.long_term_threshold_days, 365);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaxConfig {
    /// Display label for the jurisdiction (does not affect the math).
    pub jurisdiction: String,
    /// Days held strictly above which a gain is long-term.
    pub long_term_threshold_days: i64,
    /// Flat rate applied to net short-term gains.
    pub short_term_rate: Decimal,
    /// Progressive long-term brackets, ascending, unbounded bracket last.
    pub long_term_brackets: Vec<TaxBracket>,
}

impl Default for TaxConfig {
    fn default() -> Self {
        TaxConfig {
            jurisdiction: "default".to_string(),
            long_term_threshold_days: 365,
            short_term_rate: Decimal::new(35, 2),
            long_term_brackets: vec![
                TaxBracket {
                    up_to: Some(Decimal::new(47025, 0)),
                    rate: Decimal::new(0, 0),
                },
                TaxBracket {
                    up_to: Some(Decimal::new(518900, 0)),
                    rate: Decimal::new(15, 2),
                },
                TaxBracket {
                    up_to: None,
                    rate: Decimal::new(20, 2),
                },
            ],
        }
    }
}

/// The estimated tax from applying a [`TaxConfig`] to a year's gains.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaxEstimate {
    /// Net short-term gain (under the config threshold).
    pub short_term_gain: Decimal,
    /// Net long-term gain (under the config threshold).
    pub long_term_gain: Decimal,
    /// Tax on the short-term gain.
    pub short_term_tax: Decimal,
    /// Tax on the long-term gain.
    pub long_term_tax: Decimal,
    /// `short_term_tax + long_term_tax`.
    pub total_tax: Decimal,
}

/// Estimate the tax on a year's realized gains.
///
/// Short/long subtotals are re-derived from each row's holding period against
/// `config.long_term_threshold_days` (rows with no `acquired_at` fall back to the
/// row's `term`). Short-term tax is the flat rate on positive net short-term gain;
/// long-term tax is progressive over the brackets on positive net long-term gain.
/// Losses never produce tax.
///
/// # Example
/// ```
/// use coinbasis::{TaxConfig, tax, CapitalGainsReport};
/// use rust_decimal::Decimal;
/// let report = CapitalGainsReport { tax_year: 2024, rows: vec![],
///     short_term_gain: Decimal::ZERO, long_term_gain: Decimal::ZERO, total_gain: Decimal::ZERO };
/// assert_eq!(tax::estimate(&report, &TaxConfig::default()).total_tax, Decimal::ZERO);
/// ```
pub fn estimate(report: &CapitalGainsReport, config: &TaxConfig) -> TaxEstimate {
    let mut short_gain = Decimal::ZERO;
    let mut long_gain = Decimal::ZERO;
    for r in &report.rows {
        let is_long = match r.acquired_at {
            Some(acq) => (r.disposed_at - acq) > Duration::days(config.long_term_threshold_days),
            None => matches!(r.term, Some(Term::Long)),
        };
        if is_long {
            long_gain += r.gain;
        } else {
            short_gain += r.gain;
        }
    }
    let short_tax = if short_gain > Decimal::ZERO {
        short_gain * config.short_term_rate
    } else {
        Decimal::ZERO
    };
    let long_tax = progressive_tax(long_gain.max(Decimal::ZERO), &config.long_term_brackets);
    TaxEstimate {
        short_term_gain: short_gain,
        long_term_gain: long_gain,
        short_term_tax: short_tax,
        long_term_tax: long_tax,
        total_tax: short_tax + long_tax,
    }
}

/// Apply ascending progressive brackets to a non-negative gain.
fn progressive_tax(gain: Decimal, brackets: &[TaxBracket]) -> Decimal {
    let mut tax = Decimal::ZERO;
    let mut prev = Decimal::ZERO;
    for b in brackets {
        if prev >= gain {
            break;
        }
        let ceiling = b.up_to.unwrap_or(gain);
        let top = ceiling.min(gain);
        let slice = top - prev;
        if slice > Decimal::ZERO {
            tax += slice * b.rate;
        }
        prev = ceiling;
        if b.up_to.is_none() {
            break;
        }
    }
    tax
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{CapitalGainsReport, RealizedGain, Term};
    use chrono::{TimeZone, Utc};
    use rust_decimal_macros::dec;

    #[test]
    fn default_config_is_us_preset() {
        let c = TaxConfig::default();
        assert_eq!(c.long_term_threshold_days, 365);
        assert_eq!(c.short_term_rate, dec!(0.35));
        assert_eq!(c.long_term_brackets.len(), 3);
        assert_eq!(c.long_term_brackets[0].rate, dec!(0.0));
        assert_eq!(c.long_term_brackets[2].up_to, None);
        assert_eq!(c.long_term_brackets[2].rate, dec!(0.20));
    }

    fn row(acq_days_before: Option<i64>, term: Option<Term>, gain: Decimal) -> RealizedGain {
        let disposed = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        let acquired = acq_days_before.map(|d| disposed - chrono::Duration::days(d));
        RealizedGain {
            asset: "bitcoin".into(),
            wallet: "w".into(),
            disposed_at: disposed,
            acquired_at: acquired,
            quantity: dec!(1),
            proceeds: dec!(0),
            cost_basis: dec!(0),
            gain,
            term,
        }
    }

    fn report(rows: Vec<RealizedGain>) -> CapitalGainsReport {
        CapitalGainsReport {
            tax_year: 2024,
            rows,
            short_term_gain: dec!(0),
            long_term_gain: dec!(0),
            total_gain: dec!(0),
        }
    }

    #[test]
    fn short_term_flat_rate_on_gains_only() {
        let cfg = TaxConfig {
            jurisdiction: "t".into(),
            long_term_threshold_days: 365,
            short_term_rate: dec!(0.30),
            long_term_brackets: vec![],
        };
        let e = estimate(
            &report(vec![row(Some(100), Some(Term::Short), dec!(1000))]),
            &cfg,
        );
        assert_eq!(e.short_term_gain, dec!(1000));
        assert_eq!(e.short_term_tax, dec!(300));
        assert_eq!(e.long_term_tax, dec!(0));
    }

    #[test]
    fn short_term_loss_no_tax() {
        let cfg = TaxConfig {
            jurisdiction: "t".into(),
            long_term_threshold_days: 365,
            short_term_rate: dec!(0.30),
            long_term_brackets: vec![],
        };
        let e = estimate(
            &report(vec![row(Some(10), Some(Term::Short), dec!(-500))]),
            &cfg,
        );
        assert_eq!(e.short_term_tax, dec!(0));
    }

    #[test]
    fn long_term_progressive_brackets() {
        let cfg = TaxConfig {
            jurisdiction: "t".into(),
            long_term_threshold_days: 365,
            short_term_rate: dec!(0.30),
            long_term_brackets: vec![
                TaxBracket {
                    up_to: Some(dec!(1000)),
                    rate: dec!(0.0),
                },
                TaxBracket {
                    up_to: Some(dec!(3000)),
                    rate: dec!(0.10),
                },
                TaxBracket {
                    up_to: None,
                    rate: dec!(0.20),
                },
            ],
        };
        let e = estimate(
            &report(vec![row(Some(400), Some(Term::Long), dec!(4000))]),
            &cfg,
        );
        assert_eq!(e.long_term_gain, dec!(4000));
        assert_eq!(e.long_term_tax, dec!(400)); // 0*1000 + .1*2000 + .2*1000
    }

    #[test]
    fn threshold_reclassifies_independent_of_row_term() {
        let cfg = TaxConfig {
            jurisdiction: "t".into(),
            long_term_threshold_days: 500,
            short_term_rate: dec!(0.30),
            long_term_brackets: vec![TaxBracket {
                up_to: None,
                rate: dec!(0.10),
            }],
        };
        let e = estimate(
            &report(vec![row(Some(400), Some(Term::Long), dec!(1000))]),
            &cfg,
        );
        assert_eq!(e.short_term_gain, dec!(1000)); // 400d < 500d threshold => short
        assert_eq!(e.long_term_gain, dec!(0));
    }

    #[test]
    fn average_method_no_acquired_falls_back_to_row_term() {
        let cfg = TaxConfig {
            jurisdiction: "t".into(),
            long_term_threshold_days: 365,
            short_term_rate: dec!(0.30),
            long_term_brackets: vec![TaxBracket {
                up_to: None,
                rate: dec!(0.10),
            }],
        };
        assert_eq!(
            estimate(&report(vec![row(None, None, dec!(1000))]), &cfg).short_term_gain,
            dec!(1000)
        );
        assert_eq!(
            estimate(&report(vec![row(None, Some(Term::Long), dec!(1000))]), &cfg).long_term_tax,
            dec!(100)
        );
    }

    #[test]
    fn empty_report_is_zero() {
        assert_eq!(
            estimate(&report(vec![]), &TaxConfig::default()).total_tax,
            dec!(0)
        );
    }
}
