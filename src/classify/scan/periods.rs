//! Release terminal periods before indexing runs and validating hint ranges.
use super::{Token, signals, telephone_gap, whitespace_between};
use crate::{NormalizeError, WorkControl, domain::identifiers::Telephone, numerals};

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
        let adjacent =
            previous_end == 0 || whitespace_between(text, previous_end, token.range.start);
        let account_group = account && adjacent;
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
        if let Some(body) = token.text.strip_suffix('.')
            && (numerals::zero_padded_digits(body)
                || (account_group && digits_only(body))
                || (signals::telephone_group(body)
                    && ends_telephone(text, &tokens[..=index], body)))
        {
            tokens[index].text = body;
            tokens[index].range.end -= 1;
        }
        previous_end = tokens[index].range.end;
    }
    Ok(())
}

fn digits_only(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn ends_telephone(text: &str, tokens: &[Token<'_>], terminal: &str) -> bool {
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
        if Telephone::has_implicit_digit_count(start, digit_end - digit_start)
            && Telephone::parse(&text[token.range.start..end], false).is_some()
        {
            return true;
        }
    }
    false
}
