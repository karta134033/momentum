import pandas as pd
import plotly.graph_objects as go
import os
from plotly.subplots import make_subplots

output_dir = 'C:\\rust_code\\momentum\\backtest_output\\'
filenames = os.listdir(output_dir)
fig = make_subplots()

for filename in filenames:
    df = pd.read_csv(f'{output_dir}{filename}')

    fig.add_trace(
        go.Scatter(
            x=df['datetime'],
            y=df['usd_balance'],
            line=dict(width=2),
            name=f'{filename} usd_balance',
        )
    )

fig.add_trace(
    go.Scatter(
        x=df['datetime'],
        y=df['initial_captial'],
        line=dict(color='gray', width=2),
        name='initial_captial',
    )
)
fig.show()
