# bancaditalia-rates

[![CI](https://github.com/LucaCappelletti94/bancaditalia-rates/actions/workflows/ci.yml/badge.svg)](https://github.com/LucaCappelletti94/bancaditalia-rates/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/LucaCappelletti94/bancaditalia-rates/graph/badge.svg)](https://codecov.io/gh/LucaCappelletti94/bancaditalia-rates)
[![crates.io](https://img.shields.io/crates/v/bancaditalia-rates.svg)](https://crates.io/crates/bancaditalia-rates)
[![docs.rs](https://docs.rs/bancaditalia-rates/badge.svg)](https://docs.rs/bancaditalia-rates)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/LucaCappelletti94/bancaditalia-rates/blob/main/LICENSE)

Async client for the [Banca d'Italia exchange rates API](https://tassidicambio.bancaditalia.it/terzevalute-wf-ui-web/assets/files/Operating_Instructions.pdf): daily, monthly and annual rates for about 150 currencies against the euro, the dollar or the lira, back to 1918. No API key needed.

```rust
use bancaditalia_rates::{Client, Convention, Quotation};
use jiff::civil::date;
use rust_decimal::Decimal;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), bancaditalia_rates::Error> {
    let client = Client::new(reqwest::Client::new());

    let rates = client.daily_rates(date(2026, 10, 30), Quotation::Euro, &["USD"]).await?;
    let usd = &rates[0];
    assert_eq!(usd.convention, Convention::ForeignPerQuotation);
    let per_euro = usd.rate.expect("USD is quoted");
    assert_eq!((Decimal::from(100) / per_euro).round_dp(2), "89.51".parse::<Decimal>().unwrap());

    let week = client
        .daily_time_series("USD", Quotation::Euro, date(2026, 10, 26), date(2026, 11, 1))
        .await?
        .expect("USD was quoted that week");
    assert_eq!(week.points.len(), 5);
    Ok(())
}
```

Most rates are units per euro or dollar, and `Convention::QuotationPerForeign` marks the reversed ones. Days without quotes and unknown currency codes give empty results. The `parse` module reads stored responses without a client.
