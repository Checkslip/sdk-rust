// qa-customer-text: the doc comments reach docs.rs, and the InvalidArgument
// messages reach a caller's error handling and logs.
//! Building the days the API asks for.

use time::{Date, Month};

use crate::{Error, Result};

/// Builds a calendar day from year, month and day numbers.
///
/// [`UsageParams`](crate::UsageParams) and
/// [`CreateEvidenceExport`](crate::CreateEvidenceExport) take a
/// [`time::Date`], and building one otherwise means naming two `time` types:
///
/// ```
/// use checkslip::time::{Date, Month};
/// let from = Date::from_calendar_date(2026, Month::November, 1)
///     .expect("1 November 2026 is a real day");
/// # let _ = from;
/// ```
///
/// This takes the month as a number instead, which is how the API writes it:
///
/// ```
/// use checkslip::{day, UsageParams};
/// # fn main() -> checkslip::Result<()> {
/// let params = UsageParams::new(day(2026, 11, 1)?, day(2026, 11, 30)?);
/// # let _ = params;
/// # Ok(())
/// # }
/// ```
///
/// For a parameter that takes an instant rather than a day, such as the range
/// on [`CountSlipsParams`](crate::CountSlipsParams), start from the day:
/// `day(2026, 11, 1)?.midnight().assume_utc()`.
///
/// # Errors
///
/// [`Error::InvalidArgument`] when the three numbers do not name a day that
/// exists, such as the 31st of November.
///
/// ```
/// assert!(checkslip::day(2026, 11, 31).is_err());
/// ```
pub fn day(year: i32, month: u8, day: u8) -> Result<Date> {
    let month = Month::try_from(month)
        .map_err(|_| Error::InvalidArgument(format!("month must be 1 to 12, got {month}")))?;
    Date::from_calendar_date(year, month, day).map_err(|_| {
        Error::InvalidArgument(format!("{year}-{:02}-{day:02} does not exist", month as u8))
    })
}

#[cfg(test)]
mod tests {
    use super::day;
    use time::Month;

    #[test]
    fn builds_a_day() {
        let d = day(2026, 11, 1).unwrap();
        assert_eq!((d.year(), d.month(), d.day()), (2026, Month::November, 1));
    }

    #[test]
    fn a_month_outside_the_year_is_an_argument_error() {
        let err = day(2026, 13, 1).unwrap_err();
        assert!(
            format!("{err}").contains("month must be 1 to 12, got 13"),
            "{err}"
        );
    }

    #[test]
    fn a_day_the_month_does_not_have_is_an_argument_error() {
        let err = day(2026, 11, 31).unwrap_err();
        assert!(
            format!("{err}").contains("2026-11-31 does not exist"),
            "{err}"
        );
    }

    #[test]
    fn a_leap_day_exists_only_in_a_leap_year() {
        assert!(day(2024, 2, 29).is_ok());
        assert!(day(2026, 2, 29).is_err());
    }

    #[test]
    fn midnight_utc_is_reachable_from_a_day() {
        // The sentence in the doc comment above has to stay true: an instant
        // comes from a day without naming another type.
        let t = day(2026, 11, 1).unwrap().midnight().assume_utc();
        assert_eq!(t.to_string(), "2026-11-01 0:00:00.0 +00:00:00");
    }
}
