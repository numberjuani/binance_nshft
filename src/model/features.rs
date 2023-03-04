use std::sync::Arc;

use log::info;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use tokio::sync::{Notify, RwLock};

use crate::binance::models::{fapi_exchange_info::Symbol, orderbook::OrderBooksRWL, trades::Trade};
/*
feature indexes
0: trade time
1: trade price
2: trade net qty (negative for sells)
3: total bids in orderbook
4: number of ticks between best bid and size-weighted average bid price
5: total asks in orderbook
6: number of ticks between best ask and size-weighted average ask price
7: bids/asks ratio
8: price_mean
9: price_std
10: qty_mean
11: qty_std
12: mean_bid_asks_ratio
 */
pub async fn create_features(
    enough_data_notify: Arc<Notify>,
    orderbooks_rwl: OrderBooksRWL,
    trades_rwl: Arc<RwLock<Vec<Trade>>>,
    market: Symbol,
) {
    enough_data_notify.notified().await;
    let tick_size = market.get_tick_size().unwrap();
    info!("Got enough data to start creating features");
    let trades = trades_rwl.read().await.clone();
    let orderbooks = orderbooks_rwl.read().await.clone();
    //info the number of bids and asks in all orderbooks
    let rolling_period = 1000;
    let mut feature_vec: Vec<Vec<f64>> = Vec::new();
    for i in 0..trades.len() - 1 {
        //filter the orderbooks by those with time <= trade time
        let filtered_orderbooks = orderbooks
            .par_iter()
            .filter(|orderbook| orderbook.time <= trades[i].event_time)
            .collect::<Vec<_>>();
        let matching_book_at_time_of_trade = filtered_orderbooks
            .par_iter()
            .cloned()
            .max_by_key(|orderbook| orderbook.time)
            .unwrap()
            .clone();
        let mut row = vec![
            trades[i].to_feature(),
            matching_book_at_time_of_trade.to_features(tick_size),
        ]
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<f64>>();
        if i > rolling_period {
            //we can calculate rolling features
            let price_mean = feature_vec[i - rolling_period..i]
                .par_iter()
                .map(|row| row[1])
                .sum::<f64>()
                / rolling_period as f64;
            let price_std = f64::sqrt(
                feature_vec[i - rolling_period..i]
                    .par_iter()
                    .map(|row| row[1])
                    .map(|price| (price - price_mean).powi(2))
                    .sum::<f64>()
                    / rolling_period as f64,
            );
            let qty_mean = feature_vec[i - rolling_period..i]
                .par_iter()
                .map(|row| row[2])
                .sum::<f64>()
                / rolling_period as f64;
            let qty_std = f64::sqrt(
                feature_vec[i - rolling_period..i]
                    .par_iter()
                    .map(|row| row[2])
                    .map(|qty| (qty - qty_mean).powi(2))
                    .sum::<f64>()
                    / rolling_period as f64,
            );
            let mean_bid_asks_ratio = feature_vec[i - rolling_period..i]
                .par_iter()
                .map(|row| row[7])
                .sum::<f64>()
                / rolling_period as f64;
            row.append(&mut vec![
                price_mean,
                price_std,
                qty_mean,
                qty_std,
                mean_bid_asks_ratio,
            ]);
        }
        feature_vec.push(row);
    }
    // now we remove the first rows because they dont have rolling features ie dropna
    let mut feature_vec = feature_vec.into_iter().skip(rolling_period).collect::<Vec<_>>();
}
