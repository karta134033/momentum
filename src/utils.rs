use async_std::task;
use chrono::NaiveDateTime;
use trade_utils::{clients::mongo_client::MongoClient, types::kline::Kline};

use crate::consts::{KLINE_DB, LOCAL_MONGO_CONNECTION_STRING};

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
