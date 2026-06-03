# coinbasis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `coinbasis`, a pure Rust library for comprehensive crypto tax-lot cost-basis accounting (FIFO/LIFO/HIFO/Average/Specific-ID), per-wallet, with realized capital gains (holding-period classified), ordinary income, unrealized P/L, portfolio valuation, tax-year reports, and pure portfolio statistics — then publish it to crates.io.

**Architecture:** A single library crate with focused modules. Public input is a `Transaction` event enum; `Portfolio` stores the immutable ledger and answers queries by replaying it through an internal `Engine` parameterized by a cost-basis method (or a Specific-ID selection). All money math uses `rust_decimal::Decimal`; the crate performs no I/O. See the design spec at `docs/superpowers/specs/2026-06-03-coinbasis-library-design.md`.

**Tech Stack:** Rust 2021, `rust_decimal`, `chrono`, `thiserror`, optional `serde`; `proptest` + `rust_decimal_macros` for tests.

---

## Conventions used throughout this plan

- **Quote currency:** all `value`/`unit_price`/`fee` amounts are in one quote currency (USD by convention), treated as opaque `Decimal`.
- **Decimal literals in tests:** use the `dec!()` macro from `rust_decimal_macros` (a dev-dependency), e.g. `dec!(100.50)`.
- **Timestamps in tests:** build with `chrono::TimeZone`, e.g. `Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap()`. A helper `ts(y, m, d)` is added in Task 4's tests and reused.
- **Run a single test:** `cargo test <name> -- --exact` (or `cargo test <substring>`).
- **Run everything:** `cargo test` and (for the public-doc gate) `cargo test --doc`.
- **Indices:** `Portfolio` preserves the caller's original `Vec<Transaction>` order. Specific-ID `LotPick.acquisition_index` and `LotSelection` keys always refer to **original input indices**, even though the engine processes events in timestamp order.
- **Commit author:** the repo's local git config already uses an allowed author. Commit messages describe the change only — no AI/assistant attribution.

---

## File structure

```
coinbasis/
├── Cargo.toml
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── src/
│   ├── lib.rs          # crate docs + disclaimer, lints, module decls, re-exports
│   ├── error.rs        # PortfolioError
│   ├── stats.rs        # pure analytics functions
│   ├── transaction.rs  # Transaction enum, IncomeSource, timestamp()/validate()
│   ├── method.rs       # CostBasisMethod, LotPick, LotSelection, lot ordering
│   ├── lot.rs          # Lot, GiftBasis (crate-internal)
│   ├── engine.rs       # ledger replay: acquire / dispose / transfer / gifts
│   ├── report.rs       # Term, RealizedGain, IncomeEvent, Holding, valuations, reports
│   └── portfolio.rs    # Portfolio facade (public query API)
└── tests/
    └── headline.rs     # worked multi-wallet, multi-method integration test
```

---

## Task 1: Project scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `LICENSE-MIT`, `LICENSE-APACHE`
- (`.gitignore` already exists)

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "coinbasis"
version = "0.1.0"
edition = "2021"
rust-version = "1.74"
description = "Comprehensive crypto tax-lot cost-basis accounting (FIFO/LIFO/HIFO/Average/Specific-ID), per-wallet, with capital-gains and income reporting."
license = "MIT OR Apache-2.0"
repository = "https://github.com/Technical-1/coinbasis"
readme = "README.md"
keywords = ["crypto", "tax", "cost-basis", "portfolio", "accounting"]
categories = ["finance"]

[dependencies]
rust_decimal = "1"
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
thiserror = "1"
serde = { version = "1", features = ["derive"], optional = true }

[dev-dependencies]
proptest = "1"
rust_decimal_macros = "1"

[features]
default = []
serde = ["dep:serde", "rust_decimal/serde", "chrono/serde"]
```

- [ ] **Step 2: Create `src/lib.rs` skeleton**

```rust
//! `coinbasis` — comprehensive crypto tax-lot cost-basis accounting.
//!
//! Hand it a ledger of [`Transaction`]s and current prices; it returns realized
//! capital gains (with holding-period classification), ordinary income,
//! unrealized P/L, portfolio valuation, and tax-year reports. The crate performs
//! **no network access and no file I/O** — callers supply all data.
//!
//! # Not tax advice
//! `coinbasis` is a calculation library. It does not file taxes, give legal
//! advice, or guarantee conformance with any jurisdiction's rules. The default
//! model follows current US federal treatment (see the crate docs). Provided
//! "as is", without warranty of any kind.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod engine;
mod lot;

pub mod error;
pub mod method;
pub mod portfolio;
pub mod report;
pub mod stats;
pub mod transaction;

pub use error::PortfolioError;
pub use method::{CostBasisMethod, LotPick, LotSelection};
pub use portfolio::Portfolio;
pub use report::{
    AssetValuation, CapitalGainsReport, Holding, IncomeEvent, IncomeReport, PortfolioReport,
    RealizedGain, Term,
};
pub use transaction::{IncomeSource, Transaction};
```

This will not compile until the modules exist; that is expected. The next tasks add each module. To keep the tree compiling between tasks, create empty stub modules now.

- [ ] **Step 3: Create empty stub modules so the crate compiles**

Create each of these files with a single placeholder so `lib.rs` resolves. They are replaced in later tasks.

`src/error.rs`:
```rust
//! Error types. (Replaced in Task 2.)
```
`src/stats.rs`:
```rust
//! Pure analytics. (Replaced in Task 3.)
```
`src/transaction.rs`:
```rust
//! Transaction model. (Replaced in Task 4.)
```
`src/method.rs`:
```rust
//! Cost-basis methods. (Replaced in Task 6.)
```
`src/lot.rs`:
```rust
//! Internal lot model. (Replaced in Task 5.)
```
`src/engine.rs`:
```rust
//! Ledger replay engine. (Replaced in Task 7.)
```
`src/report.rs`:
```rust
//! Report types. (Replaced in Task 8.)
```
`src/portfolio.rs`:
```rust
//! Portfolio facade. (Replaced in Task 15.)
```

Temporarily comment out the `pub use` lines in `lib.rs` that reference not-yet-defined items (everything except the module declarations). Re-enable each re-export in the task that defines it. (Simplest: keep only `mod`/`pub mod` lines active in Task 1; add `pub use` lines back as items appear.)

- [ ] **Step 4: Add license files**

Create `LICENSE-MIT` with the standard MIT license text (copyright `2026 Jacob Kanfer`) and `LICENSE-APACHE` with the standard Apache-2.0 license text. (Use the canonical texts; fill the copyright line.)

- [ ] **Step 5: Verify it builds**

Run: `cargo build`
Expected: compiles with warnings about unused/empty modules, no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src LICENSE-MIT LICENSE-APACHE
git commit -m "Scaffold coinbasis crate with module layout and licenses"
```

---

## Task 2: Error type

**Files:**
- Modify: `src/error.rs`
- Modify: `src/lib.rs` (re-enable `pub use error::PortfolioError;`)

- [ ] **Step 1: Write the failing test**

Append to `src/error.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn insufficient_lots_message_names_wallet() {
        let e = PortfolioError::InsufficientLots {
            asset: "bitcoin".into(),
            wallet: "coldwallet".into(),
            attempted: dec!(2),
            available: dec!(1),
        };
        let msg = e.to_string();
        assert!(msg.contains("bitcoin"));
        assert!(msg.contains("coldwallet"));
        assert!(msg.contains('2') && msg.contains('1'));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test insufficient_lots_message_names_wallet`
Expected: FAIL — `PortfolioError` not defined.

- [ ] **Step 3: Write the implementation**

Replace `src/error.rs` contents (above the test module) with:
```rust
//! Error type for ledger validation and replay.

use rust_decimal::Decimal;

/// Errors produced when validating a ledger or computing cost basis.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum PortfolioError {
    /// A disposal asked for more units than the wallet's pool holds.
    #[error("disposed {attempted} {asset} from wallet '{wallet}' but only {available} held there")]
    InsufficientLots {
        /// Asset identifier.
        asset: String,
        /// Wallet the disposal drew from.
        wallet: String,
        /// Units requested.
        attempted: Decimal,
        /// Units available in that wallet.
        available: Decimal,
    },
    /// A transfer's `quantity + fee` exceeds the source wallet's balance.
    #[error("transfer of {quantity} (+fee {fee}) {asset} from '{wallet}' exceeds the {available} held there")]
    InsufficientTransfer {
        /// Asset identifier.
        asset: String,
        /// Source wallet.
        wallet: String,
        /// Units to move.
        quantity: Decimal,
        /// Fee units burned.
        fee: Decimal,
        /// Units available.
        available: Decimal,
    },
    /// An event carried a non-positive quantity.
    #[error("event for {asset} has non-positive quantity {quantity}")]
    NonPositiveQuantity {
        /// Asset identifier.
        asset: String,
        /// The offending quantity.
        quantity: Decimal,
    },
    /// An event carried a negative monetary value or price.
    #[error("event for {asset} has a negative value or price")]
    NegativeValue {
        /// Asset identifier.
        asset: String,
    },
    /// An event carried a negative fee.
    #[error("event for {asset} has a negative fee {fee}")]
    NegativeFee {
        /// Asset identifier.
        asset: String,
        /// The offending fee.
        fee: Decimal,
    },
    /// A Specific-ID disposal had no usable lot selection.
    #[error("Specific-ID disposal of {asset} (input index {tx_index}) lacks a valid, fully-covering lot selection")]
    MissingLotSelection {
        /// Asset identifier.
        asset: String,
        /// Original input index of the disposal.
        tx_index: usize,
    },
    /// A Specific-ID selection referenced an unknown or exhausted acquisition.
    #[error("Specific-ID selection references unknown or exhausted acquisition index {acquisition_index}")]
    InvalidLotSelection {
        /// The bad acquisition index.
        acquisition_index: usize,
    },
    /// `realized_gains`/etc. was called with `SpecificId`; use the `*_with_selection` API.
    #[error("SpecificId requires a lot selection; call the *_with_selection method")]
    SelectionRequired,
}
```

