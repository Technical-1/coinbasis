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
        // Income does not depend on the disposal method; it is read directly
        // from the ledger so it cannot surface disposal/lot errors.
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
