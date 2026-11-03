# bancaditalia-rates

[![CI](https://github.com/LucaCappelletti94/bancaditalia-rates/actions/workflows/ci.yml/badge.svg)](https://github.com/LucaCappelletti94/bancaditalia-rates/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/bancaditalia-rates.svg)](https://crates.io/crates/bancaditalia-rates)
[![docs.rs](https://docs.rs/bancaditalia-rates/badge.svg)](https://docs.rs/bancaditalia-rates)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/LucaCappelletti94/bancaditalia-rates/blob/main/LICENSE)

Typed async client for the [Banca d'Italia exchange rates REST API](https://tassidicambio.bancaditalia.it/terzevalute-wf-ui-web/assets/files/Operating_Instructions.pdf), the reference rates Italian tax law converts foreign-currency amounts with. It covers every endpoint: latest rates, daily rates, monthly and annual averages, the three time series, and the currency list, in English or Italian.

`Client` fetches, and the `parse` module reads response bodies on their own:

```rust
use bancaditalia_rates::{Convention, parse};

let body = br#"{"resultsInfo":{"totalRecords":1},"rates":[{"country":"UNITED STATES","currency":"U.S. Dollar","isoCode":"USD","uicCode":"001","avgRate":"1.1172","exchangeConvention":"Foreign currency amount for 1 Euro.","exchangeConventionCode":"C","referenceDate":"2026-10-30"}]}"#;
let rates = parse::daily_rates(body).unwrap();
assert_eq!(rates[0].iso_code, "USD");
assert_eq!(rates[0].rate, Some("1.1172".parse().unwrap()));
assert_eq!(rates[0].convention, Convention::ForeignPerQuotation);
```

Rates are quoted one of two ways, told apart by `Convention`: most currencies as units per euro or dollar, a few (pound sterling against the dollar, every currency against the lira) as euro, dollars or lire per unit. Days without quotes give an empty list, and rates published as "N.A." read as `None`.
