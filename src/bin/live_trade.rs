use std::thread;

use async_std::task;
use chrono::Utc;
use log::info;
use momentum::binance::BinanceFuturesApiClient;
use trade_utils::types::timer::FixedUpdate;
use trade_utils::types::timer::Timer;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    println!("Live trade");
    let symbol = "AVAXUSDT";
    let api_client = BinanceFuturesApiClient::new();
    let start_time = (Utc::now() - chrono::Duration::days(30))
        .timestamp_millis()
        .to_string();
    let start_time = start_time.as_str(); // Create longer binding
    let kline_api_res = task::block_on(api_client.get_klines(symbol, "1d", start_time, None, None));
    match kline_api_res {
        Ok(klines) => {
            info!("klines: {:#?}", klines);
        }
        Err(_err) => {}
    }
    let mut timer = Timer::new(FixedUpdate::Minute(1));
    loop {
        if !timer.update() {
            thread::sleep(std::time::Duration::from_secs(10));
        }
    }
}
