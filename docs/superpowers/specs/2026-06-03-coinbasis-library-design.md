# coinbasis — Library Design Spec

**Date:** 2026-06-03
**Author:** Jacob Kanfer (`Technical-1`)
**Status:** Approved (design) — pending spec review
**Repo:** `coinbasis` (private GitHub repo; published to crates.io)

## 1. Summary

`coinbasis` is a pure, data-source-agnostic Rust library for **comprehensive
cryptocurrency tax-lot accounting** and portfolio analytics. You hand it a
ledger of transactions and a set of current prices; it returns realized capital
gains (with holding-period classification), ordinary income, unrealized P/L, and
portfolio analytics. It performs **no network access, no file I/O, and no
terminal output** — those concerns belong to consumers (e.g. the separate
`Crypto-Price-Tracker-V2` TUI).

This is the first of two separate repositories in the "V2" rewrite of the
original Python `Crypto-Price-Tracker`. It is the unique, publishable artifact;
the TUI is a separate repo that depends on this crate.

> **Not tax advice.** `coinbasis` is an accounting/calculation library. It does
> not file taxes, give legal advice, or guarantee conformance with any
> jurisdiction's rules. The crate documentation states this prominently and
> ships under a permissive license with the standard no-warranty clause.

### Goals

- Be the reusable, *comprehensive* crypto **tax-lot accounting** library that
  does not yet exist on crates.io (existing crates are either API clients or
  finished apps).
- Correctly model the events and rules that DIY trackers get wrong:
  crypto-to-crypto trades, wallet-to-wallet transfers, income events, and
  **per-wallet** cost-basis segregation (US 2025 rule).
- Demonstrate correct financial software engineering: decimal precision, typed
  errors, exhaustive tests, clean public API, full documentation.
- Stay pure and fully testable — zero async, zero I/O, zero flaky tests.

### Tax model assumptions (documented in the crate)

The default model follows current **US federal** treatment, which is also a
reasonable baseline elsewhere:

- Short-term = held **≤ 365 days**; long-term = held **> 365 days**.
- Cost basis **includes acquisition fees**; disposal fees **reduce proceeds**.
- **Per-wallet/account** cost-basis segregation (each wallet is its own lot
  pool); disposals consume lots from the *same* wallet only.
- Tax year = **calendar year in UTC** (Jan 1 00:00 – Dec 31 23:59:59 UTC).

These assumptions are explicit in the API docs so consumers know the model.

### Non-goals (v0.1 — YAGNI)

- No network or price fetching (the caller supplies prices/FMVs).
- No persistence / database / CSV-import (a likely separate companion crate).
- No CLI or TUI (separate repo).
- No wash-sale rules (not currently applied to crypto under US law) and no
  jurisdiction-specific pooling variants beyond the provided methods (e.g. UK
  Section 104, Canada ACB) — the `Average` method approximates pooling but is
  not claimed to be jurisdiction-exact.
- No gift/inheritance carryover-basis special rules (gifts received are out of
  v0.1; see future ideas).
- No multi-fiat handling — all values are in one quote currency (USD by
  convention), treated as an opaque unit; the library does not convert fiat.
- No configurable tax-year boundaries / fiscal years (calendar UTC only).

## 2. Target users

- Rust developers building crypto portfolio/tax tools who need correct,
  comprehensive cost-basis math without reimplementing it (the V2 TUI is the
  first consumer).
- Anyone needing per-wallet FIFO/LIFO/HIFO/Average/Specific-ID realized-gain and
  income calculations over a transaction ledger.

## 3. Architecture

A single library crate with focused modules:

