use std::sync::Arc;

use log::error;
use serde_json::Value;
use tokio::sync::{RwLock};

use crate::{binance::models::trades::Trade};


pub async fn handle_trades(message:Value,trade_updates_rwl:Arc<RwLock<Vec<Trade>>>) {
    match serde_json::from_value::<Trade>(message) {
        Ok(trade) => {
            let mut write = trade_updates_rwl.write().await;
            write.push(trade);
        }
        Err(e) => {
            error!("Error parsing trade message: {:?}", e);
        }
    }
}