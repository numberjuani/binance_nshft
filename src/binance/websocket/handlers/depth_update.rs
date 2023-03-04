use log::{error, info};
use serde_json::Value;
use crate::binance::models::orderbook::{OrderBooksRWL, OrderbookMessage, OrderBook};

pub async fn handle_depth_update_message(message: Value, orderbooks_rwl: OrderBooksRWL) {
    match serde_json::from_value::<OrderbookMessage>(message) {
        Ok(update) => {
            let mut books_write = orderbooks_rwl.write().await;
            if books_write.is_empty() {
                let book = OrderBook::new_from_update(update.clone());
                books_write.push(book);
                return;
            }
            let latest_updated = books_write.last().unwrap().clone().update(update.clone());
            books_write.push(latest_updated.clone());
        }
        Err(e) => {
            error!("Error parsing message: {:?}", e);
        }
    }
}

