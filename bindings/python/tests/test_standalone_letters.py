import pytest
from normalizer_tr import (
    CancellationToken,
    Hint,
    NormalizationError,
    Normalizer,
)

POLICIES = ("preserve", "reject", "fallback")


def assert_partition(source, result):
    original = source.encode("utf-8")
    cursor = 0
    for segment in result.segments:
        assert segment.start_byte == cursor
        assert segment.start_byte < segment.end_byte <= len(original)
        owned = original[segment.start_byte : segment.end_byte].decode("utf-8")
        if segment.kind in ("verbatim", "unresolved"):
            assert segment.text == owned
        cursor = segment.end_byte
    assert cursor == len(original)
    assert (
        "".join(segment.text for segment in result.segments) == result.normalized_text
    )
    assert result.complete == (not result.issues)
    assert result.fallback_used == bool(result.fallbacks)


@pytest.mark.parametrize("policy", POLICIES)
def test_primary_symbol_names_and_original_unicode_ranges(policy):
    source = "ö I\u0307 ‘S’\tQ W X"
    result = Normalizer().normalize(source, ambiguity_policy=policy)
    assert result.normalized_text == "ö i ‘se’\tkü çift ve iks"
    assert result.complete and result.issues == result.fallbacks == ()
    letters = [
        segment for segment in result.segments if segment.rule_id == "letter.name"
    ]
    assert [segment.kind for segment in letters] == ["symbol"] * 5
    assert [(segment.start_byte, segment.end_byte) for segment in letters] == [
        (3, 6),
        (10, 11),
        (15, 16),
        (17, 18),
        (19, 20),
    ]
    assert_partition(source, result)


@pytest.mark.parametrize("policy", POLICIES)
@pytest.mark.parametrize(
    "source,expected",
    [
        ("“'P' ve 'S' dalgaları”", "“'pe' ve 'se' dalgaları”"),
        ("S Yılmaz; O geldi.", "se Yılmaz; o geldi."),
        ("P, S", "P, se"),
        ("V 5", "ve beş"),
        ("5 V", "beş volt"),
        ("S. Yılmaz", "S. Yılmaz"),
        ("S) dalgası", "S) dalgası"),
        ("“S’", "“S’"),
        ("“S”P", "“S”P"),
        ("e o a i ı ABC", "e o a i ı ABC"),
        ("S’nin", "S’nin"),
        ("V. yüzyıl", "beşinci yüzyıl"),
    ],
)
def test_primary_letter_boundaries(policy, source, expected):
    result = Normalizer().normalize(source, ambiguity_policy=policy)
    assert result.normalized_text == expected
    assert result.complete and not result.fallback_used
    assert_partition(source, result)


@pytest.mark.parametrize("policy", POLICIES)
def test_letter_names_leave_hints_and_whole_owners_intact(policy):
    n = Normalizer()
    result = n.normalize(
        "V yaka", ambiguity_policy=policy, hints=(Hint(0, 1, "roman"),)
    )
    assert result.normalized_text == "beş yaka"
    assert result.segments[0].kind == "roman"
    for source, start, end in [
        ("S123", 1, 4),
        ("01 V", 0, 2),
        ("1 x S", 4, 5),
        ("I\u0307", 0, 1),
    ]:
        with pytest.raises(NormalizationError) as captured:
            n.normalize(
                source,
                ambiguity_policy=policy,
                hints=(Hint(start, end, "cardinal"),),
            )
        assert captured.value.code == "invalid_hint"


@pytest.mark.parametrize("policy", POLICIES)
def test_independent_letter_does_not_hide_invalid_address_field(policy):
    source = "adres No: P ve S dalgaları Kat:3 D:4"
    n = Normalizer()
    if policy == "reject":
        with pytest.raises(NormalizationError) as captured:
            n.normalize(source, ambiguity_policy=policy)
        assert captured.value.code == "unresolved"
        assert [
            (issue.start_byte, issue.end_byte) for issue in captured.value.issues
        ] == [(6, 11)]
        return
    result = n.normalize(source, ambiguity_policy=policy)
    assert "se dalgaları" in result.normalized_text
    assert result.normalized_text.endswith("kat üç daire dört")
    records = result.fallbacks if policy == "fallback" else result.issues
    assert len(records) == 1
    assert (records[0].start_byte, records[0].end_byte) == (6, 11)
    assert result.complete == (policy == "fallback")
    letters = [
        segment for segment in result.segments if segment.rule_id == "letter.name"
    ]
    assert len(letters) == 1 and letters[0].text == "se"
    assert_partition(source, result)


@pytest.mark.parametrize("policy", POLICIES)
def test_singleton_amplification_does_not_become_preservation(policy):
    n = Normalizer()
    for source, limit in [
        (" ".join(["S"] * 4097), "candidates"),
        (" ".join(["W"] * 4096), "result"),
    ]:
        with pytest.raises(NormalizationError) as captured:
            n.normalize(source, ambiguity_policy=policy)
        assert captured.value.code == "limit_exceeded"
        assert captured.value.limit_kind == limit
    cancelled = CancellationToken()
    cancelled.cancel()
    with pytest.raises(NormalizationError) as captured:
        n.normalize("“S”", ambiguity_policy=policy, cancellation=cancelled)
    assert captured.value.code == "cancelled"
