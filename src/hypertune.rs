use rand::seq::SliceRandom;
use std::{fs::File, path::Path};

use log::info;
use serde_json::{json, Map, Value};
use trade_utils::types::kline::Kline;

use crate::{backtest, types::BacktestConfig};

pub fn hypertune(value: &Value, klines: &Vec<Kline>, symbol: String) {
    let raw_config = value.as_object().unwrap();
    let mut backtest_configs: Vec<BacktestConfig> = Vec::new();
    let mut backtest_config_value = json!({});
    let mut tune_fields = Vec::new();
    raw_config.iter().for_each(|(k, v)| {
        println!("k: {} v: {}", k, v);
        if v.as_object().is_some() {
            tune_fields.push(k);
        } else {
            backtest_config_value[k] = v.clone();
        }
    });
    parse_backtest_configs(
        raw_config,
        &mut backtest_config_value,
        &mut backtest_configs,
        &mut tune_fields,
        0,
    );
    info!("tune_fields: {:?}", tune_fields);
    let output_path = Path::new("hypertune_output.csv");
    let file = File::create(output_path).unwrap();
    let mut writer = csv::Writer::from_writer(file);
    writer
        .write_record(&[
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
            "momentum_pct",
        ])
        .unwrap();

    let num_to_pick = 30000;
    let mut rng = rand::thread_rng();
    backtest_configs.shuffle(&mut rng); // shuffle the vector randomly
    let backtest_configs = backtest_configs
        .into_iter()
        .take(num_to_pick)
        .collect::<Vec<_>>();
    backtest_configs.iter().for_each(|config| {
        let mut backtest = backtest::Backtest::new(config, false);
        let metric = backtest.run(klines, symbol.clone());
        let mut record = Vec::new();
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
        record.push(config.momentum_pct.to_string());
        writer.write_record(&record).unwrap();
        writer.flush().unwrap();
    });
}
pub fn parse_backtest_configs(
    raw_config: &Map<String, Value>,
    backtest_config_value: &mut Value,
    backtest_configs: &mut Vec<BacktestConfig>,
    tune_fields: &Vec<&String>,
    index: usize,
) {
    if index == tune_fields.len() {
        let backtest_config: BacktestConfig =
            serde_json::from_value(backtest_config_value.clone()).unwrap();
        backtest_configs.push(backtest_config);
        return;
    }
    let field_name = tune_fields[index];
    let mut min = raw_config[field_name]["min"].as_f64().unwrap();
    let max = raw_config[field_name]["max"].as_f64().unwrap();
    let step = raw_config[field_name]["step"].as_f64().unwrap();
    while min <= max {
        backtest_config_value[field_name] = json!(min);
        parse_backtest_configs(
            raw_config,
            backtest_config_value,
            backtest_configs,
            tune_fields,
            index + 1,
        );
        min += step;
    }
}