Re-enable in `src/lib.rs`:
```rust
pub use error::PortfolioError;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test insufficient_lots_message_names_wallet`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/error.rs src/lib.rs
git commit -m "Add PortfolioError type"
```

---

## Task 3: Pure stats module

**Files:**
- Modify: `src/stats.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src/stats.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    #[test]
    fn returns_from_values_basic() {
        let r = returns_from_values(&[100.0, 110.0, 99.0]);
        assert_eq!(r.len(), 2);
        approx(r[0], 0.1);
        approx(r[1], -0.1);
    }

    #[test]
    fn volatility_known_vector() {
        // sample std dev of [0.1, -0.1] = 0.14142135...
        let v = volatility(&[0.1, -0.1]).unwrap();
        approx(v, 0.1_f64.hypot(0.1)); // sqrt(0.02) = 0.141421356...
    }

    #[test]
    fn too_short_series_returns_none() {
        assert!(volatility(&[0.1]).is_none());
        assert!(max_drawdown(&[100.0]).is_none());
        assert!(cumulative_return(&[100.0]).is_none());
        assert!(sharpe_ratio(&[0.1], 0.0).is_none());
    }

    #[test]
    fn max_drawdown_peak_to_trough() {
        // peak 120 -> trough 60 = 0.5 drawdown
        let dd = max_drawdown(&[100.0, 120.0, 60.0, 80.0]).unwrap();
        approx(dd, 0.5);
    }

    #[test]
    fn cumulative_return_first_to_last() {
        approx(cumulative_return(&[100.0, 150.0]).unwrap(), 0.5);
    }

    #[test]
    fn sharpe_zero_when_no_excess() {
        // returns all equal -> zero volatility -> None (undefined)
        assert!(sharpe_ratio(&[0.05, 0.05, 0.05], 0.05).is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib stats`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Write the implementation**

Replace `src/stats.rs` (above the test module) with:
```rust
//! Pure portfolio statistics over caller-supplied numeric series.
//!
//! These functions take whatever series the caller maintains (e.g. a portfolio
//! value history) and never fetch data. Statistical work uses `f64`; exact money
//! math elsewhere in the crate uses `Decimal`. Functions return `None` for
//! series too short to be meaningful (fewer than two points), or when a quantity
//! is undefined (e.g. Sharpe with zero volatility).

/// Period-over-period simple returns from a value series.
/// Returns an empty vec if fewer than two values.
pub fn returns_from_values(values: &[f64]) -> Vec<f64> {
    values
        .windows(2)
        .filter(|w| w[0] != 0.0)
        .map(|w| (w[1] - w[0]) / w[0])
        .collect()
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Sample standard deviation of a returns series. `None` if fewer than two.
pub fn volatility(returns: &[f64]) -> Option<f64> {
    if returns.len() < 2 {
        return None;
    }
    let m = mean(returns);
    let var = returns.iter().map(|r| (r - m).powi(2)).sum::<f64>() / (returns.len() as f64 - 1.0);
    Some(var.sqrt())
}

/// Sharpe ratio: (mean(returns) - risk_free) / volatility(returns).
/// `None` if fewer than two returns or volatility is zero.
pub fn sharpe_ratio(returns: &[f64], risk_free: f64) -> Option<f64> {
    let vol = volatility(returns)?;
    if vol == 0.0 {
        return None;
    }
    Some((mean(returns) - risk_free) / vol)
}

/// Worst peak-to-trough decline of a value series, as a fraction in `0.0..=1.0`.
/// `None` if fewer than two values.
pub fn max_drawdown(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mut peak = values[0];
    let mut worst = 0.0_f64;
    for &v in &values[1..] {
        if v > peak {
            peak = v;
        } else if peak > 0.0 {
            let dd = (peak - v) / peak;
            if dd > worst {
                worst = dd;
            }
        }
    }
    Some(worst)
}

/// Total return from first to last value. `None` if fewer than two values or the
/// first value is zero.
pub fn cumulative_return(values: &[f64]) -> Option<f64> {
    if values.len() < 2 || values[0] == 0.0 {
        return None;
    }
    Some((values[values.len() - 1] - values[0]) / values[0])
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib stats`
Expected: PASS (all 6).

- [ ] **Step 5: Commit**

```bash
git add src/stats.rs
git commit -m "Add pure portfolio statistics module"
```

---

## Task 4: Transaction event model

**Files:**
- Modify: `src/transaction.rs`
- Modify: `src/lib.rs` (re-enable transaction re-exports)

- [ ] **Step 1: Write the failing tests**

Append to `src/transaction.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PortfolioError;
    use chrono::{TimeZone, Utc};
    use rust_decimal_macros::dec;

    fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
    }

    #[test]
    fn timestamp_accessor_works_for_each_variant() {
        let b = Transaction::Buy {
            timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "bitcoin".into(),
            quantity: dec!(1), unit_price: dec!(100), fee: dec!(0),
        };
        assert_eq!(b.timestamp(), ts(2021, 1, 1));
        let t = Transaction::Transfer {
            timestamp: ts(2021, 2, 2), asset: "bitcoin".into(), quantity: dec!(1),
            from_wallet: "a".into(), to_wallet: "b".into(), fee: dec!(0), fee_value: dec!(0),
        };
        assert_eq!(t.timestamp(), ts(2021, 2, 2));
    }

    #[test]
    fn validate_rejects_non_positive_quantity() {
        let b = Transaction::Buy {
            timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "eth".into(),
            quantity: dec!(0), unit_price: dec!(100), fee: dec!(0),
        };
        assert_eq!(
            b.validate(),
            Err(PortfolioError::NonPositiveQuantity { asset: "eth".into(), quantity: dec!(0) })
        );
    }

    #[test]
    fn validate_rejects_negative_fee() {
        let s = Transaction::Sell {
            timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "eth".into(),
            quantity: dec!(1), unit_price: dec!(100), fee: dec!(-1),
        };
        assert_eq!(
            s.validate(),
            Err(PortfolioError::NegativeFee { asset: "eth".into(), fee: dec!(-1) })
        );
    }

    #[test]
    fn validate_accepts_well_formed_event() {
        let b = Transaction::Buy {
            timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "eth".into(),
            quantity: dec!(1), unit_price: dec!(100), fee: dec!(1),
        };
        assert_eq!(b.validate(), Ok(()));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib transaction`
Expected: FAIL — `Transaction` not defined.

- [ ] **Step 3: Write the implementation**

Replace `src/transaction.rs` (above the tests) with:
```rust
//! The transaction event model fed into a [`crate::Portfolio`].

use crate::error::PortfolioError;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// The kind of ordinary-income event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IncomeSource {
    /// Staking rewards.
    Staking,
    /// Mining rewards.
    Mining,
    /// Airdropped tokens.
    Airdrop,
    /// Lending/interest income.
    Interest,
    /// Any other ordinary-income receipt.
    Other,
}

/// A single ledger event. All monetary fields are in the quote currency (USD by
/// convention). See the crate docs for the tax treatment of each variant.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Transaction {
    /// Fiat → crypto. Non-taxable; sets cost basis (incl. `fee`).
    Buy {
        /// When it happened.
        timestamp: DateTime<Utc>,
        /// Wallet/account the asset lands in.
        wallet: String,
        /// Asset identifier.
        asset: String,
        /// Units acquired.
        quantity: Decimal,
        /// Price per unit.
        unit_price: Decimal,
        /// Acquisition fee.
        fee: Decimal,
    },
    /// Crypto → fiat. Taxable disposal; proceeds net of `fee`.
    Sell {
        /// When it happened.
        timestamp: DateTime<Utc>,
        /// Wallet disposed from.
        wallet: String,
        /// Asset identifier.
        asset: String,
        /// Units disposed.
        quantity: Decimal,
        /// Price per unit.
        unit_price: Decimal,
        /// Disposal fee.
        fee: Decimal,
    },
    /// Crypto → crypto. Disposal of `from_*` at `value` (FMV) AND acquisition of
    /// `to_*` with basis = `value` + `fee`.
    Trade {
        /// When it happened.
        timestamp: DateTime<Utc>,
        /// Wallet both legs occur in.
        wallet: String,
        /// Asset given up.
        from_asset: String,
        /// Units given up.
        from_quantity: Decimal,
        /// Asset received.
        to_asset: String,
        /// Units received.
        to_quantity: Decimal,
        /// Fair-market value of the disposed leg.
        value: Decimal,
        /// Fee (added to the acquired lot's basis).
        fee: Decimal,
    },
    /// Staking/mining/airdrop/interest. Ordinary income at `value` (FMV); also
    /// opens a lot with basis = `value`.
    Income {
        /// When it happened.
        timestamp: DateTime<Utc>,
        /// Wallet the asset lands in.
        wallet: String,
        /// Asset identifier.
        asset: String,
        /// Units received.
        quantity: Decimal,
        /// Fair-market value at receipt (= ordinary income).
        value: Decimal,
        /// Income classification.
        source: IncomeSource,
    },
    /// Paying for goods/services with crypto. Taxable disposal at `value` (FMV).
    Spend {
        /// When it happened.
        timestamp: DateTime<Utc>,
        /// Wallet disposed from.
        wallet: String,
        /// Asset identifier.
        asset: String,
        /// Units spent.
        quantity: Decimal,
        /// Fair-market value of the spend.
        value: Decimal,
        /// Disposal fee.
        fee: Decimal,
    },
    /// Move between your own wallets. The moved `quantity` is non-taxable and
    /// preserves basis + acquisition date. A network `fee` (units) paid in the
    /// asset is a taxable disposal at `fee_value` (FMV). Total debited from
    /// `from_wallet` = `quantity` + `fee`; `quantity` arrives in `to_wallet`.
    Transfer {
        /// When it happened.
        timestamp: DateTime<Utc>,
        /// Asset identifier.
        asset: String,
        /// Units that arrive in `to_wallet`.
        quantity: Decimal,
        /// Source wallet.
        from_wallet: String,
        /// Destination wallet.
        to_wallet: String,
        /// Fee units burned (0 if none).
        fee: Decimal,
        /// FMV of the fee units (ignored when `fee` is 0).
        fee_value: Decimal,
    },
    /// Crypto given away as a gift. Non-taxable for the giver: lots are removed
    /// (FIFO order) from `wallet` with NO realized gain.
    GiftSent {
        /// When it happened.
        timestamp: DateTime<Utc>,
        /// Wallet the gift leaves from.
        wallet: String,
        /// Asset identifier.
        asset: String,
        /// Units gifted away.
        quantity: Decimal,
    },
    /// Crypto received as a gift. Opens a lot under the IRS dual-basis rule (see
    /// crate docs): `donor_basis` carries over for gains; `min(donor_basis,
    /// fmv_at_receipt)` applies for losses; sales between realize no gain/loss.
    /// Holding period tacks from `donor_acquired_at`.
    GiftReceived {
        /// When it happened.
        timestamp: DateTime<Utc>,
        /// Wallet the asset lands in.
        wallet: String,
        /// Asset identifier.
        asset: String,
        /// Units received.
        quantity: Decimal,
        /// Donor's total adjusted basis for `quantity`.
        donor_basis: Decimal,
        /// Total fair-market value of `quantity` at receipt.
        fmv_at_receipt: Decimal,
        /// Donor's acquisition date (holding period tacks from here).
        donor_acquired_at: DateTime<Utc>,
    },
}

impl Transaction {
    /// The event's timestamp.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Transaction::Buy { timestamp, .. }
            | Transaction::Sell { timestamp, .. }
            | Transaction::Trade { timestamp, .. }
            | Transaction::Income { timestamp, .. }
            | Transaction::Spend { timestamp, .. }
            | Transaction::Transfer { timestamp, .. }
            | Transaction::GiftSent { timestamp, .. }
            | Transaction::GiftReceived { timestamp, .. } => *timestamp,
        }
    }

    /// Validate field-level invariants (positive quantities, non-negative
    /// values/fees). Lot-availability errors surface later, during replay.
    pub fn validate(&self) -> Result<(), PortfolioError> {
        // Helper closures keep the per-variant checks DRY.
        let pos_qty = |asset: &str, q: Decimal| -> Result<(), PortfolioError> {
            if q <= Decimal::ZERO {
                Err(PortfolioError::NonPositiveQuantity { asset: asset.to_string(), quantity: q })
            } else {
                Ok(())
            }
        };
        let non_neg_val = |asset: &str, v: Decimal| -> Result<(), PortfolioError> {
            if v < Decimal::ZERO {
                Err(PortfolioError::NegativeValue { asset: asset.to_string() })
            } else {
                Ok(())
            }
        };
        let non_neg_fee = |asset: &str, f: Decimal| -> Result<(), PortfolioError> {
            if f < Decimal::ZERO {
                Err(PortfolioError::NegativeFee { asset: asset.to_string(), fee: f })
            } else {
                Ok(())
            }
        };

        match self {
            Transaction::Buy { asset, quantity, unit_price, fee, .. }
            | Transaction::Sell { asset, quantity, unit_price, fee, .. } => {
                pos_qty(asset, *quantity)?;
                non_neg_val(asset, *unit_price)?;
                non_neg_fee(asset, *fee)
            }
            Transaction::Trade { from_asset, from_quantity, to_asset, to_quantity, value, fee, .. } => {
                pos_qty(from_asset, *from_quantity)?;
                pos_qty(to_asset, *to_quantity)?;
                non_neg_val(from_asset, *value)?;
                non_neg_fee(from_asset, *fee)
            }
            Transaction::Income { asset, quantity, value, .. } => {
                pos_qty(asset, *quantity)?;
                non_neg_val(asset, *value)
            }
            Transaction::Spend { asset, quantity, value, fee, .. } => {
                pos_qty(asset, *quantity)?;
                non_neg_val(asset, *value)?;
                non_neg_fee(asset, *fee)
            }
            Transaction::Transfer { asset, quantity, fee, fee_value, .. } => {
                pos_qty(asset, *quantity)?;
                non_neg_fee(asset, *fee)?;
                non_neg_val(asset, *fee_value)
            }
            Transaction::GiftSent { asset, quantity, .. } => pos_qty(asset, *quantity),
            Transaction::GiftReceived { asset, quantity, donor_basis, fmv_at_receipt, .. } => {
                pos_qty(asset, *quantity)?;
                non_neg_val(asset, *donor_basis)?;
                non_neg_val(asset, *fmv_at_receipt)
            }
        }
    }
}
```

Re-enable in `src/lib.rs`:
```rust
pub use transaction::{IncomeSource, Transaction};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib transaction`
Expected: PASS (all 4).

- [ ] **Step 5: Commit**

```bash
git add src/transaction.rs src/lib.rs
git commit -m "Add Transaction event model with validation"
```

---

## Task 5: Internal lot model

**Files:**
- Modify: `src/lot.rs`

- [ ] **Step 1: Write the failing test**

Append to `src/lot.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rust_decimal_macros::dec;

    #[test]
    fn cost_basis_per_unit_divides() {
        let lot = Lot {
            asset: "btc".into(), wallet: "w".into(), quantity: dec!(2),
            cost_basis: dec!(100), acquired_at: Utc.with_ymd_and_hms(2021,1,1,0,0,0).unwrap(),
            lot_id: 1, gift: None,
        };
        assert_eq!(lot.cost_basis_per_unit(), dec!(50));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib lot`
Expected: FAIL — `Lot` not defined.

- [ ] **Step 3: Write the implementation**

Replace `src/lot.rs` (above the test) with:
```rust
//! Internal, per-wallet lot model. Not part of the public API.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Extra basis information carried only by lots that originated from a gift,
/// to support the IRS dual-basis rule.
#[derive(Clone, Debug)]
pub(crate) struct GiftBasis {
    /// Fair-market value per unit at the time the gift was received.
    pub fmv_per_unit: Decimal,
}

/// One open acquisition (or the unconsumed remainder of one), within a single
/// `(asset, wallet)` pool.
#[derive(Clone, Debug)]
pub(crate) struct Lot {
    pub asset: String,
    pub wallet: String,
    /// Units remaining in this lot.
    pub quantity: Decimal,
    /// Remaining cost basis for `quantity` (donor basis for gifted lots).
    pub cost_basis: Decimal,
    /// Acquisition date (donor's date for gifted lots — tacked holding period).
    pub acquired_at: DateTime<Utc>,
    /// Stable id assigned in chronological acquisition order.
    pub lot_id: u64,
    /// Present only for gifted lots.
    pub gift: Option<GiftBasis>,
}

impl Lot {
    /// Remaining cost basis divided by remaining quantity.
    pub fn cost_basis_per_unit(&self) -> Decimal {
        self.cost_basis / self.quantity
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib lot`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lot.rs
git commit -m "Add internal Lot and GiftBasis model"
```

---

## Task 6: Cost-basis method + lot ordering

**Files:**
- Modify: `src/method.rs`
- Modify: `src/lib.rs` (re-enable method re-exports)

- [ ] **Step 1: Write the failing tests**

Append to `src/method.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lot::Lot;
    use chrono::{TimeZone, Utc};
    use rust_decimal_macros::dec;

    fn lot(id: u64, day: u32, basis: i64, qty: i64) -> Lot {
        Lot {
            asset: "btc".into(), wallet: "w".into(), quantity: dec!(0) + Decimal::from(qty),
            cost_basis: Decimal::from(basis),
            acquired_at: Utc.with_ymd_and_hms(2021, 1, day, 0, 0, 0).unwrap(),
            lot_id: id, gift: None,
        }
    }

    #[test]
    fn fifo_orders_oldest_first() {
        let lots = vec![lot(1, 3, 30, 1), lot(2, 1, 10, 1), lot(3, 2, 20, 1)];
        assert_eq!(order_for(CostBasisMethod::Fifo, &lots), vec![1, 2, 0]);
    }

    #[test]
    fn lifo_orders_newest_first() {
        let lots = vec![lot(1, 1, 10, 1), lot(2, 3, 30, 1), lot(3, 2, 20, 1)];
        assert_eq!(order_for(CostBasisMethod::Lifo, &lots), vec![1, 2, 0]);
    }

    #[test]
    fn hifo_orders_highest_unit_cost_first() {
        // unit costs: 10, 30, 20 -> order indices 1,2,0
        let lots = vec![lot(1, 1, 10, 1), lot(2, 2, 30, 1), lot(3, 3, 20, 1)];
        assert_eq!(order_for(CostBasisMethod::Hifo, &lots), vec![1, 2, 0]);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib method`
Expected: FAIL — `CostBasisMethod` / `order_for` not defined.

- [ ] **Step 3: Write the implementation**

Replace `src/method.rs` (above the tests) with:
```rust
//! Cost-basis method selection and lot ordering.

use crate::lot::Lot;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// How disposals are matched against open lots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CostBasisMethod {
    /// Oldest lots consumed first.
    Fifo,
    /// Newest lots consumed first.
    Lifo,
    /// Highest unit-cost lots consumed first (minimizes realized gain).
    Hifo,
    /// All open units of an `(asset, wallet)` pool averaged into one lot.
    Average,
    /// Caller names the lots per disposal (see [`LotSelection`]).
    SpecificId,
}

/// A caller's choice of which acquisition to draw from for a Specific-ID
/// disposal. `acquisition_index` is the **original input index** of the
/// acquiring transaction.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LotPick {
    /// Original input index of the acquisition to draw from.
    pub acquisition_index: usize,
    /// Units to draw from that acquisition's lot.
    pub quantity: Decimal,
}

/// Map from a disposal's **original input index** to the lots it consumes.
/// Used only under [`CostBasisMethod::SpecificId`].
pub type LotSelection = HashMap<usize, Vec<LotPick>>;

/// Return the indices of `lots` in the order the given automatic method
/// consumes them. Ties break by `lot_id` for determinism. Not meaningful for
/// `Average`/`SpecificId` (the engine handles those specially).
pub(crate) fn order_for(method: CostBasisMethod, lots: &[Lot]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..lots.len()).collect();
    match method {
        CostBasisMethod::Fifo => {
            idx.sort_by(|&a, &b| {
                lots[a].acquired_at.cmp(&lots[b].acquired_at).then(lots[a].lot_id.cmp(&lots[b].lot_id))
            });
        }
        CostBasisMethod::Lifo => {
            idx.sort_by(|&a, &b| {
                lots[b].acquired_at.cmp(&lots[a].acquired_at).then(lots[b].lot_id.cmp(&lots[a].lot_id))
            });
        }
        CostBasisMethod::Hifo => {
            idx.sort_by(|&a, &b| {
                lots[b]
                    .cost_basis_per_unit()
                    .cmp(&lots[a].cost_basis_per_unit())
                    .then(lots[a].lot_id.cmp(&lots[b].lot_id))
            });
        }
        // Average and SpecificId do not use positional ordering; fall back to
        // FIFO order so callers that ask for an order still get a stable one.
        CostBasisMethod::Average | CostBasisMethod::SpecificId => {
            idx.sort_by(|&a, &b| {
                lots[a].acquired_at.cmp(&lots[b].acquired_at).then(lots[a].lot_id.cmp(&lots[b].lot_id))
            });
        }
    }
    idx
}
```

Re-enable in `src/lib.rs`:
```rust
pub use method::{CostBasisMethod, LotPick, LotSelection};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib method`
Expected: PASS (all 3).

- [ ] **Step 5: Commit**

```bash
git add src/method.rs src/lib.rs
git commit -m "Add CostBasisMethod, LotPick/LotSelection, and lot ordering"
```

---

## Task 7: Report types

**Files:**
- Modify: `src/report.rs`
- Modify: `src/lib.rs` (re-enable report re-exports)

- [ ] **Step 1: Write the failing test**

Append to `src/report.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn term_boundary_365_is_short_366_is_long() {
        let acquired = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(Term::classify(acquired, acquired + Duration::days(365)), Term::Short);
        assert_eq!(Term::classify(acquired, acquired + Duration::days(366)), Term::Long);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib report`
Expected: FAIL — `Term` not defined.

- [ ] **Step 3: Write the implementation**

Replace `src/report.rs` (above the test) with:
```rust
//! Public output types: realized gains, income, holdings, valuations, reports.

use crate::transaction::IncomeSource;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;

/// Capital-gain holding period. Short ≤ 365 days held; Long > 365 days.
///
/// The 365-day cutoff is a deliberate, documented approximation of the IRS
/// "more than one year" rule (it ignores leap-year edge days).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Term {
    /// Held 365 days or fewer.
    Short,
    /// Held more than 365 days.
    Long,
}

impl Term {
    /// Classify a holding period from acquisition to disposal.
    pub fn classify(acquired_at: DateTime<Utc>, disposed_at: DateTime<Utc>) -> Term {
        if disposed_at - acquired_at > Duration::days(365) {
            Term::Long
        } else {
            Term::Short
        }
    }
}

/// One realized capital-gain row (one matched lot). For `Average`, `acquired_at`
/// and `term` are `None`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RealizedGain {
    /// Asset disposed.
    pub asset: String,
    /// Wallet disposed from.
    pub wallet: String,
    /// Disposal time.
    pub disposed_at: DateTime<Utc>,
    /// Matched lot's acquisition date; `None` under `Average`.
    pub acquired_at: Option<DateTime<Utc>>,
    /// Units in this row.
    pub quantity: Decimal,
    /// Proceeds allocated to this row (net of disposal fee).
    pub proceeds: Decimal,
    /// Cost basis applied (donor/lesser-of for gifted lots).
    pub cost_basis: Decimal,
    /// `proceeds - cost_basis` (0 in the gift dead zone).
    pub gain: Decimal,
    /// Holding-period term; `None` under `Average`.
    pub term: Option<Term>,
}

/// One ordinary-income receipt.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IncomeEvent {
    /// Asset received.
    pub asset: String,
    /// Wallet it landed in.
    pub wallet: String,
    /// Receipt time.
    pub received_at: DateTime<Utc>,
    /// Units received.
    pub quantity: Decimal,
    /// Fair-market value at receipt (= ordinary income).
    pub value: Decimal,
    /// Income classification.
    pub source: IncomeSource,
}

/// A current open position within one wallet.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Holding {
    /// Asset held.
    pub asset: String,
    /// Wallet holding it.
    pub wallet: String,
    /// Open units.
    pub quantity: Decimal,
    /// Remaining cost basis.
    pub cost_basis: Decimal,
    /// `cost_basis / quantity`.
    pub average_cost: Decimal,
}

/// Valuation of one asset aggregated across wallets.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetValuation {
    /// Asset.
    pub asset: String,
    /// Total open units across wallets.
    pub quantity: Decimal,
    /// Total remaining cost basis.
    pub cost_basis: Decimal,
    /// Supplied current price.
    pub price: Decimal,
    /// `quantity * price`.
    pub market_value: Decimal,
    /// `market_value - cost_basis`.
    pub unrealized: Decimal,
    /// `market_value / portfolio market_value` in `0.0..=1.0`.
    pub allocation: Decimal,
}

/// Whole-portfolio valuation at supplied prices.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PortfolioReport {
    /// Per-asset valuations (priced assets only).
    pub assets: Vec<AssetValuation>,
    /// Sum of cost basis of priced assets.
    pub total_cost: Decimal,
    /// Sum of market value of priced assets.
    pub total_value: Decimal,
    /// `total_value - total_cost`.
    pub total_unrealized: Decimal,
    /// `total_unrealized / total_cost` (0 when `total_cost` is 0).
    pub total_return: Decimal,
    /// Held assets with no supplied price (excluded from totals).
    pub missing_prices: Vec<String>,
}

/// Form-8949-shaped capital-gains report for one calendar tax year (UTC).
///
/// Under `Average`, rows have no `term`, so `short_term_gain + long_term_gain`
/// may be less than `total_gain`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CapitalGainsReport {
    /// The calendar year covered.
    pub tax_year: i32,
    /// Disposals settled in this year.
    pub rows: Vec<RealizedGain>,
    /// Sum of gains on short-term rows.
    pub short_term_gain: Decimal,
    /// Sum of gains on long-term rows.
    pub long_term_gain: Decimal,
    /// Sum of gains on all rows.
    pub total_gain: Decimal,
}

/// Ordinary-income report for one calendar tax year (UTC).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IncomeReport {
    /// The calendar year covered.
    pub tax_year: i32,
    /// Income events received in this year.
    pub events: Vec<IncomeEvent>,
    /// Sum of `value` across events.
    pub total_income: Decimal,
}
```

Re-enable in `src/lib.rs`:
```rust
pub use report::{
    AssetValuation, CapitalGainsReport, Holding, IncomeEvent, IncomeReport, PortfolioReport,
    RealizedGain, Term,
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib report`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/report.rs src/lib.rs
git commit -m "Add public report types and holding-period classification"
```

---

## Task 8: Engine core — acquisitions + FIFO disposals + holding period

This task builds the replay engine skeleton and implements `Buy`, `Income`,
`GiftReceived` (acquisitions) and `Sell` (disposal) under all automatic methods'
shared consumption machinery, with FIFO/LIFO/HIFO ordering already available
from Task 6. Average and Specific-ID consumption arrive in Task 9; Trade/Spend
in Task 10; Transfer in Task 11; gifts' dual-basis disposal in Task 12.

**Files:**
- Modify: `src/engine.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src/engine.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::method::CostBasisMethod;
    use crate::transaction::Transaction;
    use chrono::{TimeZone, Utc};
    use rust_decimal_macros::dec;

    fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
    }

    #[test]
    fn fifo_sell_consumes_oldest_lot_with_term() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2020, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            Transaction::Buy { timestamp: ts(2021, 6, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(300), fee: dec!(0) },
            // Sell 1 BTC at 500 in 2022; FIFO consumes the 2020 lot (long-term).
            Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert_eq!(out.realized.len(), 1);
        let g = &out.realized[0];
        assert_eq!(g.cost_basis, dec!(100));
        assert_eq!(g.proceeds, dec!(500));
        assert_eq!(g.gain, dec!(400));
        assert_eq!(g.term, Some(crate::report::Term::Long));
        // One lot (the 2021 one) remains.
        assert_eq!(out.holdings.len(), 1);
        assert_eq!(out.holdings[0].cost_basis, dec!(300));
    }

    #[test]
    fn buy_fee_folds_into_basis() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "eth".into(),
                quantity: dec!(2), unit_price: dec!(100), fee: dec!(10) },
            Transaction::Sell { timestamp: ts(2021, 2, 1), wallet: "w".into(), asset: "eth".into(),
                quantity: dec!(1), unit_price: dec!(150), fee: dec!(0) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        // Lot basis = 2*100 + 10 = 210; per unit 105; selling 1 -> basis 105.
        assert_eq!(out.realized[0].cost_basis, dec!(105));
        assert_eq!(out.realized[0].gain, dec!(45));
    }

    #[test]
    fn oversell_errors_per_wallet() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "hot".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            // Selling from a different wallet that holds nothing.
            Transaction::Sell { timestamp: ts(2021, 2, 1), wallet: "cold".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(150), fee: dec!(0) },
        ];
        let err = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap_err();
        assert!(matches!(err, crate::error::PortfolioError::InsufficientLots { .. }));
    }

    #[test]
    fn income_records_event_and_sets_basis() {
        let txs = vec![
            Transaction::Income { timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "eth".into(),
                quantity: dec!(1), value: dec!(50), source: crate::transaction::IncomeSource::Staking },
            Transaction::Sell { timestamp: ts(2021, 2, 1), wallet: "w".into(), asset: "eth".into(),
                quantity: dec!(1), unit_price: dec!(70), fee: dec!(0) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert_eq!(out.income.len(), 1);
        assert_eq!(out.income[0].value, dec!(50));
        assert_eq!(out.realized[0].cost_basis, dec!(50)); // income FMV became basis
        assert_eq!(out.realized[0].gain, dec!(20));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine`
Expected: FAIL — `run` / `Strategy` not defined.

- [ ] **Step 3: Write the implementation**

Replace `src/engine.rs` (above the tests) with the engine. This is the
crate's core; later tasks extend the `match` in `process` and the `consume`
function, but the structure below is final.

```rust
//! Internal ledger-replay engine. Builds per-`(asset, wallet)` lot pools by
//! processing events in timestamp order, producing realized gains, income
//! events, and the remaining open lots.

use crate::error::PortfolioError;
use crate::lot::{GiftBasis, Lot};
use crate::method::{self, CostBasisMethod, LotSelection};
use crate::report::{IncomeEvent, RealizedGain, Term};
use crate::transaction::Transaction;
use rust_decimal::Decimal;
use std::collections::HashMap;

/// How to match disposals to lots for this run.
pub(crate) enum Strategy<'a> {
    /// An automatic method (FIFO/LIFO/HIFO/Average).
    Auto(CostBasisMethod),
    /// Specific-ID with caller-provided selections (disposal input index → picks).
    Specific(&'a LotSelection),
}

impl Strategy<'_> {
    fn method(&self) -> CostBasisMethod {
        match self {
            Strategy::Auto(m) => *m,
            Strategy::Specific(_) => CostBasisMethod::SpecificId,
        }
    }
}

/// What one run of the engine produces.
pub(crate) struct EngineOutput {
    pub realized: Vec<RealizedGain>,
    pub income: Vec<IncomeEvent>,
    pub holdings: Vec<Lot>,
}

/// A slice consumed out of a lot during a disposal/move.
struct Consumed {
    quantity: Decimal,
    cost_basis: Decimal,
    /// `None` under `Average` (pooled — no single date).
    acquired_at: Option<chrono::DateTime<chrono::Utc>>,
    gift: Option<GiftBasis>,
    /// The lot this came from (for transfers, which preserve identity).
    lot_id: u64,
}

struct Engine<'a> {
    strategy: Strategy<'a>,
    next_lot_id: u64,
    pools: HashMap<(String, String), Vec<Lot>>,
    realized: Vec<RealizedGain>,
    income: Vec<IncomeEvent>,
    /// Original acquisition input index → lot_id (for Specific-ID resolution).
    acq_to_lot: HashMap<usize, u64>,
}

impl<'a> Engine<'a> {
    fn new(strategy: Strategy<'a>) -> Self {
        Engine {
            strategy,
            next_lot_id: 0,
            pools: HashMap::new(),
            realized: Vec::new(),
            income: Vec::new(),
            acq_to_lot: HashMap::new(),
        }
    }

    fn pool(&mut self, asset: &str, wallet: &str) -> &mut Vec<Lot> {
        self.pools.entry((asset.to_string(), wallet.to_string())).or_default()
    }

    fn available(&self, asset: &str, wallet: &str) -> Decimal {
        self.pools
            .get(&(asset.to_string(), wallet.to_string()))
            .map(|ls| ls.iter().map(|l| l.quantity).sum())
            .unwrap_or(Decimal::ZERO)
    }

    /// Open a new lot in a wallet. `orig_index` is the acquisition's original
    /// input index (registered for Specific-ID).
    fn acquire(
        &mut self,
        orig_index: usize,
        asset: &str,
        wallet: &str,
        quantity: Decimal,
        cost_basis: Decimal,
        acquired_at: chrono::DateTime<chrono::Utc>,
        gift: Option<GiftBasis>,
    ) {
        let lot_id = self.next_lot_id;
        self.next_lot_id += 1;
        self.acq_to_lot.insert(orig_index, lot_id);
        self.pool(asset, wallet).push(Lot {
            asset: asset.to_string(),
            wallet: wallet.to_string(),
            quantity,
            cost_basis,
            acquired_at,
            lot_id,
            gift,
        });
    }

    /// Remove `quantity` units from a pool and return the consumed slices.
    /// `orig_index` is the disposal's original input index (Specific-ID lookup).
    fn consume(
        &mut self,
        orig_index: usize,
        asset: &str,
        wallet: &str,
        quantity: Decimal,
    ) -> Result<Vec<Consumed>, PortfolioError> {
        let available = self.available(asset, wallet);
        if quantity > available {
            return Err(PortfolioError::InsufficientLots {
                asset: asset.to_string(),
                wallet: wallet.to_string(),
                attempted: quantity,
                available,
            });
        }

        match self.strategy.method() {
            CostBasisMethod::Average => self.consume_average(asset, wallet, quantity),
            CostBasisMethod::SpecificId => self.consume_specific(orig_index, asset, wallet, quantity),
            auto => self.consume_ordered(auto, asset, wallet, quantity),
        }
    }

    fn consume_ordered(
        &mut self,
        method: CostBasisMethod,
        asset: &str,
        wallet: &str,
        quantity: Decimal,
    ) -> Result<Vec<Consumed>, PortfolioError> {
        let lots = self.pool(asset, wallet);
        let order = method::order_for(method, lots);
        let mut remaining = quantity;
        let mut out = Vec::new();
        for i in order {
            if remaining <= Decimal::ZERO {
                break;
            }
            let take = remaining.min(lots[i].quantity);
            if take <= Decimal::ZERO {
                continue;
            }
            let per_unit = lots[i].cost_basis_per_unit();
            let basis = per_unit * take;
            out.push(Consumed {
                quantity: take,
                cost_basis: basis,
                acquired_at: Some(lots[i].acquired_at),
                gift: lots[i].gift.clone(),
                lot_id: lots[i].lot_id,
            });
            lots[i].quantity -= take;
            lots[i].cost_basis -= basis;
            remaining -= take;
        }
        lots.retain(|l| l.quantity > Decimal::ZERO);
        Ok(out)
    }

    fn consume_average(
        &mut self,
        asset: &str,
        wallet: &str,
        quantity: Decimal,
    ) -> Result<Vec<Consumed>, PortfolioError> {
        let lots = self.pool(asset, wallet);
        let total_qty: Decimal = lots.iter().map(|l| l.quantity).sum();
        let total_basis: Decimal = lots.iter().map(|l| l.cost_basis).sum();
        let avg = total_basis / total_qty;
        let basis = avg * quantity;
        // Collapse the pool into a single remaining averaged lot.
        let remaining_qty = total_qty - quantity;
        let lot_id = lots.first().map(|l| l.lot_id).unwrap_or(0);
        let acquired_at = lots.iter().map(|l| l.acquired_at).min().unwrap();
        lots.clear();
        if remaining_qty > Decimal::ZERO {
            lots.push(Lot {
                asset: asset.to_string(),
                wallet: wallet.to_string(),
                quantity: remaining_qty,
                cost_basis: total_basis - basis,
                acquired_at,
                lot_id,
                gift: None,
            });
        }
        Ok(vec![Consumed {
            quantity,
            cost_basis: basis,
            acquired_at: None, // Average: no single date / term
            gift: None,
            lot_id,
        }])
    }

    fn consume_specific(
        &mut self,
        orig_index: usize,
        asset: &str,
        wallet: &str,
        quantity: Decimal,
    ) -> Result<Vec<Consumed>, PortfolioError> {
        let picks = match &self.strategy {
            Strategy::Specific(sel) => sel.get(&orig_index).cloned(),
            Strategy::Auto(_) => None,
        };
        let picks = picks.ok_or(PortfolioError::MissingLotSelection {
            asset: asset.to_string(),
            tx_index: orig_index,
        })?;
        let total_picked: Decimal = picks.iter().map(|p| p.quantity).sum();
        if total_picked != quantity {
            return Err(PortfolioError::MissingLotSelection {
                asset: asset.to_string(),
                tx_index: orig_index,
            });
        }
        let mut out = Vec::new();
        for pick in picks {
            let target_lot_id = *self
                .acq_to_lot
                .get(&pick.acquisition_index)
                .ok_or(PortfolioError::InvalidLotSelection { acquisition_index: pick.acquisition_index })?;
            let lots = self.pool(asset, wallet);
            let pos = lots
                .iter()
                .position(|l| l.lot_id == target_lot_id && l.quantity >= pick.quantity)
                .ok_or(PortfolioError::InvalidLotSelection { acquisition_index: pick.acquisition_index })?;
            let per_unit = lots[pos].cost_basis_per_unit();
            let basis = per_unit * pick.quantity;
            out.push(Consumed {
                quantity: pick.quantity,
                cost_basis: basis,
                acquired_at: Some(lots[pos].acquired_at),
                gift: lots[pos].gift.clone(),
                lot_id: target_lot_id,
            });
            lots[pos].quantity -= pick.quantity;
            lots[pos].cost_basis -= basis;
            lots.retain(|l| l.quantity > Decimal::ZERO);
        }
        Ok(out)
    }

    /// Compute (gain, basis_reported) for one consumed slice given allocated
    /// proceeds, applying the gift dual-basis rule when present.
    fn gain_for(c: &Consumed, proceeds: Decimal) -> (Decimal, Decimal) {
        match &c.gift {
            None => (proceeds - c.cost_basis, c.cost_basis),
            Some(g) => {
                let donor_basis = c.cost_basis; // carryover basis for this slice
                let fmv = g.fmv_per_unit * c.quantity;
                if proceeds > donor_basis {
                    (proceeds - donor_basis, donor_basis)
                } else {
                    let loss_basis = donor_basis.min(fmv);
                    if proceeds < loss_basis {
                        (proceeds - loss_basis, loss_basis)
                    } else {
                        // Dead zone: no gain, no loss.
                        (Decimal::ZERO, proceeds)
                    }
                }
            }
        }
    }

    /// Dispose `quantity` units, distributing `total_proceeds` across the
    /// consumed slices and pushing one `RealizedGain` per slice.
    fn dispose(
        &mut self,
        orig_index: usize,
        asset: &str,
        wallet: &str,
        quantity: Decimal,
        total_proceeds: Decimal,
        disposed_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), PortfolioError> {
        let consumed = self.consume(orig_index, asset, wallet, quantity)?;
        for c in &consumed {
            let proceeds = total_proceeds * (c.quantity / quantity);
            let (gain, basis) = Self::gain_for(c, proceeds);
            let term = c.acquired_at.map(|a| Term::classify(a, disposed_at));
            self.realized.push(RealizedGain {
                asset: asset.to_string(),
                wallet: wallet.to_string(),
                disposed_at,
                acquired_at: c.acquired_at,
                quantity: c.quantity,
                proceeds,
                cost_basis: basis,
                gain,
                term,
            });
        }
        Ok(())
    }

    fn process(&mut self, orig_index: usize, tx: &Transaction) -> Result<(), PortfolioError> {
        match tx {
            Transaction::Buy { timestamp, wallet, asset, quantity, unit_price, fee } => {
                let basis = *quantity * *unit_price + *fee;
                self.acquire(orig_index, asset, wallet, *quantity, basis, *timestamp, None);
            }
            Transaction::Income { timestamp, wallet, asset, quantity, value, source } => {
                self.acquire(orig_index, asset, wallet, *quantity, *value, *timestamp, None);
                self.income.push(IncomeEvent {
                    asset: asset.clone(),
                    wallet: wallet.clone(),
                    received_at: *timestamp,
                    quantity: *quantity,
                    value: *value,
                    source: *source,
                });
            }
            Transaction::GiftReceived {
                timestamp, wallet, asset, quantity, donor_basis, fmv_at_receipt, donor_acquired_at,
            } => {
                // Average ignores dual basis (pool at donor/carryover basis).
                let gift = if self.strategy.method() == CostBasisMethod::Average {
                    None
                } else {
                    Some(GiftBasis { fmv_per_unit: *fmv_at_receipt / *quantity })
                };
                self.acquire(orig_index, asset, wallet, *quantity, *donor_basis, *donor_acquired_at, gift);
                let _ = timestamp; // receipt time is not the holding-period start
            }
            Transaction::Sell { timestamp, wallet, asset, quantity, unit_price, fee } => {
                let proceeds = *quantity * *unit_price - *fee;
                self.dispose(orig_index, asset, wallet, *quantity, proceeds, *timestamp)?;
            }
            // Trade, Spend handled in Task 10; Transfer in Task 11; GiftSent in Task 12.
            _ => unimplemented!("handled in a later task"),
        }
        Ok(())
    }

    fn finish(self) -> EngineOutput {
        let mut holdings: Vec<Lot> = self.pools.into_values().flatten().collect();
        holdings.sort_by(|a, b| a.lot_id.cmp(&b.lot_id));
        EngineOutput { realized: self.realized, income: self.income, holdings }
    }
}

/// Replay a ledger under a strategy. `txs` is in original input order; events
/// are processed in timestamp order (stable), and original indices are used for
/// Specific-ID lookups.
pub(crate) fn run(txs: &[Transaction], strategy: Strategy) -> Result<EngineOutput, PortfolioError> {
    let mut order: Vec<usize> = (0..txs.len()).collect();
    order.sort_by(|&a, &b| txs[a].timestamp().cmp(&txs[b].timestamp()));
    let mut engine = Engine::new(strategy);
    for oi in order {
        engine.process(oi, &txs[oi])?;
    }
    Ok(engine.finish())
}
```

> Note: `process` uses `unimplemented!()` for the not-yet-added variants. Tasks
> 10–12 replace those arms. The tests in this task only exercise Buy/Income/
> GiftReceived/Sell, so they will pass.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib engine`
Expected: PASS (all 4).

- [ ] **Step 5: Commit**

```bash
git add src/engine.rs
git commit -m "Add replay engine: acquisitions, ordered disposals, holding period"
```

---

## Task 9: Average and Specific-ID disposal coverage

The engine already routes `Average` and `SpecificId` through `consume_average`
and `consume_specific`. This task adds tests proving they behave correctly and
fixes anything they surface.

**Files:**
- Modify: `src/engine.rs` (tests; implementation already present)

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `src/engine.rs`:
```rust
    #[test]
    fn average_pools_basis_and_drops_term() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2020, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(300), fee: dec!(0) },
            Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Average)).unwrap();
        // avg cost = (100+300)/2 = 200 -> gain = 300
        assert_eq!(out.realized[0].cost_basis, dec!(200));
        assert_eq!(out.realized[0].gain, dec!(300));
        assert_eq!(out.realized[0].term, None);
        assert_eq!(out.holdings.iter().map(|l| l.quantity).sum::<Decimal>(), dec!(1));
    }

    #[test]
    fn specific_id_consumes_named_acquisition() {
        let txs = vec![
            // index 0: cheap lot
            Transaction::Buy { timestamp: ts(2020, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            // index 1: expensive lot
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(400), fee: dec!(0) },
            // index 2: sell 1, specifically the expensive lot (index 1)
            Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
        ];
        let mut sel: crate::method::LotSelection = std::collections::HashMap::new();
        sel.insert(2, vec![crate::method::LotPick { acquisition_index: 1, quantity: dec!(1) }]);
        let out = run(&txs, Strategy::Specific(&sel)).unwrap();
        assert_eq!(out.realized[0].cost_basis, dec!(400));
        assert_eq!(out.realized[0].gain, dec!(100));
    }

    #[test]
    fn specific_id_missing_selection_errors() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2020, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
        ];
        let sel: crate::method::LotSelection = std::collections::HashMap::new();
        let err = run(&txs, Strategy::Specific(&sel)).unwrap_err();
        assert!(matches!(err, crate::error::PortfolioError::MissingLotSelection { .. }));
    }
```

- [ ] **Step 2: Run tests to verify they fail (or pass)**

Run: `cargo test --lib engine`
Expected: these three either PASS immediately (implementation already present) or
reveal a bug to fix. If `average_pools_basis_and_drops_term` fails on the
remaining-holdings assertion, confirm `consume_average` clears and repushes the
pool correctly.

- [ ] **Step 3: Fix any surfaced bug**

If a test fails, correct `consume_average`/`consume_specific` in `src/engine.rs`
to satisfy it. (No new code expected if Task 8 was implemented exactly.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib engine`
Expected: PASS (all engine tests).

- [ ] **Step 5: Commit**

```bash
git add src/engine.rs
git commit -m "Cover Average and Specific-ID disposals with tests"
```

---

## Task 10: Trade and Spend events

**Files:**
- Modify: `src/engine.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `src/engine.rs`:
```rust
    #[test]
    fn trade_disposes_from_leg_and_opens_to_leg() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            // Trade 1 BTC (FMV 500) for 10 ETH, fee 5 -> ETH basis = 505.
            Transaction::Trade { timestamp: ts(2021, 6, 1), wallet: "w".into(),
                from_asset: "btc".into(), from_quantity: dec!(1),
                to_asset: "eth".into(), to_quantity: dec!(10),
                value: dec!(500), fee: dec!(5) },
            Transaction::Sell { timestamp: ts(2021, 7, 1), wallet: "w".into(), asset: "eth".into(),
                quantity: dec!(10), unit_price: dec!(60), fee: dec!(0) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        // First realized: BTC disposal, proceeds 500, basis 100, gain 400.
        let btc = out.realized.iter().find(|r| r.asset == "btc").unwrap();
        assert_eq!(btc.gain, dec!(400));
        // Second realized: ETH sale, proceeds 600, basis 505, gain 95.
        let eth = out.realized.iter().find(|r| r.asset == "eth").unwrap();
        assert_eq!(eth.cost_basis, dec!(505));
        assert_eq!(eth.gain, dec!(95));
    }

    #[test]
    fn spend_is_a_disposal_at_fmv() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            Transaction::Spend { timestamp: ts(2021, 2, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), value: dec!(180), fee: dec!(0) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert_eq!(out.realized[0].proceeds, dec!(180));
        assert_eq!(out.realized[0].gain, dec!(80));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine trade`
Expected: FAIL — `Trade`/`Spend` hit `unimplemented!()`.

- [ ] **Step 3: Write the implementation**

In `src/engine.rs`, replace the `// Trade, Spend handled in Task 10` comment and
its `_ => unimplemented!(...)` arm by adding these match arms before the
remaining catch-all:

```rust
            Transaction::Trade {
                timestamp, wallet, from_asset, from_quantity, to_asset, to_quantity, value, fee,
            } => {
                // Disposal of the given-up leg at FMV.
                self.dispose(orig_index, from_asset, wallet, *from_quantity, *value, *timestamp)?;
                // Acquisition of the received leg; basis = FMV + fee.
                self.acquire(orig_index, to_asset, wallet, *to_quantity, *value + *fee, *timestamp, None);
            }
            Transaction::Spend { timestamp, wallet, asset, quantity, value, fee } => {
                let proceeds = *value - *fee;
                self.dispose(orig_index, asset, wallet, *quantity, proceeds, *timestamp)?;
            }
```

> Note: `acquire` registers `acq_to_lot[orig_index]`. A `Trade` is both a
> disposal and an acquisition at the same input index; Specific-ID picks
> referencing a trade's acquired lot use that trade's input index. This is
> documented behavior.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib engine`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/engine.rs
git commit -m "Handle Trade and Spend events in the engine"
```

---

## Task 11: Transfer event (move + fee disposal)

**Files:**
- Modify: `src/engine.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `src/engine.rs`:
```rust
    #[test]
    fn transfer_preserves_basis_and_acquisition_date() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2020, 1, 1), wallet: "hot".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            // Move to cold wallet in 2020, no fee.
            Transaction::Transfer { timestamp: ts(2020, 6, 1), asset: "btc".into(), quantity: dec!(1),
                from_wallet: "hot".into(), to_wallet: "cold".into(), fee: dec!(0), fee_value: dec!(0) },
            // Sell from cold in 2022; term must be Long, measured from 2020-01-01.
            Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "cold".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert_eq!(out.realized.len(), 1);
        assert_eq!(out.realized[0].wallet, "cold");
        assert_eq!(out.realized[0].cost_basis, dec!(100));
        assert_eq!(out.realized[0].term, Some(crate::report::Term::Long));
    }

    #[test]
    fn transfer_fee_is_a_disposal() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "hot".into(), asset: "eth".into(),
                quantity: dec!(10), unit_price: dec!(10), fee: dec!(0) }, // basis 100, per-unit 10
            // Move 9, burn 1 as fee at FMV 15.
            Transaction::Transfer { timestamp: ts(2021, 2, 1), asset: "eth".into(), quantity: dec!(9),
                from_wallet: "hot".into(), to_wallet: "cold".into(), fee: dec!(1), fee_value: dec!(15) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        // Fee disposal: 1 unit, basis 10, proceeds 15, gain 5.
        assert_eq!(out.realized.len(), 1);
        assert_eq!(out.realized[0].gain, dec!(5));
        // 9 units now in cold wallet, basis 90.
        let cold: Decimal = out.holdings.iter().filter(|l| l.wallet == "cold").map(|l| l.cost_basis).sum();
        assert_eq!(cold, dec!(90));
    }

    #[test]
    fn transfer_insufficient_balance_errors() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "hot".into(), asset: "eth".into(),
                quantity: dec!(1), unit_price: dec!(10), fee: dec!(0) },
            Transaction::Transfer { timestamp: ts(2021, 2, 1), asset: "eth".into(), quantity: dec!(1),
                from_wallet: "hot".into(), to_wallet: "cold".into(), fee: dec!(1), fee_value: dec!(15) },
        ];
        let err = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap_err();
        assert!(matches!(err, crate::error::PortfolioError::InsufficientTransfer { .. }));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine transfer`
Expected: FAIL — `Transfer` hits `unimplemented!()`.

- [ ] **Step 3: Write the implementation**

First add a `take` helper to `impl Engine` (near `consume`), which removes units
**without** realizing a gain and returns the consumed slices so they can be
re-homed:

```rust
    /// Remove `quantity` units from a pool without realizing gain (for moves).
    /// Uses the active method's ordering (Average pools; SpecificId falls back to
    /// FIFO, since moves are non-taxable and need no caller selection).
    fn take(&mut self, asset: &str, wallet: &str, quantity: Decimal) -> Result<Vec<Consumed>, PortfolioError> {
        let method = match self.strategy.method() {
            CostBasisMethod::Average => CostBasisMethod::Average,
            _ => CostBasisMethod::Fifo,
        };
        if method == CostBasisMethod::Average {
            self.consume_average(asset, wallet, quantity)
        } else {
            self.consume_ordered(method, asset, wallet, quantity)
        }
    }
