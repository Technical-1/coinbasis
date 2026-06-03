//! The smallest useful program: one buy, one sell, realized gain under FIFO.
//!
//! Run with: `cargo run --example quickstart`

use chrono::{TimeZone, Utc};
use coinbasis::{CostBasisMethod, Portfolio, Term, Transaction};
use rust_decimal_macros::dec;

fn main() {
    // A ledger is just a Vec<Transaction> in the order events occurred.
    let txs = vec![
        Transaction::Buy {
            timestamp: Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(),
            wallet: "hot".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(100),
            fee: dec!(0),
        },
        Transaction::Sell {
            timestamp: Utc.with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap(),
            wallet: "hot".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(500),
            fee: dec!(0),
        },
    ];

    let portfolio = Portfolio::from_transactions(&txs).expect("ledger is valid");
    let gains = portfolio
        .realized_gains(CostBasisMethod::Fifo)
        .expect("Fifo needs no lot selection");

    let g = &gains[0];
    println!(
        "Sold {} {} for proceeds {} against basis {} => gain {} ({:?}-term)",
        g.quantity, g.asset, g.proceeds, g.cost_basis, g.gain, g.term
    );

    assert_eq!(g.gain, dec!(400));
    assert_eq!(g.term, Some(Term::Long)); // held 2020 -> 2022
}
