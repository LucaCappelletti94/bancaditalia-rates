#![doc = include_str!("../README.md")]

mod model;
pub mod parse;

use jiff::civil::Date;
use reqwest::header::ACCEPT;
use url::Url;

pub use model::{
    AnnualAverageRate, Convention, CurrencyInfo, CurrencyUse, DailyRate, Language, LatestRate,
    Month, MonthError, MonthlyAverageRate, Point, Quotation, Rate, TimeSeries,
};

/// Base URL of the REST API, version 1.0.
pub const BASE_URL: &str = "https://tassidicambio.bancaditalia.it/terzevalute-wf-web/rest/v1.0/";

/// Failure to fetch or read rates.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request failed.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// The service answered with an error status. Invalid parameters give 400 with a message in the body.
    #[error("Banca d'Italia answered {status}: {body}")]
    Status {
        /// HTTP status.
        status: u16,
        /// Response body.
        body: String,
    },
    /// The body is not the documented JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A field holds a value that cannot be read.
    #[error("cannot read {field} {value:?}")]
    Malformed {
        /// Field name in the response.
        field: &'static str,
        /// Its value.
        value: String,
    },
}

/// Client for the exchange rates REST API.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base: Url,
    language: Language,
}

impl Client {
    /// A client for [`BASE_URL`], answering in English.
    ///
    /// # Panics
    ///
    /// Never: [`BASE_URL`] is a valid base URL.
    #[must_use]
    pub fn new(http: reqwest::Client) -> Self {
        Self::with_base_url(http, Url::parse(BASE_URL).expect("BASE_URL is a valid URL"))
            .expect("BASE_URL is a valid base URL")
    }

    /// A client for another deployment of the same API, such as a caching proxy.
    ///
    /// # Errors
    ///
    /// Returns [`url::ParseError::RelativeUrlWithCannotBeABaseBase`] when `base` cannot be a base URL.
    pub fn with_base_url(http: reqwest::Client, mut base: Url) -> Result<Self, url::ParseError> {
        if base.cannot_be_a_base() {
            return Err(url::ParseError::RelativeUrlWithCannotBeABaseBase);
        }
        if !base.path().ends_with('/') {
            let path = format!("{}/", base.path());
            base.set_path(&path);
        }
        Ok(Self {
            http,
            base,
            language: Language::English,
        })
    }

