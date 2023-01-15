use anyhow::Result;
use chrono::Utc;
use hmac::Hmac;
use hmac::Mac;
use log::info;
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashMap;
use trade_utils::types::kline::Kline;

use crate::binance::parse_api_kline;

pub const FUTURES_KLINE: &str = "/fapi/v1/klines";
pub const FUTURES_ACCOUNT: &str = "/fapi/v2/account";
pub const FUTURES_BASE: &str = "https://fapi.binance.com";

pub struct BinanceFuturesApiClient {
    client: reqwest::Client,
}

impl BinanceFuturesApiClient {
    pub fn new() -> BinanceFuturesApiClient {
        BinanceFuturesApiClient {
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_klines(
        &self,
        symbol: &str,
        interval: &str, // "1d, 1h, 1m"
        start_time: &str,
        end_time: Option<&str>,
        limit: Option<&str>,
    ) -> Result<Vec<Kline>> {
        let mut params = HashMap::new();
        params.insert("symbol", symbol);
        params.insert("interval", interval);
        params.insert("startTime", start_time);
        if end_time.is_some() {
            params.insert("end_time", end_time.unwrap());
        }
        if limit.is_some() {
            params.insert("limit", limit.unwrap());
        }

        let endpoint = format!("{}{}", FUTURES_BASE, FUTURES_KLINE);
        let request_url = reqwest::Url::parse_with_params(endpoint.as_str(), &params).unwrap();
        println!("request_url: {:?}", request_url.to_string());

        let response = self.client.get(request_url).send().await?;
        let content = response.text().await?;
        let values: Vec<Value> = serde_json::from_str(content.as_str())?;
        let klines = values
            .into_iter()
            .map(|value| parse_api_kline(value).unwrap())
            .collect::<Vec<_>>();
        Ok(klines)
    }

    pub async fn get_account(&self, api_key: &str, secret_key: &str) -> Result<Value> {
        let timestamp = Utc::now().timestamp_millis();
        let mut params = Vec::new();
        params.push(("timestamp".to_owned(), timestamp.to_string()));

        let request_string = format!("timestamp={}", timestamp);
        let mut signed_key = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes()).unwrap();
        signed_key.update(request_string.as_bytes());

        let signature = hex::encode(signed_key.finalize().into_bytes());
        info!("signed_key: {}", signature);
        params.push(("signature".to_owned(), signature));

        let endpoint = format!("{}{}", FUTURES_BASE, FUTURES_ACCOUNT);

        let request_url = reqwest::Url::parse_with_params(endpoint.as_str(), &params).unwrap();
        info!("request_url: {:?}", request_url.to_string());
        let response = self
            .client
            .get(request_url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await?;
        let content = response.text().await?;
        let value: Value = serde_json::from_str(content.as_str())?;
        Ok(value)
    }
}
