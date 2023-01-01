use clap::Parser;
use momentum::{
    backtest::Backtest,
    hypertune::hypertune,
    types::{BacktestConfig, Cli, SettingConfig},
    utils::get_klines_from_db,
};
use serde_json::Value;
use std::fs::File;
use trade_utils::types::cli::Mode;

use log::{info, LevelFilter};

use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};

fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();
    let args = Cli::parse();
    info!("args: {:?}", args);

    let setting_config_file = File::open(&args.setting_config.unwrap()).unwrap();
    let setting_config: SettingConfig = serde_json::from_reader(setting_config_file).unwrap();
    let klines = get_klines_from_db(
        &setting_config.from,
        &setting_config.to,
        &setting_config.collection,
    );
    match args.mode {
        Mode::Backtest => {
            let backtest_config_file = File::open(&args.backtest_config.unwrap()).unwrap();
            let backtest_config: BacktestConfig =
                serde_json::from_reader(backtest_config_file).unwrap();
            info!("backtest_config: {:?}", backtest_config);
            info!("klines num: {:?}", klines.len());
            let mut backtest = Backtest::new(&backtest_config);
            backtest.run(&klines);
        }
        Mode::Hypertune => {
            let config_file = File::open(&args.hypertune_config.unwrap()).unwrap();
            let hypertune_config_value: Value = serde_json::from_reader(config_file).unwrap();
            hypertune(&hypertune_config_value, &klines);
        }
    }
}
