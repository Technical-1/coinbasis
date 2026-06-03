//! `coinbasis` — comprehensive crypto tax-lot cost-basis accounting.
//!
//! Hand it a ledger of [`Transaction`]s and current prices; it returns realized
//! capital gains (with holding-period classification), ordinary income,
//! unrealized P/L, portfolio valuation, and tax-year reports. The crate performs
//! **no network access and no file I/O** — callers supply all data.
//!
//! # Example
//! ```
//! use coinbasis::{CostBasisMethod, Portfolio, Transaction};
//! use chrono::{TimeZone, Utc};
//! use rust_decimal::Decimal;
//!
//! let txs = vec![
//!     Transaction::Buy { timestamp: Utc.with_ymd_and_hms(2020,1,1,0,0,0).unwrap(),
//!         wallet: "hot".into(), asset: "btc".into(),
//!         quantity: Decimal::new(1, 0), unit_price: Decimal::new(100, 0), fee: Decimal::new(0, 0) },
//!     Transaction::Sell { timestamp: Utc.with_ymd_and_hms(2022,1,1,0,0,0).unwrap(),
//!         wallet: "hot".into(), asset: "btc".into(),
//!         quantity: Decimal::new(1, 0), unit_price: Decimal::new(500, 0), fee: Decimal::new(0, 0) },
//! ];
//! let portfolio = Portfolio::from_transactions(&txs).unwrap();
//! let gains = portfolio.realized_gains(CostBasisMethod::Fifo).unwrap();
//! assert_eq!(gains[0].gain, Decimal::new(400, 0));
//! ```
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

// Re-exports are added back in the tasks that define each item:
pub use error::PortfolioError;
pub use method::{CostBasisMethod, LotPick, LotSelection};
pub use portfolio::Portfolio;
pub use report::{
    AssetValuation, CapitalGainsReport, Holding, IncomeEvent, IncomeReport, PortfolioReport,
    RealizedGain, Term,
};
pub use transaction::{IncomeSource, Transaction};
