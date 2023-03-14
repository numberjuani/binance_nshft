use std::sync::Arc;

use gbdt::decision_tree::{Data, DataVec, PredVec};
use log::info;
use rust_decimal::prelude::ToPrimitive;
use tokio::sync::RwLock;

use crate::{
    binance::models::{
        fapi_exchange_info::Symbol, model_config::ModelMutex, orderbook::OrderBooksRWL,
        trades::Trade,
    },
    model::data_handling::FeatureDataFrame,
    ROLLING_WINDOW, TRAINING_INTERVAL,
};

pub async fn manage_model(
    orderbooks_rwl: OrderBooksRWL,
    trades_rwl: Arc<RwLock<Vec<Trade>>>,
    market: Symbol,
    model_mutex: ModelMutex,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(TRAINING_INTERVAL));
    interval.tick().await;
    loop {
        interval.tick().await;
        info!("Training model...");
        let trades = trades_rwl.read().await.clone();
        let orderbooks = orderbooks_rwl.read().await.clone();
        let mut features = FeatureDataFrame::new(
            trades,
            orderbooks,
            ROLLING_WINDOW,
            market.get_tick_size().unwrap(),
        )
        .unwrap();
        info!("Features shape: {:?}", features.shape());
        features.calculate_rolling_features();
        info!("With rolling Features shape: {:?}", features.shape());
        let tick_size = market.get_tick_size().unwrap().to_f32().unwrap();
        features.add_target_value(tick_size);
        info!("With target var shape: {:?}", features.shape());
        let mut train = features.data.clone();
        // the first half of the train
        let test = train.split_off(train.len() / 2);
        // convert training data into XGBoost's matrix format
        if train.is_empty() {
            info!("No training data");
            continue;
        }
        if train.last().unwrap().len() == 16 {
            let mut train_dv: DataVec = DataVec::from_iter(
                train
                    .into_iter()
                    .map(|row| Data::new_training_data(row[0..15].to_vec(), 1.0, row[15], None)),
            );
            let y_test = test.iter().map(|x| x[15]).collect::<Vec<f32>>();
            let test_dv: DataVec = DataVec::from_iter(
                test.into_iter()
                    .map(|row| Data::new_test_data(row[0..15].to_vec(), Some(row[15]))),
            );
            let mut gbdt = model_mutex.lock().await;
            info!("Fitting model...");
            gbdt.model.fit(&mut train_dv);
            gbdt.model
                .save_model("gbdt.model")
                .expect("failed to save the model");
            // load model and do inference
            let predicted: PredVec = gbdt.model.predict(&test_dv);
            let predicted: Vec<i32> = predicted
                .into_iter()
                .map(|x| ((x / tick_size).round() * tick_size) as i32)
                .collect();
            let predicted: Vec<i32> = predicted
                .into_iter()
                .map(|x| if x == -0 { 0 } else { x })
                .collect();
            let mut same_sign = 0;
            let mut exact_same = 0;
            let mut error_sum = 0.0;
            for (i, prediction) in predicted.iter().enumerate() {
                if (y_test[i] as i32 * prediction) >= 0 {
                    same_sign += 1;
                }
                if y_test[i] == *prediction as f32 {
                    exact_same += 1;
                }
                error_sum += (y_test[i] - *prediction as f32).abs();
            }
            let average_error = error_sum / predicted.len() as f32;
            //round the average error to the nearest multiple of the tick size
            let average_error = ((average_error / tick_size).round() * tick_size) as i32;
            info!("Average error: {}", average_error);
            info!(
                "Same sign %: {}",
                100.0 * (same_sign as f32 / predicted.len() as f32)
            );
            info!(
                "Exact same: {}",
                100.0 * (exact_same as f32 / predicted.len() as f32)
            );
            let real_mae = std::cmp::max(average_error, 4);
            gbdt.mae = Some(real_mae);
            features.save_to_parquet("file_name.parquet")
        }
    }
}
