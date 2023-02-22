use async_std::task;
use log::info;
use momentum::utils::{get_trades, log_trades};
use trade_utils::types::trade::{Trade, TradeSide};

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let version = "test";
    let trades = vec![Trade {
        symbol: "AVAXUSDT".to_owned(),
        entry_price: 1.0,
        entry_side: TradeSide::Buy,
        entry_ts: 1,
        exit_price: 1199.0,
        position: 22999.0,
        tp_price: 1.0,
        sl_price: 1.0,
        ..Default::default()
    }];
    log_trades(&trades, version);
    let trades_from_db = task::block_on(get_trades(version));
    info!("trades_from_db: {:?}", trades_from_db);
}
