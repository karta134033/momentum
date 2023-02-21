use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;
use trade_utils::types::kline::Kline;
use trade_utils::types::trade::Trade;

use crate::strategy::{pct_strategy, sl_tp_exit};
use crate::types::{BacktestConfig, SettingConfig};

pub struct Backtest {
    config: BacktestConfig,
    setting_config: SettingConfig,
    closed_klines: VecDeque<Kline>,
    output_result: bool,
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
    pub max_drawdown: f64,
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
    pub fn output_name(&self) -> String {
        format!(
            "./backtest_output/{}_{}_{}_{}_backtest_output.csv",
            self.config.risk_portion,
            self.config.tp_ratio,
            self.config.look_back_count,
            self.setting_config.collection_postfix
        )
    }
    pub fn new(
        config: &BacktestConfig,
        setting_config: &SettingConfig,
        output_result: bool,
    ) -> Backtest {
        let backtest = Backtest {
            config: config.clone(),
            setting_config: setting_config.clone(),
            closed_klines: VecDeque::new(),
            output_result,
        };
        if output_result {
            let output_name = &backtest.output_name();
            let output_path = Path::new(output_name);
            let file = File::create(output_path).unwrap();
            let mut writer = csv::Writer::from_writer(file);
            writer
                .write_record(&[
                    "datetime",
                    "initial_captial",
                    "usd_balance",
                    "max_usd",
                    "min_usd",
                    "win",
                    "lose",
                    "win_rate",
                    "total_fee",
                    "total_profit",
                    "risk_portion",
                    "tp_ratio",
                    "look_back_count",
                    "max_drawdown",
                ])
                .unwrap();
            writer.flush().unwrap();
        }
        backtest
    }

    pub fn run(&mut self, klines: &Vec<Kline>, symbol: String) -> BacktestMetric {
        let mut metric = BacktestMetric::new(&self.config);
        let mut trades: Vec<Trade> = Vec::new();

        let output_trade_log_name = self.output_name();
        for k_index in 0..klines.len() {
            let kline = &klines[k_index];

            sl_tp_exit(
                &mut metric,
                &mut self.config,
                &mut trades,
                self.output_result,
                &output_trade_log_name,
                &kline,
                None,
            );

            let look_back = self.config.look_back_count as usize;
            self.closed_klines.push_back(klines[k_index].clone());
            if k_index > look_back + 3 {
                self.closed_klines.pop_front();
                pct_strategy(
                    symbol.clone(),
                    &mut metric,
                    &mut self.config,
                    &mut trades,
                    &self.closed_klines,
                    self.output_result,
                    &output_trade_log_name,
                    None,
                );
            }
        }
        metric
    }
}
