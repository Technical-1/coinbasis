//! Worked, multi-wallet, multi-method example — the crate's living documentation.
//! One fixed ledger run through FIFO, LIFO, HIFO, and Average proves they produce
//! *different* realized gains; a second flow exercises Transfer, Income, and
//! valuation end-to-end. (Trade and gift handling are covered by the engine unit
//! tests in `src/engine.rs`.)

use chrono::{TimeZone, Utc};
use coinbasis::{CostBasisMethod, Portfolio, Transaction};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[test]
fn portfolio_tax_estimate_matches_module_estimate() {
    use coinbasis::{tax, TaxConfig};
    let txs = vec![
        Transaction::Buy {
            timestamp: Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(),
            wallet: "w".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(100),
            fee: dec!(0),
        },
        Transaction::Sell {
            timestamp: Utc.with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap(),
            wallet: "w".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(500),
            fee: dec!(0),
        },
    ];
    let p = Portfolio::from_transactions(&txs).unwrap();
    let cfg = TaxConfig::default();
    let via = p.tax_estimate(CostBasisMethod::Fifo, 2022, &cfg).unwrap();
    let report = p.capital_gains_report(CostBasisMethod::Fifo, 2022).unwrap();
    assert_eq!(via, tax::estimate(&report, &cfg));
    assert_eq!(via.long_term_gain, dec!(400));
}

fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
}

fn ledger() -> Vec<Transaction> {
    vec![
        Transaction::Buy {
            timestamp: ts(2020, 1, 1),
            wallet: "hot".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(100),
            fee: dec!(0),
        },
        Transaction::Buy {
            timestamp: ts(2021, 1, 1),
            wallet: "hot".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(300),
            fee: dec!(0),
        },
        // Sell 1 BTC in 2022 at 500 — the method decides which lot.
        Transaction::Sell {
            timestamp: ts(2022, 1, 1),
            wallet: "hot".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(500),
            fee: dec!(0),
        },
    ]
}

fn total_gain(method: CostBasisMethod) -> Decimal {
    let p = Portfolio::from_transactions(&ledger()).unwrap();
    p.realized_gains(method)
        .unwrap()
        .iter()
        .map(|g| g.gain)
        .sum()
}

#[test]
fn methods_produce_different_gains_on_the_same_ledger() {
    // FIFO sells the 100 lot -> gain 400. LIFO sells the 300 lot -> gain 200.
    // HIFO sells the highest-cost (300) -> gain 200. Average basis 200 -> gain 300.
    assert_eq!(total_gain(CostBasisMethod::Fifo), dec!(400));
    assert_eq!(total_gain(CostBasisMethod::Lifo), dec!(200));
    assert_eq!(total_gain(CostBasisMethod::Hifo), dec!(200));
    assert_eq!(total_gain(CostBasisMethod::Average), dec!(300));
}

#[test]
fn comprehensive_flow_with_transfer_income_and_valuation() {
    let txs = vec![
        Transaction::Buy {
            timestamp: ts(2020, 1, 1),
            wallet: "hot".into(),
            asset: "btc".into(),
            quantity: dec!(1),
            unit_price: dec!(100),
            fee: dec!(0),
        },
        Transaction::Transfer {
            timestamp: ts(2020, 6, 1),
            asset: "btc".into(),
            quantity: dec!(1),
            from_wallet: "hot".into(),
            to_wallet: "cold".into(),
            fee: dec!(0),
            fee_value: dec!(0),
        },
        Transaction::Income {
            timestamp: ts(2021, 1, 1),
            wallet: "hot".into(),
            asset: "eth".into(),
            quantity: dec!(2),
            value: dec!(200),
            source: coinbasis::IncomeSource::Staking,
        },
        // Sell the transferred BTC from cold in 2022 — long-term from 2020.
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
    assert_eq!(gains.len(), 1);
    assert_eq!(gains[0].wallet, "cold");
    assert_eq!(gains[0].term, Some(coinbasis::Term::Long));
    assert_eq!(gains[0].gain, dec!(400));

    let income = p.income_report(2021);
    assert_eq!(income.total_income, dec!(200));

    let mut prices = std::collections::HashMap::new();
    prices.insert("eth".to_string(), dec!(150));
    let val = p.valuation(CostBasisMethod::Fifo, &prices).unwrap();
    // 2 ETH @ 150 = 300 value, basis 200 -> unrealized 100.
    let eth = val.assets.iter().find(|a| a.asset == "eth").unwrap();
    assert_eq!(eth.unrealized, dec!(100));
}
