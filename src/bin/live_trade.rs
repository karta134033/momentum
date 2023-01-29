use std::collections::HashSet;
use std::collections::VecDeque;
use std::fs::File;
use std::thread;

use async_std::task;
use chrono::NaiveDateTime;
use chrono::Utc;
use clap::Parser;
use log::info;
use log::warn;
use momentum::backtest::BacktestMetric;
use momentum::types::BacktestConfig;
use momentum::types::Cli;
use momentum::types::SettingConfig;
use momentum::utils::get_trades;
use momentum::utils::log_trades;
use trade_utils::clients::binance::api::BinanceFuturesApiClient;
use trade_utils::types::kline::Kline;
use trade_utils::types::order::Order;
use trade_utils::types::order::OrderSide;
use trade_utils::types::timer::FixedUpdate;
use trade_utils::types::timer::Timer;
use trade_utils::types::trade::Trade;
use trade_utils::types::trade::TradeSide;

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
    // ===== Live =====
    loop {
        if minute_timer.update() {
            let recent_klines =
                task::block_on(api_client.get_klines(&symbol, "1d", None, None, Some("2")))
                    .unwrap();
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
                close_trade(
                    &mut trades,
                    closed_kline,
                    &backtest_config,
                    &mut metric,
                    symbol.clone(),
                    &api_client,
                );
                open_trade(
                    &mut trades,
                    closed_kline,
                    &momentums,
                    &backtest_config,
                    symbol.clone(),
                    &api_client,
                );
                replay_klines.pop_front();
                replay_klines.push_back(curr_kline.clone());

                momentums.pop_front();
                let momentum = cal_momentum(&replay_klines);
                momentums.push_back(momentum);

                log_trades(&trades, &version);
            }
        } else {
            thread::sleep(std::time::Duration::from_secs(10));
        }
    }
}

fn open_trade(
    trades: &mut Vec<Trade>,
    kline: &Kline,
    momentums: &VecDeque<f64>,
    config: &BacktestConfig,
    symbol: String,
    api_client: &BinanceFuturesApiClient,
) {
    let prev_sign = momentums[momentums.len() - 2].signum();
    let curr_sign = momentums[momentums.len() - 1].signum();
    let account = task::block_on(api_client.get_account()).unwrap();
    let usd_balance = account.get_usd_balance();

    let mut symbol_set = HashSet::new();
    symbol_set.insert(symbol.clone());
    let instruments_info = task::block_on(api_client.get_instruments(&symbol_set)).unwrap();
    let instrument_info = instruments_info.get(&symbol).unwrap();

    let now = Utc::now().timestamp_millis();
    if prev_sign == -1. && curr_sign == 1. {
        let entry_price = kline.close;
        let entry_side = TradeSide::Buy;
        let mut sl_price_diff = f64::abs(kline.close - kline.low);
        if sl_price_diff / kline.close > config.risk_portion {
            sl_price_diff = kline.close * config.risk_portion;
        }
        let sl_price = entry_price - sl_price_diff;
        let tp_price = entry_price + config.tp_ratio * sl_price_diff;
        let position = usd_balance * config.entry_portion / entry_price;
        let entry_ts = now;
        let trade = Trade {
            entry_price,
            entry_side,
            entry_ts,
            tp_price,
            sl_price,
            position,
            exit_price: -1.,
        };
        trades.push(trade);
        let order = Order::market_order(symbol, OrderSide::Buy, position);
        let place_order_res =
            task::block_on(api_client.place_order(order, instrument_info)).unwrap();
        info!("place_order_res: {:?}", place_order_res);
    } else if prev_sign == 1. && curr_sign == -1. {
        let entry_price = kline.close;
        let entry_side = TradeSide::Sell;
        let mut sl_price_diff = f64::abs(kline.close - kline.high);
        if sl_price_diff / kline.close > config.risk_portion {
            sl_price_diff = kline.close * config.risk_portion;
        }
        let sl_price = entry_price + sl_price_diff;
        let tp_price = entry_price - config.tp_ratio * sl_price_diff;
        let position = usd_balance * config.entry_portion / entry_price;
        let entry_ts = now;
        let trade = Trade {
            entry_price,
            entry_side,
            entry_ts,
            tp_price,
            sl_price,
            position,
            exit_price: -1.,
        };
        trades.push(trade);
        let order = Order::market_order(symbol, OrderSide::Sell, position);
        let place_order_res =
            task::block_on(api_client.place_order(order, instrument_info)).unwrap();
        info!("place_order_res: {:?}", place_order_res);
    }
}

