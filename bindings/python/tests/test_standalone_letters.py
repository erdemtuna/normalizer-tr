import pytest
from normalizer_tr import (
    CancellationToken,
    Hint,
    NormalizationError,
    Normalizer,
)

POLICIES = ("preserve", "reject", "fallback")
WRAPPERS = (("'", "'"), ('"', '"'), ("‘", "’"), ("“", "”"))


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
        ("''S''", "''S''"),
        ("‘'S’'", "‘'S’'"),
        ("'‘S'’", "'‘S'’"),
        ("‘'S'’P", "‘'S'’P"),
        ("'‘S’'P", "'‘S’'P"),
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


@pytest.mark.parametrize("policy", POLICIES)
@pytest.mark.parametrize("opening,closing", WRAPPERS)
@pytest.mark.parametrize("leading", ("", " ", "\t", "\r\n"))
@pytest.mark.parametrize("trailing", ("", " ", "\t", "\r\n"))
def test_certified_quote_edges_preserve_interior_whitespace(
    policy, opening, closing, leading, trailing
):
    source = f"{opening}{leading}P ve S{trailing}{closing}"
    result = Normalizer().normalize(source, ambiguity_policy=policy)
    assert result.normalized_text == f"{opening}{leading}pe ve se{trailing}{closing}"
    assert result.complete and result.issues == result.fallbacks == ()
    letters = [
        segment for segment in result.segments if segment.rule_id == "letter.name"
    ]
    original = source.encode("utf-8")
    assert [(segment.start_byte, segment.end_byte) for segment in letters] == [
        (original.index(b"P"), original.index(b"P") + 1),
        (original.index(b"S"), original.index(b"S") + 1),
    ]
    assert all(segment.kind == "symbol" for segment in letters)
    assert_partition(source, result)


@pytest.mark.parametrize("policy", POLICIES)
@pytest.mark.parametrize(
    "outer,inner",
    [(outer, inner) for outer in WRAPPERS for inner in WRAPPERS if outer != inner],
)
@pytest.mark.parametrize("letter,name", (("S", "se"), ("I\u0307", "i")))
def test_distinct_nested_quotes_own_only_the_original_grapheme(
    policy, outer, inner, letter, name
):
    source = f"{outer[0]}{inner[0]}{letter}{inner[1]}{outer[1]}"
    result = Normalizer().normalize(source, ambiguity_policy=policy)
    assert result.normalized_text == f"{outer[0]}{inner[0]}{name}{inner[1]}{outer[1]}"
    assert result.complete and result.issues == result.fallbacks == ()
    letters = [
        segment for segment in result.segments if segment.rule_id == "letter.name"
    ]
    assert len(letters) == 1 and letters[0].kind == "symbol"
    start = len((outer[0] + inner[0]).encode("utf-8"))
    assert (letters[0].start_byte, letters[0].end_byte) == (
        start,
        start + len(letter.encode("utf-8")),
    )
    assert_partition(source, result)


@pytest.mark.parametrize("policy", POLICIES)
@pytest.mark.parametrize(
    "source,expected,kind,start,end",
    [
        ("' SGK'ya'", "' se ge kaya'", "abbreviation", 2, 8),
        ("' 25 TL'", "' yirmi beş Türk lirası'", "money", 2, 7),
        ("' V. yüzyıl'", "' beşinci yüzyıl'", "roman", 2, 4),
        ("‘'S@ornek.com'’", "‘'S et ornek nokta kom'’", "electronic", 4, 15),
    ],
)
def test_shared_quote_boundaries_preserve_stronger_primary_readers(
    policy, source, expected, kind, start, end
):
    result = Normalizer().normalize(source, ambiguity_policy=policy)
    assert result.normalized_text == expected
    assert result.complete and result.issues == result.fallbacks == ()
    assert result.segments[1].kind == kind
    assert (result.segments[1].start_byte, result.segments[1].end_byte) == (start, end)
    assert not any(segment.rule_id == "letter.name" for segment in result.segments)
    assert_partition(source, result)


@pytest.mark.parametrize("policy", POLICIES)
@pytest.mark.parametrize(
    "source,expected,start,end,category,attempted",
    [
        ("' S123'", "' se bir iki üç'", 2, 6, "protected_identifier", "identifier"),
        ("' 01 V'", "' sıfır bir volt'", 2, 6, "invalid_expression", "quantity"),
        ("' SGK'a'", "' se ge ke kesme a'", 2, 7, "invalid_expression", "abbreviation"),
    ],
)
def test_certified_closing_quote_is_not_an_invalid_owner_suffix(
    policy, source, expected, start, end, category, attempted
):
    n = Normalizer()
    if policy == "reject":
        with pytest.raises(NormalizationError) as captured:
            n.normalize(source, ambiguity_policy=policy)
        assert captured.value.code == "unresolved"
        records = captured.value.issues
    else:
        result = n.normalize(source, ambiguity_policy=policy)
        assert result.normalized_text == (expected if policy == "fallback" else source)
        assert result.complete == (policy == "fallback")
        records = result.fallbacks if policy == "fallback" else result.issues
        assert_partition(source, result)
    assert len(records) == 1
    assert (records[0].start_byte, records[0].end_byte) == (start, end)
    if policy == "fallback":
        assert records[0].attempted_class == attempted
        assert records[0].original_category == category
        assert records[0].strategy == "literal"
    else:
        assert records[0].category == category


@pytest.mark.parametrize("policy", POLICIES)
@pytest.mark.parametrize(
    "source,expected,ranges",
    [
        ("“S ; ' P ve S'", "“S ; ' pe ve S'", [(9, 10)]),
        ("“P ; S”", "“pe ; se”", [(3, 4), (7, 8)]),
    ],
)
def test_quote_context_cannot_be_inferred_from_semicolon_concatenation(
    policy, source, expected, ranges
):
    result = Normalizer().normalize(source, ambiguity_policy=policy)
    assert result.normalized_text == expected
    assert result.complete and result.issues == result.fallbacks == ()
    letters = [
        segment for segment in result.segments if segment.rule_id == "letter.name"
    ]
    assert [(segment.start_byte, segment.end_byte) for segment in letters] == ranges
    assert all(segment.kind == "symbol" for segment in letters)
    assert_partition(source, result)
