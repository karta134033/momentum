use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};
use trade_utils::types::cli::Mode;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingConfig {
    pub from: String,
    pub to: String,
    pub initial_captial: f64,
    pub fee_rate: f64,
    pub entry_portion: f64,
    pub look_back_count: usize,
    pub risk_portion: f64,
    pub win_ratio: f64,
}

#[derive(Parser, Debug)]
#[command(arg_required_else_help = false)]
pub struct Cli {
    #[arg(short = 'b', required = false)]
    pub backtest_config: Option<PathBuf>,
    #[arg(short = 'm', long = "mode")]
    pub mode: Mode,
    #[arg(short = 't', required = false)]
    pub hypertune_config: Option<PathBuf>,
}
