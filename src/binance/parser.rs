use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use trade_utils::types::kline::Kline;

pub fn parse_api_kline(value: Value) -> Result<Kline> {
    let mut kline = Kline {
        ..Default::default()
    };
    let key_err = |index: usize| -> anyhow::Error {
        anyhow!(
            "Invalid type in index \"{}\", couldn't parse its value from {}",
            index,
            value
        )
    };
    kline.open_timestamp = value[0].as_i64().context(key_err(0))?;
    kline.open = value[1].as_str().context(key_err(1))?.parse()?;
    kline.high = value[2].as_str().context(key_err(2))?.parse()?;
    kline.low = value[3].as_str().context(key_err(3))?.parse()?;
    kline.close = value[4].as_str().context(key_err(4))?.parse()?;
    kline.close_timestamp = value[6].as_i64().context(key_err(6))?;

    Ok(kline)
}
