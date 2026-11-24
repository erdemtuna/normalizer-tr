//! Release terminal periods before indexing runs and validating hint ranges.
use super::{Token, signals, telephone_gap, whitespace_between};
use crate::{NormalizeError, WorkControl, domain::identifiers::Telephone, notation, numerals};

use super::super::{boundaries::quantity_tail, context};

pub(super) fn release(
    text: &str,
    tokens: &mut [Token<'_>],
    control: &WorkControl,
) -> Result<(), NormalizeError> {
    let mut account = false;
    let mut previous_end = 0;
    let mut digits = 0;
    for index in 0..tokens.len() {
        control.check()?;
        let token = tokens[index];
        digits += token.text.bytes().filter(u8::is_ascii_digit).count();
        tokens[index].digit_end = digits;
        let account_group = account && whitespace_between(text, previous_end, token.range.start);
        let start = token.text.len() >= 4
            && (token.text.starts_with("TR") || token.text.starts_with("tr"))
            && token.text.as_bytes()[2..].iter().all(u8::is_ascii_digit);
        account = start
            || token.text == "TR"
            || (account_group
                && token
                    .text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte.is_ascii_uppercase()));
        if let Some(body) = token.text.strip_suffix('.') {
            let account_period = account_group && digits_only(body);
            let padded = numerals::zero_padded_digits(body);
            let phone = if !account_period && signals::telephone_group(body) {
                terminal_telephone(text, &tokens[..=index], body, padded)
            } else {
                None
            };
            let release = match phone {
                Some(PhonePeriod::ReleaseAt(start)) => {
                    tokens[start].phone_period_start = true;
                    true
                }
                Some(PhonePeriod::KeepWithCue) => false,
                None => account_period || (padded && !quantity_neighbor(text, tokens, index)),
            };
            if release {
                tokens[index].text = body;
                tokens[index].range.end -= 1;
            }
        }
        previous_end = tokens[index].range.end;
    }
    Ok(())
}

fn digits_only(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn quantity_neighbor(text: &str, tokens: &[Token<'_>], index: usize) -> bool {
    let token = tokens[index];
    tokens.get(index + 1).is_some_and(|next| {
        whitespace_between(text, token.range.end, next.range.start) && quantity_tail(next.text)
    }) || index.checked_sub(1).is_some_and(|i| {
        let previous = tokens[i];
        whitespace_between(text, previous.range.end, token.range.start)
            && (notation::label(previous.text) || notation::unsupported_label(previous.text))
    })
}

enum PhonePeriod {
    ReleaseAt(usize),
    KeepWithCue,
}

fn terminal_telephone(
    text: &str,
    tokens: &[Token<'_>],
    terminal: &str,
    padded: bool,
) -> Option<PhonePeriod> {
    let last = tokens.len() - 1;
    let end = tokens[last].range.end - 1;
    let first = tokens.len().saturating_sub(Telephone::MAX_GROUPS);
    let digit_end = tokens[last].digit_end;
    for index in (first..=last).rev() {
        let token = tokens[index];
        if index != last
            && (!signals::telephone_group(token.text)
                || !telephone_gap(&text[token.range.end..tokens[index + 1].range.start]))
        {
            break;
        }
        let digit_start = index.checked_sub(1).map_or(0, |i| tokens[i].digit_end);
        let start = if index == last { terminal } else { token.text };
        let count = digit_end - digit_start;
        let surface = &text[token.range.start..end];
        let split_zero = start != "0"
            || tokens.get(index + 1..index + 3).is_some_and(|groups| {
                Telephone::has_split_zero_prefix(start, groups[0].text, groups[1].text)
            });
        if Telephone::has_implicit_digit_count(start, count)
            && split_zero
            && Telephone::valid_shape(surface, false)
        {
            return Some(PhonePeriod::ReleaseAt(index));
        }
        if padded
            && count == Telephone::CUE_ONLY_DIGITS
            && index.checked_sub(1).is_some_and(|i| {
                let previous = tokens[i];
                super::cue_whitespace(text, previous.range.end, token.range.start)
                    && context::TELEPHONE_CUES.contains(&context::cue_key(previous.text).as_str())
            })
            && Telephone::valid_shape(surface, true)
        {
            return Some(PhonePeriod::KeepWithCue);
        }
    }
    None
}
