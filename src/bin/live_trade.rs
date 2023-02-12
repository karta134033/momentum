use std::collections::VecDeque;
use std::fs::File;
use std::thread;

use async_std::task;
use chrono::Utc;
use clap::Parser;
use log::info;
use log::warn;
use momentum::backtest::BacktestMetric;
use momentum::strategy::open_trade;
use momentum::strategy::sl_tp_exit;
use momentum::types::BacktestConfig;
use momentum::types::Cli;
use momentum::types::SettingConfig;
use momentum::utils::get_trades;
use momentum::utils::log_trades;
use trade_utils::clients::binance::api::BinanceFuturesApiClient;
use trade_utils::types::kline::Kline;

use trade_utils::types::timer::FixedUpdate;
use trade_utils::types::timer::Timer;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let args = Cli::parse();
    info!("args: {:?}", args);
    let setting_config_file = File::open(&args.setting_config.unwrap()).unwrap();
    let setting_config: SettingConfig = serde_json::from_reader(setting_config_file).unwrap();

    let api_client =
        BinanceFuturesApiClient::new(setting_config.api_key, setting_config.secret_key);
    let symbol = setting_config.symbol;
    let account = task::block_on(api_client.get_account()).unwrap();
    info!("Current account: {:?}", account);

    // ===== Replay =====
    let start_time = (Utc::now() - chrono::Duration::days(30))
        .timestamp_millis()
        .to_string();
    let replay_klines_res =
        task::block_on(api_client.get_klines(&symbol, "1d", Some(start_time.as_str()), None, None))
            .unwrap();
    let mut replay_klines = VecDeque::from(replay_klines_res);

    let backtest_config_file = File::open(&args.backtest_config.unwrap()).unwrap();
    let backtest_config: BacktestConfig = serde_json::from_reader(backtest_config_file).unwrap();
    let mut momentums = VecDeque::new();
    let look_back_count = backtest_config.look_back_count as usize;
    for index in 0..replay_klines.len() {
        if index >= look_back_count as usize {
            let prev_close = replay_klines[index - look_back_count].close;
            let curr_close = replay_klines[index].close;
            let momentum = curr_close - prev_close;
            momentums.push_back(momentum);
        }
    }
    let mut minute_timer = Timer::new(FixedUpdate::Minute(1));
    println!("momentums: {:?}", momentums);

    let version = setting_config.version;
    let mut trades = task::block_on(get_trades(&version));
    info!("Recover trades {:?} from db", trades);
    // Close trades if needed
    // close_trades(..);
    let mut metric = BacktestMetric::new(&backtest_config);
    let cal_momentum = |replay_klines: &VecDeque<Kline>| -> f64 {
        let index = replay_klines.len() - 1;
        let prev_close = replay_klines[index - look_back_count].close;
        let curr_close = replay_klines[index].close;
        curr_close - prev_close
    };
    let retry_times = 5;
    let retry_secs = 5; // secs

    // ===== Live =====
    let output_trade_log_name = "live_trade_output";
    loop {
        if minute_timer.update() {
            let mut recent_klines_res =
                task::block_on(api_client.get_klines(&symbol, "1d", None, None, Some("2")));
            for _ in 0..retry_times {
                if recent_klines_res.is_err() {
                    recent_klines_res =
                        task::block_on(api_client.get_klines(&symbol, "1d", None, None, Some("2")));
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

                    if last_close_timestamp == curr_kline.close_timestamp {
                        replay_klines.pop_back(); // Update latest kline
                        replay_klines.push_back(curr_kline.clone());

                        momentums.pop_back();
                        let momentum = cal_momentum(&replay_klines);
                        momentums.push_back(momentum);

                        let account = task::block_on(api_client.get_account()).unwrap();
                        println!("Current account: {:?}", account);
                        println!("momentums: {:?}", momentums);
                    } else {
                        let closed_kline = recent_klines.first().unwrap();
                        replay_klines.pop_back(); // Update latest kline
                        replay_klines.push_back(closed_kline.clone());

                        momentums.pop_back();
                        let momentum = cal_momentum(&replay_klines);
                        momentums.push_back(momentum);

                        warn!("kline is crossed: {:?}", replay_klines);
                        info!("momentums: {:?}", momentums);
                        sl_tp_exit(
                            &mut metric,
                            &backtest_config,
                            &mut trades,
                            true,
                            output_trade_log_name,
                            &closed_kline,
                            Some(&api_client),
                        );
                        open_trade(
                            symbol.clone(),
                            &mut metric,
                            &backtest_config,
                            &mut trades,
                            &momentums,
                            true,
                            output_trade_log_name,
                            &closed_kline,
                            Some(&api_client),
                        );
                        replay_klines.pop_front();
                        replay_klines.push_back(curr_kline.clone());

                        momentums.pop_front();
                        let momentum = cal_momentum(&replay_klines);
                        momentums.push_back(momentum);

                        log_trades(&trades, &version);
                    }
                }
                Err(err) => {
                    warn!("Get recent kline error, {:?}", err);
                }
            }
        } else {
            thread::sleep(std::time::Duration::from_secs(10));
        }
    }
}
