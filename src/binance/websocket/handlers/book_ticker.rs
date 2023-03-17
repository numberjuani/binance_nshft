use log::error;
use serde_json::Value;

use crate::binance::models::book_ticker::BookTicker;

pub async fn handle_book_ticker(message: Value) {
    match serde_json::from_value::<BookTicker>(message) {
        Ok(ticker) => {
            println!("{:#?}", ticker);
        }
        Err(e) => {
            error!("Error parsing message: {:?}", e);
        }
    }
}
