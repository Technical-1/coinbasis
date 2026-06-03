//! Internal, per-wallet lot model. Not part of the public API.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Extra basis information carried only by lots that originated from a gift,
/// to support the IRS dual-basis rule.
#[derive(Clone, Debug)]
pub(crate) struct GiftBasis {
    /// Fair-market value per unit at the time the gift was received.
    pub fmv_per_unit: Decimal,
}

/// One open acquisition (or the unconsumed remainder of one), within a single
/// `(asset, wallet)` pool.
#[derive(Clone, Debug)]
pub(crate) struct Lot {
    pub asset: String,
    pub wallet: String,
    /// Units remaining in this lot.
    pub quantity: Decimal,
    /// Remaining cost basis for `quantity` (donor basis for gifted lots).
    pub cost_basis: Decimal,
    /// Acquisition date (donor's date for gifted lots — tacked holding period).
    pub acquired_at: DateTime<Utc>,
    /// Stable id assigned in chronological acquisition order.
    pub lot_id: u64,
    /// Present only for gifted lots.
    pub gift: Option<GiftBasis>,
}

impl Lot {
    /// Remaining cost basis divided by remaining quantity.
    pub fn cost_basis_per_unit(&self) -> Decimal {
        self.cost_basis / self.quantity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rust_decimal_macros::dec;

    #[test]
    fn cost_basis_per_unit_divides() {
        let lot = Lot {
            asset: "btc".into(),
            wallet: "w".into(),
            quantity: dec!(2),
            cost_basis: dec!(100),
            acquired_at: Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap(),
            lot_id: 1,
            gift: None,
        };
        assert_eq!(lot.cost_basis_per_unit(), dec!(50));
    }
}
