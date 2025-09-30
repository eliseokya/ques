//! AMM swap simulation models

use crate::{Result, IntelligenceError};

/// Simulate Uniswap V3 concentrated liquidity swap
pub fn simulate_uniswap_v3_swap(
    amount_in: f64,
    reserves: (f64, f64),
    fee_bps: u32,
) -> Result<(f64, f64)> {
    // Simplified constant product with fees
    let fee_multiplier = 1.0 - (fee_bps as f64 / 10000.0);
    let amount_in_with_fee = amount_in * fee_multiplier;
    
    let (reserve_in, reserve_out) = reserves;
    let k = reserve_in * reserve_out;
    
    let new_reserve_in = reserve_in + amount_in_with_fee;
    let new_reserve_out = k / new_reserve_in;
    
    let amount_out = reserve_out - new_reserve_out;
    let slippage_bps = ((amount_in / reserve_in) * 10000.0).min(1000.0);
    
    Ok((amount_out, slippage_bps))
}

/// Simulate Curve stableswap
pub fn simulate_curve_swap(
    amount_in: f64,
    virtual_price: f64,
    fee_bps: u32,
) -> Result<(f64, f64)> {
    // Simplified Curve model (actual uses invariant)
    let fee_multiplier = 1.0 - (fee_bps as f64 / 10000.0);
    let amount_out = amount_in * fee_multiplier * virtual_price;
    
    // Curve has low slippage for stables
    let slippage_bps = 2.0;
    
    Ok((amount_out, slippage_bps))
}

/// Simulate Balancer weighted pool swap
pub fn simulate_balancer_swap(
    amount_in: f64,
    weight_in: f64,
    weight_out: f64,
    reserve_in: f64,
    reserve_out: f64,
    fee_bps: u32,
) -> Result<(f64, f64)> {
    // Weighted constant product formula
    let fee_multiplier = 1.0 - (fee_bps as f64 / 10000.0);
    let amount_in_with_fee = amount_in * fee_multiplier;
    
    let ratio = (reserve_in + amount_in_with_fee) / reserve_in;
    let power = weight_in / weight_out;
    let amount_out = reserve_out * (1.0 - ratio.powf(power));
    
    let slippage_bps = ((amount_in / reserve_in) * 10000.0 * weight_in).min(500.0);
    
    Ok((amount_out, slippage_bps))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_uniswap_v3_swap() {
        let (amount_out, slippage) = simulate_uniswap_v3_swap(
            1000.0,
            (100_000.0, 100_000.0),
            30, // 0.3% fee
        ).unwrap();
        
        assert!(amount_out > 0.0);
        assert!(amount_out < 1000.0); // Should have some cost
        assert!(slippage > 0.0);
    }
    
    #[test]
    fn test_curve_swap() {
        let (amount_out, slippage) = simulate_curve_swap(
            1000.0,
            1.0,
            4, // 0.04% fee
        ).unwrap();
        
        assert!(amount_out > 990.0); // Curve has low fees
        assert!(slippage < 5.0); // Low slippage for stables
    }
}

