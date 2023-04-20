use std::sync::Arc;

use log::{debug, error};
use rust_decimal::Decimal;
use serde_json::Value;
use tokio::sync::Notify;

use crate::{
    binance::models::{orderbook::OrderBooksRWL, trades::Trade},
    model::data_handling::{Dfrwl, Observation},
    ROLLING_WINDOW,
};

pub async fn handle_trades(
    message: Value,
    orderbooks_rwl: OrderBooksRWL,
    notify: Arc<Notify>,
    tick_size: Decimal,
    dataframe_rwl: Dfrwl,
) {
    match serde_json::from_value::<Trade>(message) {
        Ok(trade) => {
            if let Some(book_features) = orderbooks_rwl.read().await.clone().to_features(tick_size)
            {
                let mut write = dataframe_rwl.write().await;
                let obs = Observation::from_trade_and_book(trade.to_features(), book_features);
                write.data.push(obs);
                write.calculate_rolling_features();
                //info the last observation
                if write.data.len() % (2 * ROLLING_WINDOW) == 0 {
                    notify.notify_one();
                }
            } else {
                debug!("No book features");
            }
        }
        Err(e) => {
            error!("Error parsing trade message: {:?}", e);
        }
    }
}
