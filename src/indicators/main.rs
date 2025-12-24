#![allow(unused)]

mod zindicator5;

use zindicator5::{IndicatorManager, IndicatorName, MarketData};

fn main() {
    let mut manager = IndicatorManager::new();

    let first = manager.update(
        "EURUSD",
        "1m",
        IndicatorName::Rsi,
        MarketData { price: 101.2 },
    );
    let vol = manager.update(
        "EURUSD",
        "1m",
        IndicatorName::Volatility,
        MarketData { price: 101.2 },
    );
    let second = manager.update(
        "EURUSD",
        "1m",
        IndicatorName::Rsi,
        MarketData { price: 102.0 },
    );

    println!("First RSI: {:?}", first);
    println!("Volatility: {:?}", vol);
    println!("Second RSI: {:?}", second);
}
