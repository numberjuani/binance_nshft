use crate::{
    binance::{
        models::{fapi_exchange_info::Symbol, orderbook::OrderBooksRWL},
        websocket::handlers::book_ticker::handle_book_ticker,
    },
    model::data_handling::Dfrwl,
};
use chrono::Duration;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::{debug, error, warn};
use rust_decimal::Decimal;
use serde_json::{Map, Value};
use std::sync::Arc;

use tokio::{net::TcpStream, sync::Notify};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use super::{
    handlers::{depth_update::handle_depth_update_message, trades::handle_trades},
    requests::{BinanceAssetType, DataRequest, FuturesType, Stream},
};

type OutgoingSocket = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type IncomingSocket = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
/// Establishes a websocket connection to Binance and persists it for the duration of the program.
/// If disconnected, it will attempt to reconnect uo to 5 times at an ever-increasing interval up to 26 seconds.
/// It will try a max of 5 times before exiting the program.
pub async fn establish_and_persist(
    orderbooks_rwl: OrderBooksRWL,
    market: Symbol,
    notify: Arc<Notify>,
    dataframe_rwl: Dfrwl,
) {
    let mut bad_attempts = 0;
    loop {
        if establish(
            orderbooks_rwl.clone(),
            market.clone(),
            notify.clone(),
            dataframe_rwl.clone(),
        )
        .await
        {
            bad_attempts += 1;
        } else {
            bad_attempts = 0;
        }
        if bad_attempts >= 5 {
            warn!("Too many failed attempts to connect to websocket, exiting");
            return;
        }
        tokio::time::sleep(Duration::seconds(bad_attempts * 5 + 1).to_std().unwrap()).await;
    }
}
/// Establishes a single websocket connection to Binance. Returns true if there was an error.
async fn establish(
    orderbooks_rwl: OrderBooksRWL,
    market: Symbol,
    notify: Arc<Notify>,
    dataframe_rwl: Dfrwl,
) -> bool {
    let request = DataRequest::new(
        BinanceAssetType::Futures(FuturesType::USDMargined),
        vec![
            Stream::Trade(market.symbol.to_string()),
            Stream::Depth(market.symbol.to_string(), 100),
        ],
    );
    for endpoint in request.get_ws_urls().iter() {
        debug!("Attempting WS connection to {}", endpoint);
        match tokio_tungstenite::connect_async(endpoint).await {
            Ok((stream, response)) => {
                debug!("Connected to {endpoint} status: {}", response.status());
                let (sender, receiver) = stream.split();
                let ping_pong = Arc::new(Notify::new());
                let tick_size = market.get_tick_size().unwrap();
                tokio::select! {
                    _= tokio::spawn(process_incoming_message(receiver, ping_pong.clone(),orderbooks_rwl.clone(),notify,tick_size,dataframe_rwl.clone())) => {
                        error!("Incoming message processing failed");
                        return true;
                    }
                    _= tokio::spawn(process_outgoing_message(sender, ping_pong.clone(),request.clone())) => {
                        error!("Outgoing message processing failed");
                        return true;
                    }
                }
            }
            Err(e) => {
                error!("{:?}", e);
                continue;
            }
        }
    }
    true
}

async fn process_incoming_message(
    mut receiver: IncomingSocket,
    ping_pong: Arc<Notify>,
    orderbooks_rwl: OrderBooksRWL,
    notify: Arc<Notify>,
    tick_size: Decimal,
    dataframe_rwl: Dfrwl,
) {
    while let Some(result) = receiver.next().await {
        match result {
            Ok(message) => {
                process_message(
                    message,
                    ping_pong.clone(),
                    orderbooks_rwl.clone(),
                    notify.clone(),
                    tick_size,
                    dataframe_rwl.clone(),
                )
                .await;
            }
            Err(e) => {
                warn!("Error receiving message: {:?}", e);
            }
        }
    }
}

async fn process_message(
    message: Message,
    ping_pong: Arc<Notify>,
    orderbooks_rwl: OrderBooksRWL,
    notify: Arc<Notify>,
    tick_size: Decimal,
    dataframe_rwl: Dfrwl,
) {
    match message {
        Message::Text(text_message) => {
            //debug!("Received message: {}", text_message);
            match serde_json::from_str::<Map<String, Value>>(&text_message) {
                Ok(unrouted_message) => match unrouted_message.contains_key("data") {
                    true => match unrouted_message["data"]["e"].as_str().unwrap() {
                        "depthUpdate" => {
                            handle_depth_update_message(
                                unrouted_message["data"].clone(),
                                orderbooks_rwl.clone(),
                            )
                            .await;
                        }
                        "trade" => {
                            handle_trades(
                                unrouted_message["data"].clone(),
                                orderbooks_rwl.clone(),
                                notify.clone(),
                                tick_size,
                                dataframe_rwl.clone(),
                            )
                            .await;
                        }
                        "bookTicker" => {
                            handle_book_ticker(unrouted_message["data"].clone()).await;
                        }
                        _ => {
                            debug!("Unrecognized message: {:?}", unrouted_message);
                        }
                    },
                    false => {
                        if unrouted_message.keys().len() == 2
                            && unrouted_message.contains_key("result")
                            && unrouted_message["result"].is_null()
                        {
                            debug!(
                                "Successfully subscribed to request id {}",
                                unrouted_message["id"]
                            );
                        } else {
                            warn!("Unrecognized message: {:?}", unrouted_message);
                        }
                    }
                },
                Err(e) => {
                    error!("Error parsing message: {:?}", e);
                }
            }
        }
        Message::Binary(_) => {
            warn!("Binary message received");
        }
        Message::Ping(_) => {
            debug!("Received ping");
            ping_pong.notify_one();
        }
        Message::Pong(_) => {}
        Message::Close(cf) => {
            warn!("Close received {cf:?}");
        }
        Message::Frame(_) => {
            warn!("Frame received");
        }
    }
}

async fn process_outgoing_message(
    mut sender: OutgoingSocket,
    ping_pong: Arc<tokio::sync::Notify>,
    request: DataRequest,
) {
    let sub_message = request.get_subscribe_message();
    match sender.send(Message::Text(sub_message.clone())).await {
        Ok(_) => {
            debug!("Sent message {}", sub_message.clone());
        }
        Err(e) => {
            error!("Error {:?} sending {}", e, sub_message);
        }
    }
    loop {
        ping_pong.notified().await;
        match sender.send(Message::Pong(vec![])).await {
            Ok(_) => {
                debug!("Sent pong");
            }
            Err(e) => {
                error!("{:?}", e);
            }
        }
    }
}