fn close_trade(
    trades: &mut Vec<Trade>,
    kline: &Kline,
    config: &BacktestConfig,
    metric: &mut BacktestMetric,
    symbol: String,
    api_client: &BinanceFuturesApiClient,
) {
    let mut symbol_set = HashSet::new();
    symbol_set.insert(symbol.clone());
    let instruments_info = task::block_on(api_client.get_instruments(&symbol_set)).unwrap();
    let instrument_info = instruments_info.get(&symbol).unwrap();

    trades.retain_mut(|trade: &mut Trade| {
        if trade.entry_side == TradeSide::Buy {
            if kline.close <= trade.sl_price {
                let profit = (kline.close - trade.entry_price) * trade.position;
                metric.usd_balance += profit;
                metric.lose += 1;
                metric.fee = kline.close * trade.position * config.fee_rate;
                metric.total_fee += metric.fee;
                metric.profit = profit;
                metric.total_profit += profit;
                trade.exit_price = kline.close;
                trade_log(metric, trade, &kline);
                let order = Order::market_order(symbol.clone(), OrderSide::Sell, trade.position);
                let place_order_res =
                    task::block_on(api_client.place_order(order, instrument_info)).unwrap();
                info!("place_order_res: {:?}", place_order_res);
                false
            } else if kline.close >= trade.tp_price {
                let profit = (kline.close - trade.entry_price) * trade.position;
                metric.usd_balance += profit;
                metric.win += 1;
                metric.fee = kline.close * trade.position * config.fee_rate;
                metric.total_fee += metric.fee;
                metric.profit = profit;
                metric.total_profit += profit;
                trade.exit_price = kline.close;
                trade_log(metric, trade, &kline);
                let order = Order::market_order(symbol.clone(), OrderSide::Sell, trade.position);
                let place_order_res =
                    task::block_on(api_client.place_order(order, instrument_info)).unwrap();
                info!("place_order_res: {:?}", place_order_res);
                false
            } else {
                true
            }
        } else if trade.entry_side == TradeSide::Sell {
            if kline.close >= trade.sl_price {
                let profit = (trade.entry_price - kline.close) * trade.position;
                metric.usd_balance += profit;
                metric.lose += 1;
                metric.fee = kline.close * trade.position * config.fee_rate;
                metric.total_fee += metric.fee;
                metric.profit = profit;
                metric.total_profit += profit;
                trade.exit_price = kline.close;
                trade_log(metric, trade, &kline);
                let order = Order::market_order(symbol.clone(), OrderSide::Buy, trade.position);
                let place_order_res =
                    task::block_on(api_client.place_order(order, instrument_info)).unwrap();
                info!("place_order_res: {:?}", place_order_res);
                false
            } else if kline.close <= trade.tp_price {
                let profit = (trade.entry_price - kline.close) * trade.position;
                metric.usd_balance += profit;
                metric.win += 1;
                metric.fee = kline.close * trade.position * config.fee_rate;
                metric.total_fee += metric.fee;
                metric.profit = profit;
                metric.total_profit += profit;
                trade.exit_price = kline.close;
                trade_log(metric, trade, &kline);
                let order = Order::market_order(symbol.clone(), OrderSide::Buy, trade.position);
                let place_order_res =
                    task::block_on(api_client.place_order(order, instrument_info)).unwrap();
                info!("place_order_res: {:?}", place_order_res);
                false
            } else {
                true
            }
        } else {
            true
        }
    });
}

fn trade_log(metric: &mut BacktestMetric, trade: &Trade, curr_kline: &Kline) {
    let curr_date = NaiveDateTime::from_timestamp_millis(curr_kline.close_timestamp).unwrap();
    let entry_date = NaiveDateTime::from_timestamp_millis(trade.entry_ts).unwrap();
    metric.max_usd = metric.max_usd.max(metric.usd_balance);
    metric.min_usd = metric.min_usd.min(metric.usd_balance);
    let mut msg = "".to_string();
    msg += &format!("date: {:?}, ", curr_date);
    msg += &format!("win: {:?}, ", metric.win);
    msg += &format!("lose: {:?}, ", metric.lose);
    msg += &format!("usd_balance: {:.4}, ", metric.usd_balance);
    msg += &format!("max_usd: {:.4}, ", metric.max_usd);
    msg += &format!("min_usd: {:.4}, ", metric.min_usd);
    msg += &format!("position: {:.4}, ", trade.position);
    msg += &format!("entry_date: {:?}, ", entry_date);
    msg += &format!("entry_side: {:?}, ", trade.entry_side);
    msg += &format!("entry_price: {:.4}, ", trade.entry_price);
    msg += &format!("tp_price: {:.4}, ", trade.tp_price);
    msg += &format!("sl_price: {:.4}, ", trade.sl_price);
    msg += &format!("exit_price: {:.4}, ", trade.exit_price);
    msg += &format!("profit: {:.4}, ", metric.profit);
    msg += &format!("fee: {:.4}, ", metric.fee);

    if metric.profit > 0. {
        info!("{}", msg);
    } else {
        warn!("{}", msg);
    }
}
