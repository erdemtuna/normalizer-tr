use crate::domain::lexicon;

/// Written components, deliberately not a validated Gregorian Date.
pub(crate) struct DateSurface {
    day: u64,
    month: &'static str,
    year: u64,
}

impl DateSurface {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let (day, month, year) = if text.contains('.') {
            let mut parts = text.split('.');
            let fields = (parts.next()?, parts.next()?, parts.next()?);
            if parts.next().is_some() {
                return None;
            }
            fields
        } else {
            let mut parts = text.split('-');
            let year = parts.next()?;
            let month = parts.next()?;
            let day = parts.next()?;
            if parts.next().is_some() {
                return None;
            }
            (day, month, year)
        };
        if !(1..=2).contains(&day.len())
            || !(1..=2).contains(&month.len())
            || year.len() != 4
            || ![day, month, year]
                .iter()
                .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return None;
        }
        let month = lexicon::month_name(month.parse().ok()?)?;
        Some(Self {
            day: day.parse().ok()?,
            month,
            year: year.parse().ok()?,
        })
    }

    pub(crate) fn day(&self) -> u64 {
        self.day
    }
    pub(crate) fn month(&self) -> &'static str {
        self.month
    }
    pub(crate) fn year(&self) -> u64 {
        self.year
    }
}

/// Two source components, without asserting that they are a valid clock.
pub(crate) struct TimeSurface {
    hour: u64,
    minute: u64,
}

impl TimeSurface {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let (hour, minute) = text.split_once([':', '.'])?;
        if !(1..=2).contains(&hour.len())
            || minute.len() != 2
            || !hour
                .bytes()
                .chain(minute.bytes())
                .all(|b| b.is_ascii_digit())
        {
            return None;
        }
        Some(Self {
            hour: hour.parse().ok()?,
            minute: minute.parse().ok()?,
        })
    }
    pub(crate) fn hour(&self) -> u64 {
        self.hour
    }
    pub(crate) fn minute(&self) -> u64 {
        self.minute
    }
}
