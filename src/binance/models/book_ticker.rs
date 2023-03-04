use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookTicker {
    #[serde(rename = "u")]
    pub orderbook_update_id: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    #[serde(with = "rust_decimal::serde::str")]
    pub bid: Decimal,
    #[serde(rename = "B")]
    #[serde(with = "rust_decimal::serde::str")]
    pub bid_size: Decimal,
    #[serde(rename = "a")]
    #[serde(with = "rust_decimal::serde::str")]
    pub ask: Decimal,
    #[serde(rename = "A")]
    #[serde(with = "rust_decimal::serde::str")]
    pub ask_size: Decimal,
}