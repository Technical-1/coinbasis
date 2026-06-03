//! Value current holdings at supplied prices, aggregating each asset across
//! wallets. Assets you don't supply a price for are reported in `missing_prices`
//! and excluded from totals. `allocation` is each asset's share of priced value.
//!
//! Run with: `cargo run --example valuation`

use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use coinbasis::{CostBasisMethod, Portfolio, Transaction};
use rust_decimal_macros::dec;

fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
}

fn main() {
    let txs = vec![
        Transaction::Buy {
            timestamp: ts(2021, 1, 1),
            wallet: "a".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(100),
            fee: dec!(0),
        },
        Transaction::Buy {
            timestamp: ts(2021, 1, 2),
            wallet: "b".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(140),
            fee: dec!(0),
        },
        Transaction::Buy {
            timestamp: ts(2021, 1, 3),
            wallet: "a".into(),
            asset: "eth".into(),
            quantity: dec!(1),
            unit_price: dec!(50),
            fee: dec!(0),
        },
    ];
    let p = Portfolio::from_transactions(&txs).unwrap();

    // Price both assets: BTC aggregates 2 units across wallets a+b.
    let mut prices = HashMap::new();
    prices.insert("btc".to_string(), dec!(200));
    prices.insert("eth".to_string(), dec!(100));
    let report = p.valuation(CostBasisMethod::Fifo, &prices).unwrap();

    for a in &report.assets {
        println!(
            "{}: qty {} value {} unrealized {} allocation {}",
            a.asset, a.quantity, a.market_value, a.unrealized, a.allocation
        );
    }
    println!(
        "TOTAL value {} cost {} unrealized {} return {}",
        report.total_value, report.total_cost, report.total_unrealized, report.total_return
    );

    let btc = report.assets.iter().find(|a| a.asset == "btc").unwrap();
    assert_eq!(btc.quantity, dec!(2)); // aggregated across wallets
    assert_eq!(btc.market_value, dec!(400));
    assert_eq!(btc.allocation, dec!(0.8)); // 400 / 500

    // Omitting a price lists the asset under missing_prices.
    let mut partial = HashMap::new();
    partial.insert("btc".to_string(), dec!(200));
    let report2 = p.valuation(CostBasisMethod::Fifo, &partial).unwrap();
    println!("missing prices: {:?}", report2.missing_prices);
    assert_eq!(report2.missing_prices, vec!["eth".to_string()]);
}
