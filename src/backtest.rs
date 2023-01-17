use chrono::NaiveDateTime;
use log::{info, warn};
use std::collections::VecDeque;
use trade_utils::types::kline::Kline;
use trade_utils::types::trade::{Trade, TradeSide};

use crate::types::BacktestConfig;

pub struct Backtest {
    config: BacktestConfig,
    momentum: VecDeque<f64>,
}

#[derive(Default)]
pub struct BacktestMetric {
    pub initial_captial: f64,
    pub usd_balance: f64,
    pub win: usize,
    pub lose: usize,
    pub total_fee: f64,
    pub total_profit: f64,
    pub max_usd: f64,
    pub min_usd: f64,
    pub fee: f64,
    pub profit: f64,
}

impl BacktestMetric {
    pub fn new(config: &BacktestConfig) -> BacktestMetric {
        BacktestMetric {
            usd_balance: config.initial_captial,
            initial_captial: config.initial_captial,
            max_usd: f64::MIN,
            min_usd: f64::MAX,
            ..Default::default()
        }
    }
}

impl Backtest {
    pub fn new(config: &BacktestConfig) -> Backtest {
        Backtest {
            config: config.clone(),
            momentum: VecDeque::new(),
        }
    }

    pub fn run(&mut self, klines: &Vec<Kline>) -> BacktestMetric {
        let mut metric = BacktestMetric::new(&self.config);
        let mut trades: Vec<Trade> = Vec::new();
        for k_index in 0..klines.len() {
            let kline = &klines[k_index];
            trades.retain_mut(|trade: &mut Trade| {
                if trade.entry_side == TradeSide::Buy {
                    if kline.close <= trade.sl_price {
                        let profit = (kline.close - trade.entry_price) * trade.position;
                        metric.usd_balance += profit;
                        metric.lose += 1;
                        metric.fee = kline.close * trade.position * self.config.fee_rate;
                        metric.total_fee += metric.fee;
                        metric.profit = profit;
                        metric.total_profit += profit;
                        trade.exit_price = kline.close;
                        trade_log(&mut metric, &trade, &kline);
                        false
                    } else if kline.close >= trade.tp_price {
                        let profit = (kline.close - trade.entry_price) * trade.position;
                        metric.usd_balance += profit;
                        metric.win += 1;
                        metric.fee = kline.close * trade.position * self.config.fee_rate;
                        metric.total_fee += metric.fee;
                        metric.profit = profit;
                        metric.total_profit += profit;
                        trade.exit_price = kline.close;
                        trade_log(&mut metric, &trade, &kline);
                        false
                    } else {
                        // if trade.tp_price >= trade.entry_price * 1.01 {
                        //     trade.tp_price -= trade.entry_price * 0.005;
                        // }
                        true
                    }
                } else if trade.entry_side == TradeSide::Sell {
                    if kline.close >= trade.sl_price {
                        let profit = (trade.entry_price - kline.close) * trade.position;
                        metric.usd_balance += profit;
                        metric.lose += 1;
                        metric.fee = kline.close * trade.position * self.config.fee_rate;
                        metric.total_fee += metric.fee;
                        metric.profit = profit;
                        metric.total_profit += profit;
                        trade.exit_price = kline.close;
                        trade_log(&mut metric, &trade, &kline);
                        false
                    } else if kline.close <= trade.tp_price {
                        let profit = (trade.entry_price - kline.close) * trade.position;
                        metric.usd_balance += profit;
                        metric.win += 1;
                        metric.fee = kline.close * trade.position * self.config.fee_rate;
                        metric.total_fee += metric.fee;
                        metric.profit = profit;
                        metric.total_profit += profit;
                        trade.exit_price = kline.close;
                        trade_log(&mut metric, &trade, &kline);
                        false
                    } else {
                        // if trade.tp_price <= trade.entry_price * 0.99 {
                        //     trade.tp_price += trade.entry_price * 0.005;
                        // }
                        true
                    }
                } else {
                    true
                }
            });

            let look_back = self.config.look_back_count as usize;
            if k_index > look_back {
                let prev_close = klines[k_index - look_back].close;
                let curr_close = klines[k_index].close;
                let momentum = curr_close - prev_close;
                self.add_momentum(momentum);
                if self.momentum.len() >= 2 {
                    let prev_sign = self.momentum[self.momentum.len() - 2].signum();
                    let curr_sign = self.momentum[self.momentum.len() - 1].signum();
                    if prev_sign == -1. && curr_sign == 1. {
                        let entry_price = kline.close;
                        let entry_side = TradeSide::Buy;
                        let mut sl_price_diff = f64::abs(kline.close - kline.low);
                        if sl_price_diff / kline.close > self.config.risk_portion {
                            sl_price_diff = kline.close * self.config.risk_portion;
                        }
                        let sl_price = entry_price - sl_price_diff;
                        let tp_price = entry_price + self.config.tp_ratio * sl_price_diff;
                        let position = metric.usd_balance * self.config.entry_portion / entry_price;
                        let entry_ts = kline.close_timestamp;
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
                        metric.fee = entry_price * position * self.config.fee_rate;
                        metric.total_fee += metric.fee;
                    } else if prev_sign == 1. && curr_sign == -1. {
                        let entry_price = kline.close;
                        let entry_side = TradeSide::Sell;
                        let mut sl_price_diff = f64::abs(kline.close - kline.high);
                        if sl_price_diff / kline.close > self.config.risk_portion {
                            sl_price_diff = kline.close * self.config.risk_portion;
                        }
                        let sl_price = entry_price + sl_price_diff;
                        let tp_price = entry_price - self.config.tp_ratio * sl_price_diff;
                        let position = metric.usd_balance * self.config.entry_portion / entry_price;
                        let entry_ts = kline.close_timestamp;
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
                        metric.fee = entry_price * position * self.config.fee_rate;
                        metric.total_fee += metric.fee;
                    }
                }
            }
        }
        metric
    }

    pub fn add_momentum(&mut self, momentum: f64) {
        self.momentum.push_back(momentum);
        if self.momentum.len() > self.config.look_back_count as usize {
            self.momentum.pop_front();
        }
    }
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
