# Project Q&A

## Overview

`coinbasis` is a pure Rust library that turns a crypto transaction history into
the numbers you need at tax time: realized capital gains (split into short- and
long-term), ordinary income, unrealized profit and loss, portfolio valuation,
and per-tax-year reports. The interesting part is that it answers every question
by replaying an immutable ledger through a cost-basis method you pick at query
time — so the *same* history can be viewed under FIFO, LIFO, HIFO, Average, or
Specific-ID, and you can see exactly how the method changes your tax bill.

## Problem Solved

Crypto cost-basis accounting is deceptively hard. A single sale's taxable gain
depends on which earlier purchase it's matched against, and that matching follows
rules (FIFO, Specific-ID, and others) that produce materially different results.
Layer on crypto-to-crypto trades (each one is both a sale and a purchase),
staking and airdrop income, spending crypto on goods, moving coins between your
own wallets, and gifts with their own special basis rules — and a spreadsheet
stops being trustworthy. `coinbasis` encodes these rules once, correctly, with
exact decimal math, so an application can compute them reliably instead of
re-deriving them.

## Target Users

- **Rust developers building crypto tax or portfolio tools** — a calculation
  core they can embed without reimplementing cost-basis rules
- **Builders of dashboards and terminal apps** — a no-I/O library that takes a
  ledger and prices and returns reports, leaving data fetching and UI to the app
- **Traders and tinkerers** — anyone who wants to compute, programmatically, how
  FIFO vs. HIFO vs. Specific-ID changes their realized gains on a real history

## Key Features

### Five cost-basis methods on one ledger
FIFO, LIFO, HIFO, Average, and Specific-ID. Because the ledger is replayed per
query, you can run the identical history through each method and compare the
realized gains directly.

### The full event model
Buys, sells, crypto-to-crypto trades, income (staking/mining/airdrop/interest),
spends, wallet-to-wallet transfers, and gifts — sent and received — are all
first-class. Each is handled with its correct tax treatment.

### Per-wallet lots with faithful transfers
Lots are tracked per wallet. Moving coins between your own wallets is
non-taxable and preserves the original basis and holding-period start, while a
network fee paid in the asset is correctly treated as a small taxable disposal.

### Tax-year reports
A Form-8949-shaped capital-gains report (with short- and long-term subtotals)
and an income report, each filtered to a calendar tax year.

### Tax liability estimation
`TaxConfig` specifies a flat short-term rate and progressive long-term brackets
(the default ships 2024 US federal rates: 35% flat short-term; 0%/15%/20%
long-term tiers). Call `Portfolio::tax_estimate` for a one-shot estimate from a
ledger, or pass an existing `CapitalGainsReport` directly to `tax::estimate`.
Rows are reclassified against the config's `long_term_threshold_days` at
estimation time, so the same report can be re-evaluated under a different
threshold or jurisdiction without re-running the engine.

## Technical Highlights

### The gift dual-basis rule in one place
Gifts follow an unusual IRS rule: when you later sell a gifted coin, you use the
donor's original basis to compute a *gain*, but the lesser of (donor's basis,
market value at receipt) to compute a *loss* — and if the sale price lands
between those two, there's neither a gain nor a loss. I implemented this as a
single function operating on a neutral "consumed slice" of a lot
(`gain_for` in `src/engine.rs`), rather than scattering it across transaction
types. Any disposal of a gifted lot, under any method, reuses the same three
branches, and the rule is unit-tested in isolation including the no-gain/no-loss
dead zone.

### Specific-ID that survives event reordering
Under Specific-ID the caller names which lots each sale draws from. The catch:
the engine processes events in timestamp order, but the caller thinks in terms
of the original input order. The selection map is therefore keyed by a disposal's
*input index* and references acquisitions by *their* input index
(`src/method.rs`), and the engine maintains a bridge from input index to the
internal lot identity it assigns during replay (`src/engine.rs`). This keeps the
public API stable and intuitive regardless of how the engine internally orders
events.

### Conservation guaranteed by property tests
The most important correctness property of a cost-basis engine is that it neither
creates nor destroys value: across any sequence of buys and sells, the basis
consumed by disposals plus the basis remaining in open lots must exactly equal
the basis originally acquired. Using exact decimals makes this hold to the cent,
and a `proptest` generates random ledgers and asserts it (`tests/properties.rs`).
This is also why money never touches `f64` — only the separate statistics module
does.