```

Then add the `Transfer` arm to `process` (before the catch-all):

```rust
            Transaction::Transfer { timestamp, asset, quantity, from_wallet, to_wallet, fee, fee_value } => {
                let available = self.available(asset, from_wallet);
                if *quantity + *fee > available {
                    return Err(PortfolioError::InsufficientTransfer {
                        asset: asset.clone(),
                        wallet: from_wallet.clone(),
                        quantity: *quantity,
                        fee: *fee,
                        available,
                    });
                }
                // Fee paid in the asset is a taxable disposal from the source.
                if *fee > Decimal::ZERO {
                    self.dispose(orig_index, asset, from_wallet, *fee, *fee_value, *timestamp)?;
                }
                // Move the rest, preserving basis, acquisition date, and lot id.
                let moved = self.take(asset, from_wallet, *quantity)?;
                for m in moved {
                    let acquired_at = m.acquired_at.unwrap_or(*timestamp);
                    self.pool(asset, to_wallet).push(Lot {
                        asset: asset.clone(),
                        wallet: to_wallet.clone(),
                        quantity: m.quantity,
                        cost_basis: m.cost_basis,
                        acquired_at,
                        lot_id: m.lot_id,
                        gift: m.gift,
                    });
                }
            }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib engine`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/engine.rs
git commit -m "Handle Transfer events: move lots and dispose fees"
```

