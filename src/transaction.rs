//! The transaction event model fed into a [`crate::Portfolio`].
//!
//! [`Transaction`] is the single input type: each variant is one ledger event
//! (buy, sell, crypto-to-crypto trade, income, spend, wallet transfer, or
//! gift). All monetary fields are [`rust_decimal::Decimal`] in one quote
//! currency (USD by convention). [`Transaction::validate`] checks field-level
//! invariants; lot-availability errors surface later during replay.

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
                Err(PortfolioError::NonPositiveQuantity {
                    asset: asset.to_string(),
                    quantity: q,
                })
            } else {
                Ok(())
            }
        };
        let non_neg_val = |asset: &str, v: Decimal| -> Result<(), PortfolioError> {
            if v < Decimal::ZERO {
                Err(PortfolioError::NegativeValue {
                    asset: asset.to_string(),
                })
            } else {
                Ok(())
            }
        };
        let non_neg_fee = |asset: &str, f: Decimal| -> Result<(), PortfolioError> {
            if f < Decimal::ZERO {
                Err(PortfolioError::NegativeFee {
                    asset: asset.to_string(),
                    fee: f,
                })
            } else {
                Ok(())
            }
        };

        match self {
            Transaction::Buy {
                asset,
                quantity,
                unit_price,
                fee,
                ..
            }
            | Transaction::Sell {
                asset,
                quantity,
                unit_price,
                fee,
                ..
            } => {
                pos_qty(asset, *quantity)?;
                non_neg_val(asset, *unit_price)?;
                non_neg_fee(asset, *fee)
            }
            Transaction::Trade {
                from_asset,
                from_quantity,
                to_asset,
                to_quantity,
                value,
                fee,
                ..
            } => {
                pos_qty(from_asset, *from_quantity)?;
                pos_qty(to_asset, *to_quantity)?;
                non_neg_val(from_asset, *value)?;
                non_neg_fee(from_asset, *fee)
            }
            Transaction::Income {
                asset,
                quantity,
                value,
                ..
            } => {
                pos_qty(asset, *quantity)?;
                non_neg_val(asset, *value)
            }
            Transaction::Spend {
                asset,
                quantity,
                value,
                fee,
                ..
            } => {
                pos_qty(asset, *quantity)?;
                non_neg_val(asset, *value)?;
                non_neg_fee(asset, *fee)
            }
            Transaction::Transfer {
                asset,
                quantity,
                fee,
                fee_value,
                ..
            } => {
                pos_qty(asset, *quantity)?;
                non_neg_fee(asset, *fee)?;
                non_neg_val(asset, *fee_value)
            }
            Transaction::GiftSent {
                asset, quantity, ..
            } => pos_qty(asset, *quantity),
            Transaction::GiftReceived {
                asset,
                quantity,
                donor_basis,
                fmv_at_receipt,
                ..
            } => {
                pos_qty(asset, *quantity)?;
                non_neg_val(asset, *donor_basis)?;
                non_neg_val(asset, *fmv_at_receipt)
            }
        }
    }
}

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
            timestamp: ts(2021, 1, 1),
            wallet: "w".into(),
            asset: "bitcoin".into(),
            quantity: dec!(1),
            unit_price: dec!(100),
            fee: dec!(0),
        };
        assert_eq!(b.timestamp(), ts(2021, 1, 1));
        let t = Transaction::Transfer {
            timestamp: ts(2021, 2, 2),
            asset: "bitcoin".into(),
            quantity: dec!(1),
            from_wallet: "a".into(),
            to_wallet: "b".into(),
            fee: dec!(0),
            fee_value: dec!(0),
        };
        assert_eq!(t.timestamp(), ts(2021, 2, 2));
    }

    #[test]
    fn validate_rejects_non_positive_quantity() {
        let b = Transaction::Buy {
            timestamp: ts(2021, 1, 1),
            wallet: "w".into(),
            asset: "eth".into(),
            quantity: dec!(0),
            unit_price: dec!(100),
            fee: dec!(0),
        };
        assert_eq!(
            b.validate(),
            Err(PortfolioError::NonPositiveQuantity {
                asset: "eth".into(),
                quantity: dec!(0)
            })
        );
    }

    #[test]
    fn validate_rejects_negative_fee() {
        let s = Transaction::Sell {
            timestamp: ts(2021, 1, 1),
            wallet: "w".into(),
            asset: "eth".into(),
            quantity: dec!(1),
            unit_price: dec!(100),
            fee: dec!(-1),
        };
        assert_eq!(
            s.validate(),
            Err(PortfolioError::NegativeFee {
                asset: "eth".into(),
                fee: dec!(-1)
            })
        );
    }

    #[test]
    fn validate_accepts_well_formed_event() {
        let b = Transaction::Buy {
            timestamp: ts(2021, 1, 1),
            wallet: "w".into(),
            asset: "eth".into(),
            quantity: dec!(1),
            unit_price: dec!(100),
            fee: dec!(1),
        };
        assert_eq!(b.validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_negative_unit_price() {
        let b = Transaction::Buy {
            timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(1), unit_price: dec!(-1), fee: dec!(0),
        };
        assert_eq!(b.validate(), Err(PortfolioError::NegativeValue { asset: "btc".into() }));
    }

    #[test]
    fn validate_rejects_negative_gift_fmv() {
        let g = Transaction::GiftReceived {
            timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(1), donor_basis: dec!(10), fmv_at_receipt: dec!(-5),
            donor_acquired_at: ts(2018, 1, 1),
        };
        assert_eq!(g.validate(), Err(PortfolioError::NegativeValue { asset: "btc".into() }));
    }

    #[test]
    fn validate_rejects_negative_transfer_fee_value() {
        let t = Transaction::Transfer {
            timestamp: ts(2021, 1, 1), asset: "btc".into(), quantity: dec!(1),
            from_wallet: "a".into(), to_wallet: "b".into(), fee: dec!(1), fee_value: dec!(-1),
        };
        assert_eq!(t.validate(), Err(PortfolioError::NegativeValue { asset: "btc".into() }));
    }

    #[test]
    fn validate_rejects_non_positive_trade_leg() {
        let tr = Transaction::Trade {
            timestamp: ts(2021, 1, 1), wallet: "w".into(),
            from_asset: "btc".into(), from_quantity: dec!(0),
            to_asset: "eth".into(), to_quantity: dec!(10),
            value: dec!(500), fee: dec!(0),
        };
        assert_eq!(
            tr.validate(),
            Err(PortfolioError::NonPositiveQuantity { asset: "btc".into(), quantity: dec!(0) })
        );
    }

    #[test]
    fn validate_accepts_well_formed_gift_and_transfer() {
        let g = Transaction::GiftReceived {
            timestamp: ts(2021, 1, 1), wallet: "w".into(), asset: "btc".into(),
            quantity: dec!(1), donor_basis: dec!(10), fmv_at_receipt: dec!(12),
            donor_acquired_at: ts(2018, 1, 1),
        };
        assert_eq!(g.validate(), Ok(()));
        let t = Transaction::Transfer {
            timestamp: ts(2021, 1, 1), asset: "btc".into(), quantity: dec!(1),
            from_wallet: "a".into(), to_wallet: "b".into(), fee: dec!(0), fee_value: dec!(0),
        };
        assert_eq!(t.validate(), Ok(()));
    }
}
