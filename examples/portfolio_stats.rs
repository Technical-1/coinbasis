//! The `stats` module computes pure analytics over a numeric series you supply
//! (e.g. a daily portfolio-value history). It never fetches data and returns
//! `None` for series too short to be meaningful.
//!
//! Run with: `cargo run --example portfolio_stats`

use coinbasis::stats;

fn main() {
    // A toy portfolio-value history.
    let values = [100.0, 110.0, 105.0, 130.0, 120.0];

    let returns = stats::returns_from_values(&values);
    println!("period returns:  {:?}", returns);
    println!("volatility:      {:?}", stats::volatility(&returns));
    println!("sharpe (rf=0):   {:?}", stats::sharpe_ratio(&returns, 0.0));
    println!("max drawdown:    {:?}", stats::max_drawdown(&values));
    println!("cumulative ret:  {:?}", stats::cumulative_return(&values));

    // Conservation sanity: 4 returns from 5 values.
    assert_eq!(returns.len(), 4);
    // Cumulative return is (last - first) / first = (120 - 100) / 100 = 0.2.
    assert!((stats::cumulative_return(&values).unwrap() - 0.2).abs() < 1e-9);
    // Max drawdown is the 130 -> 120 dip = 10/130.
    assert!((stats::max_drawdown(&values).unwrap() - (10.0 / 130.0)).abs() < 1e-9);
}
