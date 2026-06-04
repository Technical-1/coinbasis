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

#[test]
fn taxconfig_json_roundtrip() {
    use coinbasis::TaxConfig;
    let cfg = TaxConfig::default();
    let back: TaxConfig = serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
    assert_eq!(cfg, back);
}

#[test]
fn taxconfig_parses_string_decimals_and_null_up_to() {
    use coinbasis::TaxConfig;
    let json = r#"{"jurisdiction":"US","long_term_threshold_days":365,"short_term_rate":"0.35",
        "long_term_brackets":[{"up_to":"47025","rate":"0.0"},{"up_to":null,"rate":"0.20"}]}"#;
    let cfg: TaxConfig = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.long_term_brackets[1].up_to, None);
}
