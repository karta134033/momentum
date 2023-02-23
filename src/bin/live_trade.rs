use std::collections::VecDeque;
use std::fs::File;
use std::thread;

use async_std::task;
use clap::Parser;
use log::info;
use log::warn;
use momentum::backtest::BacktestMetric;
use momentum::strategy::*;
use momentum::types::BacktestConfig;
use momentum::types::Cli;
use momentum::types::SettingConfig;
use momentum::utils::get_trades;
use momentum::utils::log_account;
use momentum::utils::log_trades;
use trade_utils::clients::binance::api::BinanceFuturesApiClient;

use trade_utils::types::timer::FixedUpdate;
use trade_utils::types::timer::Timer;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let args = Cli::parse();
    info!("args: {:?}", args);
    let setting_config_file = File::open(&args.setting_config.unwrap()).unwrap();
    let setting_config: SettingConfig = serde_json::from_reader(setting_config_file).unwrap();
    let interval = setting_config.collection_postfix.replace("_", "");

    let api_client =
        BinanceFuturesApiClient::new(setting_config.api_key, setting_config.secret_key);
    let symbol = setting_config.symbol;
    let account = task::block_on(api_client.get_account()).unwrap();
    info!("Current account: {:?}", account);

    // ===== Replay =====
    let replay_klines_res =
        task::block_on(api_client.get_klines(&symbol, &interval, None, None, Some("30"))).unwrap();
    let mut replay_klines = VecDeque::from(replay_klines_res);

    let backtest_config_file = File::open(&args.backtest_config.unwrap()).unwrap();
    let backtest_config: BacktestConfig = serde_json::from_reader(backtest_config_file).unwrap();

    let mut minute_timer = Timer::new(FixedUpdate::Minute(1));

    let version = setting_config.version;
    let mut trades = task::block_on(get_trades(&version));
    info!("Recover trades {:?} from db", trades);
    // Close trades if needed
    // close_trades(..);
    let mut metric = BacktestMetric::new(&backtest_config);

    let retry_times = 5;
    let retry_secs = 5; // secs

    // ===== Live =====
    loop {
        if minute_timer.update() {
            let mut recent_klines_res =
                task::block_on(api_client.get_klines(&symbol, &interval, None, None, Some("2")));
            for _ in 0..retry_times {
                if recent_klines_res.is_err() {
                    recent_klines_res = task::block_on(api_client.get_klines(
                        &symbol,
                        &interval,
                        None,
                        None,
                        Some("2"),
                    ));
                    info!("Retry get recent klines");
                } else {
                    break;
                }
                thread::sleep(std::time::Duration::from_secs(retry_secs));
            }
            match recent_klines_res {
                Ok(recent_klines) => {
                    let curr_kline = recent_klines.last().unwrap();
                    let last_close_timestamp = replay_klines.back().unwrap().close_timestamp;
                    println!("recent_klines: {:?}", recent_klines);

                    if last_close_timestamp == curr_kline.close_timestamp {
                        let account = task::block_on(api_client.get_account()).unwrap();
                        println!("Current account: {:?}", account);
                        log_account(&account, &version);
                    } else {
                        let closed_kline = recent_klines.first().unwrap();
                        replay_klines.pop_back(); // Update latest kline
                        replay_klines.push_back(closed_kline.clone());

                        warn!("kline is crossed: {:?}", replay_klines);
                        sl_tp_exit(
                            &mut metric,
                            &backtest_config,
                            &mut trades,
                            false,
                            "",
                            &closed_kline,
                            Some(&api_client),
                        );
                        pct_strategy(
                            symbol.clone(),
                            &mut metric,
                            &backtest_config,
                            &mut trades,
                            &replay_klines,
                            false,
                            "",
                            Some(&api_client),
                        );
                        replay_klines.pop_front();
                        replay_klines.push_back(curr_kline.clone());

                        log_trades(&trades, &version);
                    }
                }
                Err(err) => {
                    warn!("Get recent kline error, {:?}", err);
                }
            }
        } else {
            thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}
