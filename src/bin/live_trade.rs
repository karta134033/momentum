use std::collections::VecDeque;
use std::fs::File;
use std::thread;

use async_std::task;
use chrono::Utc;
use clap::Parser;
use log::info;
use log::warn;
use momentum::types::Cli;
use momentum::types::SettingConfig;
use trade_utils::clients::binance::api::BinanceFuturesApiClient;
use trade_utils::types::timer::FixedUpdate;
use trade_utils::types::timer::Timer;

fn main() {
    /*
    TODO:
    1. replay
    2. cal momentum
    3. place order
        instrument_info
        lot_size
    4. gen sl_tp orders
    5. log sl_tp orders to db
        index db
        create collection
    6. get account every minute
    7. one day after -> check sl_tp -> back to approach 1
        use timer
    */
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let api_client = BinanceFuturesApiClient::new();
    let args = Cli::parse();
    info!("args: {:?}", args);
    let setting_config_file = File::open(&args.setting_config.unwrap()).unwrap();
    let setting_config: SettingConfig = serde_json::from_reader(setting_config_file).unwrap();
    let symbol = setting_config.symbol;
    let account =
        task::block_on(api_client.get_account(&setting_config.api_key, &setting_config.secret_key))
            .unwrap();
    info!("Current account: {:?}", account);
    // ===== Replay =====
    let start_time = (Utc::now() - chrono::Duration::days(30))
        .timestamp_millis()
        .to_string();
    let replay_klines_res =
        task::block_on(api_client.get_klines(&symbol, "1d", Some(start_time.as_str()), None, None))
            .unwrap();
    let mut replay_klines = VecDeque::from(replay_klines_res);
    let mut minute_timer = Timer::new(FixedUpdate::Minute(1));

    // ===== Live =====
    loop {
        if minute_timer.update() {
            let last_kline = replay_klines.back().unwrap();
            let curr_kline =
                task::block_on(api_client.get_klines(&symbol, "1d", None, None, None)).unwrap();
            let curr_kline = curr_kline.last().unwrap();
            if curr_kline.close_timestamp == last_kline.close_timestamp {
                info!("kline is not crossed {:?}", curr_kline);
            } else {
                replay_klines.pop_front();
                replay_klines.push_back(curr_kline.clone());
                warn!("kline is crossed: {:?}", replay_klines);
            }
            let account = task::block_on(
                api_client.get_account(&setting_config.api_key, &setting_config.secret_key),
            )
            .unwrap();
            info!("Current account: {:?}", account);
        }
        thread::sleep(std::time::Duration::from_secs(10));
    }
}
