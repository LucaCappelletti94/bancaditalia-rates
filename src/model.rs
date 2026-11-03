use std::fmt;
use std::str::FromStr;

use jiff::civil::Date;
use rust_decimal::Decimal;

/// The currency rates are quoted against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quotation {
    /// The euro.
    Euro,
    /// The US dollar.
    UsDollar,
    /// The Italian lira, for rates before the euro.
    ItalianLira,
}

impl Quotation {
    pub(crate) const fn iso_code(self) -> &'static str {
        match self {
            Self::Euro => "EUR",
            Self::UsDollar => "USD",
            Self::ItalianLira => "ITL",
        }
    }
}

/// Language of country names, currency names and convention texts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    /// English.
    #[default]
    English,
    /// Italian.
    Italian,
}

impl Language {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Italian => "it",
        }
    }
}

/// Which way a rate is quoted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Convention {
    /// Units of the foreign currency per one unit of the quotation currency, code `C`, such as 1.1172 US dollars per euro.
    ForeignPerQuotation,
    /// Units of the quotation currency per one unit of the foreign currency, code `I`, such as 1.3411 US dollars per pound sterling.
    QuotationPerForeign,
}

impl Convention {
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        match code {
            "C" => Some(Self::ForeignPerQuotation),
            "I" => Some(Self::QuotationPerForeign),
            _ => None,
        }
    }
}

/// A calendar month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Month {
    year: i16,
    month: i8,
}

/// A month outside 1 to 12, or text not of the form `YYYY-MM`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a month of the form YYYY-MM")]
pub struct MonthError(String);

impl Month {
    /// Month `month` (1 to 12) of `year`.
    ///
    /// # Errors
    ///
    /// Returns [`MonthError`] when `month` is outside 1 to 12.
    pub fn new(year: i16, month: i8) -> Result<Self, MonthError> {
        if (1..=12).contains(&month) {
            Ok(Self { year, month })
        } else {
            Err(MonthError(format!("{year}-{month}")))
        }
    }

    /// The year.
    #[must_use]
    pub const fn year(self) -> i16 {
        self.year
    }

    /// The month, 1 to 12.
    #[must_use]
    pub const fn month(self) -> i8 {
        self.month
    }
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

impl FromStr for Month {
    type Err = MonthError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let error = || MonthError(text.to_owned());
        let (year, month) = text.split_once('-').ok_or_else(error)?;
        Self::new(
            year.parse().map_err(|_| error())?,
            month.parse().map_err(|_| error())?,
        )
        .map_err(|_| error())
    }
}

/// One currency's rate for one period: a day ([`Date`]), a month ([`Month`]) or a year (`i16`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Rate<P> {
    /// Country, in the language asked for.
    pub country: String,
    /// Currency name, in the language asked for.
    pub currency: String,
    /// ISO 4217 code.
    pub iso_code: String,
    /// Banca d'Italia's own currency code.
    pub uic_code: String,
    /// The rate, `None` where Banca d'Italia publishes "N.A.".
    pub rate: Option<Decimal>,
    /// Which way the rate is quoted.
    pub convention: Convention,
    /// The convention, in words.
    pub convention_text: String,
    /// The day, month or year of the rate, Central European Time.
    pub period: P,
}

/// A rate quoted on one day.
pub type DailyRate = Rate<Date>;
/// A monthly average rate.
pub type MonthlyAverageRate = Rate<Month>;
/// An annual average rate.
pub type AnnualAverageRate = Rate<i16>;

/// A currency's rates against the euro and the US dollar on the latest day quoted for every currency.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LatestRate {
    /// Country, in the language asked for.
    pub country: String,
    /// Currency name, in the language asked for.
    pub currency: String,
    /// ISO 4217 code.
    pub iso_code: String,
    /// Banca d'Italia's own currency code.
    pub uic_code: String,
    /// Units of the currency per euro, `None` where published as "N.A.".
    pub eur_rate: Option<Decimal>,
    /// Rate against the US dollar, quoted as [`usd_convention`](Self::usd_convention) says, `None` where published as "N.A.".
    pub usd_rate: Option<Decimal>,
    /// Which way the US dollar rate is quoted, `None` for the US dollar itself.
    pub usd_convention: Option<Convention>,
    /// The US dollar convention, in words.
    pub usd_convention_text: String,
    /// Day of the quote, Central European Time.
    pub reference_date: Date,
}

/// One currency's rates over a range of days, months or years.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TimeSeries<P> {
    /// Currency name, in the language asked for.
    pub currency: String,
    /// ISO 4217 code.
    pub iso_code: String,
    /// Banca d'Italia's own currency code.
    pub uic_code: String,
    /// Which way the rates are quoted.
    pub convention: Convention,
    /// The rates, in period order.
    pub points: Vec<Point<P>>,
}

/// One rate of a [`TimeSeries`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Point<P> {
    /// The day, month or year.
    pub period: P,
    /// The rate, `None` where published as "N.A.".
    pub rate: Option<Decimal>,
    /// The convention, in words.
    pub convention_text: String,
}

/// A currency known to Banca d'Italia, current or expired.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CurrencyInfo {
    /// ISO 4217 code.
    pub iso_code: String,
    /// Currency name, in the language asked for.
    pub name: String,
    /// The `graph` flag as published, its meaning undocumented by Banca d'Italia.
    pub graph: bool,
    /// Where and when the currency was or is in use.
    pub countries: Vec<CurrencyUse>,
}

/// A country's use of a currency over a period.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CurrencyUse {
    /// The country, in the language asked for.
    pub country: String,
    /// ISO 3166 code of the country, when published.
    pub country_iso: Option<String>,
    /// First day of use.
    pub valid_from: Date,
    /// Last day of use, `None` while still in use.
    pub valid_until: Option<Date>,
}
