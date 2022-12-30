## Backtest
cargo run -- -b ./backtest_config.json -m b

config example:
```
{
    "from": "2020-01-01 00:00:00",
    "to": "2022-12-01 00:00:00",
    "initial_captial": 10000.0,
    "fee_rate": 0.0004,
    "entry_portion": 0.3,
    "look_back_count": 10,
    "risk_portion": 0.05,
    "win_ratio": 1.5
}
```