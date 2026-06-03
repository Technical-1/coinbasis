# coinbasis — Library Design Spec

**Date:** 2026-06-03
**Author:** Jacob Kanfer (`Technical-1`)
**Status:** Approved (design) — pending spec review
**Repo:** `coinbasis` (private GitHub repo; published to crates.io)

## 1. Summary

`coinbasis` is a pure, data-source-agnostic Rust library for cryptocurrency
portfolio accounting. You hand it a list of transactions (buys/sells) and a set
of current prices; it returns tax-lot cost-basis accounting (realized and
unrealized gains) and portfolio analytics. It performs **no network access, no
file I/O, and no terminal output** — those concerns belong to consumers (e.g.
the separate `Crypto-Price-Tracker-V2` TUI).

This is the first of two separate repositories in the "V2" rewrite of the
original Python `Crypto-Price-Tracker`. It is the unique, publishable artifact;
the TUI is a separate repo that depends on this crate.

### Goals

- Be the reusable crypto **tax-lot cost-basis** library that does not yet exist
  on crates.io (existing crates are either API clients or finished apps).
- Demonstrate correct financial software engineering: decimal precision, typed
  errors, exhaustive tests, clean public API, full documentation.
- Stay small, pure, and fully testable — zero async, zero I/O, zero flaky tests.

### Non-goals (v0.1 — YAGNI)

- No network or price fetching (the caller supplies prices).
- No persistence / database.
- No CLI or TUI (separate repo).
- No jurisdiction-specific tax rules (wash sales, short/long-term holding
  period classification, lot-level tax rates). Noted as possible future work;
  the data model leaves room (acquisition timestamps are retained) but v0.1 does
  not classify or apply tax rules.
- No multi-currency quote handling — all amounts are in a single quote currency
  (USD by convention); the library treats the quote as an opaque unit and does
  not convert.

## 2. Target users

- Rust developers building crypto portfolio tools who need correct cost-basis
  math without reimplementing it (the TUI in repo 2 is the first consumer).
- Anyone needing FIFO/LIFO/HIFO/Average realized-gain calculations over a
  transaction ledger.

## 3. Architecture

The crate is a single library with a small number of focused modules:

```
coinbasis/
├── src/
│   ├── lib.rs          # crate docs, re-exports, feature gating
│   ├── transaction.rs  # Transaction, TxKind input types
│   ├── lot.rs          # Lot (open acquisition) internal model
│   ├── method.rs       # CostBasisMethod enum + lot-selection logic
│   ├── portfolio.rs    # Portfolio engine: build lots, dispose, value
│   ├── report.rs       # RealizedGain, Holding, Valuation, PortfolioReport
│   ├── stats.rs        # pure analytics: volatility, sharpe, max_drawdown
│   └── error.rs        # PortfolioError (thiserror)
└── tests/              # integration + worked-example tests
```

### Data flow

1. Caller constructs a `Vec<Transaction>` (each with timestamp, asset, kind,
   quantity, price, fee).
2. `Portfolio::from_transactions(&txs)` validates and stores transactions
   (sorted chronologically; ties broken by input order).
3. The caller chooses a `CostBasisMethod` and calls `realized_gains(method)`:
   the engine replays the ledger, consuming open lots on each `Sell` according
   to the method, producing a `Vec<RealizedGain>`.
4. `holdings(method)` returns the remaining open lots per asset (current
   position and aggregate cost basis).
5. `valuation(method, &prices)` values open holdings at supplied current prices,
   returning unrealized P/L, allocation %, total cost, current value, and return.
6. The `stats` module operates independently on caller-supplied numeric series
   (e.g. a portfolio value history the caller maintains).

## 4. Public API

### Input types (`transaction.rs`)

