use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rust_decimal::{prelude::ToPrimitive, Decimal, MathematicalOps};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    binance::models::{orderbook::BookFeatures, trades::TradeFeatures},
    ROLLING_WINDOW,
};
pub type Dfrwl = Arc<RwLock<FeatureDataFrame>>;
#[derive(Debug, Clone)]
pub struct Observation {
    pub timestamp: i64,
    pub price: Decimal,
    pub net_qty: Decimal,
    pub notional: Decimal,
    pub bid_total: Decimal,
    pub num_ticks_from_best_bid: Decimal,
    pub ask_total: Decimal,
    pub num_ticks_from_best_ask: Decimal,
    pub bids_asks_ratio: Decimal,
    pub bid_notional: Decimal,
    pub ask_notional: Decimal,
    pub rolling_qty: Option<Decimal>,
    pub mean_qty: Option<Decimal>,
    pub qty_std: Option<Decimal>,
    pub mean_price: Option<Decimal>,
    pub price_std: Option<Decimal>,
    pub book_ratio_rolling_mean: Option<Decimal>,
    pub rolling_qty_abs: Option<Decimal>,
    pub target: Option<Decimal>,
}
impl Observation {
    pub fn from_trade_and_book(trade_features: TradeFeatures, book_features: BookFeatures) -> Self {
        Self {
            timestamp: trade_features.timestamp,
            price: trade_features.price,
            net_qty: trade_features.net_qty,
            notional: trade_features.notional,
            bid_total: book_features.bid_total,
            num_ticks_from_best_bid: book_features.num_ticks_from_best_bid,
            ask_total: book_features.ask_total,
            num_ticks_from_best_ask: book_features.num_ticks_from_best_ask,
            bids_asks_ratio: book_features.bids_asks_ratio,
            bid_notional: book_features.bid_notional,
            ask_notional: book_features.ask_notional,
            rolling_qty: None,
            mean_qty: None,
            qty_std: None,
            mean_price: None,
            price_std: None,
            book_ratio_rolling_mean: None,
            rolling_qty_abs: None,
            target: None,
        }
    }
    ///Returns true if the `Observation` has rolling features.
    pub fn has_rolling_features(&self) -> bool {
        self.rolling_qty.is_some()
            && self.mean_qty.is_some()
            && self.qty_std.is_some()
            && self.mean_price.is_some()
            && self.price_std.is_some()
            && self.book_ratio_rolling_mean.is_some()
            && self.rolling_qty_abs.is_some()
    }
    pub fn has_target(&self) -> bool {
        self.target.is_some()
    }
    pub fn to_training_data(&self) -> [f32; 18] {
        [
            self.timestamp as f32,
            self.price.to_f32().unwrap(),
            self.net_qty.to_f32().unwrap(),
            self.notional.to_f32().unwrap(),
            self.bid_total.to_f32().unwrap(),
            self.num_ticks_from_best_bid.to_f32().unwrap(),
            self.ask_total.to_f32().unwrap(),
            self.num_ticks_from_best_ask.to_f32().unwrap(),
            self.bids_asks_ratio.to_f32().unwrap(),
            self.bid_notional.to_f32().unwrap(),
            self.ask_notional.to_f32().unwrap(),
            self.rolling_qty.unwrap().to_f32().unwrap(),
            self.mean_qty.unwrap().to_f32().unwrap(),
            self.qty_std.unwrap().to_f32().unwrap(),
            self.mean_price.unwrap().to_f32().unwrap(),
            self.price_std.unwrap().to_f32().unwrap(),
            self.book_ratio_rolling_mean.unwrap().to_f32().unwrap(),
            self.rolling_qty_abs.unwrap().to_f32().unwrap(),
        ]
    }
}

pub fn new_dataframe_rwl() -> Dfrwl {
    let df = FeatureDataFrame::new_empty();
    Arc::new(RwLock::new(df))
}

