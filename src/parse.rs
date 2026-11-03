//! Readers for JSON response bodies, for responses fetched or stored outside [`Client`](crate::Client).

use jiff::civil::Date;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::Error;
use crate::model::{
    AnnualAverageRate, Convention, CurrencyInfo, CurrencyUse, DailyRate, LatestRate, Month,
    MonthlyAverageRate, Point, Rate, TimeSeries,
};

fn malformed(field: &'static str, value: &str) -> Error {
    Error::Malformed {
        field,
        value: value.to_owned(),
    }
}

fn rate(field: &'static str, text: &str) -> Result<Option<Decimal>, Error> {
    match text {
        "N.A." => Ok(None),
        _ => text.parse().map(Some).map_err(|_| malformed(field, text)),
    }
}

fn date(field: &'static str, text: &str) -> Result<Date, Error> {
    text.parse().map_err(|_| malformed(field, text))
}

fn convention(text: &str) -> Result<Convention, Error> {
    Convention::from_code(text).ok_or_else(|| malformed("exchangeConventionCode", text))
}

fn json<T: DeserializeOwned>(body: &[u8]) -> Result<T, Error> {
    Ok(serde_json::from_slice(body)?)
}

#[derive(Deserialize)]
struct Rates<R> {
    rates: Vec<R>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRate {
    country: String,
    currency: String,
    iso_code: String,
    uic_code: String,
    avg_rate: String,
    exchange_convention: String,
    exchange_convention_code: String,
}

impl RawRate {
    fn with_period<P>(self, period: P) -> Result<Rate<P>, Error> {
        Ok(Rate {
            rate: rate("avgRate", &self.avg_rate)?,
            convention: convention(&self.exchange_convention_code)?,
            country: self.country,
            currency: self.currency,
            iso_code: self.iso_code,
            uic_code: self.uic_code,
            convention_text: self.exchange_convention,
            period,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDailyRate {
    #[serde(flatten)]
    rate: RawRate,
    reference_date: String,
}

#[derive(Deserialize)]
struct RawMonthlyRate {
    #[serde(flatten)]
    rate: RawRate,
    year: i16,
    month: i8,
}

#[derive(Deserialize)]
struct RawAnnualRate {
    #[serde(flatten)]
    rate: RawRate,
    year: i16,
}

/// Reads a `dailyRates` response. A day without quotes gives an empty list.
///
/// # Errors
///
/// Returns [`Error::Json`] when the body is not the documented JSON and [`Error::Malformed`] when a value cannot be read.
pub fn daily_rates(body: &[u8]) -> Result<Vec<DailyRate>, Error> {
    json::<Rates<RawDailyRate>>(body)?
        .rates
        .into_iter()
        .map(|raw| {
            let day = date("referenceDate", &raw.reference_date)?;
            raw.rate.with_period(day)
        })
        .collect()
}

/// Reads a `monthlyAverageRates` response.
///
/// # Errors
///
/// Returns [`Error::Json`] when the body is not the documented JSON and [`Error::Malformed`] when a value cannot be read.
pub fn monthly_average_rates(body: &[u8]) -> Result<Vec<MonthlyAverageRate>, Error> {
    json::<Rates<RawMonthlyRate>>(body)?
        .rates
        .into_iter()
        .map(|raw| {
            let month = Month::new(raw.year, raw.month)
                .map_err(|_| malformed("month", &raw.month.to_string()))?;
            raw.rate.with_period(month)
        })
        .collect()
}

/// Reads an `annualAverageRates` response.
///
/// # Errors
///
/// Returns [`Error::Json`] when the body is not the documented JSON and [`Error::Malformed`] when a value cannot be read.
pub fn annual_average_rates(body: &[u8]) -> Result<Vec<AnnualAverageRate>, Error> {
    json::<Rates<RawAnnualRate>>(body)?
        .rates
        .into_iter()
        .map(|raw| raw.rate.with_period(raw.year))
        .collect()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Latest {
    latest_rates: Vec<RawLatestRate>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLatestRate {
    country: String,
    currency: String,
    iso_code: String,
    uic_code: String,
    eur_rate: String,
    usd_rate: String,
    usd_exchange_convention: String,
    usd_exchange_convention_code: String,
    reference_date: String,
}

/// Reads a `latestRates` response.
///
/// # Errors
///
/// Returns [`Error::Json`] when the body is not the documented JSON and [`Error::Malformed`] when a value cannot be read.
pub fn latest_rates(body: &[u8]) -> Result<Vec<LatestRate>, Error> {
    json::<Latest>(body)?
        .latest_rates
        .into_iter()
        .map(|raw| {
            let usd_convention = match raw.usd_exchange_convention_code.as_str() {
                "-" => None,
                code => Some(convention(code)?),
            };
            Ok(LatestRate {
                eur_rate: rate("eurRate", &raw.eur_rate)?,
                usd_rate: rate("usdRate", &raw.usd_rate)?,
                usd_convention,
                reference_date: date("referenceDate", &raw.reference_date)?,
                country: raw.country,
                currency: raw.currency,
                iso_code: raw.iso_code,
                uic_code: raw.uic_code,
                usd_convention_text: raw.usd_exchange_convention,
            })
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Series {
    results_info: SeriesInfo,
    rates: Vec<RawPoint>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeriesInfo {
    currency: Option<String>,
    iso_code: Option<String>,
    uic_code: Option<String>,
    exchange_convention_code: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPoint {
    reference_date: String,
    avg_rate: String,
    exchange_convention: String,
}

fn time_series<P>(
    body: &[u8],
    period: impl Fn(&str) -> Result<P, Error>,
) -> Result<Option<TimeSeries<P>>, Error> {
    let series: Series = json(body)?;
    if series.rates.is_empty() {
        return Ok(None);
    }
    let info = series.results_info;
    let missing = |field| malformed(field, "");
    let points = series
        .rates
        .into_iter()
        .map(|raw| {
            Ok(Point {
                period: period(&raw.reference_date)?,
                rate: rate("avgRate", &raw.avg_rate)?,
                convention_text: raw.exchange_convention,
            })
        })
        .collect::<Result<_, Error>>()?;
    Ok(Some(TimeSeries {
        convention: convention(
            &info
                .exchange_convention_code
                .ok_or_else(|| missing("exchangeConventionCode"))?,
        )?,
        currency: info.currency.ok_or_else(|| missing("currency"))?,
        iso_code: info.iso_code.ok_or_else(|| missing("isoCode"))?,
        uic_code: info.uic_code.ok_or_else(|| missing("uicCode"))?,
        points,
    }))
}

/// Reads a `dailyTimeSeries` response, `None` when no day in the range was quoted.
///
/// # Errors
///
/// Returns [`Error::Json`] when the body is not the documented JSON and [`Error::Malformed`] when a value cannot be read.
pub fn daily_time_series(body: &[u8]) -> Result<Option<TimeSeries<Date>>, Error> {
    time_series(body, |text| date("referenceDate", text))
}

/// Reads a `monthlyTimeSeries` response, `None` when no month in the range was quoted.
///
/// # Errors
///
/// Returns [`Error::Json`] when the body is not the documented JSON and [`Error::Malformed`] when a value cannot be read.
pub fn monthly_time_series(body: &[u8]) -> Result<Option<TimeSeries<Month>>, Error> {
    time_series(body, |text| {
        text.parse().map_err(|_| malformed("referenceDate", text))
    })
}

/// Reads an `annualTimeSeries` response, `None` when no year in the range was quoted.
///
/// # Errors
///
/// Returns [`Error::Json`] when the body is not the documented JSON and [`Error::Malformed`] when a value cannot be read.
pub fn annual_time_series(body: &[u8]) -> Result<Option<TimeSeries<i16>>, Error> {
    time_series(body, |text| {
        text.parse().map_err(|_| malformed("referenceDate", text))
    })
}

#[derive(Deserialize)]
struct Currencies {
    currencies: Vec<RawCurrency>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCurrency {
    iso_code: String,
    name: String,
    graph: bool,
    countries: Vec<RawCurrencyUse>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCurrencyUse {
    country: String,
    #[serde(rename = "countryISO")]
    country_iso: Option<String>,
    validity_start_date: String,
    validity_end_date: Option<String>,
}

/// Reads a `currencies` response.
///
/// # Errors
///
/// Returns [`Error::Json`] when the body is not the documented JSON and [`Error::Malformed`] when a date cannot be read.
pub fn currencies(body: &[u8]) -> Result<Vec<CurrencyInfo>, Error> {
    json::<Currencies>(body)?
        .currencies
        .into_iter()
        .map(|raw| {
            let countries = raw
                .countries
                .into_iter()
                .map(|use_| {
                    Ok(CurrencyUse {
                        valid_from: date("validityStartDate", &use_.validity_start_date)?,
                        valid_until: use_
                            .validity_end_date
                            .as_deref()
                            .map(|end| date("validityEndDate", end))
                            .transpose()?,
                        country: use_.country,
                        country_iso: use_.country_iso,
                    })
                })
                .collect::<Result<_, Error>>()?;
            Ok(CurrencyInfo {
                iso_code: raw.iso_code,
                name: raw.name,
                graph: raw.graph,
                countries,
            })
        })
        .collect()
}
