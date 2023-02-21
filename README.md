## Backtest
cargo run --bin momentum -- -b ./backtest_config.json -s ./backtest_setting_config.json -m b

## Hypertune
cargo run --bin momentum -- -t ./hypertune_config.json -s ./hypertune_setting_config.json -m h

## Live trade
cargo run --bin live_trade -- -b ./backtest_4h_config.json -s ./setting_4h_config.json -m l

## Compare backtest results
python plot_backtest.py

config example:
setting_config.json
```
{
    "from": "2019-01-01 00:00:00",
    "to": "2022-12-31 00:00:00",
    "collection": "AVAXUSDT_1d"
}
```

backtest_config.json
```
{
    "initial_captial": 10000.0,
    "fee_rate": 0.0004,
    "entry_portion": 0.2,
    "look_back_count": 10,
    "risk_portion": 0.05,
    "win_ratio": 4.0
}
```

hypertune_config.json
```
{
    "initial_captial": 10000.0,
    "fee_rate": 0.0004,
    "entry_portion": 0.2,
    "look_back_count": 10,
    "risk_portion": {
        "min": 0.01,
        "max": 0.05,
        "step": 0.002
    },
    "win_ratio": {
        "min": 1.0,
        "max": 5.0,
        "step": 0.02
    }
}
```