# Tech Stack

## Core Technologies

| Category | Technology | Version | Why this choice |
|----------|------------|---------|-----------------|
| Language | Rust (edition 2021) | MSRV 1.74 | Memory safety and an expressive type system let the transaction model and error cases be encoded so invalid states are hard to represent |
| Money math | `rust_decimal` | 1.x | Exact base-10 fixed-point arithmetic — cost-basis accounting must conserve value to the cent, which binary floats cannot guarantee |
| Time | `chrono` | 0.4 | Timestamps and date arithmetic for ordering events and classifying holding periods; pulled in with `default-features = false` plus only `clock` and `std` to keep the dependency lean |
| Errors | `thiserror` | 1.x | Derives `std::error::Error` with descriptive, per-variant messages without hand-written boilerplate |
| Serialization (optional) | `serde` | 1.x | Behind a feature flag so callers who don't need it pay nothing; enables persisting ledgers and reports as JSON or other formats |

## Distribution

- **Published**: [crates.io](https://crates.io/crates/coinbasis) as `coinbasis`
- **API docs**: [docs.rs/coinbasis](https://docs.rs/coinbasis), built with all
  features so the optional `serde` implementations are documented
- **No runtime services**: it is a calculation library — no server, database, or
  hosting

## Testing & Quality

- **Unit & integration tests**: `cargo test` — per-module unit tests (including
  a dedicated suite in `src/tax.rs` covering progressive brackets, threshold
  reclassification, and the Average-method fallback) plus a worked, multi-method
  integration test that doubles as living documentation
- **Property tests**: `proptest` checks conservation invariants (consumed basis
  plus remaining basis equals acquired basis; quantities are conserved) over
  randomly generated ledgers
- **Doc tests**: every public method and statistics function carries a runnable
  example, executed by `cargo test --doc`
- **Coverage**: line coverage measured with `cargo-llvm-cov`; the few uncovered
  lines are unreachable defensive guards
- **Linting/formatting**: `cargo clippy` (warnings denied) and `cargo fmt`

## Key Dependencies

| Package | Purpose |
|---------|---------|
| `rust_decimal` | Exact decimal type for all monetary amounts |
| `chrono` | Timestamps, durations, and holding-period date math |
| `thiserror` | Ergonomic, typed error definitions |
| `serde` (optional) | Serialize/Deserialize for the public types |
| `proptest` (dev) | Property-based testing of conservation invariants |
| `rust_decimal_macros` (dev) | The `dec!` literal macro used in tests and examples |
