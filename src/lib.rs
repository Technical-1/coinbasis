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

// Re-exports are added back in the tasks that define each item:
// pub use error::PortfolioError;
// pub use method::{CostBasisMethod, LotPick, LotSelection};
// pub use portfolio::Portfolio;
// pub use report::{
//     AssetValuation, CapitalGainsReport, Holding, IncomeEvent, IncomeReport, PortfolioReport,
//     RealizedGain, Term,
// };
// pub use transaction::{IncomeSource, Transaction};
