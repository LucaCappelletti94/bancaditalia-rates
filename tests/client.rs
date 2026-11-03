//! Requests `Client` sends, checked against the parameters the API documents, answered with recorded responses.

use bancaditalia_rates::{Client, Error, Language, Month, Quotation};
use jiff::civil::date;
use reqwest::StatusCode;
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

/// Matches a request whose query holds exactly these pairs, in any order.
struct ExactQuery(Vec<(String, String)>);

impl ExactQuery {
    fn new(pairs: &[(&str, &str)]) -> Self {
        let mut pairs: Vec<_> = pairs
            .iter()
            .map(|&(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        pairs.sort();
        Self(pairs)
    }
}

impl Match for ExactQuery {
    fn matches(&self, request: &Request) -> bool {
        let mut pairs: Vec<_> = request
            .url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        pairs.sort();
        pairs == self.0
    }
}

/// A server answering `endpoint` with `body` only for exactly `query`, expecting one such request.
async fn serve(endpoint: &str, query: &[(&str, &str)], body: &[u8]) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(endpoint))
        .and(header("accept", "application/json"))
        .and(ExactQuery::new(query))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .expect(1)
        .mount(&server)
        .await;
    server
}

fn client(server: &MockServer) -> Client {
    Client::with_base_url(reqwest::Client::new(), Url::parse(&server.uri()).unwrap()).unwrap()
}

#[tokio::test]
async fn latest_rates_ask_for_the_language_only() {
    let server = serve(
        "/latestRates",
        &[("lang", "en")],
        include_bytes!("fixtures/latest.json"),
    )
    .await;
    assert_eq!(client(&server).latest_rates().await.unwrap().len(), 172);
}

#[tokio::test]
async fn daily_rates_repeat_the_currency_parameter() {
    let query = [
        ("referenceDate", "2026-10-30"),
        ("currencyIsoCode", "EUR"),
        ("baseCurrencyIsoCode", "USD"),
        ("baseCurrencyIsoCode", "CHF"),
        ("lang", "en"),
    ];
    let server = serve(
        "/dailyRates",
        &query,
        include_bytes!("fixtures/daily-2026-10-30-eur.json"),
    )
    .await;
    let rates = client(&server)
        .daily_rates(date(2026, 10, 30), Quotation::Euro, &["USD", "CHF"])
        .await
        .unwrap();
    assert_eq!(rates.len(), 171);
}

#[tokio::test]
async fn daily_rates_against_the_dollar_omit_currencies_when_none_are_asked() {
    let query = [
        ("referenceDate", "2026-10-30"),
        ("currencyIsoCode", "USD"),
        ("lang", "en"),
    ];
    let server = serve(
        "/dailyRates",
        &query,
        include_bytes!("fixtures/daily-2026-10-30-usd.json"),
    )
    .await;
    let rates = client(&server)
        .daily_rates(date(2026, 10, 30), Quotation::UsDollar, &[])
        .await
        .unwrap();
    assert_eq!(rates.len(), 171);
}

#[tokio::test]
async fn italian_lira_rates_in_italian() {
    let query = [
        ("referenceDate", "1950-03-01"),
        ("currencyIsoCode", "ITL"),
        ("baseCurrencyIsoCode", "USD"),
        ("lang", "it"),
    ];
    let server = serve(
        "/dailyRates",
        &query,
        include_bytes!("fixtures/daily-1950-03-01-itl-it.json"),
    )
    .await;
    let rates = client(&server)
        .with_language(Language::Italian)
        .daily_rates(date(1950, 3, 1), Quotation::ItalianLira, &["USD"])
        .await
        .unwrap();
    assert_eq!(rates[0].country, "STATI UNITI");
}

#[tokio::test]
async fn monthly_average_rates_split_month_and_year() {
    let query = [
        ("month", "9"),
        ("year", "2026"),
        ("currencyIsoCode", "EUR"),
        ("lang", "en"),
    ];
    let server = serve(
        "/monthlyAverageRates",
        &query,
        include_bytes!("fixtures/monthly-2026-09-eur.json"),
    )
    .await;
    let rates = client(&server)
        .monthly_average_rates(Month::new(2026, 9).unwrap(), Quotation::Euro, &[])
        .await
        .unwrap();
    assert_eq!(rates.len(), 171);
}

