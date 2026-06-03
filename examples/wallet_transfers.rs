//! Moving crypto between your own wallets is non-taxable and preserves cost
//! basis + the holding-period clock. A network fee paid in the asset, however,
//! is a taxable disposal at its fair-market value.
//!
//! Run with: `cargo run --example wallet_transfers`

use chrono::{TimeZone, Utc};
use coinbasis::{CostBasisMethod, Portfolio, Term, Transaction};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
}

fn main() {
    let txs = vec![
        // Buy in the hot wallet in 2020.
        Transaction::Buy {
            timestamp: ts(2020, 1, 1),
            wallet: "hot".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(100),
            fee: dec!(0),
        },
        // Move it to cold storage in 2020 — non-taxable, basis & date preserved.
        Transaction::Transfer {
            timestamp: ts(2020, 6, 1),
            asset: "btc".into(),
            quantity: dec!(1),
            from_wallet: "hot".into(),
            to_wallet: "cold".into(),
            fee: dec!(0),
            fee_value: dec!(0),
        },
        // Buy 10 ETH, then move 9 to cold and burn 1 as a network fee (FMV 15).
        Transaction::Buy {
            timestamp: ts(2021, 1, 1),
            wallet: "hot".into(),
            asset: "eth".into(),
            quantity: dec!(10),
            unit_price: dec!(10),
            fee: dec!(0),
        },
        Transaction::Transfer {
            timestamp: ts(2021, 2, 1),
            asset: "eth".into(),
            quantity: dec!(9),
            from_wallet: "hot".into(),
            to_wallet: "cold".into(),
            fee: dec!(1),
            fee_value: dec!(15),
        },
        // Sell the transferred BTC from cold in 2022 — still long-term from 2020.
        Transaction::Sell {
            timestamp: ts(2022, 1, 1),
            wallet: "cold".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(500),
            fee: dec!(0),
        },
    ];

    let p = Portfolio::from_transactions(&txs).unwrap();
    let gains = p.realized_gains(CostBasisMethod::Fifo).unwrap();

    for g in &gains {
        println!(
            "{}: disposed {} {} -> gain {} ({:?}-term)",
            g.wallet, g.quantity, g.asset, g.gain, g.term
        );
    }

    // The ETH fee disposal: 1 unit, basis 10, proceeds 15, gain 5.
    let eth_fee = gains.iter().find(|g| g.asset == "eth").unwrap();
    assert_eq!(eth_fee.gain, dec!(5));

    // The BTC sale from cold: basis carried from the 2020 buy, long-term.
    let btc = gains.iter().find(|g| g.asset == "btc").unwrap();
    assert_eq!(btc.wallet, "cold");
    assert_eq!(btc.cost_basis, dec!(100));
    assert_eq!(btc.term, Some(Term::Long));

    // 9 ETH now live in cold with basis 90.
    let holdings = p.holdings(CostBasisMethod::Fifo).unwrap();
    let cold_eth: Decimal = holdings
        .iter()
        .filter(|h| h.asset == "eth" && h.wallet == "cold")
        .map(|h| h.cost_basis)
        .sum();
    assert_eq!(cold_eth, dec!(90));
}
