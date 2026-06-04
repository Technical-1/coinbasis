# coinbasis

[![crates.io](https://img.shields.io/crates/v/coinbasis.svg)](https://crates.io/crates/coinbasis)
[![docs.rs](https://docs.rs/coinbasis/badge.svg)](https://docs.rs/coinbasis)
[![license](https://img.shields.io/crates/l/coinbasis.svg)](#license)

Comprehensive crypto **tax-lot cost-basis accounting** as a pure Rust library.

Feed it a ledger of transactions (buys, sells, crypto-to-crypto trades, income,
spends, wallet transfers, gifts) and current prices; it returns realized capital
gains (short/long-term classified), ordinary income, unrealized P/L, portfolio
valuation, and per-tax-year reports. No network access, no file I/O — you supply
the data.

## Features

- Cost-basis methods: **FIFO, LIFO, HIFO, Average, Specific-ID**
- **Per-wallet** lot pools (US per-account rule); transfers preserve basis and
  the holding-period clock
- Crypto-to-crypto **trades**, **income** (staking/mining/airdrop/interest),
  **spends**, and **gifts** with the full IRS **dual-basis** rule
- Holding-period (short/long-term) classification
- Form-8949-shaped capital-gains report + income report, filtered by tax year
- Exact decimal math (`rust_decimal`) — never floats for money
- Optional `serde` support on every public type

## Install

```toml
[dependencies]
coinbasis = "0.1"

# with serialization (Serialize/Deserialize on all public types):
coinbasis = { version = "0.1", features = ["serde"] }
```

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

## Examples

Runnable, self-contained examples live in [`examples/`](examples). Run any with
`cargo run --example <name>`:

| Example | What it shows |
|---------|---------------|
| `quickstart` | One buy and sell; a realized gain under FIFO |
| `cost_basis_methods` | The same ledger through FIFO/LIFO/HIFO/Average/Specific-ID, producing different gains |
| `wallet_transfers` | Moving lots between wallets; an in-asset network fee as a taxable disposal |
| `gifts` | The IRS dual-basis rule — gain, loss, and the no-gain/no-loss "dead zone" |
| `tax_reports` | Per-tax-year capital-gains (short/long split) and income reports |
| `valuation` | Mark-to-market across wallets, with missing-price handling |
| `portfolio_stats` | Volatility, Sharpe, max drawdown, and returns over a value series |

## Tax estimation

`coinbasis` 0.2.0 adds a `tax` module that turns a year's realized gains into an
estimated tax liability using a configurable [`TaxConfig`] — a flat short-term
rate plus progressive long-term brackets.

```rust
use coinbasis::{CostBasisMethod, Portfolio, TaxConfig, Transaction};
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;

let txs = vec![
    Transaction::Buy { timestamp: Utc.with_ymd_and_hms(2020,1,1,0,0,0).unwrap(),
        wallet: "hot".into(), asset: "btc".into(),
        quantity: Decimal::new(1,0), unit_price: Decimal::new(10000,0), fee: Decimal::new(0,0) },
    Transaction::Sell { timestamp: Utc.with_ymd_and_hms(2022,6,1,0,0,0).unwrap(),
        wallet: "hot".into(), asset: "btc".into(),
        quantity: Decimal::new(1,0), unit_price: Decimal::new(60000,0), fee: Decimal::new(0,0) },
];

let p = Portfolio::from_transactions(&txs).unwrap();
let est = p.tax_estimate(CostBasisMethod::Fifo, 2022, &TaxConfig::default()).unwrap();
println!("long-term gain: {}", est.long_term_gain); // 50000
println!("total tax:      {}", est.total_tax);
```

Key points:

- `TaxConfig::default()` ships US 2024 federal rates (35% short-term flat; 0%/15%/20%
  long-term progressive brackets).
- The `long_term_threshold_days` field (default 365) controls the short/long
  reclassification. If the config threshold differs from the method used to build the
  `CapitalGainsReport`, rows are reclassified against the config threshold — the
  `term` stored on each row is re-derived from actual holding days.
- Rows with no `acquired_at` (Average method) fall back to the row's stored `term`.
- Losses never produce tax; only positive net gains are taxed.
- Call `coinbasis::tax::estimate(&report, &config)` directly if you already have a
  `CapitalGainsReport`, or use `Portfolio::tax_estimate` as a one-shot convenience.

See `examples/tax_brackets.rs` for a runnable demo: `cargo run --example tax_brackets`.

**Not tax advice** — see below.

## Not tax advice

`coinbasis` is a calculation library. It does not file taxes or give legal
advice, and makes no guarantee of conformance with any jurisdiction's rules. The
default model follows current US federal treatment. Provided "as is", without
warranty.

## Development

```bash
cargo test                  # unit + integration + doctests
cargo test --all-features   # include the serde feature
cargo run --example gifts   # run any example in examples/
cargo doc --open            # browse the API docs locally
```

## Project structure

```
src/
├── lib.rs         # crate docs + public re-exports
├── transaction.rs # Transaction event model + field validation
├── method.rs      # CostBasisMethod, Specific-ID selection, lot ordering
├── lot.rs         # internal per-(asset, wallet) lot model
├── engine.rs      # ledger-replay engine — the core
├── portfolio.rs   # public Portfolio facade (all queries)
├── report.rs      # output types (gains, income, holdings, reports)
├── stats.rs       # pure portfolio statistics over a value series
└── error.rs       # PortfolioError
examples/          # runnable usage examples
```

## Minimum supported Rust version

Rust 1.74.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at
your option.

## Author

Jacob Kanfer — [GitHub](https://github.com/Technical-1)
