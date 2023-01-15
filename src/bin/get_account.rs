use std::fs::File;

use async_std::task;
use log::info;
use momentum::{binance::BinanceFuturesApiClient, types::SettingConfig};

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let setting_config_file = File::open("./setting_config.json").unwrap();
    let setting_config: SettingConfig = serde_json::from_reader(setting_config_file).unwrap();
    let api_key = &setting_config.api_key;
    let secret_key = &setting_config.secret_key;

    let client = BinanceFuturesApiClient::new();
    let account = task::block_on(client.get_account(api_key, secret_key)).unwrap();
    let assets = account["assets"].as_array().unwrap();
    info!("assets: {:?}", assets);
    let positions = account["positions"].as_array().unwrap();
    positions.iter().for_each(|position| {
        if position["positionAmt"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap()
            != 0.
        {
            info!("position: {:?}", position);
        }
    });
}
