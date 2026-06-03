# coinbasis

Comprehensive crypto **tax-lot cost-basis accounting** as a pure Rust library.

Feed it a ledger of transactions (buys, sells, crypto-to-crypto trades, income,
spends, wallet transfers, gifts) and current prices; it returns realized capital
gains (short/long-term classified), ordinary income, unrealized P/L, portfolio
valuation, and per-tax-year reports. No network access, no file I/O — you supply
the data.

## Features

- Cost-basis methods: **FIFO, LIFO, HIFO, Average, Specific-ID**
- **Per-wallet** lot pools (US 2025 per-account rule); transfers preserve basis
  and the holding-period clock
- Crypto-to-crypto **trades**, **income** (staking/mining/airdrop/interest),
  **spends**, and **gifts** with the full IRS **dual-basis** rule
- Holding-period (short/long-term) classification
- Form-8949-shaped capital-gains report + income report, filtered by tax year
- Exact decimal math (`rust_decimal`) — never floats for money

## Quickstart

```rust
use coinbasis::{CostBasisMethod, Portfolio, Transaction};
use chrono::{TimeZone, Utc};
use rust_decimal_macros::dec;

let txs = vec![
    Transaction::Buy { timestamp: Utc.with_ymd_and_hms(2020,1,1,0,0,0).unwrap(),
        wallet: "hot".into(), asset: "btc".into(),
        quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
    Transaction::Sell { timestamp: Utc.with_ymd_and_hms(2022,1,1,0,0,0).unwrap(),
        wallet: "hot".into(), asset: "btc".into(),
        quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
];

let portfolio = Portfolio::from_transactions(&txs).unwrap();
let gains = portfolio.realized_gains(CostBasisMethod::Fifo).unwrap();
assert_eq!(gains[0].gain, dec!(400));
```

## Not tax advice

`coinbasis` is a calculation library. It does not file taxes or give legal
advice, and makes no guarantee of conformance with any jurisdiction's rules. The
default model follows current US federal treatment. Provided "as is", without
warranty.

## Minimum supported Rust version

Rust 1.74.

## License

MIT OR Apache-2.0.