---

## Task 12: Gifts — sent and received (dual-basis disposal)

The dual-basis math already lives in `gain_for` (Task 8). This task wires up
`GiftSent` (non-taxable removal) and proves the `GiftReceived` dual-basis rule
across all three branches.

**Files:**
- Modify: `src/engine.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `src/engine.rs`:
```rust
    fn gift_received(qty: i64, donor_basis: i64, fmv: i64, donor_day_year: i32) -> Transaction {
        Transaction::GiftReceived {
            timestamp: ts(2021, 6, 1),
            wallet: "w".into(),
            asset: "btc".into(),
            quantity: Decimal::from(qty),
            donor_basis: Decimal::from(donor_basis),
            fmv_at_receipt: Decimal::from(fmv),
            donor_acquired_at: ts(donor_day_year, 1, 1),
        }
    }

    #[test]
    fn gift_sent_removes_lots_without_gain() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(2), unit_price: dec!(100), fee: dec!(0) },
            Transaction::GiftSent { timestamp: ts(2021, 2, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert!(out.realized.is_empty());
        assert_eq!(out.holdings.iter().map(|l| l.quantity).sum::<Decimal>(), dec!(1));
    }

    #[test]
    fn gift_gain_uses_donor_basis() {
        // donor_basis 100, fmv 120; sell at 200 -> gain = 100 (carryover basis).
        let txs = vec![
            gift_received(1, 100, 120, 2018),
            Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(200), fee: dec!(0) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert_eq!(out.realized[0].cost_basis, dec!(100));
        assert_eq!(out.realized[0].gain, dec!(100));
        // Holding period tacks from donor's 2018 date -> Long.
        assert_eq!(out.realized[0].term, Some(crate::report::Term::Long));
    }

    #[test]
    fn gift_loss_uses_lesser_of_basis_or_fmv() {
        // donor_basis 100, fmv 80; sell at 50 -> loss vs 80 = -30.
        let txs = vec![
            gift_received(1, 100, 80, 2018),
            Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(50), fee: dec!(0) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert_eq!(out.realized[0].cost_basis, dec!(80));
        assert_eq!(out.realized[0].gain, dec!(-30));
    }

    #[test]
    fn gift_dead_zone_realizes_nothing() {
        // donor_basis 100, fmv 80; sell at 90 (between 80 and 100) -> gain 0.
        let txs = vec![
            gift_received(1, 100, 80, 2018),
            Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(90), fee: dec!(0) },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert_eq!(out.realized[0].gain, dec!(0));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine gift`
Expected: FAIL — `GiftSent` hits `unimplemented!()` (the `GiftReceived` and
dual-basis tests may already pass from Tasks 8's `gain_for`).

- [ ] **Step 3: Write the implementation**

Replace the remaining `_ => unimplemented!("handled in a later task")` arm in
`process` with the `GiftSent` arm and a real catch-all is no longer needed (all
variants are now handled):

```rust
            Transaction::GiftSent { timestamp, wallet, asset, quantity } => {
                // Non-taxable: remove lots (no realized gain), discard them.
                let _ = self.take(asset, wallet, *quantity)?;
                let _ = timestamp;
            }
```

After adding this arm, every `Transaction` variant is matched, so remove the
`_ => unimplemented!(...)` line entirely. The compiler will confirm exhaustiveness.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib engine`
Expected: PASS (all engine tests).

- [ ] **Step 5: Commit**

```bash
git add src/engine.rs
git commit -m "Handle gifts: non-taxable GiftSent and dual-basis GiftReceived"
```

---

## Task 13: Portfolio facade — construction and core queries

**Files:**
- Modify: `src/portfolio.rs`
- Modify: `src/lib.rs` (re-enable `pub use portfolio::Portfolio;`)

- [ ] **Step 1: Write the failing tests**

Append to `src/portfolio.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::method::{CostBasisMethod, LotPick, LotSelection};
    use crate::transaction::Transaction;
    use chrono::{TimeZone, Utc};
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
    }

    fn sample() -> Vec<Transaction> {
        vec![
            Transaction::Buy { timestamp: ts(2020, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
        ]
    }

    #[test]
    fn from_transactions_validates() {
        let bad = vec![Transaction::Buy { timestamp: ts(2020, 1, 1), wallet: "w".into(),
            asset: "btc".into(), quantity: dec!(0), unit_price: dec!(100), fee: dec!(0) }];
        assert!(Portfolio::from_transactions(&bad).is_err());
    }

    #[test]
    fn realized_gains_auto_method() {
        let p = Portfolio::from_transactions(&sample()).unwrap();
        let g = p.realized_gains(CostBasisMethod::Fifo).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].gain, dec!(400));
    }

    #[test]
    fn realized_gains_rejects_specific_id() {
        let p = Portfolio::from_transactions(&sample()).unwrap();
        assert!(matches!(
            p.realized_gains(CostBasisMethod::SpecificId),
            Err(crate::error::PortfolioError::SelectionRequired)
        ));
    }

    #[test]
    fn realized_gains_with_selection_works() {
        let p = Portfolio::from_transactions(&sample()).unwrap();
        let mut sel: LotSelection = HashMap::new();
        sel.insert(1, vec![LotPick { acquisition_index: 0, quantity: dec!(1) }]);
        let g = p.realized_gains_with_selection(&sel).unwrap();
        assert_eq!(g[0].gain, dec!(400));
    }

    #[test]
    fn holdings_reports_open_positions() {
        let txs = vec![Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "w".into(),
            asset: "eth".into(), quantity: dec!(2), unit_price: dec!(50), fee: dec!(0) }];
        let p = Portfolio::from_transactions(&txs).unwrap();
        let h = p.holdings(CostBasisMethod::Fifo).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].quantity, dec!(2));
        assert_eq!(h[0].average_cost, dec!(50));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib portfolio`
Expected: FAIL — `Portfolio` not defined.

- [ ] **Step 3: Write the implementation**

Replace `src/portfolio.rs` (above the tests) with:
```rust
//! The public [`Portfolio`] facade: stores an immutable ledger and answers
//! cost-basis, income, holdings, valuation, and tax-report queries.

use crate::engine::{run, Strategy};
use crate::error::PortfolioError;
use crate::lot::Lot;
use crate::method::{CostBasisMethod, LotSelection};
use crate::report::{Holding, IncomeEvent, RealizedGain};
use crate::transaction::Transaction;

/// An immutable ledger you query under a chosen cost-basis method.
///
/// Construct with [`Portfolio::from_transactions`], then call query methods.
/// For [`CostBasisMethod::SpecificId`], use the `*_with_selection` variants.
#[derive(Clone, Debug)]
pub struct Portfolio {
    txs: Vec<Transaction>,
}

fn to_holding(l: &Lot) -> Holding {
    Holding {
        asset: l.asset.clone(),
        wallet: l.wallet.clone(),
        quantity: l.quantity,
        cost_basis: l.cost_basis,
        average_cost: l.cost_basis / l.quantity,
    }
}

impl Portfolio {
    /// Build a portfolio from a ledger, validating each event's fields. The
    /// original order is preserved (Specific-ID indices refer to it).
    pub fn from_transactions(txs: &[Transaction]) -> Result<Self, PortfolioError> {
        for tx in txs {
            tx.validate()?;
        }
        Ok(Portfolio { txs: txs.to_vec() })
    }

    /// Realized capital-gain rows under an automatic method. Returns
    /// [`PortfolioError::SelectionRequired`] for `SpecificId`.
    pub fn realized_gains(&self, method: CostBasisMethod) -> Result<Vec<RealizedGain>, PortfolioError> {
        if method == CostBasisMethod::SpecificId {
            return Err(PortfolioError::SelectionRequired);
        }
        Ok(run(&self.txs, Strategy::Auto(method))?.realized)
    }

    /// Realized capital-gain rows using a Specific-ID selection.
    pub fn realized_gains_with_selection(&self, selection: &LotSelection) -> Result<Vec<RealizedGain>, PortfolioError> {
        Ok(run(&self.txs, Strategy::Specific(selection))?.realized)
    }

    /// Ordinary-income events (method-independent).
    pub fn income_events(&self) -> Vec<IncomeEvent> {
        // Income does not depend on the disposal method; FIFO is a safe choice
        // and cannot error on a validated ledger that has no disposals issues...
        // but to be robust we ignore disposal errors by recomputing income from
        // the ledger directly.
        self.txs
            .iter()
            .filter_map(|tx| match tx {
                Transaction::Income { timestamp, wallet, asset, quantity, value, source } => {
                    Some(IncomeEvent {
                        asset: asset.clone(),
                        wallet: wallet.clone(),
                        received_at: *timestamp,
                        quantity: *quantity,
                        value: *value,
                        source: *source,
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// Current open positions (per wallet) under an automatic method.
    pub fn holdings(&self, method: CostBasisMethod) -> Result<Vec<Holding>, PortfolioError> {
        if method == CostBasisMethod::SpecificId {
            return Err(PortfolioError::SelectionRequired);
        }
        Ok(run(&self.txs, Strategy::Auto(method))?.holdings.iter().map(to_holding).collect())
    }
}
```

Re-enable in `src/lib.rs`:
```rust
pub use portfolio::Portfolio;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib portfolio`
Expected: PASS (all 5).

- [ ] **Step 5: Commit**

```bash
git add src/portfolio.rs src/lib.rs
git commit -m "Add Portfolio facade: construction, realized gains, income, holdings"
```

---

## Task 14: Valuation

**Files:**
- Modify: `src/portfolio.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `src/portfolio.rs`:
```rust
    #[test]
    fn valuation_aggregates_and_flags_missing_prices() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "a".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            Transaction::Buy { timestamp: ts(2021, 1, 2), wallet: "b".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(140), fee: dec!(0) },
            Transaction::Buy { timestamp: ts(2021, 1, 3), wallet: "a".into(), asset: "eth".into(),
                quantity: dec!(1), unit_price: dec!(50), fee: dec!(0) },
        ];
        let p = Portfolio::from_transactions(&txs).unwrap();
        let mut prices = HashMap::new();
        prices.insert("btc".to_string(), dec!(200));
        // eth price intentionally omitted.
        let r = p.valuation(CostBasisMethod::Fifo, &prices).unwrap();
        // BTC aggregated across wallets: qty 2, basis 240, value 400, unrealized 160.
        let btc = r.assets.iter().find(|a| a.asset == "btc").unwrap();
        assert_eq!(btc.quantity, dec!(2));
        assert_eq!(btc.cost_basis, dec!(240));
        assert_eq!(btc.market_value, dec!(400));
        assert_eq!(btc.unrealized, dec!(160));
        assert_eq!(btc.allocation, dec!(1)); // only priced asset
        assert_eq!(r.total_value, dec!(400));
        assert_eq!(r.missing_prices, vec!["eth".to_string()]);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib valuation_aggregates`
Expected: FAIL — `valuation` not defined.

- [ ] **Step 3: Write the implementation**

Add `valuation` to `impl Portfolio` in `src/portfolio.rs`, and the needed imports
at the top (`use crate::report::{AssetValuation, PortfolioReport};` alongside the
existing report import — merge into one `use` line; and `use rust_decimal::Decimal;`,
`use std::collections::{BTreeMap, HashMap};`):

```rust
    /// Value current holdings at supplied prices, aggregating per asset across
    /// wallets. Held assets with no supplied price are excluded from totals and
    /// listed in `missing_prices`.
    pub fn valuation(
        &self,
        method: CostBasisMethod,
        prices: &HashMap<String, Decimal>,
    ) -> Result<PortfolioReport, PortfolioError> {
        let holdings = self.holdings(method)?;

        // Aggregate quantity + basis per asset (BTreeMap for stable ordering).
        let mut agg: BTreeMap<String, (Decimal, Decimal)> = BTreeMap::new();
        for h in &holdings {
            let e = agg.entry(h.asset.clone()).or_insert((Decimal::ZERO, Decimal::ZERO));
            e.0 += h.quantity;
            e.1 += h.cost_basis;
        }

        let mut missing_prices = Vec::new();
        let mut priced: Vec<(String, Decimal, Decimal, Decimal)> = Vec::new(); // asset, qty, basis, price
        for (asset, (qty, basis)) in agg {
            match prices.get(&asset) {
                Some(&price) => priced.push((asset, qty, basis, price)),
                None => missing_prices.push(asset),
            }
        }

        let total_value: Decimal = priced.iter().map(|(_, q, _, p)| *q * *p).sum();
        let total_cost: Decimal = priced.iter().map(|(_, _, b, _)| *b).sum();

        let assets = priced
            .into_iter()
            .map(|(asset, quantity, cost_basis, price)| {
                let market_value = quantity * price;
                let allocation = if total_value.is_zero() {
                    Decimal::ZERO
                } else {
                    market_value / total_value
                };
                AssetValuation {
                    asset,
                    quantity,
                    cost_basis,
                    price,
                    market_value,
                    unrealized: market_value - cost_basis,
                    allocation,
                }
            })
            .collect();

        let total_unrealized = total_value - total_cost;
        let total_return = if total_cost.is_zero() {
            Decimal::ZERO
        } else {
            total_unrealized / total_cost
        };

        Ok(PortfolioReport {
            assets,
            total_cost,
            total_value,
            total_unrealized,
            total_return,
            missing_prices,
        })
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib valuation_aggregates`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/portfolio.rs
git commit -m "Add portfolio valuation with per-asset aggregation"
```

---

## Task 15: Tax-year reports — capital gains and income

**Files:**
- Modify: `src/portfolio.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `src/portfolio.rs`:
```rust
    #[test]
    fn capital_gains_report_filters_year_and_splits_terms() {
        let txs = vec![
            Transaction::Buy { timestamp: ts(2019, 1, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(2), unit_price: dec!(100), fee: dec!(0) },
            // Long-term sale in 2021 (held > 1y): gain 400.
            Transaction::Sell { timestamp: ts(2021, 3, 1), wallet: "w".into(), asset: "btc".into(),
                quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
            // Short-term: buy and sell within 2021: basis 100, proceeds 130 -> gain 30.
            Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "eth".into(),
                quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
            Transaction::Sell { timestamp: ts(2021, 6, 1), wallet: "w".into(), asset: "eth".into(),
                quantity: dec!(1), unit_price: dec!(130), fee: dec!(0) },
        ];
        let p = Portfolio::from_transactions(&txs).unwrap();
        let r = p.capital_gains_report(CostBasisMethod::Fifo, 2021).unwrap();
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.long_term_gain, dec!(400));
        assert_eq!(r.short_term_gain, dec!(30));
        assert_eq!(r.total_gain, dec!(430));
    }

    #[test]
    fn income_report_filters_year() {
        let txs = vec![
            Transaction::Income { timestamp: ts(2020, 5, 1), wallet: "w".into(), asset: "eth".into(),
                quantity: dec!(1), value: dec!(40), source: crate::transaction::IncomeSource::Staking },
            Transaction::Income { timestamp: ts(2021, 5, 1), wallet: "w".into(), asset: "eth".into(),
                quantity: dec!(1), value: dec!(60), source: crate::transaction::IncomeSource::Airdrop },
        ];
        let p = Portfolio::from_transactions(&txs).unwrap();
        let r = p.income_report(2021);
        assert_eq!(r.events.len(), 1);
        assert_eq!(r.total_income, dec!(60));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib report`
Expected: FAIL — `capital_gains_report` / `income_report` not defined.

- [ ] **Step 3: Write the implementation**

Add to `impl Portfolio` in `src/portfolio.rs` (add imports
`use crate::report::{CapitalGainsReport, IncomeReport, Term};` and
`use chrono::Datelike;`):

```rust
    /// Form-8949-shaped capital-gains report for one calendar tax year (UTC),
    /// under an automatic method. Use
    /// [`Portfolio::capital_gains_report_with_selection`] for Specific-ID.
    pub fn capital_gains_report(
        &self,
        method: CostBasisMethod,
        tax_year: i32,
    ) -> Result<CapitalGainsReport, PortfolioError> {
        if method == CostBasisMethod::SpecificId {
            return Err(PortfolioError::SelectionRequired);
        }
        let realized = self.realized_gains(method)?;
        Ok(Self::build_gains_report(realized, tax_year))
    }

    /// Capital-gains report using a Specific-ID selection.
    pub fn capital_gains_report_with_selection(
        &self,
        selection: &LotSelection,
        tax_year: i32,
    ) -> Result<CapitalGainsReport, PortfolioError> {
        let realized = self.realized_gains_with_selection(selection)?;
        Ok(Self::build_gains_report(realized, tax_year))
    }

    fn build_gains_report(realized: Vec<RealizedGain>, tax_year: i32) -> CapitalGainsReport {
        let rows: Vec<RealizedGain> =
            realized.into_iter().filter(|r| r.disposed_at.year() == tax_year).collect();
        let mut short_term_gain = Decimal::ZERO;
        let mut long_term_gain = Decimal::ZERO;
        let mut total_gain = Decimal::ZERO;
        for r in &rows {
            total_gain += r.gain;
            match r.term {
                Some(Term::Short) => short_term_gain += r.gain,
                Some(Term::Long) => long_term_gain += r.gain,
                None => {} // Average: untermed; counted only in total_gain.
            }
        }
        CapitalGainsReport { tax_year, rows, short_term_gain, long_term_gain, total_gain }
    }

    /// Ordinary-income report for one calendar tax year (UTC).
    pub fn income_report(&self, tax_year: i32) -> IncomeReport {
        let events: Vec<IncomeEvent> =
            self.income_events().into_iter().filter(|e| e.received_at.year() == tax_year).collect();
        let total_income = events.iter().map(|e| e.value).sum();
        IncomeReport { tax_year, events, total_income }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS (whole lib test suite).

- [ ] **Step 5: Commit**

```bash
git add src/portfolio.rs
git commit -m "Add tax-year capital-gains and income reports"
```

---

## Task 16: Headline integration test

**Files:**
- Create: `tests/headline.rs`

- [ ] **Step 1: Write the integration test**

Create `tests/headline.rs`:
```rust
//! Worked, multi-wallet, multi-method example — the crate's living documentation.
//! One fixed ledger run through FIFO, LIFO, HIFO, and Average proves they produce
//! *different* realized gains, and exercises Transfer, Trade, Income, and gifts.

use coinbasis::{CostBasisMethod, Portfolio, Transaction};
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
}

fn ledger() -> Vec<Transaction> {
    vec![
        Transaction::Buy { timestamp: ts(2020, 1, 1), wallet: "hot".into(), asset: "btc".into(),
            quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
        Transaction::Buy { timestamp: ts(2021, 1, 1), wallet: "hot".into(), asset: "btc".into(),
            quantity: dec!(1), unit_price: dec!(300), fee: dec!(0) },
        // Sell 1 BTC in 2022 at 500 — the method decides which lot.
        Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "hot".into(), asset: "btc".into(),
            quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
    ]
}

fn total_gain(method: CostBasisMethod) -> Decimal {
    let p = Portfolio::from_transactions(&ledger()).unwrap();
    p.realized_gains(method).unwrap().iter().map(|g| g.gain).sum()
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
fn comprehensive_flow_with_transfer_trade_income_and_gift() {
    let txs = vec![
        Transaction::Buy { timestamp: ts(2020, 1, 1), wallet: "hot".into(), asset: "btc".into(),
            quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
        Transaction::Transfer { timestamp: ts(2020, 6, 1), asset: "btc".into(), quantity: dec!(1),
            from_wallet: "hot".into(), to_wallet: "cold".into(), fee: dec!(0), fee_value: dec!(0) },
        Transaction::Income { timestamp: ts(2021, 1, 1), wallet: "hot".into(), asset: "eth".into(),
            quantity: dec!(2), value: dec!(200), source: coinbasis::IncomeSource::Staking },
        // Sell the transferred BTC from cold in 2022 — long-term from 2020.
        Transaction::Sell { timestamp: ts(2022, 1, 1), wallet: "cold".into(), asset: "btc".into(),
            quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
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
```

- [ ] **Step 2: Run the integration test to verify it fails (or passes)**

Run: `cargo test --test headline`
Expected: PASS if Tasks 8–15 are correct. If a gain assertion is off, debug the
relevant engine path rather than weakening the assertion.

- [ ] **Step 3: Fix anything surfaced**

Address any failure in the engine/portfolio code (not the test).

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: PASS (lib + headline).

- [ ] **Step 5: Commit**

```bash
git add tests/headline.rs
git commit -m "Add headline multi-method, multi-wallet integration test"
```

---

## Task 17: Property tests (conservation invariants)

**Files:**
- Create: `tests/properties.rs`

- [ ] **Step 1: Write the property test**

Create `tests/properties.rs`:
```rust
//! Property tests: invariants that must hold for any random buy/sell ledger.

use coinbasis::{CostBasisMethod, Portfolio, Transaction};
use chrono::{Duration, TimeZone, Utc};
use proptest::prelude::*;
use rust_decimal::Decimal;

fn base() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap()
}

proptest! {
    #[test]
    fn buys_then_one_sell_conserve_basis(
        // 1..=5 buys of (qty 1..=10, unit_price 1..=100), then sell `sell_qty`.
        buys in prop::collection::vec((1u32..=10, 1u32..=100), 1..=5),
        sell_frac in 0u32..=100,
    ) {
        let total_qty: u32 = buys.iter().map(|(q, _)| *q).sum();
        let sell_qty = (total_qty * sell_frac / 100).max(0);

        let mut txs: Vec<Transaction> = Vec::new();
        let mut day = 0i64;
        let mut total_basis = Decimal::ZERO;
        for (q, p) in &buys {
            txs.push(Transaction::Buy {
                timestamp: base() + Duration::days(day),
                wallet: "w".into(), asset: "btc".into(),
                quantity: Decimal::from(*q), unit_price: Decimal::from(*p), fee: Decimal::ZERO,
            });
            total_basis += Decimal::from(*q) * Decimal::from(*p);
            day += 1;
        }
        if sell_qty > 0 {
            txs.push(Transaction::Sell {
                timestamp: base() + Duration::days(day),
                wallet: "w".into(), asset: "btc".into(),
                quantity: Decimal::from(sell_qty), unit_price: Decimal::from(50u32), fee: Decimal::ZERO,
            });
        }

        let p = Portfolio::from_transactions(&txs).unwrap();
        let realized = p.realized_gains(CostBasisMethod::Fifo).unwrap();
        let holdings = p.holdings(CostBasisMethod::Fifo).unwrap();

        // Conservation: consumed basis + remaining basis == total acquired basis.
        let consumed_basis: Decimal = realized.iter().map(|r| r.cost_basis).sum();
        let remaining_basis: Decimal = holdings.iter().map(|h| h.cost_basis).sum();
        prop_assert_eq!(consumed_basis + remaining_basis, total_basis);

        // Quantity conservation.
        let remaining_qty: Decimal = holdings.iter().map(|h| h.quantity).sum();
        prop_assert_eq!(remaining_qty, Decimal::from(total_qty - sell_qty));
    }
}
```

- [ ] **Step 2: Run the property test**

Run: `cargo test --test properties`
Expected: PASS. If it fails, proptest prints a minimal counterexample — debug the
engine (likely a rounding or retain bug), don't relax the invariant.

- [ ] **Step 3: Commit**

```bash
git add tests/properties.rs
git commit -m "Add proptest conservation invariants"
```

---

## Task 18: serde feature smoke test

**Files:**
- Create: `tests/serde_roundtrip.rs`

- [ ] **Step 1: Write the feature-gated test**

Create `tests/serde_roundtrip.rs`:
```rust
//! Verifies public types (de)serialize when the `serde` feature is enabled.
#![cfg(feature = "serde")]

use coinbasis::Transaction;
use chrono::{TimeZone, Utc};
use rust_decimal_macros::dec;

#[test]
fn transaction_json_roundtrip() {
    let tx = Transaction::Buy {
        timestamp: Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap(),
        wallet: "w".into(), asset: "btc".into(),
        quantity: dec!(1), unit_price: dec!(100), fee: dec!(0),
    };
    let json = serde_json::to_string(&tx).unwrap();
    let back: Transaction = serde_json::from_str(&json).unwrap();
    assert_eq!(tx, back);
}
```

- [ ] **Step 2: Add `serde_json` as a dev-dependency**

In `Cargo.toml` under `[dev-dependencies]`, add:
```toml
serde_json = "1"
```

- [ ] **Step 3: Run with the feature enabled**

Run: `cargo test --features serde --test serde_roundtrip`
Expected: PASS. Also confirm the default build still works: `cargo test`.

- [ ] **Step 4: Commit**

```bash
git add tests/serde_roundtrip.rs Cargo.toml
git commit -m "Add serde feature round-trip test"
```

---

## Task 19: README and crate-level doctest

**Files:**
- Create: `README.md`
- Modify: `src/lib.rs` (add a runnable doctest to the crate docs)

- [ ] **Step 1: Write `README.md`**

Create `README.md`:
```markdown
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

## License

MIT OR Apache-2.0.
```

- [ ] **Step 2: Add a crate-level doctest to `src/lib.rs`**

Add to the top doc comment of `src/lib.rs` (after the existing summary, before
the lint attributes), a runnable example fenced block:
```rust
//! # Example
//! ```
//! use coinbasis::{CostBasisMethod, Portfolio, Transaction};
//! use chrono::{TimeZone, Utc};
//! use rust_decimal_macros::dec;
//!
//! let txs = vec![
//!     Transaction::Buy { timestamp: Utc.with_ymd_and_hms(2020,1,1,0,0,0).unwrap(),
//!         wallet: "hot".into(), asset: "btc".into(),
//!         quantity: dec!(1), unit_price: dec!(100), fee: dec!(0) },
//!     Transaction::Sell { timestamp: Utc.with_ymd_and_hms(2022,1,1,0,0,0).unwrap(),
//!         wallet: "hot".into(), asset: "btc".into(),
//!         quantity: dec!(1), unit_price: dec!(500), fee: dec!(0) },
//! ];
//! let portfolio = Portfolio::from_transactions(&txs).unwrap();
//! let gains = portfolio.realized_gains(CostBasisMethod::Fifo).unwrap();
//! assert_eq!(gains[0].gain, dec!(400));
//! ```
```

The doctest needs `chrono` and `rust_decimal_macros` available to doctests. They
already are (`chrono` is a normal dep; add `rust_decimal_macros` to `[dependencies]`
or make doctests use `rust_decimal::Decimal` directly). To keep `rust_decimal_macros`
a dev-only concern, rewrite the doctest literals using `Decimal::new`:
replace `dec!(1)` → `Decimal::new(1, 0)`, `dec!(100)` → `Decimal::new(100, 0)`,
`dec!(500)` → `Decimal::new(500, 0)`, `dec!(400)` → `Decimal::new(400, 0)`, and
`use rust_decimal::Decimal;` instead of the macro import.

- [ ] **Step 3: Run doctests**

Run: `cargo test --doc`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add README.md src/lib.rs
git commit -m "Add README and runnable crate-level doctest"
```

---

## Task 20: Publish readiness

**Files:**
- Modify: `Cargo.toml` (only if `cargo publish --dry-run` flags metadata)

- [ ] **Step 1: Lint and format**

Run: `cargo fmt --all` then `cargo clippy --all-targets --all-features -- -D warnings`
Expected: no warnings. Fix any clippy findings.

- [ ] **Step 2: Full test matrix**

Run: `cargo test` and `cargo test --all-features`
Expected: PASS.

- [ ] **Step 3: Verify docs build cleanly**

Run: `cargo doc --no-deps --all-features`
Expected: builds with no `missing_docs` errors.

- [ ] **Step 4: Dry-run publish**

Run: `cargo publish --dry-run`
Expected: packages successfully. Resolve any metadata complaints in `Cargo.toml`.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "Finalize publish readiness: fmt, clippy, docs, dry-run"
```

> Actual `cargo publish` (and `cargo login`) is a manual step Jacob runs when
> ready — not part of automated execution.

---

## Self-review against the spec

- **Event model (Buy/Sell/Trade/Income/Spend/Transfer/GiftSent/GiftReceived):** Tasks 4, 8, 10, 11, 12. ✓
- **Per-wallet pools + same-wallet disposal:** Tasks 8 (`InsufficientLots` per wallet), 11. ✓
- **FIFO/LIFO/HIFO/Average/SpecificId:** Tasks 6, 8, 9. ✓
- **Holding-period Short/Long (365-day boundary):** Tasks 7, 8. ✓
- **Transfer preserves basis + acquisition date; fee as disposal:** Task 11. ✓
- **Gift dual-basis (gain/loss/dead-zone) + tacked term; GiftSent non-taxable:** Tasks 8 (`gain_for`), 12. ✓
- **Realized gains / income events / holdings / valuation:** Tasks 13, 14. ✓
- **Capital-gains + income reports, per tax year, ST/LT subtotals:** Task 15. ✓
- **`missing_prices` handling + zero-cost guard:** Task 14. ✓
- **Pure stats (volatility/Sharpe/max_drawdown/cumulative/returns):** Task 3. ✓
- **`rust_decimal` everywhere; `thiserror`; optional `serde`:** Tasks 1, 2, 4–7, 18. ✓
- **`#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, doctests, README, license, metadata:** Tasks 1, 19, 20. ✓
- **Headline multi-method test + property tests:** Tasks 16, 17. ✓

**Known scoping decisions carried from the spec (intentional, documented):**
- Specific-ID affects realized gains + capital-gains report; `holdings`/`valuation` use an automatic method (calling them with `SpecificId` returns `SelectionRequired`).
- `Average` drops per-lot dates → `term`/`acquired_at` are `None`; ST/LT subtotals may sum to less than `total_gain`; gifted lots pool at carryover basis under `Average`.
- `GiftSent` and transfer moves use FIFO ordering internally (non-taxable; ordering only affects which lots remain, not gains).
- Holding-period boundary uses a 365-day approximation (documented on `Term`).

**Type-consistency check:** `run`/`Strategy`/`EngineOutput`/`Consumed` names are
used identically across Tasks 8–12; `Portfolio` query method names match between
Tasks 13–15 and the headline/property tests in Tasks 16–17. ✓
