//! Property tests: invariants that must hold for any random buy/sell ledger.

use coinbasis::{CostBasisMethod, Portfolio, Transaction};
use chrono::{Duration, TimeZone, Utc};
use proptest::prelude::*;
use rust_decimal::Decimal;

fn base() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap()
}

proptest! {
    #[test]
    fn buys_then_one_sell_conserve_basis(
        // 1..=5 buys of (qty 1..=10, unit_price 1..=100), then sell `sell_qty`.
        buys in prop::collection::vec((1u32..=10, 1u32..=100), 1..=5),
        sell_frac in 0u32..=100,
    ) {
        let total_qty: u32 = buys.iter().map(|(q, _)| *q).sum();
        let sell_qty = (total_qty * sell_frac / 100).max(0);

        let mut txs: Vec<Transaction> = Vec::new();
        let mut day = 0i64;
        let mut total_basis = Decimal::ZERO;
        for (q, p) in &buys {
            txs.push(Transaction::Buy {
                timestamp: base() + Duration::days(day),
                wallet: "w".into(), asset: "btc".into(),
                quantity: Decimal::from(*q), unit_price: Decimal::from(*p), fee: Decimal::ZERO,
            });
            total_basis += Decimal::from(*q) * Decimal::from(*p);
            day += 1;
        }
        if sell_qty > 0 {
            txs.push(Transaction::Sell {
                timestamp: base() + Duration::days(day),
                wallet: "w".into(), asset: "btc".into(),
                quantity: Decimal::from(sell_qty), unit_price: Decimal::from(50u32), fee: Decimal::ZERO,
            });
        }

        let p = Portfolio::from_transactions(&txs).unwrap();
        let realized = p.realized_gains(CostBasisMethod::Fifo).unwrap();
        let holdings = p.holdings(CostBasisMethod::Fifo).unwrap();

        // Conservation: consumed basis + remaining basis == total acquired basis.
        let consumed_basis: Decimal = realized.iter().map(|r| r.cost_basis).sum();
        let remaining_basis: Decimal = holdings.iter().map(|h| h.cost_basis).sum();
        prop_assert_eq!(consumed_basis + remaining_basis, total_basis);

        // Quantity conservation.
        let remaining_qty: Decimal = holdings.iter().map(|h| h.quantity).sum();
        prop_assert_eq!(remaining_qty, Decimal::from(total_qty - sell_qty));
    }
}
