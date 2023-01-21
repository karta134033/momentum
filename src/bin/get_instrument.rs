use std::collections::HashSet;

use async_std::task;
use log::info;
use trade_utils::clients::binance::api::BinanceFuturesApiClient;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let mut symbol_set = HashSet::new();
    symbol_set.insert("BTCUSDT".to_owned());
    symbol_set.insert("AVAXUSDT".to_owned());

    let client = BinanceFuturesApiClient::new("".to_owned(), "".to_owned());
    let instrument_info = task::block_on(client.get_instruments(&symbol_set));
    info!("instrument_info: {:?}", instrument_info);
}
