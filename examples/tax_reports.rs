//! Per-tax-year, Form-8949-shaped capital-gains report (short/long split) plus
//! an ordinary-income report.
//!
//! Run with: `cargo run --example tax_reports`

use chrono::{TimeZone, Utc};
use coinbasis::{CostBasisMethod, IncomeSource, Portfolio, Transaction};
use rust_decimal_macros::dec;

fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
}

fn main() {
    let txs = vec![
        // Long-term: bought 2019, sold 2021 -> gain 400, long-term.
        Transaction::Buy {
            timestamp: ts(2019, 1, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(2), unit_price: dec!(100), fee: dec!(0),
        },
        Transaction::Sell {
            timestamp: ts(2021, 3, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(1), unit_price: dec!(500), fee: dec!(0),
        },
        // Short-term: bought and sold within 2021 -> gain 30, short-term.
        Transaction::Buy {
            timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "eth".into(),
            quantity: dec!(1), unit_price: dec!(100), fee: dec!(0),
        },
        Transaction::Sell {
            timestamp: ts(2021, 6, 1), wallet: "w".into(), asset: "eth".into(),
            quantity: dec!(1), unit_price: dec!(130), fee: dec!(0),
        },
        // Staking income received in 2021.
        Transaction::Income {
            timestamp: ts(2021, 5, 1), wallet: "w".into(), asset: "eth".into(),
            quantity: dec!(1), value: dec!(60), source: IncomeSource::Staking,
        },
    ];

    let p = Portfolio::from_transactions(&txs).unwrap();

    let cg = p.capital_gains_report(CostBasisMethod::Fifo, 2021).unwrap();
    println!(
        "2021 capital gains: short={} long={} total={} ({} rows)",
        cg.short_term_gain, cg.long_term_gain, cg.total_gain, cg.rows.len()
    );

    let inc = p.income_report(2021);
    println!("2021 ordinary income: {}", inc.total_income);

    assert_eq!(cg.short_term_gain, dec!(30));
    assert_eq!(cg.long_term_gain, dec!(400));
    assert_eq!(cg.total_gain, dec!(430));
    assert_eq!(inc.total_income, dec!(60));
}
