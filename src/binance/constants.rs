#![allow(dead_code)]
pub const SPOT_BASE_WS_ENDPOINTS: [&str; 2] = ["wss://stream.binance.com:9443","wss://stream.binance.com:443"];
pub const SPOT_BASE_HTTP_ENDPOINTS: [&str; 4] = ["https://api.binance.com","https://api1.binance.com","https://api2.binance.com","https://api3.binance.com"];
pub const USDT_M_BASE_HTTP_ENDPOINT:[&str;1] = ["https://fapi.binance.com"];
pub const USDT_M_BASE_WS_ENDPOINTS: [&str;2] = [" wss://fstream.binance.com","wss://fstream-auth.binance.com"];
pub const COIN_M_BASE_HTTP_ENDPOINT:[&str;1] = ["https://dapi.binance.com"];
pub const COIN_M_BASE_WS_ENDPOINT:[&str;1] = ["wss://dstream.binance.com"];
pub const OPTIONS_BASE_WS_ENDPOINT:[&str;1] = ["wss://nbstream.binance.com/eoptions/"];
pub const OPTIONS_BASE_HTTP_ENDPOINT:[&str;1] = ["https://eapi.binance.com"];
pub type Symbol = String;