### A single replay path for every method
All five methods, plus transfers and gifts, flow through one `run` → `dispose`
path in the engine, switched by a small `Strategy` enum. Adding the awkward
Specific-ID case did not require threading optional parameters through every
public query; it stayed contained in one consume function.

## Engineering Decisions

### Recompute on query vs. maintain live balances
- **Constraint**: the same portfolio must be answerable under five different
  cost-basis methods, and tax results depend on full history.
- **Options**: keep mutable per-method lot pools updated as events arrive, or
  store the ledger and replay it on demand.
- **Choice**: store an immutable ledger and replay per query.
- **Why**: one source of truth, no risk of five mutable views drifting out of
  sync, and replay is cheap for realistic histories. The simplicity is worth far
  more than the negligible recompute cost.

### Decimal everywhere money lives
- **Constraint**: cost-basis math must conserve value exactly.
- **Options**: `f64`, or a fixed-point decimal type.
- **Choice**: `rust_decimal::Decimal` for all amounts; `f64` only in the
  statistics module.
- **Why**: binary floats can't represent most decimal cents exactly, so sums and
  splits would drift; decimals make conservation provable and keep reported
  figures matching hand calculation.

### A library with no I/O
- **Constraint**: cost-basis logic is reusable, but data sources (exchanges,
  price feeds) and presentation differ wildly between applications.
- **Options**: bundle price-fetching and persistence, or stay purely
  computational.
- **Choice**: no network or file access — the caller passes in the ledger,
  prices, and any value series.
- **Why**: it makes the crate trivial to test, easy to embed in anything from a
  CLI to a terminal UI, and keeps the dependency surface small.

## Frequently Asked Questions

### Can I compare cost-basis methods on the same history?
Yes — that's the point. Build one `Portfolio` and call `realized_gains` (or
`capital_gains_report`) with each `CostBasisMethod`. The `cost_basis_methods`
example does exactly this and prints the differing gains.

### How does Specific-ID lot selection work?
You pass a `LotSelection` — a map from a disposal's input index to the
acquisitions (by their input index) and quantities it should consume. The
`*_with_selection` query methods use it; the picks must exactly cover the
disposed quantity, or you get a descriptive error.

### How are gifts handled?
Received gifts open a lot at the donor's carryover basis, and the holding period
tacks back to the donor's acquisition date. On a later sale the dual-basis rule
applies: donor basis for gains, the lesser of donor basis and value-at-receipt
for losses, and no gain or loss in between. Sending a gift removes lots with no
realized gain.

### What happens when I move coins between my own wallets?
A `Transfer` moves the lots to the destination wallet, preserving their basis and
holding-period start — it is not a taxable event. If you pay a network fee in the
asset, that portion is treated as a separate taxable disposal at the fair value
you supply.

### Does it fetch prices or hit the network?
No. Valuation takes a map of current prices you provide, and the statistics
helpers take a value series you maintain. The crate never performs I/O.

### Is this US-only?
The default model follows current US federal treatment (per-account lots, the
short/long holding-period split, the gift dual-basis rule). It's a calculation
library, and the modeled behavior is documented — but it is not tax advice and
makes no guarantee for any jurisdiction.

### Can I serialize the inputs and outputs?
Yes. Enable the `serde` feature and every public type — transactions and report
types alike — gains `Serialize`/`Deserialize`, so you can persist ledgers and
results as JSON.

### How is the holding period decided?
Each realized lot is classified `Short` or `Long` at a 365-day boundary, a
documented approximation of the IRS "more than one year" rule.

### How does tax estimation work?
Pass a `TaxConfig` (or use `TaxConfig::default()` for 2024 US federal rates) to
`Portfolio::tax_estimate(method, tax_year, &config)`. It builds the
`CapitalGainsReport` for that year, then runs `tax::estimate` over it —
re-deriving short/long classification from actual holding days against
`config.long_term_threshold_days`, applying the flat rate to net short-term
gains, and applying the progressive long-term brackets. Losses are never taxed.
The returned `TaxEstimate` breaks out short and long gains, their respective
taxes, and the total. This is an estimate only — not tax advice.
