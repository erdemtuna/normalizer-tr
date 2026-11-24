use crate::numerals;

#[derive(Clone, Debug)]
pub(crate) struct Telephone {
    national: String,
    international: bool,
    national_zero: bool,
}

impl Telephone {
    pub(crate) const MAX_GROUPS: usize = 5;
    const CUE_ONLY_DIGITS: usize = 10;
    const NATIONAL_DIGITS: usize = 11;
    const INTERNATIONAL_DIGITS: usize = 12;

    pub(crate) fn has_written_national_prefix(source: &str) -> bool {
        if !source.starts_with('0') {
            return false;
        }
        let prefix = source.split(['-', '(']).next().unwrap_or(source);
        (prefix.len() == 4 && prefix.bytes().all(|byte| byte.is_ascii_digit()))
            || (source.len() >= 10 && source.bytes().all(|byte| byte.is_ascii_digit()))
    }

    pub(crate) fn has_split_zero_prefix(source: &str, first: &str, second: &str) -> bool {
        source == "0"
            && first.len() == 3
            && first.bytes().all(|b| b.is_ascii_digit())
            && second.len() >= 2
            && second.bytes().all(|b| b.is_ascii_digit())
    }

    pub(crate) fn has_implicit_digit_count(text: &str, count: usize) -> bool {
        (count == Self::NATIONAL_DIGITS
            && (text == "0" || text.starts_with("0-") || Self::has_written_national_prefix(text)))
            || (count == Self::INTERNATIONAL_DIGITS && text.starts_with("+90"))
    }

    fn shape(text: &str, explicit: bool) -> Option<(bool, bool)> {
        if !text
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '-' | '(' | ')'))
        {
            return None;
        }
        if text.matches('+').count() > usize::from(text.starts_with('+')) {
            return None;
        }
        let mut depth = 0;
        for ch in text.chars() {
            match ch {
                '(' if depth == 0 => depth = 1,
                ')' if depth == 1 => depth = 0,
                '(' | ')' => return None,
                _ => {}
            }
        }
        if depth != 0
            || text.ends_with(['-', ' ', '('])
            || text.contains("--")
            || text.contains("  ")
        {
            return None;
        }
        let international = text.starts_with("+90");
        let digits = text.bytes().filter(u8::is_ascii_digit);
        let count = digits.clone().count();
        let national_zero = if international {
            if count != Self::INTERNATIONAL_DIGITS {
                return None;
            }
            false
        } else if count == Self::NATIONAL_DIGITS && digits.clone().next() == Some(b'0') {
            true
        } else if count == Self::CUE_ONLY_DIGITS && explicit {
            false
        } else {
            return None;
        };
        let sizes = text
            .split([' ', '-', '(', ')'])
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_start_matches('+').len());
        if sizes.clone().nth(1).is_some() {
            let allowed = if international {
                sizes.eq([2, 3, 3, 2, 2])
            } else if national_zero {
                sizes.clone().eq([4, 3, 2, 2]) || sizes.eq([1, 3, 3, 2, 2])
            } else {
                explicit && sizes.eq([3, 3, 2, 2])
            };
            if !allowed {
                return None;
            }
        }
        Some((international, national_zero))
    }

    pub(crate) fn valid_shape(text: &str, explicit: bool) -> bool {
        Self::shape(text, explicit).is_some()
    }

    pub(crate) fn parse(text: &str, explicit: bool) -> Option<Self> {
        let (international, national_zero) = Self::shape(text, explicit)?;
        let skip = if international {
            2
        } else {
            usize::from(national_zero)
        };
        Some(Self {
            national: text
                .bytes()
                .filter(u8::is_ascii_digit)
                .skip(skip)
                .map(char::from)
                .collect(),
            international,
            national_zero,
        })
    }
    pub(crate) fn render(&self) -> String {
        let mut chunks = Vec::new();
        if self.international {
            chunks.push("artı doksan".to_owned());
        }
        if self.national_zero {
            chunks.push("sıfır".to_owned());
        }
        for range in [0..3, 3..6, 6..8, 8..10] {
            let digits = &self.national[range];
            let output = if digits.starts_with('0') {
                numerals::digits(digits)
            } else {
                // The constructor restricts every group to at most three ASCII digits.
                let value = digits
                    .bytes()
                    .fold(0_u64, |n, b| n * 10 + u64::from(b - b'0'));
                numerals::cardinal(value).into_text()
            };
            chunks.push(output);
        }
        chunks.join(" ")
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Iban {
    canonical: String,
}

impl Iban {
    pub(crate) const CHARACTERS: usize = 26;

    pub(crate) fn parse(text: &str) -> Option<Self> {
        let groups: Vec<_> = text.split(' ').collect();
        if groups.iter().any(|g| g.is_empty()) {
            return None;
        }
        if groups.len() > 1
            && (groups.len() != 7
                || groups[..6].iter().any(|g| g.len() != 4)
                || groups[6].len() != 2)
        {
            return None;
        }
        let canonical = groups.concat();
        if canonical.len() != Self::CHARACTERS
            || !canonical.starts_with("TR")
            || !canonical[2..].bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let check = (canonical.as_bytes()[2] - b'0') * 10 + canonical.as_bytes()[3] - b'0';
        if !(2..=98).contains(&check) {
            return None;
        }
        let mut remainder = 0_u32;
        for byte in canonical.as_bytes()[4..]
            .iter()
            .copied()
            .chain("2927".bytes())
            .chain(canonical.as_bytes()[2..4].iter().copied())
        {
            remainder = (remainder * 10 + u32::from(byte - b'0')) % 97;
        }
        (remainder == 1).then_some(Self { canonical })
    }
    pub(crate) fn render(&self) -> String {
        let mut output = String::from("te re ");
        output.push_str(&numerals::digits(&self.canonical[2..4]));
        for group in self.canonical.as_bytes()[4..].chunks(4) {
            output.push_str(", ");
            let digits: String = group.iter().map(|b| char::from(*b)).collect();
            output.push_str(&numerals::digits(&digits));
        }
        output
    }
}
