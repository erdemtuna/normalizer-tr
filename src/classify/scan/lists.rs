//! Complete tight-list admission, independent of pronunciation and policy rendering.
use super::punctuation::{boundary, expression_range, trimmed_range};
use crate::{
    NormalizeError, SourceRange, WorkControl,
    domain::{electronic, lexicon},
    resources::Resources,
};

#[derive(Clone, Copy)]
pub(in crate::classify) struct CatalogGroup {
    pub(in crate::classify) range: SourceRange,
    pub(in crate::classify) approved: bool,
    pub(in crate::classify) catalog: bool,
}

pub(super) fn groups(
    text: &str,
    rules: &Resources,
    quotes: &[usize],
    control: &WorkControl,
) -> Result<Vec<CatalogGroup>, NormalizeError> {
    let mut groups = Vec::new();
    let mut covered = 0;
    for matched in rules.tokens.find_iter(text) {
        control.check()?;
        if matched.start() < covered {
            continue;
        }
        let Some(range) = expression_range(matched.as_str(), matched.start(), quotes, control)?
        else {
            continue;
        };
        let raw = &text[range.start..range.end];
        if electronic::looks_like(raw) || super::punctuation::numeric_parenthesis_compound(raw) {
            continue;
        }
        let protected = super::identifier(raw);
        let semicolons =
            !protected || super::punctuation::quantity_separators(raw, range.start, quotes);
        let mut start = range.start;
        loop {
            control.check()?;
            if start >= range.end {
                break;
            }
            let end = if semicolons {
                text[start..range.end]
                    .find(';')
                    .map_or(range.end, |i| start + i)
            } else {
                range.end
            };
            if start >= covered
                && let Some(head) = trimmed_range(&text[start..end], start, quotes)
                && seed(&text[head.start..head.end])
                && let Some(group) = group(text, head.start, quotes, control)?
            {
                covered = group.range.end;
                groups.push(group);
            }
            if end == range.end {
                break;
            }
            start = end + 1;
        }
    }
    Ok(groups)
}

fn seed(text: &str) -> bool {
    if text.contains(',') {
        return true;
    }
    let first = text.split(['\'', '’']).next().unwrap_or(text);
    lexicon::pronunciation_candidates(first)
        .iter()
        .any(|(key, _)| key.contains(' '))
}

fn group(
    text: &str,
    start: usize,
    quotes: &[usize],
    control: &WorkControl,
) -> Result<Option<CatalogGroup>, NormalizeError> {
    let mut cursor = start;
    let mut approved = true;
    let mut catalog = false;
    let mut tight = false;
    loop {
        control.check()?;
        let (end, member, known) = member(text, cursor, quotes, control)?;
        approved &= member;
        catalog |= known;
        cursor = end;
        if text.as_bytes().get(cursor) != Some(&b',') {
            break;
        }
        let next = cursor + 1;
        let following = text[next..].chars().next();
        if following.is_none_or(|ch| ch.is_whitespace() || boundary(ch, next, quotes)) {
            cursor = next;
            break;
        }
        tight = true;
        cursor = next;
    }
    Ok(tight.then_some(CatalogGroup {
        range: SourceRange::new(start, cursor),
        approved,
        catalog,
    }))
}

fn member(
    text: &str,
    start: usize,
    quotes: &[usize],
    control: &WorkControl,
) -> Result<(usize, bool, bool), NormalizeError> {
    let first_end = word_end(text, start, quotes, control)?;
    let first = &text[start..first_end];
    let base = first.split(['\'', '’']).next().unwrap_or(first);
    let mut end = first_end;
    for &(key, _) in lexicon::pronunciation_candidates(base).iter().rev() {
        control.check()?;
        if !key.contains(' ') || text.get(start..start + key.len()) != Some(key) {
            continue;
        }
        let next = start + key.len();
        if text[next..].chars().next().is_some_and(|ch| {
            !ch.is_whitespace()
                && !boundary(ch, next, quotes)
                && !matches!(ch, ',' | ';' | '\'' | '’' | '.' | ':')
        }) {
            continue;
        }
        let last = start + key.len() - key.rsplit(' ').next().unwrap().len();
        end = word_end(text, last, quotes, control)?;
        break;
    }
    let Some(range) = trimmed_range(&text[start..end], start, quotes) else {
        return Ok((end, false, false));
    };
    let source = &text[range.start..range.end];
    let approved = !electronic::looks_like(source)
        && !super::identifier(source)
        && (lexicon::plain_abbreviation(source) || lexicon::pronunciation_word(source));
    let base = source.split(['\'', '’']).next().unwrap_or(source);
    let known = lexicon::abbreviation(base).is_some() || lexicon::pronunciation_word(source);
    Ok((end, approved, known))
}

fn word_end(
    text: &str,
    start: usize,
    quotes: &[usize],
    control: &WorkControl,
) -> Result<usize, NormalizeError> {
    for (offset, ch) in text[start..].char_indices() {
        control.check()?;
        if ch.is_whitespace() || matches!(ch, ',' | ';') || boundary(ch, start + offset, quotes) {
            return Ok(start + offset);
        }
    }
    Ok(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::scan::punctuation::quotation_boundaries;

    #[test]
    fn comma_free_inputs_never_admit_a_catalog_group() {
        let rules = Resources::shared().unwrap();
        let control = WorkControl::default();
        for text in [
            "ordinary prose",
            "GitHub Copilot",
            "Visual Studio Code'ye",
            "Claude; GitHub Copilot; UNKNOWN",
            "‘GitHub Copilot’",
            "\"GitHub Copilot\"@ornek.com",
            "https://ornek.com/Claude?x=2",
            "adres No:5 Kat:3",
            "AB12(Claude)",
        ] {
            assert!(
                groups(text, &rules, &quotation_boundaries(text), &control)
                    .unwrap()
                    .is_empty(),
                "{text}"
            );
        }
        let text = format!("AB{}", "7".repeat(crate::MAX_INPUT_BYTES - "AB".len()));
        assert!(groups(&text, &rules, &[], &control).unwrap().is_empty());
    }

    #[test]
    fn complete_group_metadata_preserves_late_rejection_and_invalid_suffix_admission() {
        let rules = Resources::shared().unwrap();
        let control = WorkControl::default();
        for (text, approved) in [
            ("GitHub Copilot,Claude", true),
            ("GitHub Copilot,Claude,UNKNOWN", false),
            ("Claude,,Hugging Face", false),
            ("Claude,Hugging Face,AB12", false),
            ("SGK'a,PDF'lik,TRT", true),
        ] {
            let groups = groups(text, &rules, &quotation_boundaries(text), &control).unwrap();
            assert_eq!(groups.len(), 1, "{text}");
            assert_eq!(groups[0].range, SourceRange::new(0, text.len()), "{text}");
            assert_eq!(groups[0].approved, approved, "{text}");
            assert!(groups[0].catalog, "{text}");
        }
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(128))]
        #[test]
        fn generated_comma_free_source_has_no_groups(
            pieces in proptest::collection::vec(
                proptest::sample::select(vec![
                    "GitHub", "Copilot", "Claude", "Hugging", "Face", "UNKNOWN",
                    "AB12", "5", "No:", ";", "'", "’", "“", "”", ".", "ö", "\r\n",
                ]),
                1..64
            )
        ) {
            let text = pieces.join(" ");
            let rules = Resources::shared().unwrap();
            let control = WorkControl::default();
            proptest::prop_assert!(
                groups(&text, &rules, &quotation_boundaries(&text), &control)
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
