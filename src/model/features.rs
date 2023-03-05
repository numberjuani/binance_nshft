use std::sync::Arc;

use log::info;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use tokio::sync::{Notify, RwLock};
use xgboost::{parameters, Booster, DMatrix};

use crate::binance::models::{
    fapi_exchange_info::Symbol,
    orderbook::{OrderBook, OrderBooksRWL},
    trades::Trade,
};
/*
feature indexes
0: trade time
1: trade price
2: trade net qty (negative for sells)
3: total bids in orderbook
4: number of ticks between best bid and size-weighted average bid price
5: total asks in orderbook
6: number of ticks between best ask and size-weighted average ask price
7: bids/asks ratio
8: price_mean
9: price_std
10: qty_mean
11: qty_std
12: mean_bid_asks_ratio
13:target
 */
pub async fn manage_model(
    enough_data_notify: Arc<Notify>,
    orderbooks_rwl: OrderBooksRWL,
    trades_rwl: Arc<RwLock<Vec<Trade>>>,
    market: Symbol,
) {
    enough_data_notify.notified().await;
    let tick_size = market.get_tick_size().unwrap();
    info!("Training model...");
    let mut trades = trades_rwl.read().await.clone();
    let mut orderbooks = orderbooks_rwl.read().await.clone();
    //info the number of bids and asks in all orderbooks
    //sort the trades by time
    trades.sort_by_key(|trade| trade.trade_time);
    //sort the orderbooks by time
    orderbooks.sort_by_key(|orderbook| orderbook.time);
    let rolling_period = 1000;
    let mut feature_vec: Vec<Vec<f32>> = Vec::new();
    let mut feature_names = vec![Trade::feature_names(), OrderBook::feature_names()].concat();
    for (i,trade) in trades.iter().enumerate() {
        let filtered_orderbooks = orderbooks
            .par_iter()
            .filter(|orderbook| orderbook.time <= trade.trade_time)
            .collect::<Vec<_>>();
        let matching_book_at_time_of_trade = filtered_orderbooks
            .par_iter()
            .cloned()
            .max_by_key(|orderbook| orderbook.time)
            .clone();
        if matching_book_at_time_of_trade.is_none() {
            println!("continue");
            continue;
        }
        let mut row = vec![
            trade.to_feature(),
            matching_book_at_time_of_trade
                .unwrap()
                .clone()
                .to_features(tick_size),
        ]
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<f32>>();
        if i > rolling_period {
            //we can calculate rolling features
            let price_mean = feature_vec[i - rolling_period..i]
                .par_iter()
                .map(|row| row[1])
                .sum::<f32>()
                / rolling_period as f32;
            let price_std = f32::sqrt(
                feature_vec[i - rolling_period..i]
                    .par_iter()
                    .map(|row| row[1])
                    .map(|price| (price - price_mean).powi(2))
                    .sum::<f32>()
                    / rolling_period as f32,
            );
            let qty_mean = feature_vec[i - rolling_period..i]
                .par_iter()
                .map(|row| row[2])
                .sum::<f32>()
                / rolling_period as f32;
            let qty_std = f32::sqrt(
                feature_vec[i - rolling_period..i]
                    .par_iter()
                    .map(|row| row[2])
                    .map(|qty| (qty - qty_mean).powi(2))
                    .sum::<f32>()
                    / rolling_period as f32,
            );
            let mean_bid_asks_ratio = feature_vec[i - rolling_period..i]
                .par_iter()
                .map(|row| row[7])
                .sum::<f32>()
                / rolling_period as f32;
            row.append(&mut vec![
                price_mean,
                price_std,
                qty_mean,
                qty_std,
                mean_bid_asks_ratio,
            ]);
            feature_names.append(&mut vec![
                "price_mean".to_string(),
                "price_std".to_string(),
                "qty_mean".to_string(),
                "qty_std".to_string(),
                "mean_bid_asks_ratio".to_string(),
            ])
        }
        feature_vec.push(row);
    }
    // now we remove the first rows because they dont have rolling features ie dropna
    let mut feature_vec = feature_vec
        .into_iter()
        .filter(|row| row.len() == 13)
        .collect::<Vec<_>>();
    // create the target, add it to the displaced row already
    for i in 0..feature_vec.len() - 1 {
        if i > rolling_period {
            let difference = feature_vec[i][8] - feature_vec[i - rolling_period][8];
            if difference > 0.0 {
                feature_vec[i - rolling_period].push(1.0);
            } else {
                feature_vec[i - rolling_period].push(0.0);
            }
        }
    }
    // now we remove the last rows again because they dont have a target
    let mut train = feature_vec
        .into_iter()
        .filter(|row| row.len() == 14)
        .collect::<Vec<_>>();
    // break into train and test
    let mut test = train.split_off(train.len() / 2);
    //begin work on model
    let mut y_train = Vec::new();
    for row in train.iter_mut() {
        y_train.push(row.pop().unwrap());
    }
    let num_rows = train.len();
    info!("Length of train: {} labels length: {}", num_rows,y_train.len());
    let train: Vec<f32> = train.into_iter().flatten().collect();
    let x_train: &[f32] = train.as_slice();
    // convert training data into XGBoost's matrix format
    let mut dtrain = DMatrix::from_dense(x_train, num_rows).unwrap();
    // set ground truth labels for the training matrix
    dtrain.set_labels(y_train.as_slice()).unwrap();
    // test matrix with 1 row
    let mut y_test: Vec<f32> = Vec::new();
    for row in test.iter_mut() {
        y_test.push(row.pop().unwrap());
    }
    let num_rows = test.len();
    let three = test.into_iter().flatten().collect::<Vec<_>>();
    let x_test = three.as_slice();
    let mut dtest = DMatrix::from_dense(x_test, num_rows).unwrap();
    dtest.set_labels(y_test.as_slice()).unwrap();
    // configure objectives, metrics, etc.
    let learning_params = parameters::learning::LearningTaskParametersBuilder::default()
        .objective(parameters::learning::Objective::BinaryLogistic)
        .build()
        .unwrap();

    // configure the tree-based learning model's parameters
    let tree_params = parameters::tree::TreeBoosterParametersBuilder::default()
        .max_depth(6)
        .eta(1.0)
        .colsample_bytree(0.8)
        .build()
        .unwrap();

    // overall configuration for Booster
    let booster_params = parameters::BoosterParametersBuilder::default()
        .booster_type(parameters::BoosterType::Tree(tree_params))
        .learning_params(learning_params)
        .build()
        .unwrap();

    // specify datasets to evaluate against during training
    let evaluation_sets = &[(&dtrain, "train"), (&dtest, "test")];

    // overall configuration for training/evaluation
    let params = parameters::TrainingParametersBuilder::default()
        .dtrain(&dtrain) // dataset to train with
        .boost_rounds(2000) // number of training iterations
        .booster_params(booster_params) // model parameters
        .evaluation_sets(Some(evaluation_sets)) // optional datasets to evaluate against in each iteration
        .build()
        .unwrap();

    // train model, and print evaluation data
    info!("Training model...");
    let bst = Booster::train(&params).unwrap();
    info!("Model trained");
    let preds = bst.predict(&dtest).unwrap();
    let preds = preds.into_iter().map(|x| x.round()).collect::<Vec<_>>();
    let mut correct = 0;
    for i in 0..preds.len() {
        if preds[i] == y_test[i] {
            correct += 1;
        }
    }
    let accuracy = correct as f32 / preds.len() as f32;
    info!("Accuracy: {}", accuracy);
}
