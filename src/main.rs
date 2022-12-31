use clap::Parser;
use momentum::{
    backtest::Backtest,
    consts::*,
    types::{Cli, SettingConfig},
    utils::get_klines_from_db,
};
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
    match args.mode {
        Mode::Backtest => {
            let config_file = File::open(&args.backtest_config.unwrap()).unwrap();
            let backtest_config: SettingConfig = serde_json::from_reader(config_file).unwrap();
            info!("backtest_config: {:?}", backtest_config);
            let klines =
                get_klines_from_db(&backtest_config.from, &backtest_config.to, AVAXUSDT_1D);
            info!("klines num: {:?}", klines.len());
            let mut backtest = Backtest::new(backtest_config);
            backtest.run(klines);
        }
        Mode::Hypertune => todo!(),
    }
}