#[tokio::test]
async fn annual_average_rates_ask_for_the_year() {
    let query = [
        ("year", "2025"),
        ("currencyIsoCode", "EUR"),
        ("baseCurrencyIsoCode", "USD"),
        ("lang", "en"),
    ];
    let server = serve(
        "/annualAverageRates",
        &query,
        include_bytes!("fixtures/annual-2025-eur.json"),
    )
    .await;
    let rates = client(&server)
        .annual_average_rates(2025, Quotation::Euro, &["USD"])
        .await
        .unwrap();
    assert_eq!(rates.len(), 173);
}

#[tokio::test]
async fn daily_time_series_ask_for_a_date_range() {
    let query = [
        ("startDate", "2026-10-29"),
        ("endDate", "2026-11-02"),
        ("baseCurrencyIsoCode", "USD"),
        ("currencyIsoCode", "EUR"),
        ("lang", "en"),
    ];
    let server = serve(
        "/dailyTimeSeries",
        &query,
        include_bytes!("fixtures/daily-series-usd.json"),
    )
    .await;
    let series = client(&server)
        .daily_time_series(
            "USD",
            Quotation::Euro,
            date(2026, 10, 29),
            date(2026, 11, 2),
        )
        .await
        .unwrap();
    assert_eq!(series.unwrap().points.len(), 3);
}

#[tokio::test]
async fn monthly_time_series_ask_for_a_month_range() {
    let query = [
        ("startMonth", "8"),
        ("startYear", "2026"),
        ("endMonth", "9"),
        ("endYear", "2026"),
        ("baseCurrencyIsoCode", "USD"),
        ("currencyIsoCode", "EUR"),
        ("lang", "en"),
    ];
    let server = serve(
        "/monthlyTimeSeries",
        &query,
        include_bytes!("fixtures/monthly-series-usd.json"),
    )
    .await;
    let series = client(&server)
        .monthly_time_series(
            "USD",
            Quotation::Euro,
            Month::new(2026, 8).unwrap(),
            Month::new(2026, 9).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(series.unwrap().points.len(), 2);
}

#[tokio::test]
async fn annual_time_series_ask_for_a_year_range() {
    let query = [
        ("startYear", "2024"),
        ("endYear", "2025"),
        ("baseCurrencyIsoCode", "USD"),
        ("currencyIsoCode", "EUR"),
        ("lang", "en"),
    ];
    let server = serve(
        "/annualTimeSeries",
        &query,
        include_bytes!("fixtures/annual-series-usd.json"),
    )
    .await;
    let series = client(&server)
        .annual_time_series("USD", Quotation::Euro, 2024, 2025)
        .await
        .unwrap();
    assert_eq!(series.unwrap().points.len(), 2);
}

#[tokio::test]
async fn currencies_ask_for_the_language_only() {
    let server = serve(
        "/currencies",
        &[("lang", "en")],
        include_bytes!("fixtures/currencies.json"),
    )
    .await;
    assert_eq!(client(&server).currencies().await.unwrap().len(), 212);
}

#[tokio::test]
async fn a_base_url_without_trailing_slash_keeps_its_path() {
    let server = serve(
        "/proxy/latestRates",
        &[("lang", "en")],
        include_bytes!("fixtures/latest.json"),
    )
    .await;
    let base = Url::parse(&format!("{}/proxy", server.uri())).unwrap();
    let client = Client::with_base_url(reqwest::Client::new(), base).unwrap();
    assert_eq!(client.latest_rates().await.unwrap().len(), 172);
}

#[test]
fn a_base_url_must_be_able_to_hold_paths() {
    let base = Url::parse("mailto:rates@example.org").unwrap();
    assert!(Client::with_base_url(reqwest::Client::new(), base).is_err());
}

#[tokio::test]
async fn error_statuses_carry_the_service_message() {
    let server = MockServer::start().await;
    Mock::given(path("/dailyTimeSeries"))
        .respond_with(ResponseTemplate::new(400).set_body_string("WARNING! Invalid date range"))
        .mount(&server)
        .await;
    let error = client(&server)
        .daily_time_series("USD", Quotation::Euro, date(2026, 11, 2), date(2026, 11, 1))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        Error::Status { status: StatusCode::BAD_REQUEST, ref body } if body == "WARNING! Invalid date range"
    ));
}
