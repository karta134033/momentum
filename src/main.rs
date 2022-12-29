use clap::Parser;
use momentum::{backtest::Backtest, types::SettingConfig, utils::get_klines_from_db};
use std::fs::File;
use trade_utils::types::cli::{Cli, Mode};

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
    let config_file = File::open(&args.config_path).unwrap();
    let config: SettingConfig = serde_json::from_reader(config_file).unwrap();
    info!("args: {:?}, config: {:?}", args, config);

    let klines = get_klines_from_db(&config.from, &config.to);
    info!("klines num: {:?}", klines.len());
    match args.mode {
        Mode::Backtest => {
            let mut backtest = Backtest::new(config);
            backtest.run(klines);
        }
        Mode::Hypertune => todo!(),
    }
}
