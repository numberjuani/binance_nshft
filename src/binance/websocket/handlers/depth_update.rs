use crate::binance::models::orderbook::{OrderBook, OrderBooksRWL, OrderbookMessage};
use log::error;
use serde_json::Value;

pub async fn handle_depth_update_message(message: Value, orderbooks_rwl: OrderBooksRWL) {
    match serde_json::from_value::<OrderbookMessage>(message) {
        Ok(update) => {
            let mut book = orderbooks_rwl.write().await;
            if book.is_empty() {
                *book = OrderBook::new_from_update(update);
            } else {
                book.update(update);
            }
        }
        Err(e) => {
            error!("Error parsing message: {:?}", e);
        }
    }
}
