//! Bounded alternative rendering; source interpretation is prepared elsewhere.
mod literal;
mod output;
use crate::{
    FallbackClass, FallbackStrategy, NormalizeError, WorkControl,
    domain::numeric::NumericPreference,
    interpretation::{Alternative, TemporalPreference, UnresolvedFinding, Value},
    notation::{DateSurface, TimeSurface},
    numerals, verbalize,
};
use output::Output;

pub(crate) fn render(
    finding: &UnresolvedFinding,
    source: &str,
    maximum: usize,
    control: &WorkControl,
) -> Result<(String, FallbackStrategy), NormalizeError> {
    let mut output = Output::new(maximum, control);
    let strategy = match finding.alternative() {
        Alternative::Number(NumericPreference::Number(number)) => {
            output.word(&number.render().into_text())?;
            FallbackStrategy::PreferredNumber
        }
        Alternative::Number(NumericPreference::SentenceNumber(number)) => {
            output.word(&numerals::number(number).into_text())?;
            output.append(".")?;
            FallbackStrategy::PreferredNumber
        }
        Alternative::Date(date) => {
            output.word(&date_surface(date))?;
            FallbackStrategy::SurfaceDate
        }
        Alternative::Temporal(TemporalPreference::Date(date, locative)) => {
            output.word(&verbalize::render(&Value::Date(*date, *locative)).2)?;
            FallbackStrategy::PreferredDate
        }
        Alternative::Temporal(TemporalPreference::Time(clock, locative)) => {
            output.word(&verbalize::render(&Value::Time(*clock, *locative)).2)?;
            FallbackStrategy::PreferredTime
        }
        Alternative::Time(time) => {
            output.word(&time_surface(time))?;
            FallbackStrategy::SurfaceTime
        }
        Alternative::Literal(letters) => {
            if literal::render(
                source,
                *letters,
                finding.class() == FallbackClass::Quantity,
                &mut output,
            )? {
                FallbackStrategy::UnicodeCodePoint
            } else {
                FallbackStrategy::Literal
            }
        }
    };
    let text = output.finish();
    if text.trim().is_empty() {
        return Err(NormalizeError::Internal);
    }
    Ok((text, strategy))
}

fn date_surface(date: &DateSurface) -> String {
    format!(
        "{} {} {}",
        numerals::cardinal(date.day()).into_text(),
        date.month(),
        numerals::cardinal(date.year()).into_text()
    )
}

fn time_surface(time: &TimeSurface) -> String {
    format!(
        "{} {}",
        numerals::cardinal(time.hour()).into_text(),
        numerals::cardinal(time.minute()).into_text()
    )
}
