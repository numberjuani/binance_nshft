use std::sync::Arc;

use log::error;
use serde_json::Value;
use tokio::sync::{RwLock, Notify};

use crate::{binance::models::trades::Trade, MIN_TRADES_TO_START};


pub async fn handle_trades(message:Value,trade_updates_rwl:Arc<RwLock<Vec<Trade>>>,enough_data_notify: Arc<Notify>) {
    match serde_json::from_value::<Trade>(message) {
        Ok(trade) => {
            let mut write = trade_updates_rwl.write().await;
            write.push(trade);
            if write.len() == MIN_TRADES_TO_START {
                enough_data_notify.notify_one();
            }
        }
        Err(e) => {
            error!("Error parsing trade message: {:?}", e);
        }
    }
}