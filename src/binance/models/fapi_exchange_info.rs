use std::str::FromStr;

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct USDMExchangeInfo {
    pub assets: Vec<Asset>,
    pub exchange_filters: Vec<Value>,
    pub futures_type: String,
    pub rate_limits: Vec<RateLimit>,
    pub server_time: i64,
    pub symbols: Vec<Symbol>,
    pub timezone: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Asset {
    pub asset: String,
    pub auto_asset_exchange: String,
    pub margin_available: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct RateLimit {
    pub interval: String,
    pub interval_num: i64,
    pub limit: i64,
    pub rate_limit_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Symbol {
    pub base_asset: String,
    pub base_asset_precision: i64,
    pub contract_type: String,
    pub delivery_date: i64,
    pub filters: Vec<Filter>,
    pub liquidation_fee: String,
    pub maint_margin_percent: String,
    pub margin_asset: String,
    pub market_take_bound: String,
    pub onboard_date: i64,
    pub order_types: Vec<String>,
    pub pair: String,
    pub price_precision: i64,
    pub quantity_precision: i64,
    pub quote_asset: String,
    pub quote_precision: i64,
    pub required_margin_percent: String,
    pub settle_plan: i64,
    pub status: String,
    pub symbol: String,
    pub time_in_force: Vec<String>,
    pub trigger_protect: String,
    pub underlying_sub_type: Vec<String>,
    pub underlying_type: String,
}
impl Symbol  {
    pub fn get_tick_size(&self) -> Option<Decimal> {
        match self.filters.par_iter().find_first(|f|f.filter_type == "PRICE_FILTER") {
            Some(f) => Some(Decimal::from_str(&f.tick_size.as_ref().unwrap()).unwrap()),
            None => None
        }
    }
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Filter {
    pub filter_type: String,
    pub max_price: Option<String>,
    pub min_price: Option<String>,
    pub tick_size: Option<String>,
    pub max_qty: Option<String>,
    pub min_qty: Option<String>,
    pub step_size: Option<String>,
    pub limit: Option<i64>,
    pub notional: Option<String>,
    pub multiplier_decimal: Option<String>,
    pub multiplier_down: Option<String>,
    pub multiplier_up: Option<String>,
}
