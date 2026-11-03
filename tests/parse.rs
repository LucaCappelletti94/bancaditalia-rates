//! Readers over responses recorded from the live API.

use bancaditalia_rates::parse;
use bancaditalia_rates::{Convention, Error, Month};
use jiff::civil::date;
use rust_decimal::Decimal;

fn dec(text: &str) -> Decimal {
    text.parse().unwrap()
}

#[test]
fn daily_rates_list_every_quoted_currency() {
    let rates = parse::daily_rates(include_bytes!("fixtures/daily-2026-10-30-eur.json")).unwrap();
    assert_eq!(rates.len(), 171);
    let usd = rates.iter().find(|rate| rate.iso_code == "USD").unwrap();
    assert_eq!(usd.rate, Some(dec("1.1172")));
    assert_eq!(usd.period, date(2026, 10, 30));
    assert_eq!(usd.currency, "U.S. Dollar");
    assert_eq!(usd.convention, Convention::ForeignPerQuotation);

    let integer = rates.iter().find(|rate| rate.iso_code == "LAK").unwrap();
    assert_eq!(integer.rate, Some(Decimal::from(25034)));
    let unpublished = rates.iter().find(|rate| rate.iso_code == "RUB").unwrap();
    assert_eq!(unpublished.rate, None);
}

#[test]
fn daily_rates_against_the_dollar_mix_both_conventions() {
    let rates = parse::daily_rates(include_bytes!("fixtures/daily-2026-10-30-usd.json")).unwrap();
    let gbp = rates.iter().find(|rate| rate.iso_code == "GBP").unwrap();
    assert_eq!(gbp.convention, Convention::QuotationPerForeign);
    let eur = rates.iter().find(|rate| rate.iso_code == "EUR").unwrap();
    assert_eq!(
        (eur.rate, eur.convention),
        (Some(dec("0.8951")), Convention::ForeignPerQuotation)
    );
}

#[test]
fn lira_rates_come_in_italian_and_per_foreign_unit() {
    let rates =
        parse::daily_rates(include_bytes!("fixtures/daily-1950-03-01-itl-it.json")).unwrap();
    let usd = rates.iter().find(|rate| rate.iso_code == "USD").unwrap();
    assert_eq!(usd.country, "STATI UNITI");
    assert_eq!(usd.rate, Some(dec("624.800")));
    assert_eq!(usd.convention, Convention::QuotationPerForeign);
}

#[test]
fn an_unquoted_day_is_empty() {
    assert_eq!(
        parse::daily_rates(include_bytes!("fixtures/daily-2026-11-01-eur.json")).unwrap(),
        Vec::new()
    );
}

#[test]
fn latest_rates_carry_both_quotations() {
    let rates = parse::latest_rates(include_bytes!("fixtures/latest.json")).unwrap();
    assert_eq!(rates.len(), 172);
    let gbp = rates.iter().find(|rate| rate.iso_code == "GBP").unwrap();
    assert_eq!(
        (gbp.eur_rate, gbp.usd_rate),
        (Some(dec("0.83358")), Some(dec("1.3411")))
    );
    assert_eq!(gbp.usd_convention, Some(Convention::QuotationPerForeign));
    assert_eq!(gbp.reference_date, date(2026, 11, 3));
    let usd = rates.iter().find(|rate| rate.iso_code == "USD").unwrap();
    assert_eq!(
        (usd.usd_rate, usd.usd_convention),
        (Some(Decimal::ONE), None)
    );
    let rub = rates.iter().find(|rate| rate.iso_code == "RUB").unwrap();
    assert_eq!((rub.eur_rate, rub.usd_rate), (None, None));
}

#[test]
fn monthly_and_annual_averages_carry_their_period() {
    let monthly =
        parse::monthly_average_rates(include_bytes!("fixtures/monthly-2026-09-eur.json")).unwrap();
    let usd = monthly.iter().find(|rate| rate.iso_code == "USD").unwrap();
    assert_eq!(
        (usd.rate, usd.period),
        (Some(dec("1.1513")), Month::new(2026, 9).unwrap())
    );

    let annual =
        parse::annual_average_rates(include_bytes!("fixtures/annual-2025-eur.json")).unwrap();
    let usd = annual.iter().find(|rate| rate.iso_code == "USD").unwrap();
    assert_eq!((usd.rate, usd.period), (Some(dec("1.1300")), 2025));
}

#[test]
fn daily_series_skip_unquoted_days() {
    let series = parse::daily_time_series(include_bytes!("fixtures/daily-series-usd.json"))
        .unwrap()
        .unwrap();
    assert_eq!(
        (series.iso_code.as_str(), series.convention),
        ("USD", Convention::ForeignPerQuotation)
    );
    let days: Vec<_> = series.points.iter().map(|point| point.period).collect();
    assert_eq!(
        days,
        [date(2026, 10, 29), date(2026, 10, 30), date(2026, 11, 2)]
    );
    assert_eq!(series.points[1].rate, Some(dec("1.1172")));
}

#[test]
fn an_empty_series_is_none() {
    assert_eq!(
        parse::daily_time_series(include_bytes!("fixtures/daily-series-empty.json")).unwrap(),
        None
    );
}

