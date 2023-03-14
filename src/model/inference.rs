use std::sync::Arc;

use gbdt::decision_tree::{Data, DataVec, PredVec};

use log::{info, debug};
use rust_decimal::prelude::ToPrimitive;
use tokio::sync::{Notify, mpsc, RwLock};



use crate::{binance::{models::{model_config::ModelMutex, orderbook::OrderBooksRWL, trades::Trade, fapi_exchange_info::Symbol}}, ROLLING_WINDOW};

use super::{data_handling::FeatureDataFrame};

pub async fn make_predictions(
    orderbooks_rwl: OrderBooksRWL,
    trades_rwl: Arc<RwLock<Vec<Trade>>>,
    market: Symbol,
    model_mutex: ModelMutex,
    notify: Arc<Notify>,
    order_send: mpsc::Sender<String>,
) {
    let mut position = 0;
    let mut entry_index = None;
    loop {
        notify.notified().await;
        let ts = trades_rwl.read().await.clone();
        let ts_index = ts.len();
        let ob = orderbooks_rwl.read().await.clone();
        let tick_size = market.get_tick_size().unwrap();
        if ts.len() >= ROLLING_WINDOW+1 && ob.len() >= ROLLING_WINDOW+1 {
            let mut features = FeatureDataFrame::new(
                ts,
                ob,
                ROLLING_WINDOW,
                tick_size,
            )
            .unwrap();
            features.calculate_rolling_features();
            let test = features.data.last().unwrap();
            if test.len() == 15 {
                let test_dv: DataVec = vec![Data::new_test_data(
                    test[0..15].to_vec(),
                    None,
                )];
                let gbdt = model_mutex.lock().await;
                let predicted: PredVec = gbdt.model.predict(&test_dv);
                let round_pred = ((predicted.first().unwrap() / tick_size.to_f32().unwrap())
                    .round()
                    * tick_size.to_f32().unwrap()) as i32;
                let exit = ts_index >= entry_index.unwrap_or(1000000000000) + ROLLING_WINDOW;
                if exit {
                    info!("Exit price {}", test[1]);
                    position = 0;
                    entry_index = None;
                }
                if round_pred >= gbdt.mae.unwrap_or(100000000) || exit {
                    if round_pred > 0 && position != 1 {
                        info!("Buy price {}", test[1]);
                        order_send.send("buy!".to_string()).await.unwrap();
                        entry_index = Some(ts_index);
                        position = 1;
                    } else if round_pred < 0 && position != -1 {
                        info!("Sell price {}",test[1]);
                        order_send.send("sell!".to_string()).await.unwrap();
                        entry_index = Some(ts_index);
                        position = -1;
                    }
                }
            } else {
                debug!("Not enough data < 15 {}", test.len());
            }
            
        } else {
            debug!("Not enough data");
        }
    }
}
