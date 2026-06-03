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
#[derive(Debug)]
pub(crate) struct EngineOutput {
    pub realized: Vec<RealizedGain>,
    /// Income events produced during replay. The public facade reads income
    /// directly from the ledger instead, so this is consumed only by engine
    /// tests; kept as part of the engine's complete replay output.
    #[allow(dead_code)]
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
        self.pools
            .entry((asset.to_string(), wallet.to_string()))
            .or_default()
    }

    fn available(&self, asset: &str, wallet: &str) -> Decimal {
        self.pools
            .get(&(asset.to_string(), wallet.to_string()))
            .map(|ls| ls.iter().map(|l| l.quantity).sum())
            .unwrap_or(Decimal::ZERO)
    }

    /// Open a new lot in a wallet. `orig_index` is the acquisition's original
    /// input index (registered for Specific-ID).
    #[allow(clippy::too_many_arguments)]
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
            CostBasisMethod::SpecificId => {
                self.consume_specific(orig_index, asset, wallet, quantity)
            }
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
            let target_lot_id = *self.acq_to_lot.get(&pick.acquisition_index).ok_or(
                PortfolioError::InvalidLotSelection {
                    acquisition_index: pick.acquisition_index,
                },
            )?;
            let lots = self.pool(asset, wallet);
            let pos = lots
                .iter()
                .position(|l| l.lot_id == target_lot_id && l.quantity >= pick.quantity)
                .ok_or(PortfolioError::InvalidLotSelection {
                    acquisition_index: pick.acquisition_index,
                })?;
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

    /// Remove `quantity` units from a pool without realizing gain (for moves).
    /// Uses the active method's ordering (Average pools; SpecificId falls back to
    /// FIFO, since moves are non-taxable and need no caller selection).
    fn take(
        &mut self,
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
            Transaction::Buy {
                timestamp,
                wallet,
                asset,
                quantity,
                unit_price,
                fee,
            } => {
                let basis = *quantity * *unit_price + *fee;
                self.acquire(
                    orig_index, asset, wallet, *quantity, basis, *timestamp, None,
                );
            }
            Transaction::Income {
                timestamp,
                wallet,
                asset,
                quantity,
                value,
                source,
            } => {
                self.acquire(
                    orig_index, asset, wallet, *quantity, *value, *timestamp, None,
                );
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
                timestamp,
                wallet,
                asset,
                quantity,
                donor_basis,
                fmv_at_receipt,
                donor_acquired_at,
            } => {
                // Average ignores dual basis (pool at donor/carryover basis).
                let gift = if self.strategy.method() == CostBasisMethod::Average {
                    None
                } else {
                    Some(GiftBasis {
                        fmv_per_unit: *fmv_at_receipt / *quantity,
                    })
                };
                self.acquire(
                    orig_index,
                    asset,
                    wallet,
                    *quantity,
                    *donor_basis,
                    *donor_acquired_at,
                    gift,
                );
                let _ = timestamp; // receipt time is not the holding-period start
            }
            Transaction::Sell {
                timestamp,
                wallet,
                asset,
                quantity,
                unit_price,
                fee,
            } => {
                let proceeds = *quantity * *unit_price - *fee;
                self.dispose(orig_index, asset, wallet, *quantity, proceeds, *timestamp)?;
            }
            Transaction::Trade {
                timestamp,
                wallet,
                from_asset,
                from_quantity,
                to_asset,
                to_quantity,
                value,
                fee,
            } => {
                // Disposal of the given-up leg at FMV.
                self.dispose(
                    orig_index,
                    from_asset,
                    wallet,
                    *from_quantity,
                    *value,
                    *timestamp,
                )?;
                // Acquisition of the received leg; basis = FMV + fee.
                self.acquire(
                    orig_index,
                    to_asset,
                    wallet,
                    *to_quantity,
                    *value + *fee,
                    *timestamp,
                    None,
                );
            }
            Transaction::Spend {
                timestamp,
                wallet,
                asset,
                quantity,
                value,
                fee,
            } => {
                let proceeds = *value - *fee;
                self.dispose(orig_index, asset, wallet, *quantity, proceeds, *timestamp)?;
            }
            Transaction::Transfer {
                timestamp,
                asset,
                quantity,
                from_wallet,
                to_wallet,
                fee,
                fee_value,
            } => {
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
            Transaction::GiftSent {
                timestamp,
                wallet,
                asset,
                quantity,
            } => {
                // Non-taxable: remove lots (no realized gain), discard them.
                let _ = self.take(asset, wallet, *quantity)?;
                let _ = timestamp;
            }
        }
        Ok(())
    }

    fn finish(self) -> EngineOutput {
        let mut holdings: Vec<Lot> = self.pools.into_values().flatten().collect();
        holdings.sort_by(|a, b| a.lot_id.cmp(&b.lot_id));
        EngineOutput {
            realized: self.realized,
            income: self.income,
            holdings,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::method::CostBasisMethod;
    use crate::transaction::Transaction;
    use chrono::{TimeZone, Utc};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn ts(y: i32, m: u32, d: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
    }

    #[test]
    fn fifo_sell_consumes_oldest_lot_with_term() {
        let txs = vec![
            Transaction::Buy {
                timestamp: ts(2020, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            Transaction::Buy {
                timestamp: ts(2021, 6, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(300),
                fee: dec!(0),
            },
            // Sell 1 BTC at 500 in 2022; FIFO consumes the 2020 lot (long-term).
            Transaction::Sell {
                timestamp: ts(2022, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(500),
                fee: dec!(0),
            },
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
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "w".into(),
                asset: "eth".into(),
                quantity: dec!(2),
                unit_price: dec!(100),
                fee: dec!(10),
            },
            Transaction::Sell {
                timestamp: ts(2021, 2, 1),
                wallet: "w".into(),
                asset: "eth".into(),
                quantity: dec!(1),
                unit_price: dec!(150),
                fee: dec!(0),
            },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        // Lot basis = 2*100 + 10 = 210; per unit 105; selling 1 -> basis 105.
        assert_eq!(out.realized[0].cost_basis, dec!(105));
        assert_eq!(out.realized[0].gain, dec!(45));
    }

    #[test]
    fn oversell_errors_per_wallet() {
        let txs = vec![
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "hot".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            // Selling from a different wallet that holds nothing.
            Transaction::Sell {
                timestamp: ts(2021, 2, 1),
                wallet: "cold".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(150),
                fee: dec!(0),
            },
        ];
        let err = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap_err();
        assert!(matches!(
            err,
            crate::error::PortfolioError::InsufficientLots { .. }
        ));
    }

    #[test]
    fn income_records_event_and_sets_basis() {
        let txs = vec![
            Transaction::Income {
                timestamp: ts(2021, 1, 1),
                wallet: "w".into(),
                asset: "eth".into(),
                quantity: dec!(1),
                value: dec!(50),
                source: crate::transaction::IncomeSource::Staking,
            },
            Transaction::Sell {
                timestamp: ts(2021, 2, 1),
                wallet: "w".into(),
                asset: "eth".into(),
                quantity: dec!(1),
                unit_price: dec!(70),
                fee: dec!(0),
            },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert_eq!(out.income.len(), 1);
        assert_eq!(out.income[0].value, dec!(50));
        assert_eq!(out.realized[0].cost_basis, dec!(50)); // income FMV became basis
        assert_eq!(out.realized[0].gain, dec!(20));
    }

    #[test]
    fn average_pools_basis_and_drops_term() {
        let txs = vec![
            Transaction::Buy {
                timestamp: ts(2020, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(300),
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
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Average)).unwrap();
        // avg cost = (100+300)/2 = 200 -> gain = 300
        assert_eq!(out.realized[0].cost_basis, dec!(200));
        assert_eq!(out.realized[0].gain, dec!(300));
        assert_eq!(out.realized[0].term, None);
        assert_eq!(
            out.holdings.iter().map(|l| l.quantity).sum::<Decimal>(),
            dec!(1)
        );
    }

    #[test]
    fn specific_id_consumes_named_acquisition() {
        let txs = vec![
            // index 0: cheap lot
            Transaction::Buy {
                timestamp: ts(2020, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            // index 1: expensive lot
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(400),
                fee: dec!(0),
            },
            // index 2: sell 1, specifically the expensive lot (index 1)
            Transaction::Sell {
                timestamp: ts(2022, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(500),
                fee: dec!(0),
            },
        ];
        let mut sel: crate::method::LotSelection = std::collections::HashMap::new();
        sel.insert(
            2,
            vec![crate::method::LotPick {
                acquisition_index: 1,
                quantity: dec!(1),
            }],
        );
        let out = run(&txs, Strategy::Specific(&sel)).unwrap();
        assert_eq!(out.realized[0].cost_basis, dec!(400));
        assert_eq!(out.realized[0].gain, dec!(100));
    }

    #[test]
    fn specific_id_missing_selection_errors() {
        let txs = vec![
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
        ];
        let sel: crate::method::LotSelection = std::collections::HashMap::new();
        let err = run(&txs, Strategy::Specific(&sel)).unwrap_err();
        assert!(matches!(
            err,
            crate::error::PortfolioError::MissingLotSelection { .. }
        ));
    }

    #[test]
    fn trade_disposes_from_leg_and_opens_to_leg() {
        let txs = vec![
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            // Trade 1 BTC (FMV 500) for 10 ETH, fee 5 -> ETH basis = 505.
            Transaction::Trade {
                timestamp: ts(2021, 6, 1),
                wallet: "w".into(),
                from_asset: "btc".into(),
                from_quantity: dec!(1),
                to_asset: "eth".into(),
                to_quantity: dec!(10),
                value: dec!(500),
                fee: dec!(5),
            },
            Transaction::Sell {
                timestamp: ts(2021, 7, 1),
                wallet: "w".into(),
                asset: "eth".into(),
                quantity: dec!(10),
                unit_price: dec!(60),
                fee: dec!(0),
            },
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
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            Transaction::Spend {
                timestamp: ts(2021, 2, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                value: dec!(180),
                fee: dec!(0),
            },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert_eq!(out.realized[0].proceeds, dec!(180));
        assert_eq!(out.realized[0].gain, dec!(80));
    }

    #[test]
    fn transfer_preserves_basis_and_acquisition_date() {
        let txs = vec![
            Transaction::Buy {
                timestamp: ts(2020, 1, 1),
                wallet: "hot".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            // Move to cold wallet in 2020, no fee.
            Transaction::Transfer {
                timestamp: ts(2020, 6, 1),
                asset: "btc".into(),
                quantity: dec!(1),
                from_wallet: "hot".into(),
                to_wallet: "cold".into(),
                fee: dec!(0),
                fee_value: dec!(0),
            },
            // Sell from cold in 2022; term must be Long, measured from 2020-01-01.
            Transaction::Sell {
                timestamp: ts(2022, 1, 1),
                wallet: "cold".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(500),
                fee: dec!(0),
            },
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
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "hot".into(),
                asset: "eth".into(),
                quantity: dec!(10),
                unit_price: dec!(10),
                fee: dec!(0),
            }, // basis 100, per-unit 10
            // Move 9, burn 1 as fee at FMV 15.
            Transaction::Transfer {
                timestamp: ts(2021, 2, 1),
                asset: "eth".into(),
                quantity: dec!(9),
                from_wallet: "hot".into(),
                to_wallet: "cold".into(),
                fee: dec!(1),
                fee_value: dec!(15),
            },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        // Fee disposal: 1 unit, basis 10, proceeds 15, gain 5.
        assert_eq!(out.realized.len(), 1);
        assert_eq!(out.realized[0].gain, dec!(5));
        // 9 units now in cold wallet, basis 90.
        let cold: Decimal = out
            .holdings
            .iter()
            .filter(|l| l.wallet == "cold")
            .map(|l| l.cost_basis)
            .sum();
        assert_eq!(cold, dec!(90));
    }

    #[test]
    fn transfer_insufficient_balance_errors() {
        let txs = vec![
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "hot".into(),
                asset: "eth".into(),
                quantity: dec!(1),
                unit_price: dec!(10),
                fee: dec!(0),
            },
            Transaction::Transfer {
                timestamp: ts(2021, 2, 1),
                asset: "eth".into(),
                quantity: dec!(1),
                from_wallet: "hot".into(),
                to_wallet: "cold".into(),
                fee: dec!(1),
                fee_value: dec!(15),
            },
        ];
        let err = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap_err();
        assert!(matches!(
            err,
            crate::error::PortfolioError::InsufficientTransfer { .. }
        ));
    }

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
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(2),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            Transaction::GiftSent {
                timestamp: ts(2021, 2, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
            },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert!(out.realized.is_empty());
        assert_eq!(
            out.holdings.iter().map(|l| l.quantity).sum::<Decimal>(),
            dec!(1)
        );
    }

    #[test]
    fn gift_gain_uses_donor_basis() {
        // donor_basis 100, fmv 120; sell at 200 -> gain = 100 (carryover basis).
        let txs = vec![
            gift_received(1, 100, 120, 2018),
            Transaction::Sell {
                timestamp: ts(2022, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(200),
                fee: dec!(0),
            },
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
            Transaction::Sell {
                timestamp: ts(2022, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(50),
                fee: dec!(0),
            },
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
            Transaction::Sell {
                timestamp: ts(2022, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(90),
                fee: dec!(0),
            },
        ];
        let out = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap();
        assert_eq!(out.realized[0].gain, dec!(0));
    }

    #[test]
    fn gift_sent_over_balance_errors_not_panics() {
        let txs = vec![
            Transaction::Buy {
                timestamp: ts(2021, 1, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(1),
                unit_price: dec!(100),
                fee: dec!(0),
            },
            Transaction::GiftSent {
                timestamp: ts(2021, 2, 1),
                wallet: "w".into(),
                asset: "btc".into(),
                quantity: dec!(5),
            },
        ];
        let err = run(&txs, Strategy::Auto(CostBasisMethod::Fifo)).unwrap_err();
        assert!(matches!(
            err,
            crate::error::PortfolioError::InsufficientLots { .. }
        ));
    }
}
