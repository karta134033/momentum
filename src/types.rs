use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingConfig {
    pub from: String,
    pub to: String,
    pub initial_captial: f64,
    pub fee_rate: f64,
    pub entry_portion: f64,
    pub look_back_count: usize,
}
