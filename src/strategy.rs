use std::{collections::VecDeque, fs::OpenOptions};

use async_std::task;
use chrono::NaiveDateTime;
use log::*;
use trade_utils::{
    clients::binance::api::{BinanceFuturesApiClient, SYMBOL_TO_INSTRUMENT_INFO},
    types::{
        kline::Kline,
        order::{Order, OrderSide},
        trade::{Trade, TradeSide},
    },
};

use crate::{backtest::BacktestMetric, types::BacktestConfig};

pub fn place_order(
    symbol: String,
    api_client_opt: Option<&BinanceFuturesApiClient>,
    trade: &Trade,
    unwind: bool,
) {
    if trade.entry_side == TradeSide::None {
        return;
    }
    if let Some(api_client) = api_client_opt {
        let order_side = if trade.entry_side == TradeSide::Buy {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
        let order_side = if unwind {
            if order_side == OrderSide::Buy {
                OrderSide::Sell
            } else {
                OrderSide::Buy
            }
        } else {
            order_side
        };
        let instrument_info = SYMBOL_TO_INSTRUMENT_INFO.get(&symbol).unwrap();
        let order = Order::market_order(symbol.clone(), order_side, trade.position);
        let place_order_res =
            task::block_on(api_client.place_order(order, instrument_info)).unwrap();
        info!("place_order_res: {:?}", place_order_res);
    }
}

pub fn close_trade(
    metric: &mut BacktestMetric,
    config: &BacktestConfig,
    trades: &mut Vec<Trade>,
    closed_klines: &VecDeque<Kline>,
    output_trade_log: bool,
    output_trade_log_name: &str,
    api_client_opt: Option<&BinanceFuturesApiClient>,
    trade_side: TradeSide,
) {
    let kline = closed_klines.back().unwrap();
    trades.retain_mut(|trade: &mut Trade| {
        if trade.entry_side == trade_side {
            let profit = (kline.close - trade.entry_price) * trade.position * trade_side.value();
            metric.usd_balance += profit;
            metric.fee = kline.close * trade.position * config.fee_rate;
            metric.total_fee += metric.fee;
            metric.profit = profit;
            metric.total_profit += profit;
            trade.exit_price = kline.close;
            error!("Early exit buy");
            trade_log(
                metric,
                config,
                output_trade_log,
                output_trade_log_name,
                kline,
                trade,
            );
            place_order(trade.symbol.clone(), api_client_opt, &trade, true);
            false
        } else {
            true
        }
    });
}

pub fn open_trade(
    symbol: String,
    metric: &mut BacktestMetric,
    config: &BacktestConfig,
    trades: &mut Vec<Trade>,
    closed_klines: &VecDeque<Kline>,
    api_client_opt: Option<&BinanceFuturesApiClient>,
    entry_side: TradeSide,
) {
    if entry_side == TradeSide::None {
        return;
    }
    let kline = closed_klines.back().unwrap();
    let entry_price = kline.close;

    let mut sl_price_diff;
    let sl_price;
    let tp_price;
    match entry_side {
        TradeSide::Sell => {
            sl_price_diff = f64::abs(kline.close - kline.high);
            if sl_price_diff / kline.close > config.risk_portion {
                sl_price_diff = kline.close * config.risk_portion;
            }
            sl_price = entry_price + sl_price_diff;
            tp_price = entry_price - config.tp_ratio * sl_price_diff
        }
        TradeSide::Buy => {
            sl_price_diff = f64::abs(kline.close - kline.low);
            if sl_price_diff / kline.close > config.risk_portion {
                sl_price_diff = kline.close * config.risk_portion;
            }
            sl_price = entry_price - sl_price_diff;
            tp_price = entry_price + config.tp_ratio * sl_price_diff;
        }
        TradeSide::None => {
            panic!("Unsupport open_trade type")
        }
    }

    let position = metric.usd_balance * config.entry_portion / entry_price;
    let entry_ts = kline.close_timestamp;
    let trade = Trade {
        symbol: symbol.clone(),
        entry_price,
        entry_side,
        entry_ts,
        tp_price,
        sl_price,
        position,
        exit_price: -1.,
    };
    place_order(trade.symbol.clone(), api_client_opt, &trade, false);
    trades.push(trade);
    metric.fee = entry_price * position * config.fee_rate;
    metric.total_fee += metric.fee;
}

pub fn pct_strategy(
    symbol: String,
    metric: &mut BacktestMetric,
    config: &BacktestConfig,
    trades: &mut Vec<Trade>,
    closed_klines: &VecDeque<Kline>,
    output_trade_log: bool,
    output_trade_log_name: &str,
    api_client_opt: Option<&BinanceFuturesApiClient>,
) {
    let prev_l_close =
        closed_klines[closed_klines.len() - 1 - 1 - config.look_back_count as usize].close;
    let prev_r_close = closed_klines[closed_klines.len() - 1 - 1].close;
    let curr_l_close =
        closed_klines[closed_klines.len() - 1 - config.look_back_count as usize].close;
    let curr_r_close = closed_klines[closed_klines.len() - 1].close;
    let momentum_pct_prev = prev_r_close / prev_l_close - 1.;
    let momentum_pct_curr = curr_r_close / curr_l_close - 1.;
    let kline = closed_klines.back().unwrap();
    let _uptrend = kline.close > kline.open;

    if let Some(api_client) = api_client_opt {
        let account = task::block_on(api_client.get_account()).unwrap();
        let usd_balance = account.get_usd_balance(); // Correct the usd_balance during live trade
        info!(
            "momentum_pct_prev: {}, momentum_pct_curr: {}",
            momentum_pct_prev, momentum_pct_curr
        );
        metric.usd_balance = usd_balance;
    }

    if momentum_pct_prev >= config.momentum_pct && momentum_pct_curr <= config.momentum_pct {
        // Close buy trades
        close_trade(
            metric,
            config,
            trades,
            closed_klines,
            output_trade_log,
            output_trade_log_name,
            api_client_opt,
            TradeSide::Buy,
        );
    }

    if momentum_pct_prev <= config.momentum_pct * -1.
        && momentum_pct_curr >= config.momentum_pct * -1.
    {
        // Close sell trades
        close_trade(
            metric,
            config,
            trades,
            closed_klines,
            output_trade_log,
            output_trade_log_name,
            api_client_opt,
            TradeSide::Sell,
        );
    }

    if momentum_pct_prev <= config.momentum_pct && momentum_pct_curr >= config.momentum_pct {
        // Close sell trades
        close_trade(
            metric,
            config,
            trades,
            closed_klines,
            output_trade_log,
            output_trade_log_name,
            api_client_opt,
            TradeSide::Sell,
        );
        open_trade(
            symbol,
            metric,
            config,
            trades,
            closed_klines,
            api_client_opt,
            TradeSide::Buy,
        );
    } else if momentum_pct_prev >= config.momentum_pct * -1.
        && momentum_pct_curr <= config.momentum_pct * -1.
    {
        // Close buy trades
        close_trade(
            metric,
            config,
            trades,
            closed_klines,
            output_trade_log,
            output_trade_log_name,
            api_client_opt,
            TradeSide::Buy,
        );
        open_trade(
            symbol,
            metric,
            config,
            trades,
            closed_klines,
            api_client_opt,
            TradeSide::Sell,
        );
    }
}

pub fn sl_tp_exit(
    metric: &mut BacktestMetric,
    config: &BacktestConfig,
    trades: &mut Vec<Trade>,
    output_trade_log: bool,
    output_trade_log_name: &str,
    kline: &Kline,
    api_client_opt: Option<&BinanceFuturesApiClient>,
) {
    trades.retain_mut(|trade: &mut Trade| {
        if trade.entry_side == TradeSide::Buy {
            if kline.close <= trade.sl_price {
                let profit = (kline.close - trade.entry_price) * trade.position;
                metric.usd_balance += profit;
                metric.fee = kline.close * trade.position * config.fee_rate;
                metric.total_fee += metric.fee;
                metric.profit = profit;
                metric.total_profit += profit;
                trade.exit_price = kline.close;
                trade_log(
                    metric,
                    config,
                    output_trade_log,
                    output_trade_log_name,
                    kline,
                    trade,
                );
                info!("sl trade: {:?}", trade);
                place_order(trade.symbol.clone(), api_client_opt, &trade, true);
                false
            } else if kline.close >= trade.tp_price {
                let profit = (kline.close - trade.entry_price) * trade.position;
                metric.usd_balance += profit;
                metric.fee = kline.close * trade.position * config.fee_rate;
                metric.total_fee += metric.fee;
                metric.profit = profit;
                metric.total_profit += profit;
                trade.exit_price = kline.close;
                trade_log(
                    metric,
                    config,
                    output_trade_log,
                    output_trade_log_name,
                    kline,
                    trade,
                );
                info!("tp trade: {:?}", trade);
                place_order(trade.symbol.clone(), api_client_opt, &trade, true);
                false
            } else {
                true
            }
        } else if trade.entry_side == TradeSide::Sell {
            if kline.close >= trade.sl_price {
                let profit = (trade.entry_price - kline.close) * trade.position;
                metric.usd_balance += profit;
                metric.fee = kline.close * trade.position * config.fee_rate;
                metric.total_fee += metric.fee;
                metric.profit = profit;
                metric.total_profit += profit;
                trade.exit_price = kline.close;
                trade_log(
                    metric,
                    config,
                    output_trade_log,
                    output_trade_log_name,
                    kline,
                    trade,
                );
                info!("sl trade: {:?}", trade);
                place_order(trade.symbol.clone(), api_client_opt, &trade, true);
                false
            } else if kline.close <= trade.tp_price {
                let profit = (trade.entry_price - kline.close) * trade.position;
                metric.usd_balance += profit;
                metric.fee = kline.close * trade.position * config.fee_rate;
                metric.total_fee += metric.fee;
                metric.profit = profit;
                metric.total_profit += profit;
                trade.exit_price = kline.close;
                trade_log(
                    metric,
                    config,
                    output_trade_log,
                    output_trade_log_name,
                    kline,
                    trade,
                );
                info!("tp trade: {:?}", trade);
                place_order(trade.symbol.clone(), api_client_opt, &trade, true);
                false
            } else {
                true
            }
        } else {
            true
        }
    });
}

pub fn trade_log(
    metric: &mut BacktestMetric,
    config: &BacktestConfig,
    output_trade_log: bool,
    output_trade_log_name: &str,
    kline: &Kline,
    trade: &Trade,
) {
    let curr_date = NaiveDateTime::from_timestamp_millis(kline.close_timestamp).unwrap();
    let entry_date = NaiveDateTime::from_timestamp_millis(trade.entry_ts).unwrap();
    metric.max_usd = metric.max_usd.max(metric.usd_balance);
    metric.min_usd = metric.min_usd.min(metric.usd_balance);
    metric.max_drawdown = metric
        .max_drawdown
        .max((metric.usd_balance - metric.max_usd) / metric.max_usd * -1.);
    let mut msg = "".to_string();
    msg += &format!("date: {:?}, ", curr_date);
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
    msg += &format!("max_drawdown: {:.4}, ", metric.max_drawdown);

    if metric.profit > 0. {
        metric.win += 1;
        msg += &format!("win: {:?}, ", metric.win);
        msg += &format!("lose: {:?}, ", metric.lose);
        info!("{}", msg);
    } else {
        metric.lose += 1;
        msg += &format!("win: {:?}, ", metric.win);
        msg += &format!("lose: {:?}, ", metric.lose);
        warn!("{}", msg);
    }
    if output_trade_log {
        let output_name = output_trade_log_name.to_owned();
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(output_name)
            .unwrap();
        let mut writer = csv::Writer::from_writer(file);
        let mut record = Vec::new();
        record.push(curr_date.to_string());
        record.push(metric.initial_captial.to_string());
        record.push(metric.usd_balance.to_string());
        record.push(metric.max_usd.to_string());
        record.push(metric.min_usd.to_string());
        record.push(metric.win.to_string());
        record.push(metric.lose.to_string());
        record.push((metric.win as f64 / (metric.win + metric.lose) as f64).to_string());
        record.push(metric.total_fee.to_string());
        record.push(metric.total_profit.to_string());
        record.push(config.risk_portion.to_string());
        record.push(config.tp_ratio.to_string());
        record.push(config.look_back_count.to_string());
        record.push(format!("{:.*}", 4, metric.max_drawdown));
        writer.write_record(&record).unwrap();
        writer.flush().unwrap();
    }
}
