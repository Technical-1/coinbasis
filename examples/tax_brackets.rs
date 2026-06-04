//! Estimate tax with progressive long-term brackets. Run: `cargo run --example tax_brackets`.
use chrono::{TimeZone, Utc};
use coinbasis::{CostBasisMethod, Portfolio, TaxConfig, Transaction};
use rust_decimal::Decimal;

fn main() {
    let txs = vec![
        Transaction::Buy {
            timestamp: Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(),
            wallet: "hot".into(),
            asset: "btc".into(),
            quantity: Decimal::new(1, 0),
            unit_price: Decimal::new(10000, 0),
            fee: Decimal::new(0, 0),
        },
        Transaction::Sell {
            timestamp: Utc.with_ymd_and_hms(2022, 6, 1, 0, 0, 0).unwrap(),
            wallet: "hot".into(),
            asset: "btc".into(),
            quantity: Decimal::new(1, 0),
            unit_price: Decimal::new(60000, 0),
            fee: Decimal::new(0, 0),
        },
    ];
    let p = Portfolio::from_transactions(&txs).unwrap();
    let est = p
        .tax_estimate(CostBasisMethod::Fifo, 2022, &TaxConfig::default())
        .unwrap();
    println!("long-term gain: {}", est.long_term_gain);
    println!("long-term tax:  {}", est.long_term_tax);
    println!("total tax:      {}", est.total_tax);
}
