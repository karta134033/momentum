use std::{collections::HashSet, fs::File};

use async_std::task;
use log::info;
use momentum::types::SettingConfig;
use trade_utils::{
    clients::binance::api::BinanceFuturesApiClient,
    types::order::{Order, OrderSide},
};

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let setting_config_file = File::open("./setting_config.json").unwrap();
    let setting_config: SettingConfig = serde_json::from_reader(setting_config_file).unwrap();
    let api_key = setting_config.api_key;
    let secret_key = setting_config.secret_key;

    let client = BinanceFuturesApiClient::new(api_key, secret_key);
    let symbol = "AVAXUSDT".to_owned();
    let mut symbol_set = HashSet::new();
    symbol_set.insert(symbol.clone());

    let instruments_info = task::block_on(client.get_instruments(&symbol_set)).unwrap();
    let instrument_info = instruments_info.get(&symbol).unwrap();
    info!("instrument_info: {:?}", instrument_info);
    // let order = Order::market_order(symbol.clone(), OrderSide::Sell, 1.0);
    let order = Order::unwind_market_order(symbol.clone(), OrderSide::Buy);
    let place_order_res = task::block_on(client.place_order(order, instrument_info)).unwrap();
    info!("place_order_res: {:?}", place_order_res);
}
