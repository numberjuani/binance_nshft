use std::sync::Arc;

use gbdt::decision_tree::{Data, DataVec, PredVec};

use log::{debug, info};
use rust_decimal::prelude::ToPrimitive;
use tokio::sync::{mpsc, Notify};

use crate::{
    binance::models::{fapi_exchange_info::Symbol, model_config::ModelMutex},
    ROLLING_WINDOW,
};

use super::data_handling::Dfrwl;

pub async fn make_predictions(
    dataframe_rwl: Dfrwl,
    market: Symbol,
    model_mutex: ModelMutex,
    notify: Arc<Notify>,
    order_send: mpsc::Sender<String>,
) {
    let mut position = 0;
    let mut entry_index = None;
    loop {
        notify.notified().await;
        let df = dataframe_rwl.read().await.clone();
        let ts_index = df.data.len();
        let tick_size = market.get_tick_size().unwrap();
        if ts_index > ROLLING_WINDOW {
            let test = df.data.last().unwrap();
            if test.has_rolling_features() {
                let test_dv: DataVec =
                    vec![Data::new_test_data(test.to_training_data().to_vec(), None)];
                let mut gbdt = model_mutex.lock().await;
                gbdt.model.conf.feature_size = 18;
                let predicted: PredVec = gbdt.model.predict(&test_dv);
                let round_pred = ((predicted.first().unwrap() / tick_size.to_f32().unwrap())
                    .round()
                    * tick_size.to_f32().unwrap()) as i32;
                let exit = ts_index >= entry_index.unwrap_or(1000000000000) + ROLLING_WINDOW;
                if exit {
                    info!("Exit price {}", test.price);
                    position = 0;
                    entry_index = None;
                }
                if round_pred >= gbdt.mae.unwrap_or(100000000) || exit {
                    if round_pred > 0 && position != 1 {
                        info!("Buy price {}", test.price);
                        order_send.send("buy!".to_string()).await.unwrap();
                        entry_index = Some(ts_index);
                        position = 1;
                    } else if round_pred < 0 && position != -1 {
                        info!("Sell price {}", test.price);
                        order_send.send("sell!".to_string()).await.unwrap();
                        entry_index = Some(ts_index);
                        position = -1;
                    }
                }
            }
        } else {
            debug!("Not enough data {}", ts_index);
        }
    }
}
