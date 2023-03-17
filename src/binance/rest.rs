use super::models::fapi_exchange_info::USDMExchangeInfo;

pub async fn get_exchange_info() -> Result<USDMExchangeInfo, reqwest::Error> {
    let url = "https://fapi.binance.com/fapi/v1/exchangeInfo";
    reqwest::get(url).await?.json().await
}
