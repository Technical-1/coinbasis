//! Public output types: realized gains, income, holdings, valuations, reports.

use crate::transaction::IncomeSource;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;

/// Capital-gain holding period. Short ≤ 365 days held; Long > 365 days.
///
/// The 365-day cutoff is a deliberate, documented approximation of the IRS
/// "more than one year" rule (it ignores leap-year edge days).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Term {
    /// Held 365 days or fewer.
    Short,
    /// Held more than 365 days.
    Long,
}

impl Term {
    /// Classify a holding period from acquisition to disposal.
    pub fn classify(acquired_at: DateTime<Utc>, disposed_at: DateTime<Utc>) -> Term {
        if disposed_at - acquired_at > Duration::days(365) {
            Term::Long
        } else {
            Term::Short
        }
    }
}

/// One realized capital-gain row (one matched lot). For `Average`, `acquired_at`
/// and `term` are `None`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RealizedGain {
    /// Asset disposed.
    pub asset: String,
    /// Wallet disposed from.
    pub wallet: String,
    /// Disposal time.
    pub disposed_at: DateTime<Utc>,
    /// Matched lot's acquisition date; `None` under `Average`.
    pub acquired_at: Option<DateTime<Utc>>,
    /// Units in this row.
    pub quantity: Decimal,
    /// Proceeds allocated to this row (net of disposal fee).
    pub proceeds: Decimal,
    /// Cost basis applied (donor/lesser-of for gifted lots).
    pub cost_basis: Decimal,
    /// `proceeds - cost_basis` (0 in the gift dead zone).
    pub gain: Decimal,
    /// Holding-period term; `None` under `Average`.
    pub term: Option<Term>,
}

/// One ordinary-income receipt.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IncomeEvent {
    /// Asset received.
    pub asset: String,
    /// Wallet it landed in.
    pub wallet: String,
    /// Receipt time.
    pub received_at: DateTime<Utc>,
    /// Units received.
    pub quantity: Decimal,
    /// Fair-market value at receipt (= ordinary income).
    pub value: Decimal,
    /// Income classification.
    pub source: IncomeSource,
}

/// A current open position within one wallet.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Holding {
    /// Asset held.
    pub asset: String,
    /// Wallet holding it.
    pub wallet: String,
    /// Open units.
    pub quantity: Decimal,
    /// Remaining cost basis.
    pub cost_basis: Decimal,
    /// `cost_basis / quantity`.
    pub average_cost: Decimal,
}

/// Valuation of one asset aggregated across wallets.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetValuation {
    /// Asset.
    pub asset: String,
    /// Total open units across wallets.
    pub quantity: Decimal,
    /// Total remaining cost basis.
    pub cost_basis: Decimal,
    /// Supplied current price.
    pub price: Decimal,
    /// `quantity * price`.
    pub market_value: Decimal,
    /// `market_value - cost_basis`.
    pub unrealized: Decimal,
    /// `market_value / portfolio market_value` in `0.0..=1.0`.
    pub allocation: Decimal,
}

/// Whole-portfolio valuation at supplied prices.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PortfolioReport {
    /// Per-asset valuations (priced assets only).
    pub assets: Vec<AssetValuation>,
    /// Sum of cost basis of priced assets.
    pub total_cost: Decimal,
    /// Sum of market value of priced assets.
    pub total_value: Decimal,
    /// `total_value - total_cost`.
    pub total_unrealized: Decimal,
    /// `total_unrealized / total_cost` (0 when `total_cost` is 0).
    pub total_return: Decimal,
    /// Held assets with no supplied price (excluded from totals).
    pub missing_prices: Vec<String>,
}

/// Form-8949-shaped capital-gains report for one calendar tax year (UTC).
///
/// Under `Average`, rows have no `term`, so `short_term_gain + long_term_gain`
/// may be less than `total_gain`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CapitalGainsReport {
    /// The calendar year covered.
    pub tax_year: i32,
    /// Disposals settled in this year.
    pub rows: Vec<RealizedGain>,
    /// Sum of gains on short-term rows.
    pub short_term_gain: Decimal,
    /// Sum of gains on long-term rows.
    pub long_term_gain: Decimal,
    /// Sum of gains on all rows.
    pub total_gain: Decimal,
}

/// Ordinary-income report for one calendar tax year (UTC).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IncomeReport {
    /// The calendar year covered.
    pub tax_year: i32,
    /// Income events received in this year.
    pub events: Vec<IncomeEvent>,
    /// Sum of `value` across events.
    pub total_income: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn term_boundary_365_is_short_366_is_long() {
        let acquired = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(
            Term::classify(acquired, acquired + Duration::days(365)),
            Term::Short
        );
        assert_eq!(
            Term::classify(acquired, acquired + Duration::days(366)),
            Term::Long
        );
    }
}
