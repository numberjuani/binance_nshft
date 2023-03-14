
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
13: total_volume
14:target
 */


use rayon::{slice::ParallelSliceMut, prelude::{IntoParallelRefIterator, ParallelIterator}};
use rust_decimal::Decimal;
use polars::prelude::*;

use crate::binance::models::{trades::Trade, orderbook::OrderBook};

#[derive(Debug)]
pub struct FeatureDataFrame {
    pub data: Vec<Vec<f32>>,
    pub rolling_window:usize
}
impl FeatureDataFrame {
    pub fn new(mut trades: Vec<Trade>, mut orderbooks: Vec<OrderBook>,rolling_window:usize,tick_size:Decimal) -> Option<Self> {
        //sort time and sales by f32
        trades.par_sort_unstable_by_key(|t| t.event_time);
        orderbooks.par_sort_unstable_by_key(|t| t.time);
        let mut train: Vec<Vec<f32>> = Vec::new();
        for trade in trades {
            //keep only the orderbooks with timestamp <= tas timestamp
            let valid_books = orderbooks
                .par_iter()
                .filter(|b| b.time <= trade.event_time)
                .collect::<Vec<_>>();
            //get the book with the closest timestamp
            let closest_book = valid_books.par_iter().max_by_key(|b| b.time);
            if closest_book.is_none() {
                continue;
            }
            let mut row = Vec::new();
            row.append(&mut trade.to_features());
            row.append(&mut closest_book.unwrap().clone().clone().to_features(tick_size));
            train.push(row);
        }
        Some(Self { data: train  ,rolling_window})
    }
    pub fn calculate_rolling_features(&mut self) {
        if self.data.is_empty() || self.data.len() < self.rolling_window {
            return;
        };
        for i in self.rolling_window..self.data.len() - 1 {
            let net_qty_rolling = self.data[i - self.rolling_window..i]
                .par_iter()
                .map(|x| x[2])
                .collect::<Vec<_>>();
            let rolling_qty = net_qty_rolling.par_iter().sum::<f32>();
            let mean_qty = rolling_qty / self.rolling_window as f32;
            let qty_std = f32::sqrt(
                net_qty_rolling
                    .par_iter()
                    .map(|x| (x - mean_qty).powi(2))
                    .sum::<f32>()
                    / self.rolling_window as f32,
            );
            let last_price_rolling = self.data[i - self.rolling_window..i]
                .par_iter()
                .map(|x| x[1])
                .collect::<Vec<_>>();
            let mean_price = last_price_rolling.par_iter().sum::<f32>() / self.rolling_window as f32;
            let price_std = f32::sqrt(
                last_price_rolling
                    .par_iter()
                    .map(|x| (x - mean_price).powi(2))
                    .sum::<f32>()
                    / self.rolling_window as f32,
            );
            let book_ratio_rolling_mean = self.data[i - self.rolling_window..i]
                .par_iter()
                .map(|x| x[7])
                .collect::<Vec<_>>()
                .par_iter()
                .sum::<f32>()
                / self.rolling_window as f32;
            let rolling_qty_abs = self.data[i - self.rolling_window..i]
                .par_iter()
                .map(|x| x[2].abs())
                .collect::<Vec<_>>()
                .par_iter()
                .sum::<f32>();
            self.data[i].append(&mut vec![
                rolling_qty,
                mean_qty,
                qty_std,
                mean_price,
                price_std,
                book_ratio_rolling_mean,
                rolling_qty_abs,
            ]);
        }
        self.drop_na();
    }
    pub fn add_target_value(&mut self, tick_size: f32) {
        if self.data.is_empty() || self.data.len() < self.rolling_window {
            return;
        };
        for i in self.rolling_window..self.data.len() - 1 {
            let start_of_period = &self.data[i - self.rolling_window][1];
            let price_rp = self.data[i - self.rolling_window..i]
                .par_iter()
                .map(|x| x[1])
                .collect::<Vec<_>>();
            let highest_price = price_rp
                .par_iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            let lowest_price = price_rp
                .par_iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            let distance_to_high = (highest_price - start_of_period) / tick_size;
            let distance_to_low = (start_of_period - lowest_price) / tick_size;
            let diff = distance_to_high - distance_to_low;
            self.data[i - self.rolling_window].push(diff);
        }
        self.drop_na();
    }
    pub fn drop_na(&mut self) {
        //get the max length of the rows
        let max_len = self
            .data
            .par_iter()
            .map(|x| x.len())
            .max()
            .unwrap();
        //drop the rows that are not the max length
        self.data.retain(|x| x.len() == max_len);
    }
    pub fn shape(&self) -> (usize, usize) {
        let rows = self.data.len();
        let cols = self.data.first().unwrap_or(&Vec::new()).len();
        (rows, cols)
    }
    pub fn save_to_parquet(self, file_name: &str) {
        let mut dataframe = DataFrame::empty();
        for (index,_) in self.data[0].iter().enumerate() {
            let series = Series::new(&format!("col_{index}"), self.data.par_iter().map(|x| x[index]).collect::<Vec<_>>());
            dataframe.with_column(series).unwrap();
        }
        let mut file = std::fs::File::create(file_name).unwrap();
        ParquetWriter::new(&mut file)
            .with_compression(ParquetCompression::Snappy)
            .finish(&mut dataframe)
            .unwrap();
    }
    // pub fn read_from_parquet(file_name: &str,rolling_window:usize) -> Result<Self, Box<dyn std::error::Error>> {     
    //     let mut file = std::fs::File::open(file_name)?;
    //     let df = ParquetReader::new(&mut file).finish()?;
    //     let mut out = Vec::new();
    //     for row in 0..df.height() {
    //         let row = df.get_row(row).unwrap();
    //         out.push(row.0.into_iter().map(|x| x.try_extract().unwrap()).collect::<Vec<f32>>());
    //     }
    //     Ok(Self { data: out ,rolling_window})
    // }
}

