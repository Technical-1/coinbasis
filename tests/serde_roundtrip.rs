//! Verifies public types (de)serialize when the `serde` feature is enabled.
#![cfg(feature = "serde")]

use chrono::{TimeZone, Utc};
use coinbasis::Transaction;
use rust_decimal_macros::dec;

#[test]
fn transaction_json_roundtrip() {
    let tx = Transaction::Buy {
        timestamp: Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap(),
        wallet: "w".into(),
        asset: "btc".into(),
        quantity: dec!(1),
        unit_price: dec!(100),
        fee: dec!(0),
    };
    let json = serde_json::to_string(&tx).unwrap();
    let back: Transaction = serde_json::from_str(&json).unwrap();
    assert_eq!(tx, back);
}
