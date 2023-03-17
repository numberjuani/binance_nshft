use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::fmt::Display;
use std::fmt::Formatter;

use crate::binance::constants::COIN_M_BASE_WS_ENDPOINT;

use crate::binance::constants::OPTIONS_BASE_WS_ENDPOINT;

use crate::binance::constants::Symbol;
use crate::binance::constants::SPOT_BASE_WS_ENDPOINTS;

use crate::binance::constants::USDT_M_BASE_WS_ENDPOINTS;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FuturesType {
    USDMargined,
    CoinMargined,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BinanceAssetType {
    Spot,
    Futures(FuturesType),
    Options,
}
impl BinanceAssetType {
    pub fn get_ws_base_url_list(&self) -> Vec<String> {
        match self {
            BinanceAssetType::Spot => {
                return SPOT_BASE_WS_ENDPOINTS
                    .iter()
                    .map(|endpoint| endpoint.to_string())
                    .collect();
            }
            BinanceAssetType::Futures(futures_type) => match futures_type {
                FuturesType::USDMargined => {
                    return USDT_M_BASE_WS_ENDPOINTS
                        .iter()
                        .map(|endpoint| endpoint.to_string())
                        .collect();
                }
                FuturesType::CoinMargined => {
                    return COIN_M_BASE_WS_ENDPOINT
                        .iter()
                        .map(|endpoint| endpoint.to_string())
                        .collect();
                }
            },
            BinanceAssetType::Options => {
                return OPTIONS_BASE_WS_ENDPOINT
                    .iter()
                    .map(|endpoint| endpoint.to_string())
                    .collect();
            }
        }
    }
    // pub fn get_http_base_url_list(&self) -> Vec<String> {
    //     match self {
    //         BinanceAssetType::Spot => {
    //             return SPOT_BASE_HTTP_ENDPOINTS.iter().map(|endpoint| endpoint.to_string()).collect();
    //         }
    //         BinanceAssetType::Futures(futures_type) => {
    //             match futures_type {
    //                 FuturesType::USDMargined => {
    //                     return USDT_M_BASE_HTTP_ENDPOINT.iter().map(|endpoint| endpoint.to_string()).collect();
    //                 }
    //                 FuturesType::CoinMargined => {
    //                     return COIN_M_BASE_HTTP_ENDPOINT.iter().map(|endpoint| endpoint.to_string()).collect();
    //                 }
    //             }
    //         }
    //         BinanceAssetType::Options => {
    //             return OPTIONS_BASE_HTTP_ENDPOINT.iter().map(|endpoint| endpoint.to_string()).collect();
    //         }
    //     }
    // }
}
impl Display for BinanceAssetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BinanceAssetType::Spot => {
                write!(f, "SPOT")
            }
            BinanceAssetType::Futures(futures_type) => match futures_type {
                FuturesType::USDMargined => {
                    write!(f, "USDM_FUT")
                }
                FuturesType::CoinMargined => {
                    write!(f, "COINM_FUT")
                }
            },
            BinanceAssetType::Options => {
                write!(f, "OPTIONS")
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Stream {
    Depth(Symbol, i32),
    Trade(Symbol),
    BookTicker(Symbol),
}
// impl Stream {
//     pub fn get_symbol(&self) -> Symbol {
//         match self {
//             Stream::Depth(symbol,_) => {
//                 symbol.clone()
//             }
//             Stream::Trade(symbol) => {
//                 symbol.clone()
//             }
//             Stream::BookTicker(symbol) => {
//                 symbol.clone()
//             }
//         }
//     }
// }
impl Display for Stream {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Stream::Depth(symbol, depth) => {
                write!(f, "{}@depth@{}ms", symbol.to_lowercase(), depth)
            }
            Stream::Trade(symbol) => write!(f, "{}@trade", symbol.to_lowercase()),
            Stream::BookTicker(symbol) => write!(f, "{}@bookTicker", symbol.to_lowercase()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataRequest {
    pub asset_type: BinanceAssetType,
    pub streams: Vec<Stream>,
}
impl DataRequest {
    pub fn new(asset_type: BinanceAssetType, streams: Vec<Stream>) -> Self {
        Self {
            asset_type,
            streams,
        }
    }
    pub fn get_ws_urls(&self) -> Vec<String> {
        let individual_streams = self
            .streams
            .iter()
            .map(|stream| stream.to_string())
            .collect::<Vec<String>>();
        let combined_streams = individual_streams.join("/");
        let path = format!("/stream?streams={}", combined_streams);
        return self
            .asset_type
            .get_ws_base_url_list()
            .iter()
            .map(|base_url| {
                url::Url::parse(&format!("{}{}", base_url, path))
                    .unwrap()
                    .to_string()
            })
            .collect();
    }
    pub fn get_subscribe_message(&self) -> String {
        let individual_streams = self
            .streams
            .iter()
            .map(|stream| stream.to_string())
            .collect::<Vec<String>>();
        json!({"method":"SUBSCRIBE","params":individual_streams,"id":1}).to_string()
    }
    // pub fn get_http_base_url_list(&self) -> Vec<String> {
    //     self.asset_type.get_http_base_url_list()
    // }
    // pub fn get_orderbook_endpoint(&self) -> Vec<String> {
    //     let path = match self.asset_type {
    //         BinanceAssetType::Spot => "/api/v3/depth",
    //         BinanceAssetType::Futures(_) => {
    //             match FuturesType::USDMargined {
    //                 FuturesType::USDMargined => "/fapi/v1/depth",
    //                 FuturesType::CoinMargined => "/dapi/v1/depth",
    //             }
    //         }
    //         BinanceAssetType::Options => "/eapi/v1/depth",
    //     };
    //     return self.get_http_base_url_list().iter().map(|base_url| url::Url::parse(&format!("{}{}",base_url,path)).unwrap().to_string()).collect();
    // }
}