    /// The same client answering in `language`.
    #[must_use]
    pub const fn with_language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    async fn get(&self, endpoint: &str, query: &[(&str, &str)]) -> Result<Vec<u8>, Error> {
        let url = self
            .base
            .join(endpoint)
            .expect("endpoint names are relative paths");
        let response = self
            .http
            .get(url)
            .query(query)
            .query(&[("lang", self.language.code())])
            .header(ACCEPT, "application/json")
            .send()
            .await?;
        let status = response.status();
        let body = response.bytes().await?;
        if !status.is_success() {
            return Err(Error::Status {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&body).into_owned(),
            });
        }
        Ok(body.to_vec())
    }

    /// Rates against the euro and the US dollar on the latest day quoted for every currency.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the request fails, the service answers with an error status, or the body cannot be read.
    pub async fn latest_rates(&self) -> Result<Vec<LatestRate>, Error> {
        parse::latest_rates(&self.get("latestRates", &[]).await?)
    }

    /// Rates quoted on `date` against `quotation`, for `currencies` or for every currency when empty. A day without quotes gives an empty list.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the request fails, the service answers with an error status, or the body cannot be read.
    pub async fn daily_rates(
        &self,
        date: Date,
        quotation: Quotation,
        currencies: &[&str],
    ) -> Result<Vec<DailyRate>, Error> {
        let date = date.to_string();
        let query = with_currencies(
            vec![
                ("referenceDate", date.as_str()),
                ("currencyIsoCode", quotation.iso_code()),
            ],
            currencies,
        );
        parse::daily_rates(&self.get("dailyRates", &query).await?)
    }

    /// Average rates of `month` against `quotation`, for `currencies` or for every currency when empty.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the request fails, the service answers with an error status, or the body cannot be read.
    pub async fn monthly_average_rates(
        &self,
        month: Month,
        quotation: Quotation,
        currencies: &[&str],
    ) -> Result<Vec<MonthlyAverageRate>, Error> {
        let (year, number) = (month.year().to_string(), month.month().to_string());
        let query = with_currencies(
            vec![
                ("month", number.as_str()),
                ("year", year.as_str()),
                ("currencyIsoCode", quotation.iso_code()),
            ],
            currencies,
        );
        parse::monthly_average_rates(&self.get("monthlyAverageRates", &query).await?)
    }

    /// Average rates of `year` against `quotation`, for `currencies` or for every currency when empty.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the request fails, the service answers with an error status, or the body cannot be read.
    pub async fn annual_average_rates(
        &self,
        year: i16,
        quotation: Quotation,
        currencies: &[&str],
    ) -> Result<Vec<AnnualAverageRate>, Error> {
        let year = year.to_string();
        let query = with_currencies(
            vec![
                ("year", year.as_str()),
                ("currencyIsoCode", quotation.iso_code()),
            ],
            currencies,
        );
        parse::annual_average_rates(&self.get("annualAverageRates", &query).await?)
    }

    /// Daily rates of `currency` against `quotation` from `start` to `end` inclusive, `None` when no day was quoted.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the request fails, the service answers with an error status (such as `end` before `start`), or the body cannot be read.
    pub async fn daily_time_series(
        &self,
        currency: &str,
        quotation: Quotation,
        start: Date,
        end: Date,
    ) -> Result<Option<TimeSeries<Date>>, Error> {
        let (start, end) = (start.to_string(), end.to_string());
        let query = [
            ("startDate", start.as_str()),
            ("endDate", end.as_str()),
            ("baseCurrencyIsoCode", currency),
            ("currencyIsoCode", quotation.iso_code()),
        ];
        parse::daily_time_series(&self.get("dailyTimeSeries", &query).await?)
    }

    /// Monthly average rates of `currency` against `quotation` from `start` to `end` inclusive, `None` when no month was quoted.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the request fails, the service answers with an error status (such as `end` before `start`), or the body cannot be read.
    pub async fn monthly_time_series(
        &self,
        currency: &str,
        quotation: Quotation,
        start: Month,
        end: Month,
    ) -> Result<Option<TimeSeries<Month>>, Error> {
        let fields = [start.month(), end.month()].map(|month| month.to_string());
        let years = [start.year(), end.year()].map(|year| year.to_string());
        let query = [
            ("startMonth", fields[0].as_str()),
            ("startYear", years[0].as_str()),
            ("endMonth", fields[1].as_str()),
            ("endYear", years[1].as_str()),
            ("baseCurrencyIsoCode", currency),
            ("currencyIsoCode", quotation.iso_code()),
        ];
        parse::monthly_time_series(&self.get("monthlyTimeSeries", &query).await?)
    }

    /// Annual average rates of `currency` against `quotation` from `start` to `end` inclusive, `None` when no year was quoted.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the request fails, the service answers with an error status (such as `end` before `start`), or the body cannot be read.
    pub async fn annual_time_series(
        &self,
        currency: &str,
        quotation: Quotation,
        start: i16,
        end: i16,
    ) -> Result<Option<TimeSeries<i16>>, Error> {
        let (start, end) = (start.to_string(), end.to_string());
        let query = [
            ("startYear", start.as_str()),
            ("endYear", end.as_str()),
            ("baseCurrencyIsoCode", currency),
            ("currencyIsoCode", quotation.iso_code()),
        ];
        parse::annual_time_series(&self.get("annualTimeSeries", &query).await?)
    }

    /// Every currency Banca d'Italia knows, current or expired.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the request fails, the service answers with an error status, or the body cannot be read.
    pub async fn currencies(&self) -> Result<Vec<CurrencyInfo>, Error> {
        parse::currencies(&self.get("currencies", &[]).await?)
    }
}

fn with_currencies<'q>(
    mut query: Vec<(&'q str, &'q str)>,
    currencies: &[&'q str],
) -> Vec<(&'q str, &'q str)> {
    query.extend(currencies.iter().map(|code| ("baseCurrencyIsoCode", *code)));
    query
}
