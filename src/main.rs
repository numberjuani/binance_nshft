use crate::{binance::models::orderbook::new_orderbooks_rwl, model::features::create_features};
mod binance;
use binance::websocket::connection::establish_and_persist;
use log::{info, warn};
mod model;
mod utils;
pub const MIN_TRADES_TO_START: usize = 10000;


#[tokio::main]
async fn main() {
    log4rs::init_file("log_config.yaml", Default::default()).unwrap();
    info!("Starting program");
    let symbol = "BTCUSDT";
    let markets_info = binance::rest::get_markets_info().await.unwrap().symbols;
    match markets_info.into_iter().find(|market| market.symbol == symbol) {
        Some(market) => {
            info!("Market Info found: \n{:#?}", market);
            let orderbooks_rwl = new_orderbooks_rwl();
            let trade_update_messages = std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new()));
            // this will notify the model thread that more than MIN_TRADES_TO_START ticks are stored.
            let enough_data_notify = std::sync::Arc::new(tokio::sync::Notify::new());
            tokio::select! {
                _ = tokio::spawn(establish_and_persist(orderbooks_rwl.clone(), trade_update_messages.clone(),enough_data_notify.clone(),market.clone())) => {
                    warn!("Websocket connection closed");
                }
                _ = tokio::spawn(create_features(enough_data_notify.clone(),orderbooks_rwl.clone(),trade_update_messages,market)) => {
                    warn!("Model thread closed");
                }
                _ = tokio::signal::ctrl_c() => {
                    warn!("Ctrl-C received, exiting");
                }
            }
        }
        None => {
            warn!("Market {} not found", symbol);
            return;
        }
    }
}
