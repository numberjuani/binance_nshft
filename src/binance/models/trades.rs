use chrono::DateTime;
use chrono::Utc;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;
use serde_with::{serde_as, TimestampMilliSeconds};
#[serde_as]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Trade {
    #[serde(rename(deserialize = "e"))]
    pub event_type: String,
    #[serde(rename(deserialize = "E"))]
    #[serde_as(as = "TimestampMilliSeconds")]
    pub event_time: DateTime<Utc>,
    #[serde(rename(deserialize = "T"))]
    #[serde_as(as = "TimestampMilliSeconds")]
    pub trade_time: DateTime<Utc>,
    #[serde(rename(deserialize = "s"))]
    pub symbol: String,
    #[serde(rename(deserialize = "t"))]
    pub trade_id: i64,
    #[serde(rename(deserialize = "p"))]
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(rename(deserialize = "q"))]
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    #[serde(rename(deserialize = "X"))]
    pub x: Option<String>,
    #[serde(rename(deserialize = "m"))]
    pub buyer_is_the_market_maker: bool,
}
impl Trade {
    pub fn to_feature(&self) -> Vec<f64> {
        let net_qty = if self.buyer_is_the_market_maker {
            -self.quantity.to_f64().unwrap()
        } else {
            self.quantity.to_f64().unwrap()
        };
        vec![
            self.trade_time.timestamp_millis() as f64,
            self.price.to_f64().unwrap(),
            net_qty,
        ]
    }
}
