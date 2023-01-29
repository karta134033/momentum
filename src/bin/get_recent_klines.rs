use std::fs::File;

use async_std::task;
use momentum::types::SettingConfig;
use trade_utils::clients::binance::api::BinanceFuturesApiClient;

fn main() {
    let setting_config_file = File::open("./setting_config.json").unwrap();
    let setting_config: SettingConfig = serde_json::from_reader(setting_config_file).unwrap();

    let api_client =
        BinanceFuturesApiClient::new(setting_config.api_key, setting_config.secret_key);
    let symbol = "AVAXUSDT".to_owned();
    let recent_klines =
        task::block_on(api_client.get_klines(&symbol, "1d", None, None, Some("2"))).unwrap();
    println!("{:#?}", recent_klines);
}
