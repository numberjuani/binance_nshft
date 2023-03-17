/*
0: timestamp
1: price
2: net qty
3: notional
4: bid_total.
5: num_ticks_from_best_bid
6: ask_total
7: num_ticks_from_best_ask
8: bids_asks_ratio
9: bid_notional
10: ask_notional
rolling
11: rolling_qty,
12: mean_qty,
13: qty_std,
14: mean_price,
15: price_std,
16: book_ratio_rolling_mean,
17 : rolling_qty_abs,
18: target
 */

use polars::prelude::*;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::sync::Arc;
use tokio::sync::RwLock;
pub type Dfrwl = Arc<RwLock<FeatureDataFrame>>;

pub fn new_dataframe_rwl() -> Dfrwl {
    let df = FeatureDataFrame::new_empty();
    Arc::new(RwLock::new(df))
}

#[derive(Debug, Clone)]
pub struct FeatureDataFrame {
    pub data: Vec<Vec<f32>>,
    pub rolling_window: usize,
}
impl FeatureDataFrame {
    pub fn new_empty() -> Self {
        Self {
            data: Vec::with_capacity(10000000),
            rolling_window: 0,
        }
    }
    pub fn calculate_rolling_features(&mut self) {
        if self.data.is_empty() || self.data.len() < self.rolling_window {
            return;
        };
        for i in self.data.len() - 1..self.rolling_window {
            if self.data[i].len() >= 11 {
                continue;
            }
            let net_qty_rolling = self.data[i - self.rolling_window..i]
                .par_iter()
                .map(|x| x[3])
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
            let mean_price =
                last_price_rolling.par_iter().sum::<f32>() / self.rolling_window as f32;
            let price_std = f32::sqrt(
                last_price_rolling
                    .par_iter()
                    .map(|x| (x - mean_price).powi(2))
                    .sum::<f32>()
                    / self.rolling_window as f32,
            );
            let book_ratio_rolling_mean = self.data[i - self.rolling_window..i]
                .par_iter()
                .map(|x| x[8])
                .collect::<Vec<_>>()
                .par_iter()
                .sum::<f32>()
                / self.rolling_window as f32;
            let rolling_qty_abs = self.data[i - self.rolling_window..i]
                .par_iter()
                .map(|x| x[3].abs())
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
        let max_len = self.data.par_iter().map(|x| x.len()).max().unwrap();
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
        for (index, _) in self.data[0].iter().enumerate() {
            let series = Series::new(
                &format!("col_{index}"),
                self.data.par_iter().map(|x| x[index]).collect::<Vec<_>>(),
            );
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
