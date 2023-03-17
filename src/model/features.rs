use gbdt::decision_tree::{Data, DataVec, PredVec};
use log::info;
use rust_decimal::prelude::ToPrimitive;

use crate::{
    binance::models::{
        fapi_exchange_info::Symbol, model_config::ModelMutex,
    }, TRAINING_INTERVAL,
};

use super::data_handling::DFRWL;

pub async fn manage_model(
    dataframe_rwl:DFRWL,
    market: Symbol,
    model_mutex: ModelMutex,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(TRAINING_INTERVAL));
    interval.tick().await;
    loop {
        interval.tick().await;
        info!("Training model...");
        let tick_size = market.get_tick_size().unwrap().to_f32().unwrap();
        let mut features = dataframe_rwl.read().await.clone();
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
        if train.last().unwrap().len() == 19 {
            let mut train_dv: DataVec = DataVec::from_iter(
                train
                    .into_iter()
                    .map(|row| Data::new_training_data(row[0..18].to_vec(), 1.0, row[18], None)),
            );
            let y_test = test.iter().map(|x| x[18]).collect::<Vec<f32>>();
            let test_dv: DataVec = DataVec::from_iter(
                test.into_iter()
                    .map(|row| Data::new_test_data(row[0..18].to_vec(), Some(row[18]))),
            );
            let mut gbdt = model_mutex.lock().await;
            gbdt.model.conf.feature_size = 18;
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
                "Exact same: {:.2}%",
                100.0 * (exact_same as f32 / predicted.len() as f32)
            );
            let real_mae = std::cmp::max(average_error, 4);
            gbdt.mae = Some(real_mae);
            features.save_to_parquet("file_name.parquet")
        }
    }
}
