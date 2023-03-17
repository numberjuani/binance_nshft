use crate::{binance::models::{orderbook::new_orderbooks_rwl, model_config::new_model_data}, model::{features::manage_model, inference::make_predictions, data_handling::new_dataframe_rwl}};
mod binance;
use binance::websocket::connection::establish_and_persist;
use log::{info, warn, debug};
use tokio::sync::mpsc;
mod model;
mod utils;
//pub const MIN_TRADES_TO_START: usize = 10000;
pub const MIN_TICKS_FOR_SIGNAL: i32 = 30;
pub const ROLLING_WINDOW: usize = 1000;
pub const TRAINING_INTERVAL:u64 = 60*10;
mod log_config;

#[tokio::main]
async fn main() {
    log_config::configure_log(log::LevelFilter::Debug);
    info!("Starting program");
    let symbol = "BTCUSDT";
    let exchange_info = binance::rest::get_exchange_info().await.unwrap();
    let dataframe_rwl = new_dataframe_rwl();
    match exchange_info.symbols.into_iter().find(|market| market.symbol == symbol) {
        Some(market) => {
            //sleep for 10 seconds
            debug!("Market Info found: \n{:#?}", market);
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let orderbooks_rwl = new_orderbooks_rwl();
            let (order_send, _order_receive):(mpsc::Sender<String>,mpsc::Receiver<String>) = mpsc::channel(10);
            let model_mutex = new_model_data();
            let notify = std::sync::Arc::new(tokio::sync::Notify::new());
            tokio::select! {
                biased;
                _ = tokio::signal::ctrl_c() => {
                    warn!("Ctrl-C received, exiting");
                },
                _ = tokio::spawn(establish_and_persist(orderbooks_rwl.clone(),market.clone(),notify.clone(),dataframe_rwl.clone())) => {
                    warn!("Websocket connection closed");
                }
                _ = tokio::spawn(make_predictions(dataframe_rwl.clone(),market.clone(),model_mutex.clone(),notify,order_send)) => {
                    info!("Exiting prediction thread");
                }
                _ = tokio::spawn(manage_model(dataframe_rwl.clone(),market,model_mutex.clone())) => {
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
