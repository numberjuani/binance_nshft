use std::sync::Arc;

use log::error;
use rust_decimal::Decimal;
use serde_json::Value;
use tokio::sync::Notify;

use crate::{binance::models::{trades::Trade, orderbook::OrderBooksRWL}, ROLLING_WINDOW, model::data_handling::DFRWL};


pub async fn handle_trades(message:Value,orderbooks_rwl: OrderBooksRWL,notify: Arc<Notify>,tick_size:Decimal,dataframe_rwl:DFRWL) {
    match serde_json::from_value::<Trade>(message) {
        Ok(trade) => {
            if let Some(mut book_features) = orderbooks_rwl.read().await.clone().to_features(tick_size) {
                let mut row = Vec::new();
                row.append(&mut trade.to_features());
                row.append(&mut book_features);
                let mut write = dataframe_rwl.write().await;
                write.data.push(row);
                write.calculate_rolling_features();
                if write.data.len() > 2*ROLLING_WINDOW {
                    notify.notify_one();
                }
            }
            
        }
        Err(e) => {
            error!("Error parsing trade message: {:?}", e);
        }
    }
}