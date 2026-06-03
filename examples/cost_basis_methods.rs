//! The same two-lot ledger produces *different* realized gains under each
//! cost-basis method — the core reason this crate exists.
//!
//! Run with: `cargo run --example cost_basis_methods`

use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use coinbasis::{CostBasisMethod, LotPick, LotSelection, Portfolio, Transaction};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
}

fn ledger() -> Vec<Transaction> {
    vec![
        // Lot A: cheap, bought first.
        Transaction::Buy {
            timestamp: ts(2020, 1, 1),
            wallet: "w".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(100),
            fee: dec!(0),
        },
        // Lot B: expensive, bought later.
        Transaction::Buy {
            timestamp: ts(2021, 1, 1),
            wallet: "w".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(300),
            fee: dec!(0),
        },
        // Sell one unit at 500 — which lot is consumed depends on the method.
        Transaction::Sell {
            timestamp: ts(2022, 1, 1),
            wallet: "w".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(500),
            fee: dec!(0),
        },
    ]
}

fn gain(method: CostBasisMethod) -> Decimal {
    let p = Portfolio::from_transactions(&ledger()).unwrap();
    p.realized_gains(method)
        .unwrap()
        .iter()
        .map(|g| g.gain)
        .sum()
}

fn main() {
    for m in [
        CostBasisMethod::Fifo,
        CostBasisMethod::Lifo,
        CostBasisMethod::Hifo,
        CostBasisMethod::Average,
    ] {
        println!("{:?}: gain = {}", m, gain(m));
    }

    // Specific-ID: the caller names which acquisition to draw from. Here we
    // sell the expensive lot (the Buy at input index 1) for the disposal at
    // input index 2.
    let mut selection: LotSelection = HashMap::new();
    selection.insert(
        2,
        vec![LotPick {
            acquisition_index: 1,
            quantity: dec!(1),
        }],
    );
    let p = Portfolio::from_transactions(&ledger()).unwrap();
    let spec_gain: Decimal = p
        .realized_gains_with_selection(&selection)
        .unwrap()
        .iter()
        .map(|g| g.gain)
        .sum();
    println!("SpecificId (lot B): gain = {}", spec_gain);

    assert_eq!(gain(CostBasisMethod::Fifo), dec!(400));
    assert_eq!(gain(CostBasisMethod::Lifo), dec!(200));
    assert_eq!(gain(CostBasisMethod::Hifo), dec!(200));
    assert_eq!(gain(CostBasisMethod::Average), dec!(300));
    assert_eq!(spec_gain, dec!(200));
}
