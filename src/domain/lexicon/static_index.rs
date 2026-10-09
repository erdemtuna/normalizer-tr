//! Compile-time indexing of thematically authored immutable lexical data.
pub(super) const fn count<T>(groups: &[&[(&str, T)]]) -> usize {
    let mut count = 0;
    let mut group = 0;
    while group < groups.len() {
        count += groups[group].len();
        group += 1;
    }
    count
}

pub(super) const fn build<T, const N: usize>(
    groups: &'static [&'static [(&'static str, T)]],
) -> [(&'static str, &'static T); N] {
    assert!(N == count(groups) && N > 0);
    let mut first_group = 0;
    while groups[first_group].is_empty() {
        first_group += 1;
    }
    let first = &groups[first_group][0];
    let mut index = [(first.0, &first.1); N];
    let mut count = 0;
    let mut group = 0;
    while group < groups.len() {
        let mut entry = 0;
        while entry < groups[group].len() {
            let value = &groups[group][entry];
            let mut position = count;
            while position > 0 && key_before(value.0, index[position - 1].0) {
                index[position] = index[position - 1];
                position -= 1;
            }
            index[position] = (value.0, &value.1);
            count += 1;
            entry += 1;
        }
        group += 1;
    }
    let mut entry = 1;
    while entry < N {
        assert!(key_before(index[entry - 1].0, index[entry].0));
        entry += 1;
    }
    index
}

const fn key_before(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut offset = 0;
    while offset < left.len() && offset < right.len() {
        if left[offset] != right[offset] {
            return left[offset] < right[offset];
        }
        offset += 1;
    }
    left.len() < right.len()
}

pub(super) const fn maximum_key_bytes<T>(index: &[(&str, &T)]) -> usize {
    let mut maximum = 0;
    let mut entry = 0;
    while entry < index.len() {
        if index[entry].0.len() > maximum {
            maximum = index[entry].0.len();
        }
        entry += 1;
    }
    maximum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_utf8_keys_like_runtime_lookup() {
        for left in ["A", "AB", "DSÖ", "DSİ", "Ö", "İ", "vb."] {
            for right in ["A", "AB", "DSÖ", "DSİ", "Ö", "İ", "vb."] {
                assert_eq!(key_before(left, right), left < right);
            }
        }
    }

    #[test]
    fn indexes_unsorted_groups_without_copying_payloads() {
        const GROUPS: &[&[(&str, u8)]] = &[&[], &[("Z", 1), ("A", 2)], &[], &[("İ", 3), ("B", 4)]];
        const INDEX: [(&str, &u8); count(GROUPS)] = build(GROUPS);
        assert_eq!(INDEX, [("A", &2), ("B", &4), ("Z", &1), ("İ", &3)]);
    }
}