#[derive(Debug, Clone)]
pub struct FeatureDataFrame {
    pub data: Vec<Observation>,
}
impl FeatureDataFrame {
    pub fn new_empty() -> Self {
        Self {
            data: Vec::with_capacity(10000000),
        }
    }
    pub fn calculate_rolling_features(&mut self) {
        if self.data.is_empty() || self.data.len() < ROLLING_WINDOW + 2 {
            return;
        };
        let max_index = self.data.len() - 1;
        for i in (ROLLING_WINDOW..max_index).rev() {
            if self.data[i].has_rolling_features() {
                continue;
            }
            let net_qty_rolling = self.data[i - ROLLING_WINDOW..i]
                .par_iter()
                .map(|x| x.notional)
                .collect::<Vec<_>>();
            let rolling_qty = net_qty_rolling.par_iter().sum::<Decimal>();
            let mean_qty = rolling_qty / Decimal::from(ROLLING_WINDOW);
            let qty_std = Decimal::sqrt(
                &(net_qty_rolling
                    .par_iter()
                    .map(|x| (x - mean_qty).powi(2))
                    .sum::<Decimal>()
                    / Decimal::from(ROLLING_WINDOW)),
            );
            let last_price_rolling = self.data[i - ROLLING_WINDOW..i]
                .par_iter()
                .map(|x| x.price)
                .collect::<Vec<_>>();
            let mean_price =
                last_price_rolling.par_iter().sum::<Decimal>() / Decimal::from(ROLLING_WINDOW);
            let price_std = Decimal::sqrt(
                &(last_price_rolling
                    .par_iter()
                    .map(|x| (x - mean_price).powi(2))
                    .sum::<Decimal>()
                    / Decimal::from(ROLLING_WINDOW)),
            );
            let book_ratio_rolling_mean = self.data[i - ROLLING_WINDOW..i]
                .par_iter()
                .map(|x| x.bids_asks_ratio)
                .collect::<Vec<_>>()
                .par_iter()
                .sum::<Decimal>()
                / Decimal::from(ROLLING_WINDOW);
            let rolling_qty_abs = self.data[i - ROLLING_WINDOW..i]
                .par_iter()
                .map(|x| x.notional.abs())
                .collect::<Vec<_>>()
                .par_iter()
                .sum::<Decimal>();
            self.data[i].rolling_qty = Some(rolling_qty);
            self.data[i].mean_qty = Some(mean_qty);
            self.data[i].qty_std = qty_std;
            self.data[i].mean_price = Some(mean_price);
            self.data[i].price_std = price_std;
            self.data[i].book_ratio_rolling_mean = Some(book_ratio_rolling_mean);
            self.data[i].rolling_qty_abs = Some(rolling_qty_abs);
        }
        //self.drop_na_without_target();
    }
    pub fn add_target_value(&mut self, tick_size: Decimal) {
        if self.data.is_empty() || self.data.len() < ROLLING_WINDOW {
            return;
        };
        for i in ROLLING_WINDOW..self.data.len() - 1 {
            let start_of_period = &self.data[i - ROLLING_WINDOW].price;
            let price_rp = self.data[i - ROLLING_WINDOW..i]
                .par_iter()
                .map(|x| x.price)
                .collect::<Vec<_>>();
            let highest_price = price_rp.par_iter().max().unwrap();
            let lowest_price = price_rp.par_iter().min().unwrap();
            let distance_to_high = (highest_price - start_of_period) / tick_size;
            let distance_to_low = (start_of_period - lowest_price) / tick_size;
            let diff = distance_to_high - distance_to_low;
            self.data[i - ROLLING_WINDOW].target = Some(diff);
        }
        self.drop_na_with_target();
    }
    pub fn drop_na_without_target(&mut self) {
        //drop all rows that have a None value
        self.data.retain(|x| x.has_rolling_features());
    }
    pub fn drop_na_with_target(&mut self) {
        //drop all rows that have a None value
        self.data
            .retain(|x| x.has_rolling_features() && x.has_target());
    }
}
