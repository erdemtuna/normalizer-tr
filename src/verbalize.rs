use crate::{
    NormalizeError, SegmentKind,
    domain::lexicon::{Lexeme, MONTHS},
    domain::temporal::{Clock, Date},
    interpretation::Value,
    morphology::{Inflection, Spoken},
    numerals,
};

pub(crate) fn date_spoken(date: Date) -> Spoken {
    let mut year = numerals::cardinal(u64::from(date.year()));
    let prefix = format!(
        "{} {} ",
        numerals::cardinal(u64::from(date.day())).into_text(),
        MONTHS[usize::from(date.month() - 1)]
    );
    year.prefix(&prefix);
    year
}

pub(crate) fn time_spoken(time: Clock) -> Spoken {
    let hour = numerals::cardinal(u64::from(time.hour()));
    if time.minute() == 0 {
        hour
    } else {
        let mut minute = numerals::cardinal(u64::from(time.minute()));
        minute.prefix(&format!("{} ", hour.into_text()));
        minute
    }
}

pub(crate) fn render(value: &Value) -> Result<(SegmentKind, &'static str, String), NormalizeError> {
    Ok(match value {
        Value::Composite(_) => return Err(NormalizeError::Internal),
        Value::Numeric(number) => (number.kind(), "number", number.render().into_text()),
        Value::Digits(text) => (SegmentKind::Digits, "digits.hint", numerals::digits(text)),
        Value::Date(date, tail) => {
            let mut spoken = date_spoken(*date);
            tail.apply(&mut spoken);
            (SegmentKind::Date, "date.gregorian", spoken.into_text())
        }
        Value::Time(time, locative) => {
            let mut spoken = time_spoken(*time);
            if *locative {
                spoken.inflect(Inflection::Locative);
            }
            (SegmentKind::Time, "time.digital", spoken.into_text())
        }
        Value::Percent(number, case) => {
            let mut spoken = numerals::number(number);
            if let Some(case) = case {
                case.apply(&mut spoken);
            }
            spoken.prefix("yüzde ");
            (SegmentKind::Percent, "percent", spoken.into_text())
        }
        Value::Quantity(quantity) => (
            if quantity.is_money() {
                SegmentKind::Money
            } else {
                SegmentKind::Unit
            },
            "quantity",
            quantity.render().into_text(),
        ),
        Value::Lexical(entry, case) => (
            SegmentKind::Abbreviation,
            "abbreviation",
            lexical(*entry, *case),
        ),
        Value::Pronunciation(entry, inflection) => (
            SegmentKind::Pronunciation,
            "pronunciation.name",
            inflection.map_or_else(
                || entry.output.to_owned(),
                |inflection| inflection.render(entry.output, entry.target),
            ),
        ),
        Value::Range(range) => (SegmentKind::Range, "range.context", range.render()),
        Value::ClockRange(range) => (
            SegmentKind::Range,
            "range.clock",
            format!(
                "{} {} {}",
                time_spoken(range.start()).into_text(),
                range.connector(),
                time_spoken(range.end()).into_text()
            ),
        ),
        Value::Telephone(phone) => (SegmentKind::Telephone, "telephone.tr", phone.render()),
        Value::Iban(iban) => (SegmentKind::Iban, "iban.tr.mod97", iban.render()),
        Value::Roman(number) => (
            SegmentKind::Roman,
            "roman.canonical",
            number.render().into_text(),
        ),
        Value::Electronic(address) => (
            SegmentKind::Electronic,
            "electronic.ascii",
            address.render().to_owned(),
        ),
        Value::Symbol(text) => (SegmentKind::Symbol, "symbol.prose", text.clone()),
        Value::Letter(letter) => (SegmentKind::Symbol, "letter.name", letter.text().to_owned()),
    })
}

pub(crate) fn render_component(
    value: &crate::domain::composite::ComponentValue,
) -> (SegmentKind, &'static str, String) {
    use crate::domain::composite::ComponentValue;
    match value {
        ComponentValue::Numeric(number) => (number.kind(), "number", number.render().into_text()),
        ComponentValue::Quantity(quantity) => (
            if quantity.is_money() {
                SegmentKind::Money
            } else {
                SegmentKind::Unit
            },
            "quantity",
            quantity.render().into_text(),
        ),
        ComponentValue::Text {
            kind,
            rule_id,
            text,
        } => (*kind, rule_id, text.clone()),
    }
}

fn lexical(entry: Lexeme, case: Option<Inflection>) -> String {
    let mut spoken = Spoken::lexical(entry.output, entry.target);
    if let Some(case) = case {
        spoken.inflect(case);
    }
    spoken.into_text()
}
