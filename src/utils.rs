use async_std::task;
use chrono::{NaiveDateTime, Utc};
use futures::TryStreamExt;
use log::{info, warn};
use mongodb::{
    bson::{self, doc, Document},
    options::FindOptions,
};
use serde_json::json;
use trade_utils::{
    clients::mongo_client::MongoClient,
    types::{kline::Kline, trade::Trade},
};

use crate::consts::{KLINE_DB, LOCAL_MONGO_CONNECTION_STRING};

pub const LOG_DB: &str = "momentum_logs";
pub const LOG_COLLECTION: &str = "trades";

pub fn get_klines_from_db(from_str: &str, to_str: &str, collection: &str) -> Vec<Kline> {
    let from_datetime = NaiveDateTime::parse_from_str(from_str, "%Y-%m-%d %H:%M:%S").unwrap();
    let to_datetime = NaiveDateTime::parse_from_str(to_str, "%Y-%m-%d %H:%M:%S").unwrap();
    let from_ts_ms = from_datetime.timestamp_millis();
    let to_ts_ms = to_datetime.timestamp_millis();

    let mongo_clinet = task::block_on(MongoClient::new(LOCAL_MONGO_CONNECTION_STRING));
    let klines =
        task::block_on(mongo_clinet.get_klines(KLINE_DB, collection, from_ts_ms, Some(to_ts_ms)));
    klines
}

pub fn log_trades(trades: &Vec<Trade>, version: &str) {
    let mongo_clinet = task::block_on(MongoClient::new(LOCAL_MONGO_CONNECTION_STRING));
    let collection = mongo_clinet
        .client
        .database(LOG_DB)
        .collection(LOG_COLLECTION);
    let now = Utc::now().timestamp();
    let doc = json!({
        "version": version,
        "trades": trades,
        "timestamp": now
    });
    let log_res = task::block_on(collection.insert_one(doc.clone(), None));
    if log_res.is_ok() {
        info!("Log {:?} success", trades);
    } else {
        warn!("Failed to log {:?}", trades);
    }
}

pub async fn get_trades(version: &str) -> Vec<Trade> {
    let mongo_clinet = MongoClient::new(LOCAL_MONGO_CONNECTION_STRING).await;
    let collection = mongo_clinet
        .client
        .database(LOG_DB)
        .collection::<Document>(LOG_COLLECTION);
    let filter = doc! { "version": version };
    let find_options = FindOptions::builder()
        .sort(doc! { "timestamp": -1 })
        .build();
    let mut cursor = collection.find(filter, find_options).await.unwrap();
    while let Some(doc) = cursor.try_next().await.unwrap() {
        let trades_bson = doc.get("trades").unwrap().to_owned();
        let trades: Vec<Trade> = bson::from_bson(trades_bson).unwrap();
        return trades; // return the newest one
    }
    Vec::new()
}
