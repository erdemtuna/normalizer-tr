use std::borrow::Cow;

use unicode_normalization::{UnicodeNormalization, is_nfc};
use unicode_segmentation::UnicodeSegmentation;

use crate::{Hint, NormalizeError, SourceRange, WorkControl};

enum Mapping {
    Identity(Vec<usize>),
    Normalized(Vec<(usize, usize)>),
}

pub(crate) struct SourceMap<'a> {
    recognition: Cow<'a, str>,
    mapping: Mapping,
}

impl<'a> SourceMap<'a> {
    pub(crate) fn new(input: &'a str, control: &WorkControl) -> Result<Self, NormalizeError> {
        control.check()?;
        if is_nfc(input) {
            let mut boundaries = Vec::with_capacity(input.len() / 2 + 1);
            boundaries.push(0);
            for (offset, grapheme) in input.grapheme_indices(true) {
                control.check()?;
                boundaries.push(offset + grapheme.len());
            }
            return Ok(Self {
                recognition: Cow::Borrowed(input),
                mapping: Mapping::Identity(boundaries),
            });
        }
        let mut recognition = String::with_capacity(input.len());
        let mut boundaries = vec![(0, 0)];
        for (offset, grapheme) in input.grapheme_indices(true) {
            control.check()?;
            recognition.extend(grapheme.nfc());
            boundaries.push((recognition.len(), offset + grapheme.len()));
        }
        Ok(Self {
            recognition: Cow::Owned(recognition),
            mapping: Mapping::Normalized(boundaries),
        })
    }
    pub(crate) fn text(&self) -> &str {
        &self.recognition
    }
    pub(crate) fn cover(&self, range: SourceRange) -> SourceRange {
        match &self.mapping {
            Mapping::Identity(boundaries) => {
                let start = boundaries.partition_point(|offset| *offset <= range.start) - 1;
                let end = boundaries.partition_point(|offset| *offset < range.end);
                SourceRange::new(boundaries[start], boundaries[end])
            }
            Mapping::Normalized(boundaries) => {
                let start = boundaries.partition_point(|pair| pair.0 <= range.start) - 1;
                let end = boundaries.partition_point(|pair| pair.0 < range.end);
                SourceRange::new(boundaries[start].0, boundaries[end].0)
            }
        }
    }
    pub(crate) fn original(&self, range: SourceRange) -> Result<SourceRange, NormalizeError> {
        Ok(SourceRange::new(
            self.lookup(range.start, false)?,
            self.lookup(range.end, false)?,
        ))
    }
    fn lookup(&self, offset: usize, original: bool) -> Result<usize, NormalizeError> {
        match &self.mapping {
            Mapping::Identity(boundaries) => boundaries.binary_search(&offset).map(|_| offset),
            Mapping::Normalized(boundaries) => boundaries
                .binary_search_by_key(&offset, |pair| if original { pair.1 } else { pair.0 })
                .map(|index| {
                    if original {
                        boundaries[index].0
                    } else {
                        boundaries[index].1
                    }
                }),
        }
        .map_err(|_| NormalizeError::InvalidHint)
    }
    pub(crate) fn hints(&self, hints: &[Hint]) -> Result<Vec<Hint>, NormalizeError> {
        let mut mapped = Vec::with_capacity(hints.len());
        for hint in hints {
            if hint.range.start >= hint.range.end {
                return Err(NormalizeError::InvalidHint);
            }
            mapped.push(Hint::new(
                SourceRange::new(
                    self.lookup(hint.range.start, true)?,
                    self.lookup(hint.range.end, true)?,
                ),
                hint.kind,
            ));
        }
        mapped.sort_by_key(|hint| hint.range.start);
        if mapped
            .windows(2)
            .any(|pair| pair[0].range.end > pair[1].range.start)
        {
            return Err(NormalizeError::InvalidHint);
        }
        Ok(mapped)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use proptest::prelude::*;

    use super::*;
    use crate::HintKind;

    fn boundaries(text: &str) -> Vec<usize> {
        text.grapheme_indices(true)
            .map(|(start, _)| start)
            .chain(std::iter::once(text.len()))
            .collect()
    }

    fn covered(boundaries: &[usize], range: SourceRange) -> SourceRange {
        SourceRange::new(
            *boundaries.iter().rfind(|&&b| b <= range.start).unwrap(),
            *boundaries.iter().find(|&&b| b >= range.end).unwrap(),
        )
    }

    fn check_mapping(input: &str) {
        let source = SourceMap::new(input, &WorkControl::default()).unwrap();
        let recognition: String = input.nfc().collect();
        assert_eq!(source.text(), recognition);
        let original = boundaries(input);
        let recognized = boundaries(&recognition);
        assert_eq!(original.len(), recognized.len());
        for (offsets, targets, from_original) in [
            (&original, &recognized, true),
            (&recognized, &original, false),
        ] {
            for offset in 0..=*offsets.last().unwrap() {
                let expected = offsets
                    .iter()
                    .position(|&b| b == offset)
                    .map(|index| targets[index])
                    .ok_or(NormalizeError::InvalidHint);
                assert_eq!(source.lookup(offset, from_original), expected, "{input:?}");
            }
            for offset in [offsets.last().unwrap() + 1, usize::MAX] {
                assert_eq!(
                    source.lookup(offset, from_original),
                    Err(NormalizeError::InvalidHint)
                );
            }
        }
        for start in 0..=recognition.len() {
            for end in start..=recognition.len() {
                let range = SourceRange::new(start, end);
                assert_eq!(
                    source.cover(range),
                    covered(&recognized, range),
                    "{input:?}"
                );
                let expected = recognized
                    .iter()
                    .position(|&b| b == start)
                    .zip(recognized.iter().position(|&b| b == end))
                    .map(|(s, e)| SourceRange::new(original[s], original[e]))
                    .ok_or(NormalizeError::InvalidHint);
                assert_eq!(source.original(range), expected, "{input:?}");
            }
        }
    }

    #[test]
    fn endpoint_projection_and_cover_match_the_segmenter() {
        for input in [
            "",
            "a",
            "\r\n",
            "a\r\nb",
            "\r\r\n\n",
            "\r\n\r\n",
            "\tword\rword\nword\r\n",
            "ü 12",
            "e\u{301} 12",
            "o\u{308}\r\n12",
            "I\u{307}12",
            ";\u{301}12",
            "\u{301}q\u{301}",
            "a\u{315}\u{300}\u{301} 12",
            "👩‍👩‍👧‍👦🙂\u{fe0f}\r\n12",
        ] {
            check_mapping(input);
        }
        let source = SourceMap::new("a\r\nb", &WorkControl::default()).unwrap();
        assert_eq!(source.cover(SourceRange::new(2, 2)), SourceRange::new(1, 3));
        assert_eq!(source.cover(SourceRange::new(1, 1)), SourceRange::new(1, 1));
        assert_eq!(
            source.original(SourceRange::new(2, 3)),
            Err(NormalizeError::InvalidHint)
        );
    }

    #[test]
    fn exact_hint_projection_preserves_sorting_adjacency_and_rejection() {
        let source = SourceMap::new("e\u{301}\r\n12", &WorkControl::default()).unwrap();
        let hint = |start, end| Hint::new(SourceRange::new(start, end), HintKind::Digits);
        assert_eq!(
            source.hints(&[hint(5, 7), hint(0, 3)]),
            Ok(vec![hint(0, 2), hint(4, 6)])
        );
        let ascii = SourceMap::new("ab", &WorkControl::default()).unwrap();
        assert_eq!(
            ascii.hints(&[hint(1, 2), hint(0, 1)]),
            Ok(vec![hint(0, 1), hint(1, 2)])
        );
        for hints in [
            vec![hint(0, 1)],
            vec![hint(3, 4)],
            vec![hint(4, 5)],
            vec![hint(5, 5)],
            vec![hint(7, 5)],
            vec![hint(5, 8)],
            vec![hint(5, usize::MAX)],
            vec![hint(usize::MAX, usize::MAX)],
            vec![hint(5, 7), hint(5, 7)],
            vec![hint(0, 7), hint(5, 7)],
        ] {
            assert_eq!(source.hints(&hints), Err(NormalizeError::InvalidHint));
        }
        assert_eq!(source.hints(&[]), Ok(vec![]));
    }

    #[test]
    fn construction_propagates_pre_cancelled_and_expired_controls() {
        for input in ["", "word\r\nword", "ö", "o\u{308}"] {
            let cancelled = WorkControl::default();
            cancelled.cancel();
            for control in [cancelled, WorkControl::new(Some(Instant::now()))] {
                assert!(matches!(
                    SourceMap::new(input, &control),
                    Err(NormalizeError::Cancelled)
                ));
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn permitted_ascii_mapping_matches_grapheme_oracle(
            bytes in prop::collection::vec(
                prop_oneof![0x20_u8..=0x7e, Just(b'\r'), Just(b'\n'), Just(b'\t')],
                0..32,
            )
        ) {
            let input = String::from_utf8(bytes).unwrap();
            check_mapping(&input);
        }

        #[test]
        fn ascii_hint_ranges_match_independent_boundary_and_overlap_checks(
            bytes in prop::collection::vec(
                prop_oneof![Just(b'a'), Just(b'\r'), Just(b'\n'), Just(b'\t')],
                0..16,
            ),
            ranges in prop::collection::vec((0_usize..18, 0_usize..18), 0..8),
        ) {
            let input = String::from_utf8(bytes).unwrap();
            let source = SourceMap::new(&input, &WorkControl::default()).unwrap();
            let bounds = boundaries(&input);
            let hints: Vec<_> = ranges.into_iter()
                .map(|(start, end)| Hint::new(SourceRange::new(start, end), HintKind::Digits))
                .collect();
            let mut expected = hints.clone();
            expected.sort_by_key(|hint| hint.range.start);
            let valid = expected.iter().all(|hint|
                hint.range.start < hint.range.end
                    && bounds.contains(&hint.range.start)
                    && bounds.contains(&hint.range.end)
            ) && expected.windows(2).all(|pair| pair[0].range.end <= pair[1].range.start);
            let expected = if valid { Ok(expected) } else { Err(NormalizeError::InvalidHint) };
            prop_assert_eq!(source.hints(&hints), expected);
        }
    }
}
