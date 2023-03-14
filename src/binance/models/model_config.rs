use std::sync::Arc;

use gbdt::{config::Config, gradient_boost::GBDT};
use log::info;

use crate::MIN_TICKS_FOR_SIGNAL;


pub type ModelMutex = Arc<tokio::sync::Mutex<ModelData>>;

pub fn new_model_data() -> ModelMutex {
    match GBDT::load_model("gbdt.model") {
        Ok(model) => {
            info!("Loaded model");
            return Arc::new(tokio::sync::Mutex::new(ModelData::new(model)));
        }
        Err(_) => {
            info!("No model found, creating new model");
            let mut cfg = Config::new();
            cfg.set_feature_size(15);
            cfg.set_max_depth(6);
            cfg.set_iterations(500);
            cfg.set_shrinkage(0.1);
            cfg.set_loss("SquaredError");
            cfg.set_debug(false);
            cfg.set_data_sample_ratio(1.0);
            cfg.set_feature_sample_ratio(1.0);
            cfg.set_training_optimization_level(2);
            Arc::new(tokio::sync::Mutex::new(ModelData::new(GBDT::new(&cfg))))
        }
    } 
}



#[derive(Clone)]
pub struct ModelData {
    pub model:GBDT,
    pub mae:Option<i32>,
}
impl ModelData {
    pub fn new(model:GBDT) -> Self {
        Self {
            model,
            mae:Some(MIN_TICKS_FOR_SIGNAL),
        }
    }
}