//! Written apostrophe suffix decomposition; no spoken-text inference.
pub(crate) fn suffix_parts(text: &str) -> Option<(&str, Vec<&str>)> {
    let mut parts = text.split(['\'', '’']);
    let base = parts.next()?;
    let suffixes: Vec<_> = parts.collect();
    if suffixes.len() > 2 || suffixes.iter().any(|s| s.is_empty()) {
        return None;
    }
    Some((base, suffixes))
}

pub(crate) fn split_suffix(text: &str) -> Option<(&str, Option<&str>)> {
    let mut parts = text.split(['\'', '’']);
    let base = parts.next()?;
    let suffix = parts.next();
    if parts.next().is_some() || suffix.is_some_and(str::is_empty) {
        return None;
    }
    Some((base, suffix))
}