```rust
pub struct Transaction {
    pub timestamp: DateTime<Utc>,
    pub asset: String,        // asset identifier, e.g. CoinGecko id "bitcoin"
    pub kind: TxKind,
    pub quantity: Decimal,    // units transacted (> 0)
    pub price: Decimal,       // price per unit in the quote currency (>= 0)
    pub fee: Decimal,         // fee in the quote currency (>= 0)
}

pub enum TxKind { Buy, Sell }
```

### Method (`method.rs`)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostBasisMethod {
    Fifo,    // oldest lots consumed first
    Lifo,    // newest lots consumed first
    Hifo,    // highest-unit-cost lots consumed first (minimizes realized gain)
    Average, // single pooled average-cost lot per asset
}
```

### Engine (`portfolio.rs`)

```rust
impl Portfolio {
    pub fn from_transactions(txs: &[Transaction]) -> Result<Self, PortfolioError>;
    pub fn realized_gains(&self, method: CostBasisMethod) -> Vec<RealizedGain>;
    pub fn holdings(&self, method: CostBasisMethod) -> Vec<Holding>;
    pub fn valuation(
        &self,
        method: CostBasisMethod,
        prices: &HashMap<String, Decimal>,
    ) -> PortfolioReport;
}
```

### Output types (`report.rs`)

```rust
pub struct RealizedGain {
    pub asset: String,
    pub disposed_at: DateTime<Utc>,
    pub acquired_at: Option<DateTime<Utc>>, // matched lot's acquisition date;
                                            // None for Average (pooled, no single date)
    pub quantity: Decimal,
    pub proceeds: Decimal,          // sale value net of sell fee
    pub cost_basis: Decimal,        // matched lot cost (incl. allocated buy fee)
    pub gain: Decimal,              // proceeds - cost_basis
}

pub struct Holding {
    pub asset: String,
    pub quantity: Decimal,          // total open units
    pub cost_basis: Decimal,        // total remaining cost basis
    pub average_cost: Decimal,      // cost_basis / quantity
}

pub struct AssetValuation {
    pub asset: String,
    pub quantity: Decimal,
    pub cost_basis: Decimal,
    pub price: Decimal,
    pub market_value: Decimal,      // quantity * price
    pub unrealized: Decimal,        // market_value - cost_basis
    pub allocation: Decimal,        // market_value / portfolio market_value (0..1)
}

