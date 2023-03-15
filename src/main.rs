use crate::{binance::models::{orderbook::new_orderbooks_rwl, model_config::new_model_data}, model::{features::manage_model, inference::make_predictions}};
mod binance;
use binance::websocket::connection::establish_and_persist;
use log::{info, warn, debug};
use tokio::sync::mpsc;
mod model;
mod utils;
pub const MIN_TRADES_TO_START: usize = 10000;
pub const MIN_TICKS_FOR_SIGNAL: i32 = 30;
pub const ROLLING_WINDOW: usize = 500;
pub const TRAINING_INTERVAL:u64 = 60*2;
mod log_config;

#[tokio::main]
async fn main() {
    log_config::configure_log(log::LevelFilter::Info);
    info!("Starting program");
    let symbol = "BTCUSDT";
    let exchange_info = binance::rest::get_exchange_info().await.unwrap();
    match exchange_info.symbols.into_iter().find(|market| market.symbol == symbol) {
        Some(market) => {
            debug!("Market Info found: \n{:#?}", market);
            let orderbooks_rwl = new_orderbooks_rwl();
            let trade_update_messages = std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new()));
            let (order_send, order_receive):(mpsc::Sender<String>,mpsc::Receiver<String>) = mpsc::channel(10);
            let model_mutex = new_model_data();
            let notify = std::sync::Arc::new(tokio::sync::Notify::new());
            tokio::select! {
                biased;
                _ = tokio::signal::ctrl_c() => {
                    warn!("Ctrl-C received, exiting");
                },
                _ = tokio::spawn(establish_and_persist(orderbooks_rwl.clone(), trade_update_messages.clone(),market.clone(),notify.clone())) => {
                    warn!("Websocket connection closed");
                }
                _ = tokio::spawn(make_predictions(orderbooks_rwl.clone(),trade_update_messages.clone(),market.clone(),model_mutex.clone(),notify,order_send)) => {
                    info!("Exiting prediction thread");
                }
                _ = tokio::spawn(manage_model(orderbooks_rwl.clone(),trade_update_messages,market,model_mutex.clone())) => {
                    warn!("Model thread closed");
                }
                
            }
        }
        None => {
            warn!("Market {} not found", symbol);
            return;
        }
    }
}
