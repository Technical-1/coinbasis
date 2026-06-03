//! The public [`Portfolio`] facade: stores an immutable ledger and answers
//! cost-basis, income, holdings, valuation, and tax-report queries.

use crate::engine::{run, Strategy};
use crate::error::PortfolioError;
use crate::lot::Lot;
use crate::method::{CostBasisMethod, LotSelection};
use crate::report::{
    AssetValuation, CapitalGainsReport, Holding, IncomeEvent, IncomeReport, PortfolioReport,
    RealizedGain, Term,
};
use crate::transaction::Transaction;
use chrono::Datelike;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};

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
    pub fn realized_gains(
        &self,
        method: CostBasisMethod,
    ) -> Result<Vec<RealizedGain>, PortfolioError> {
        if method == CostBasisMethod::SpecificId {
            return Err(PortfolioError::SelectionRequired);
        }
        Ok(run(&self.txs, Strategy::Auto(method))?.realized)
    }

    /// Realized capital-gain rows using a Specific-ID selection.
    pub fn realized_gains_with_selection(
        &self,
        selection: &LotSelection,
    ) -> Result<Vec<RealizedGain>, PortfolioError> {
        Ok(run(&self.txs, Strategy::Specific(selection))?.realized)
    }

    /// Ordinary-income events (method-independent).
    pub fn income_events(&self) -> Vec<IncomeEvent> {
        // Income does not depend on the disposal method; it is read directly
        // from the ledger so it cannot surface disposal/lot errors.
        self.txs
            .iter()
            .filter_map(|tx| match tx {
                Transaction::Income {
                    timestamp,
                    wallet,
                    asset,
                    quantity,
                    value,
                    source,
                } => Some(IncomeEvent {
                    asset: asset.clone(),
                    wallet: wallet.clone(),
                    received_at: *timestamp,
                    quantity: *quantity,
                    value: *value,
                    source: *source,
                }),
                _ => None,
            })
            .collect()
    }

    /// Current open positions (per wallet) under an automatic method.
    pub fn holdings(&self, method: CostBasisMethod) -> Result<Vec<Holding>, PortfolioError> {
        if method == CostBasisMethod::SpecificId {
            return Err(PortfolioError::SelectionRequired);
        }
        Ok(run(&self.txs, Strategy::Auto(method))?
            .holdings
            .iter()
            .map(to_holding)
            .collect())
    }

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
            let e = agg
                .entry(h.asset.clone())
                .or_insert((Decimal::ZERO, Decimal::ZERO));
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
        let rows: Vec<RealizedGain> = realized
            .into_iter()
            .filter(|r| r.disposed_at.year() == tax_year)
            .collect();
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
        CapitalGainsReport {
            tax_year,
            rows,
            short_term_gain,
            long_term_gain,
            total_gain,
        }
    }

    /// Ordinary-income report for one calendar tax year (UTC).
    pub fn income_report(&self, tax_year: i32) -> IncomeReport {
        let events: Vec<IncomeEvent> = self
            .income_events()
            .into_iter()
            .filter(|e| e.received_at.year() == tax_year)
            .collect();
        let total_income = events.iter().map(|e| e.value).sum();
        IncomeReport {
            tax_year,
            events,
            total_income,
        }
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
            Transaction::Buy {
                timestamp: ts(2020, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            Transaction::Sell {
                timestamp: ts(2022, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(500),
                fee: dec!(0),
            },
        ]
    }

    #[test]
    fn from_transactions_validates() {
        let bad = vec![Transaction::Buy {
            timestamp: ts(2020, 1, 1),
            wallet: "w".into(),
            asset: "btc".into(),
            quantity: dec!(0),
            unit_price: dec!(100),
            fee: dec!(0),
        }];
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
        sel.insert(
            1,
            vec![LotPick {
                acquisition_index: 0,
                quantity: dec!(1),
            }],
        );
        let g = p.realized_gains_with_selection(&sel).unwrap();
        assert_eq!(g[0].gain, dec!(400));
    }

    #[test]
    fn holdings_reports_open_positions() {
        let txs = vec![Transaction::Buy {
            timestamp: ts(2021, 1, 1),
            wallet: "w".into(),
            asset: "eth".into(),
            quantity: dec!(2),
            unit_price: dec!(50),
            fee: dec!(0),
        }];
        let p = Portfolio::from_transactions(&txs).unwrap();
        let h = p.holdings(CostBasisMethod::Fifo).unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].quantity, dec!(2));
        assert_eq!(h[0].average_cost, dec!(50));
    }

    #[test]
    fn valuation_aggregates_and_flags_missing_prices() {
        let txs = vec![
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "a".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            Transaction::Buy {
                timestamp: ts(2021, 1, 2),
                wallet: "b".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(140),
                fee: dec!(0),
            },
            Transaction::Buy {
                timestamp: ts(2021, 1, 3),
                wallet: "a".into(),
                asset: "eth".into(),
                quantity: dec!(1),
                unit_price: dec!(50),
                fee: dec!(0),
            },
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

    #[test]
    fn capital_gains_report_filters_year_and_splits_terms() {
        let txs = vec![
            Transaction::Buy {
                timestamp: ts(2019, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(2),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            // Long-term sale in 2021 (held > 1y): gain 400.
            Transaction::Sell {
                timestamp: ts(2021, 3, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(500),
                fee: dec!(0),
            },
            // Short-term: buy and sell within 2021: basis 100, proceeds 130 -> gain 30.
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "w".into(),
                asset: "eth".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            Transaction::Sell {
                timestamp: ts(2021, 6, 1),
                wallet: "w".into(),
                asset: "eth".into(),
                quantity: dec!(1),
                unit_price: dec!(130),
                fee: dec!(0),
            },
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
            Transaction::Income {
                timestamp: ts(2020, 5, 1),
                wallet: "w".into(),
                asset: "eth".into(),
                quantity: dec!(1),
                value: dec!(40),
                source: crate::transaction::IncomeSource::Staking,
            },
            Transaction::Income {
                timestamp: ts(2021, 5, 1),
                wallet: "w".into(),
                asset: "eth".into(),
                quantity: dec!(1),
                value: dec!(60),
                source: crate::transaction::IncomeSource::Airdrop,
            },
        ];
        let p = Portfolio::from_transactions(&txs).unwrap();
        let r = p.income_report(2021);
        assert_eq!(r.events.len(), 1);
        assert_eq!(r.total_income, dec!(60));
    }
}
