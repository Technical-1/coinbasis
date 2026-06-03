//! Error type for ledger validation and replay.

use rust_decimal::Decimal;

/// Errors produced when validating a ledger or computing cost basis.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum PortfolioError {
    /// A disposal asked for more units than the wallet's pool holds.
    #[error("disposed {attempted} {asset} from wallet '{wallet}' but only {available} held there")]
    InsufficientLots {
        /// Asset identifier.
        asset: String,
        /// Wallet the disposal drew from.
        wallet: String,
        /// Units requested.
        attempted: Decimal,
        /// Units available in that wallet.
        available: Decimal,
    },
    /// A transfer's `quantity + fee` exceeds the source wallet's balance.
    #[error("transfer of {quantity} (+fee {fee}) {asset} from '{wallet}' exceeds the {available} held there")]
    InsufficientTransfer {
        /// Asset identifier.
        asset: String,
        /// Source wallet.
        wallet: String,
        /// Units to move.
        quantity: Decimal,
        /// Fee units burned.
        fee: Decimal,
        /// Units available.
        available: Decimal,
    },
    /// An event carried a non-positive quantity.
    #[error("event for {asset} has non-positive quantity {quantity}")]
    NonPositiveQuantity {
        /// Asset identifier.
        asset: String,
        /// The offending quantity.
        quantity: Decimal,
    },
    /// An event carried a negative monetary value or price.
    #[error("event for {asset} has a negative value or price")]
    NegativeValue {
        /// Asset identifier.
        asset: String,
    },
    /// An event carried a negative fee.
    #[error("event for {asset} has a negative fee {fee}")]
    NegativeFee {
        /// Asset identifier.
        asset: String,
        /// The offending fee.
        fee: Decimal,
    },
    /// A Specific-ID disposal had no usable lot selection.
    #[error("Specific-ID disposal of {asset} (input index {tx_index}) lacks a valid, fully-covering lot selection")]
    MissingLotSelection {
        /// Asset identifier.
        asset: String,
        /// Original input index of the disposal.
        tx_index: usize,
    },
    /// A Specific-ID selection referenced an unknown or exhausted acquisition.
    #[error("Specific-ID selection references unknown or exhausted acquisition index {acquisition_index}")]
    InvalidLotSelection {
        /// The bad acquisition index.
        acquisition_index: usize,
    },
    /// `realized_gains`/etc. was called with `SpecificId`; use the `*_with_selection` API.
    #[error("SpecificId requires a lot selection; call the *_with_selection method")]
    SelectionRequired,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn insufficient_lots_message_names_wallet() {
        let e = PortfolioError::InsufficientLots {
            asset: "bitcoin".into(),
            wallet: "coldwallet".into(),
            attempted: dec!(2),
            available: dec!(1),
        };
        let msg = e.to_string();
        assert!(msg.contains("bitcoin"));
        assert!(msg.contains("coldwallet"));
        assert!(msg.contains('2') && msg.contains('1'));
    }
}
