//! Gifts follow the IRS dual-basis rule. The receiver inherits the donor's
//! basis for computing GAINS, but the lesser of (donor basis, FMV at receipt)
//! for computing LOSSES — and a sale price between those two realizes nothing
//! (the "dead zone"). Sending a gift is non-taxable for the giver.
//!
//! Run with: `cargo run --example gifts`

use chrono::{TimeZone, Utc};
use coinbasis::{CostBasisMethod, Portfolio, Term, Transaction};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
}

/// Build a ledger: receive a gift, then sell it at `sell_price`.
fn gift_then_sell(donor_basis: Decimal, fmv: Decimal, sell_price: Decimal) -> Decimal {
    let txs = vec![
        Transaction::GiftReceived {
            timestamp: ts(2021, 6, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(1), donor_basis, fmv_at_receipt: fmv,
            donor_acquired_at: ts(2018, 1, 1),
        },
        Transaction::Sell {
            timestamp: ts(2022, 1, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(1), unit_price: sell_price, fee: dec!(0),
        },
    ];
    let p = Portfolio::from_transactions(&txs).unwrap();
    p.realized_gains(CostBasisMethod::Fifo).unwrap()[0].gain
}

fn main() {
    // GAIN: sell above donor basis -> gain measured from donor basis (carryover).
    let gain = gift_then_sell(dec!(100), dec!(120), dec!(200));
    println!("gain branch:      {}", gain); // 100

    // LOSS: sell below the lesser-of(basis, fmv)=80 -> loss measured from 80.
    let loss = gift_then_sell(dec!(100), dec!(80), dec!(50));
    println!("loss branch:      {}", loss); // -30

    // DEAD ZONE: sell between fmv (80) and basis (100) -> no gain, no loss.
    let dead = gift_then_sell(dec!(100), dec!(80), dec!(90));
    println!("dead-zone branch: {}", dead); // 0

    // Holding period tacks from the donor's 2018 date -> long-term.
    let p = Portfolio::from_transactions(&vec![
        Transaction::GiftReceived {
            timestamp: ts(2021, 6, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(1), donor_basis: dec!(100), fmv_at_receipt: dec!(120),
            donor_acquired_at: ts(2018, 1, 1),
        },
        Transaction::Sell {
            timestamp: ts(2022, 1, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(1), unit_price: dec!(200), fee: dec!(0),
        },
    ])
    .unwrap();
    assert_eq!(p.realized_gains(CostBasisMethod::Fifo).unwrap()[0].term, Some(Term::Long));

    // Sending a gift removes lots with NO realized gain.
    let sent = Portfolio::from_transactions(&vec![
        Transaction::Buy {
            timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(2), unit_price: dec!(100), fee: dec!(0),
        },
        Transaction::GiftSent {
            timestamp: ts(2021, 2, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(1),
        },
    ])
    .unwrap();
    assert!(sent.realized_gains(CostBasisMethod::Fifo).unwrap().is_empty());
    println!("gift sent: no realized gain, 1 unit remains");

    assert_eq!(gain, dec!(100));
    assert_eq!(loss, dec!(-30));
    assert_eq!(dead, dec!(0));
}
