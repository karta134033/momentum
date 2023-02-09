use log::info;
use trade_utils::clients::binance::api::SYMBOL_TO_INSTRUMENT_INFO;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    info!(
        "instrument_info: {:?}",
        SYMBOL_TO_INSTRUMENT_INFO.get("AVAXUSDT")
    );
}