```
coinbasis/
├── src/
│   ├── lib.rs          # crate docs, disclaimer, re-exports, feature gating
│   ├── transaction.rs  # Transaction event enum + IncomeSource
│   ├── lot.rs          # Lot (open acquisition, per-wallet) internal model
│   ├── method.rs       # CostBasisMethod + lot-selection logic
│   ├── engine.rs       # ledger replay: acquire / dispose / transfer
│   ├── portfolio.rs    # Portfolio facade: build + query
│   ├── report.rs       # RealizedGain, IncomeEvent, Holding, valuation, reports
│   ├── stats.rs        # pure analytics: volatility, sharpe, max_drawdown
│   └── error.rs        # PortfolioError (thiserror)
└── tests/              # integration + worked-example tests
```

### Data flow

1. Caller builds a `Vec<Transaction>` (an event enum — see §4).
2. `Portfolio::from_transactions(&txs)` validates events and stores them, sorted
   chronologically (ties broken by input order).
3. The caller picks a `CostBasisMethod`. Queries replay the ledger:
   - **Acquisitions** (`Buy`, `Income`, the *received* side of `Trade`) open a
     lot in the event's wallet, with `cost_basis` = value + acquisition fee and
     `acquired_at` = event time.
   - **Disposals** (`Sell`, `Spend`, the *given* side of `Trade`) consume open
     lots **in the same wallet** per the method, producing `RealizedGain`s with
     proceeds (net of disposal fee), matched cost basis, gain, and holding-period
     `Term`.
   - **Transfers** move lots from `from_wallet` to `to_wallet`, **preserving
     `cost_basis` and `acquired_at`** (holding-period clock keeps running);
     non-taxable.
   - **Income** events are also recorded as `IncomeEvent`s (ordinary income at
     FMV) in addition to opening a basis lot.
4. Remaining open lots = current holdings (per wallet and aggregated).
5. `valuation(method, &prices)` values open holdings at supplied prices.
6. Report queries can be filtered by **tax year**.
7. `stats` operates independently on caller-supplied numeric series.

## 4. Public API

### Input event model (`transaction.rs`)

`Transaction` is an enum; each variant carries exactly the fields its event
needs. All monetary `value`/`unit_price`/`fee` fields are in the quote currency.

```rust
pub enum Transaction {
    /// Fiat -> crypto. Non-taxable; sets cost basis (incl. fee).
    Buy { timestamp: DateTime<Utc>, wallet: String, asset: String,
          quantity: Decimal, unit_price: Decimal, fee: Decimal },

    /// Crypto -> fiat. Taxable disposal; proceeds net of fee.
    Sell { timestamp: DateTime<Utc>, wallet: String, asset: String,
           quantity: Decimal, unit_price: Decimal, fee: Decimal },

    /// Crypto -> crypto. Disposal of `from_*` at `value` (FMV) AND
    /// acquisition of `to_*` with basis = `value` (+ fee).
    Trade { timestamp: DateTime<Utc>, wallet: String,
            from_asset: String, from_quantity: Decimal,
            to_asset: String, to_quantity: Decimal,
            value: Decimal, fee: Decimal },

    /// Staking/mining/airdrop/interest. Ordinary income at `value` (FMV);
    /// also opens a lot with basis = `value`.
    Income { timestamp: DateTime<Utc>, wallet: String, asset: String,
             quantity: Decimal, value: Decimal, source: IncomeSource },

    /// Paying for goods/services with crypto. Taxable disposal at `value` (FMV).
    Spend { timestamp: DateTime<Utc>, wallet: String, asset: String,
            quantity: Decimal, value: Decimal, fee: Decimal },

    /// Move between your own wallets. Non-taxable; preserves basis + date.
    Transfer { timestamp: DateTime<Utc>, asset: String, quantity: Decimal,
               from_wallet: String, to_wallet: String },
}

pub enum IncomeSource { Staking, Mining, Airdrop, Interest, Other }
```

> **Transfer fees:** v0.1 transfers move the full `quantity` with no fee field.
> A network fee paid in the moved asset is itself a disposal; modeling it
> correctly needs an FMV, so it is deferred — callers needing it record a
> separate `Spend` for the fee. This limitation is documented.