pub struct PortfolioReport {
    pub assets: Vec<AssetValuation>,
    pub total_cost: Decimal,
    pub total_value: Decimal,
    pub total_unrealized: Decimal,
    pub total_return: Decimal,      // total_unrealized / total_cost (0 when cost == 0)
    pub missing_prices: Vec<String>,// held assets with no price supplied
}
```

A held asset with no supplied price is excluded from totals and listed in
`missing_prices`, rather than being treated as zero value (which would
misreport returns). This mirrors the V1 "skip with a notice" philosophy.

### Stats (`stats.rs`)

Pure functions over a caller-supplied series. Inputs use `f64` for statistical
work (variance/standard deviation are inherently approximate); cost-basis money
math stays in `Decimal`.

```rust
pub fn returns_from_values(values: &[f64]) -> Vec<f64>; // period-over-period
pub fn volatility(returns: &[f64]) -> Option<f64>;       // sample std dev
pub fn sharpe_ratio(returns: &[f64], risk_free: f64) -> Option<f64>;
pub fn max_drawdown(values: &[f64]) -> Option<f64>;      // worst peak-to-trough (0..1)
pub fn cumulative_return(values: &[f64]) -> Option<f64>;
```

Functions return `None` for series too short to be meaningful (e.g. fewer than
two points) rather than panicking or returning misleading zeros.

### Errors (`error.rs`)

```rust
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum PortfolioError {
    #[error("sold {attempted} {asset} but only {available} held")]
    OversoldAsset { asset: String, attempted: Decimal, available: Decimal },
    #[error("transaction for {asset} has non-positive quantity {quantity}")]
    NonPositiveQuantity { asset: String, quantity: Decimal },
    #[error("transaction for {asset} has negative price {price}")]
    NegativePrice { asset: String, price: Decimal },
    #[error("transaction for {asset} has negative fee {fee}")]
    NegativeFee { asset: String, fee: Decimal },
}
```

Validation runs in `from_transactions`. Overselling is detected when replaying
disposals; because the error can depend on method ordering only by quantity
(total held is method-independent), it is reported deterministically against the
running per-asset balance.

## 5. Key design decisions

### Decimal, never float, for money
All quantities and monetary values use `rust_decimal::Decimal`. Floating point
silently misreports financial math (e.g. `0.1 + 0.2`). This is the single most
important correctness decision and a deliberate quality signal.

### Fees fold into basis and proceeds
Buy fees are added to a lot's cost basis; sell fees are subtracted from
proceeds. This is the standard treatment and keeps realized gain = proceeds −
cost_basis honest.

### Average-cost method modeled as a single pooled lot
For `Average`, all open units of an asset collapse into one pooled lot whose
unit cost is the running average; a sell reduces the pool proportionally. This
keeps the engine uniform (every method is "select lots, consume them").

### Method chosen at query time, not construction
`Portfolio` stores the immutable ledger; the method is a parameter to
`realized_gains`/`holdings`/`valuation`. This lets a consumer compare methods on
the same ledger cheaply (and powers the headline test below).

### Purity boundary: stats take a caller-supplied series
The library never builds a time series itself (that needs historical prices it
won't fetch). `stats` operates on whatever numeric series the caller provides,
keeping the crate free of async, I/O, and flaky tests.

## 6. Dependencies

| Crate | Purpose | Notes |
|-------|---------|-------|
| `rust_decimal` | Exact decimal money/quantity math | core; `Decimal` in the public API |
| `chrono` | Transaction timestamps | `DateTime<Utc>` in the public API |
| `thiserror` | Typed error enum | |
| `serde` | (De)serialization of public types | **optional**, behind `serde` feature |

`proptest` is a dev-dependency only.

## 7. Crate hygiene / quality signals

- `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` in `lib.rs`.
- Full rustdoc on every public item, with runnable doctests.
- Declared MSRV in `Cargo.toml` and `README`.
- Cargo metadata complete (description, license, repository, keywords,
  categories) for a clean crates.io listing.
- License: MIT OR Apache-2.0 (Rust ecosystem convention).
- `README.md` with a quickstart example and a worked FIFO-vs-LIFO comparison.

## 8. Testing strategy

- **Headline worked-example test:** one fixed ledger (a few buys at different
  prices, then a partial sell) run through FIFO, LIFO, HIFO, and Average,
  asserting the *different* realized-gain numbers each method produces. Serves
  as both correctness proof and living documentation.
- **Per-method unit tests:** partial lot consumption, exact lot consumption,
  multiple lots spanning one sell, fees applied to basis and proceeds.
- **Valuation tests:** unrealized P/L, allocation summing to 1.0 (within a
  rounding tolerance), `missing_prices` behavior, zero-cost guard.
- **Error tests:** oversell, non-positive quantity, negative price/fee.
- **Stats tests:** known-answer volatility/Sharpe/drawdown vectors; `None` on
  too-short series.
- **Property tests (`proptest`):** total realized + remaining cost basis equals
  total acquired cost (conservation); holdings quantity equals net buys − sells.
- **Doctests** on all public methods.

## 9. Out-of-scope future ideas (not in v0.1)

- Short-term vs long-term gain classification by holding period.
- Wash-sale and jurisdiction-specific rules.
- Transfers between wallets / non-taxable events.
- CSV/exchange import helpers (likely a separate companion crate).
- Time-weighted and money-weighted return helpers in `stats`.

## 10. Delivery / sequencing

1. Build and test `coinbasis` to completion in its own private repo.
2. Publish `coinbasis` v0.1 to crates.io.
3. The separate `Crypto-Price-Tracker-V2` TUI repo (own spec/plan/cycle) depends
   on it — via a git dependency during development, switching to the crates.io
   version once published.
