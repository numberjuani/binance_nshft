use chrono::DateTime;
use chrono::Utc;
use std::borrow::Cow;

use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;
use serde_with::{serde_as, TimestampMilliSeconds};

#[serde_as]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Trade<'a> {
    #[serde(rename = "e")]
    pub event_type: Cow<'a, str>,
    #[serde(rename = "E")]
    #[serde_as(as = "TimestampMilliSeconds")]
    pub event_time: DateTime<Utc>,
    #[serde(rename = "T")]
    #[serde_as(as = "TimestampMilliSeconds")]
    pub trade_time: DateTime<Utc>,
    #[serde(rename = "s")]
    pub symbol: Cow<'a, str>,
    #[serde(rename = "t")]
    pub trade_id: i64,
    #[serde(rename = "p")]
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(rename = "q")]
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    #[serde(rename = "X")]
    pub x: Option<Cow<'a, str>>,
    #[serde(rename = "m")]
    pub buyer_is_the_market_maker: bool,
}

impl<'a> Trade<'a> {
    pub fn to_features(&self) -> TradeFeatures {
        TradeFeatures {
            timestamp: self.trade_time.timestamp_millis(),
            price: self.price,
            net_qty: if self.buyer_is_the_market_maker {
                -self.quantity
            } else {
                self.quantity
            },
            notional: self.price * self.quantity,
        }
    }
    // pub fn miliseconds_since_event(&self) -> i64 {
    //     Utc::now().timestamp_millis() - self.event_time.timestamp_millis()
    // }
}

/*
0: timestamp
1: price
2: net qty
3: notional
 */
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeFeatures {
    pub timestamp: i64,
    pub price: Decimal,
    pub net_qty: Decimal,
    pub notional: Decimal,
}