### Method (`method.rs`)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostBasisMethod {
    Fifo,       // oldest lots first
    Lifo,       // newest lots first
    Hifo,       // highest unit cost first (minimizes gain)
    Average,    // pooled average cost per (asset, wallet)
    SpecificId, // caller names the lots per disposal (see below)
}
```

**Specific Identification:** under `SpecificId`, disposals are matched against
caller-provided lot selections. A disposal carries an optional
`Vec<LotSelection { lot_id, quantity }>`; lots expose a stable `LotId`. If a
disposal under `SpecificId` lacks a valid, fully-covering selection, the engine
returns an error. (The exact ergonomic surface — selection-carrying disposal
variants vs. a separate selection map keyed by event index — is finalized in the
implementation plan; the design intent is fixed here.) The other four methods
ignore selections.

### Engine facade (`portfolio.rs`)

```rust
impl Portfolio {
    pub fn from_transactions(txs: &[Transaction]) -> Result<Self, PortfolioError>;

    /// All realized capital-gain rows (one per consumed lot), with holding term.
    pub fn realized_gains(&self, method: CostBasisMethod)
        -> Result<Vec<RealizedGain>, PortfolioError>;

    /// Ordinary-income events (method-independent).
    pub fn income_events(&self) -> Vec<IncomeEvent>;

    /// Open lots remaining = current holdings, per wallet and asset.
    pub fn holdings(&self, method: CostBasisMethod)
        -> Result<Vec<Holding>, PortfolioError>;

    /// Value open holdings at supplied current prices.
    pub fn valuation(&self, method: CostBasisMethod,
                     prices: &HashMap<String, Decimal>)
        -> Result<PortfolioReport, PortfolioError>;

    /// Form-8949-shaped capital-gains report for one calendar tax year (UTC),
    /// with short/long-term subtotals.
    pub fn capital_gains_report(&self, method: CostBasisMethod, tax_year: i32)
        -> Result<CapitalGainsReport, PortfolioError>;

    /// Ordinary-income report for one calendar tax year (UTC).
    pub fn income_report(&self, tax_year: i32) -> IncomeReport;
}
```

### Output types (`report.rs`)

```rust
pub enum Term { Short, Long }   // Short = held <= 365 days; Long = > 365 days

pub struct RealizedGain {
    pub asset: String,
    pub wallet: String,
    pub disposed_at: DateTime<Utc>,
    pub acquired_at: Option<DateTime<Utc>>, // matched lot date; None for Average
    pub quantity: Decimal,
    pub proceeds: Decimal,          // disposal value net of fee
    pub cost_basis: Decimal,        // matched lot basis (incl. allocated acq. fee)
    pub gain: Decimal,              // proceeds - cost_basis
    pub term: Option<Term>,         // None for Average (no single acquisition date)
}

pub struct IncomeEvent {
    pub asset: String,
    pub wallet: String,
    pub received_at: DateTime<Utc>,
    pub quantity: Decimal,
    pub value: Decimal,             // FMV at receipt = ordinary income
    pub source: IncomeSource,
}

pub struct Holding {
    pub asset: String,
    pub wallet: String,             // per-wallet position
    pub quantity: Decimal,
    pub cost_basis: Decimal,
    pub average_cost: Decimal,      // cost_basis / quantity
}

pub struct AssetValuation {
    pub asset: String,              // aggregated across wallets
    pub quantity: Decimal,
    pub cost_basis: Decimal,
    pub price: Decimal,
    pub market_value: Decimal,
    pub unrealized: Decimal,        // market_value - cost_basis
    pub allocation: Decimal,        // market_value / portfolio value (0..1)
}

pub struct PortfolioReport {
    pub assets: Vec<AssetValuation>,
    pub total_cost: Decimal,
    pub total_value: Decimal,
    pub total_unrealized: Decimal,
    pub total_return: Decimal,      // total_unrealized / total_cost (0 if cost==0)
    pub missing_prices: Vec<String>,
}

