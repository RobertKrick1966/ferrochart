# Axum OHLCV Endpoint Example

Add `powerchart-core` as a dependency with serde support:

```toml
[dependencies]
powerchart-core = { git = "https://github.com/RobertKrick1966/powerchart", features = ["serde"] }
```

## Endpoint

```rust
use axum::{extract::Query, Json};
use powerchart_core::Ohlcv;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ChartQuery {
    pub symbol: String,
    pub interval: Option<String>,  // "1d", "1h", etc.
    pub limit: Option<usize>,
}

pub async fn get_ohlcv(
    Query(params): Query<ChartQuery>,
) -> Json<Vec<Ohlcv>> {
    let limit = params.limit.unwrap_or(200);

    // Fetch from your data source (database, API, etc.)
    let data: Vec<Ohlcv> = fetch_ohlcv(&params.symbol, limit).await;

    Json(data)
}
```

## Response Format

```json
[
  {
    "timestamp": 1700000000,
    "open": 100.0,
    "high": 110.5,
    "low": 98.2,
    "close": 108.3,
    "volume": 15000.0
  }
]
```

The frontend fetches this JSON and passes it to `PowerChart.setData()`.
