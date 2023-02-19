import pandas as pd
import random
import plotly.graph_objects as go
import os
from plotly.subplots import make_subplots

output_dir = 'C:\\rust_code\\momentum\\backtest_output\\'
filenames = os.listdir(output_dir)
fig = make_subplots(rows=2, cols=1)

for filename in filenames:
    df = pd.read_csv(f'{output_dir}{filename}')
    name = f'{filename}'
    color = (random.randint(100, 255), random.randint(
        100, 255), random.randint(100, 255))
    color_str = 'rgb' + str(color)
    usd_trace = go.Scatter(
        x=df['datetime'],
        y=df['usd_balance'],
        line=dict(width=2, color=color_str),
        name=name,
        legendgroup=name,
        showlegend=False,
    )

    roll_max = df["usd_balance"].rolling(10000, min_periods=1).max()
    daily_dd = df["usd_balance"] / roll_max - 1.0
    max_daily_dd = daily_dd.rolling(10000, min_periods=1).min()
    dd_trace = go.Scatter(
        x=df['datetime'],
        y=daily_dd,
        line=dict(width=2, color=color_str),
        name=name,
        legendgroup=name,
        showlegend=True,  # Only show one legend
    )

    usd_trace.name = name
    dd_trace.name = name
    fig.add_trace(
        usd_trace,
        row=1, col=1
    )
    fig.add_trace(
        dd_trace,
        row=2, col=1
    )


fig.add_trace(
    go.Scatter(
        x=df['datetime'],
        y=df['initial_captial'],
        line=dict(color='gray', width=2),
        name='initial_captial',
    ),
    row=1, col=1
)

fig.update_layout(legend=dict(
    y=0.5, traceorder='reversed', font=dict(size=16)))
fig.show()