#[test]
fn monthly_and_annual_series_read_their_periods() {
    let monthly = parse::monthly_time_series(include_bytes!("fixtures/monthly-series-usd.json"))
        .unwrap()
        .unwrap();
    let months: Vec<_> = monthly
        .points
        .iter()
        .map(|point| (point.period, point.rate))
        .collect();
    assert_eq!(
        months,
        [
            (Month::new(2026, 8).unwrap(), Some(dec("1.1593"))),
            (Month::new(2026, 9).unwrap(), Some(dec("1.1513")))
        ]
    );

    let annual = parse::annual_time_series(include_bytes!("fixtures/annual-series-usd.json"))
        .unwrap()
        .unwrap();
    let years: Vec<_> = annual
        .points
        .iter()
        .map(|point| (point.period, point.rate))
        .collect();
    assert_eq!(
        years,
        [(2024, Some(dec("1.0824"))), (2025, Some(dec("1.1300")))]
    );
}

#[test]
fn currencies_include_expired_ones() {
    let currencies = parse::currencies(include_bytes!("fixtures/currencies.json")).unwrap();
    assert_eq!(currencies.len(), 212);
    let peseta = currencies
        .iter()
        .find(|currency| currency.iso_code == "ADP")
        .unwrap();
    assert_eq!(peseta.name, "Andorra Peseta");
    assert_eq!(peseta.countries[0].country_iso.as_deref(), Some("AD"));
    assert_eq!(peseta.countries[0].valid_from, date(1999, 2, 1));
    assert_eq!(peseta.countries[0].valid_until, Some(date(2003, 10, 31)));
    let dirham = currencies
        .iter()
        .find(|currency| currency.iso_code == "AED")
        .unwrap();
    assert_eq!(
        (
            dirham.countries[0].country_iso.as_deref(),
            dirham.countries[0].valid_until
        ),
        (None, None)
    );
}

#[test]
fn unreadable_values_are_errors() {
    let rate = |value: &str, code: &str| {
        format!(
            r#"{{"resultsInfo":{{"totalRecords":1}},"rates":[{{"country":"X","currency":"X","isoCode":"XXX","uicCode":"1","avgRate":"{value}","exchangeConvention":"c","exchangeConventionCode":"{code}","referenceDate":"2026-10-30"}}]}}"#
        )
    };
    assert!(matches!(
        parse::daily_rates(rate("1,5", "C").as_bytes()),
        Err(Error::Malformed {
            field: "avgRate",
            ..
        })
    ));
    assert!(matches!(
        parse::daily_rates(rate("1.5", "-").as_bytes()),
        Err(Error::Malformed {
            field: "exchangeConventionCode",
            ..
        })
    ));
}

fn malformed_field<T: std::fmt::Debug>(result: Result<T, Error>) -> &'static str {
    match result {
        Err(Error::Malformed { field, .. }) => field,
        other => panic!("expected a malformed field, got {other:?}"),
    }
}

#[test]
fn a_series_with_an_unknown_convention_is_an_error() {
    let body = br#"{"resultsInfo":{"totalRecords":1,"currency":"U.S. Dollar","isoCode":"USD","uicCode":"001","exchangeConventionCode":"X"},"rates":[{"referenceDate":"2026","avgRate":"1.13","exchangeConvention":"c"}]}"#;
    assert_eq!(
        malformed_field(parse::annual_time_series(body)),
        "exchangeConventionCode"
    );
}

#[test]
fn an_average_for_a_month_outside_the_calendar_is_an_error() {
    let body = br#"{"resultsInfo":{"totalRecords":1},"rates":[{"country":"X","currency":"X","isoCode":"XXX","uicCode":"1","avgRate":"1.5","exchangeConvention":"c","exchangeConventionCode":"C","year":2026,"month":13}]}"#;
    assert_eq!(malformed_field(parse::monthly_average_rates(body)), "month");
}

#[test]
fn a_series_without_its_currency_header_is_an_error() {
    let body = br#"{"resultsInfo":{"totalRecords":1,"currency":"U.S. Dollar","isoCode":"USD","uicCode":"001"},"rates":[{"referenceDate":"2026-10-30","avgRate":"1.1172","exchangeConvention":"c"}]}"#;
    assert_eq!(
        malformed_field(parse::daily_time_series(body)),
        "exchangeConventionCode"
    );
}

#[test]
fn a_series_point_with_an_unreadable_period_is_an_error() {
    let body = br#"{"resultsInfo":{"totalRecords":1,"currency":"U.S. Dollar","isoCode":"USD","uicCode":"001","exchangeConventionCode":"C"},"rates":[{"referenceDate":"2026-13","avgRate":"1.1172","exchangeConvention":"c"}]}"#;
    assert_eq!(
        malformed_field(parse::monthly_time_series(body)),
        "referenceDate"
    );
}

#[test]
fn a_currency_with_an_unreadable_end_date_is_an_error() {
    let body = br#"{"resultsInfo":{"totalRecords":1},"currencies":[{"isoCode":"ADP","name":"Andorra Peseta","graph":false,"countries":[{"currencyISO":"ADP","country":"ANDORRA","countryISO":"AD","validityStartDate":"1999-02-01","validityEndDate":"31/10/2003"}]}]}"#;
    assert_eq!(malformed_field(parse::currencies(body)), "validityEndDate");
}

#[test]
fn months_parse_and_print_as_year_dash_month() {
    let month: Month = "2026-09".parse().unwrap();
    assert_eq!((month.year(), month.month()), (2026, 9));
    assert_eq!(month.to_string(), "2026-09");
    for invalid in ["2026-13", "2026-00", "2026", "2026-9x", "-09"] {
        assert!(
            invalid.parse::<Month>().is_err(),
            "{invalid:?} was accepted"
        );
    }
}