pub struct CapitalGainsReport {
    pub tax_year: i32,
    pub rows: Vec<RealizedGain>,    // disposals settled in this tax year
    pub short_term_gain: Decimal,
    pub long_term_gain: Decimal,
    pub total_gain: Decimal,
}

pub struct IncomeReport {
    pub tax_year: i32,
    pub events: Vec<IncomeEvent>,
    pub total_income: Decimal,
}
```

A held asset with no supplied price is excluded from totals and listed in
`missing_prices` (rather than counted as zero, which would misreport returns).

> **Average and ST/LT subtotals:** because `Average` yields `term: None`, its
> realized rows contribute to `total_gain` only; `short_term_gain` and
> `long_term_gain` cover the termed rows, so under `Average` they may sum to less
> than `total_gain`. This is documented on `CapitalGainsReport`. (US crypto does
> not permit Average anyway; it is provided for non-US/illustrative use.)

### Stats (`stats.rs`)

Pure functions over a caller-supplied series. Statistical work uses `f64`
(variance/std-dev are inherently approximate); cost-basis money math stays in
`Decimal`.

```rust
pub fn returns_from_values(values: &[f64]) -> Vec<f64>;
pub fn volatility(returns: &[f64]) -> Option<f64>;          // sample std dev
pub fn sharpe_ratio(returns: &[f64], risk_free: f64) -> Option<f64>;
pub fn max_drawdown(values: &[f64]) -> Option<f64>;         // worst peak-to-trough (0..1)
pub fn cumulative_return(values: &[f64]) -> Option<f64>;
```

`None` for series too short to be meaningful (< 2 points).

### Errors (`error.rs`)

```rust
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum PortfolioError {
    #[error("disposed {attempted} {asset} from wallet '{wallet}' but only {available} held there")]
    InsufficientLots { asset: String, wallet: String, attempted: Decimal, available: Decimal },
    #[error("transfer of {quantity} {asset} from '{wallet}' exceeds the {available} held there")]
    InsufficientTransfer { asset: String, wallet: String, quantity: Decimal, available: Decimal },
    #[error("event for {asset} has non-positive quantity {quantity}")]
    NonPositiveQuantity { asset: String, quantity: Decimal },
    #[error("event for {asset} has a negative value/price")]
    NegativeValue { asset: String },
    #[error("event for {asset} has a negative fee {fee}")]
    NegativeFee { asset: String, fee: Decimal },
    #[error("SpecificId disposal of {asset} lacks a valid lot selection")]
    MissingLotSelection { asset: String },
    #[error("SpecificId selection references unknown or exhausted lot {lot_id}")]
    InvalidLotSelection { lot_id: String },
}
```

Field validation runs in `from_transactions`; lot-availability and selection
errors surface during ledger replay. The per-wallet rule means a disposal can be
`InsufficientLots` even when the asset is held in *another* wallet.

## 5. Key design decisions

### Decimal, never float, for money
All quantities/monetary values use `rust_decimal::Decimal`. Floats silently
misreport financial math. This is the single most important correctness decision.

### Per-wallet lot pools (US 2025 rule)
Lots are keyed by `(asset, wallet)`. Disposals consume same-wallet lots only;
`Transfer` migrates lots between wallets while preserving basis and acquisition
date. This is the headline differentiator and the model most DIY trackers lack.

### Transfers are non-taxable and preserve the holding-period clock
Moving coins between your own wallets is not a disposal. Critically, the moved
lot keeps its original `acquired_at`, so a later sale's short/long-term term is
computed from the true acquisition date — not the transfer date.

### Crypto-to-crypto trades decompose into disposal + acquisition
A `Trade` realizes gain on the asset given up (proceeds = `value`) and opens a
new lot for the asset received (basis = `value` + fee). This is where most naive
trackers are wrong.

### Income sets basis and is reported separately
`Income` events count as ordinary income at FMV *and* establish the received
lot's cost basis (so a later sale isn't double-taxed on that basis). Income and
capital gains are reported through separate methods.

### Method chosen at query time
`Portfolio` stores the immutable ledger; the method is a query parameter, so a
consumer can compare methods on the same ledger cheaply (powers the headline
test).

### Average and SpecificId edge semantics
Under `Average`, per-(asset,wallet) units pool into one running-average lot;
individual acquisition dates are lost, so `acquired_at`/`term` are `None`.
`SpecificId` requires caller-supplied lot selections (errors otherwise).

### Purity boundary: stats take a caller-supplied series
The library never builds a time series itself (that needs historical prices it
won't fetch). `stats` works on whatever series the caller provides — keeping the
crate free of async, I/O, and flaky tests.

## 6. Dependencies

| Crate | Purpose | Notes |
|-------|---------|-------|
| `rust_decimal` | Exact decimal money/quantity math | core; in the public API |
| `chrono` | Timestamps + holding-period / tax-year math | `DateTime<Utc>` in the public API |
| `thiserror` | Typed error enum | |
| `serde` | (De)serialization of public types | **optional**, `serde` feature |

`proptest` is a dev-dependency only.

## 7. Crate hygiene / quality signals

- `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` in `lib.rs`.
- Prominent "not tax advice / no warranty" note in crate-level docs and README.
- Full rustdoc on every public item, with runnable doctests.
- Declared MSRV in `Cargo.toml` and `README`.
- Complete Cargo metadata (description, license, repository, keywords,
  categories: `finance`) for a clean crates.io listing.
- License: MIT OR Apache-2.0 (Rust ecosystem convention).
- `README.md` with quickstart + a worked, per-wallet, multi-method example.

## 8. Testing strategy

- **Headline worked-example test:** one fixed multi-wallet ledger — buys at
  different prices, a wallet-to-wallet `Transfer`, a crypto-to-crypto `Trade`, an
  `Income` event, then a partial `Sell` — run through FIFO/LIFO/HIFO/Average,
  asserting the *different* realized-gain numbers and short/long-term terms each
  method produces. Doubles as living documentation.
- **Per-method unit tests:** partial/exact/multi-lot consumption; fees on basis
  and proceeds.
- **Per-wallet tests:** a sell drawing only from its own wallet; `InsufficientLots`
  when the asset exists only in another wallet; transfer preserving basis +
  `acquired_at` (verified by the resulting holding-period term).
- **Event-model tests:** `Trade` decomposition (disposal + new lot with correct
  basis); `Income` both income-reported and basis-establishing; `Spend` as a
  disposal at FMV.
- **Holding-period tests:** boundary at exactly 365 days (short) vs 366 (long).
- **SpecificId tests:** honored selection; `MissingLotSelection`;
  `InvalidLotSelection`.
- **Tax-year tests:** disposals/income partitioned into the correct calendar-UTC
  year; short/long-term subtotals.
- **Valuation tests:** unrealized P/L, allocation summing to ~1.0,
  `missing_prices`, zero-cost guard.
- **Stats tests:** known-answer vectors; `None` on too-short series.
- **Property tests (`proptest`):** conservation — realized cost basis + remaining
  basis == total acquired basis; net holdings quantity == acquired − disposed.
- **Doctests** on all public methods.

## 9. Out-of-scope future ideas (not in v0.1)

- Gifts/inheritance received (carryover/stepped-up basis rules).
- Transfer fees modeled as disposals (needs FMV at transfer).
- Wash-sale rules (if/when applied to crypto) and jurisdiction-specific pooling
  (UK Section 104, Canada ACB) and configurable fiscal years.
- CSV / exchange import helpers (likely a separate companion crate).
- Time-weighted / money-weighted returns in `stats`.
- Margin/futures/derivatives, NFTs, and DeFi LP-position accounting.

## 10. Delivery / sequencing

1. Build and test `coinbasis` to completion in its own private repo.
2. Publish `coinbasis` v0.1 to crates.io.
3. The separate `Crypto-Price-Tracker-V2` TUI repo (own spec/plan/cycle) depends
   on it — via a git dependency during development, switching to the crates.io
   version once published.
