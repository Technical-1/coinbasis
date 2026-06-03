//! Cost-basis method selection and lot ordering.
//!
//! [`CostBasisMethod`] selects how disposals are matched to open lots. Under
//! [`CostBasisMethod::SpecificId`] the caller supplies a [`LotSelection`]
//! (a map from a disposal's original input index to the [`LotPick`]s it
//! consumes).

use crate::lot::Lot;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// How disposals are matched against open lots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CostBasisMethod {
    /// Oldest lots consumed first.
    Fifo,
    /// Newest lots consumed first.
    Lifo,
    /// Highest unit-cost lots consumed first (minimizes realized gain).
    Hifo,
    /// All open units of an `(asset, wallet)` pool averaged into one lot.
    Average,
    /// Caller names the lots per disposal (see [`LotSelection`]).
    SpecificId,
}

/// A caller's choice of which acquisition to draw from for a Specific-ID
/// disposal. `acquisition_index` is the **original input index** of the
/// acquiring transaction.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LotPick {
    /// Original input index of the acquisition to draw from.
    pub acquisition_index: usize,
    /// Units to draw from that acquisition's lot.
    pub quantity: Decimal,
}

/// Map from a disposal's **original input index** to the lots it consumes.
/// Used only under [`CostBasisMethod::SpecificId`].
pub type LotSelection = HashMap<usize, Vec<LotPick>>;

/// Return the indices of `lots` in the order the given automatic method
/// consumes them. Ties break by `lot_id` for determinism. Not meaningful for
/// `Average`/`SpecificId` (the engine handles those specially).
pub(crate) fn order_for(method: CostBasisMethod, lots: &[Lot]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..lots.len()).collect();
    match method {
        CostBasisMethod::Fifo => {
            idx.sort_by(|&a, &b| {
                lots[a]
                    .acquired_at
                    .cmp(&lots[b].acquired_at)
                    .then(lots[a].lot_id.cmp(&lots[b].lot_id))
            });
        }
        CostBasisMethod::Lifo => {
            idx.sort_by(|&a, &b| {
                lots[b]
                    .acquired_at
                    .cmp(&lots[a].acquired_at)
                    .then(lots[b].lot_id.cmp(&lots[a].lot_id))
            });
        }
        CostBasisMethod::Hifo => {
            idx.sort_by(|&a, &b| {
                lots[b]
                    .cost_basis_per_unit()
                    .cmp(&lots[a].cost_basis_per_unit())
                    .then(lots[a].lot_id.cmp(&lots[b].lot_id))
            });
        }
        // Average and SpecificId do not use positional ordering; fall back to
        // FIFO order so callers that ask for an order still get a stable one.
        CostBasisMethod::Average | CostBasisMethod::SpecificId => {
            idx.sort_by(|&a, &b| {
                lots[a]
                    .acquired_at
                    .cmp(&lots[b].acquired_at)
                    .then(lots[a].lot_id.cmp(&lots[b].lot_id))
            });
        }
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lot::Lot;
    use chrono::{TimeZone, Utc};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn lot(id: u64, day: u32, basis: i64, qty: i64) -> Lot {
        Lot {
            asset: "btc".into(),
            wallet: "w".into(),
            quantity: dec!(0) + Decimal::from(qty),
            cost_basis: Decimal::from(basis),
            acquired_at: Utc.with_ymd_and_hms(2021, 1, day, 0, 0, 0).unwrap(),
            lot_id: id,
            gift: None,
        }
    }

    #[test]
    fn fifo_orders_oldest_first() {
        let lots = vec![lot(1, 3, 30, 1), lot(2, 1, 10, 1), lot(3, 2, 20, 1)];
        assert_eq!(order_for(CostBasisMethod::Fifo, &lots), vec![1, 2, 0]);
    }

    #[test]
    fn lifo_orders_newest_first() {
        let lots = vec![lot(1, 1, 10, 1), lot(2, 3, 30, 1), lot(3, 2, 20, 1)];
        assert_eq!(order_for(CostBasisMethod::Lifo, &lots), vec![1, 2, 0]);
    }

    #[test]
    fn hifo_orders_highest_unit_cost_first() {
        // unit costs: 10, 30, 20 -> order indices 1,2,0
        let lots = vec![lot(1, 1, 10, 1), lot(2, 2, 30, 1), lot(3, 3, 20, 1)];
        assert_eq!(order_for(CostBasisMethod::Hifo, &lots), vec![1, 2, 0]);
    }
}
