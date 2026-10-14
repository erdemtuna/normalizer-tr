import dataclasses
import json
from importlib.metadata import version
from pathlib import Path

import pytest
from normalizer_tr import (
    NORMALIZER_ID,
    Hint,
    NormalizationError,
    Normalizer,
)


def test_single_identity_current_coverage_and_frozen_records():
    n = Normalizer()
    assert version("normalizer-tr") == "0.7.0"
    assert n.normalizer_id == NORMALIZER_ID == "normalizer-tr/0.7.0"
    for text, output, kind in [
        ("1.'nin", "birincinin", "ordinal"),
        ("25 TL'den", "yirmi beş Türk lirasından", "money"),
        ("5 kg'dan", "beş kilogramdan", "unit"),
        ("10-15 kişi", "on ila on beş kişi", "range"),
        (
            "0850 222 33 44",
            "sıfır sekiz yüz elli iki yüz yirmi iki otuz üç kırk dört",
            "telephone",
        ),
        (
            "TR330006100519786457841326",
            "te re üç üç, sıfır sıfır sıfır altı, bir sıfır sıfır beş, bir dokuz yedi sekiz, altı dört beş yedi, sekiz dört bir üç, iki altı",
            "iban",
        ),
        ("info@ornek.com", "info et ornek nokta kom", "electronic"),
        ("&", "ve", "symbol"),
    ]:
        result = n.normalize(text)
        assert result.complete and result.normalized_text == output
        assert result.normalizer_id == n.normalizer_id
        assert result.segments[0].kind == kind
        with pytest.raises(dataclasses.FrozenInstanceError):
            result.segments[0].kind = "changed"


@pytest.mark.parametrize(
    "text,kind,expected",
    [
        ("25.", "ordinal", "yirmi beşinci"),
        ("IV", "roman", "dört"),
        ("10-15", "range", "on ila on beş"),
        (
            "5321234567",
            "telephone",
            "beş yüz otuz iki yüz yirmi üç kırk beş altmış yedi",
        ),
        ("ornek.com", "electronic", "ornek nokta kom"),
    ],
)
def test_all_new_intent_literals_and_original_ranges(text, kind, expected):
    source = "ö " + text
    hint = Hint(3, len(source.encode("utf-8")), kind)
    result = Normalizer().normalize(source, hints=(hint,))
    assert result.complete and result.normalized_text == "ö " + expected
    assert result.segments[1].start_byte == 3
    assert result.segments[1].end_byte == hint.end_byte


PHONE = "0532 123 45 67'den"
PHONE_CASES = json.loads(
    (
        Path(__file__).resolve().parents[3]
        / "tests"
        / "fixtures"
        / "policy-contract"
        / "telephone-ownership.json"
    ).read_text(encoding="utf-8")
)


@pytest.mark.parametrize(
    "phone,literal,strategy",
    [
        (case["text"], case["fallback"], case["strategy"])
        for case in PHONE_CASES
        if case["id"]
        in (
            "phone-suffixed-national-owner",
            "phone-suffixed-mixed-hyphen",
            "phone-suffixed-adjoining-parentheses",
            "phone-suffixed-parenthesized-final-group",
            "phone-suffixed-parenthesized-tail",
            "phone-suffixed-noncomposing-mark",
            "phone-suffixed-marked-closer-before-tail",
            "phone-suffixed-marked-closer-after-tail",
            "phone-suffixed-empty-parenthesis-chain",
        )
    ],
)
@pytest.mark.parametrize("policy", ("preserve", "reject", "fallback"))
@pytest.mark.parametrize("prefix", ("", "O\u0308nce ", "ö\r\n", "tel "))
def test_failed_phone_has_one_original_owner_and_independent_unit(
    policy, prefix, phone, literal, strategy
):
    source = f"{prefix}{phone}; 5 kg"
    start = len(prefix.encode("utf-8"))
    end = start + len(phone.encode("utf-8"))
    normalizer = Normalizer()
    if policy == "reject":
        with pytest.raises(NormalizationError) as raised:
            normalizer.normalize(source, ambiguity_policy=policy)
        error = raised.value
        assert error.code == "unresolved" and error.limit_kind is None
        assert error.issues == normalizer.normalize(source).issues
        assert not hasattr(error, "normalized_text")
        return
    result = normalizer.normalize(source, ambiguity_policy=policy)
    expected_phone = literal if policy == "fallback" else phone
    assert result.normalized_text == f"{prefix}{expected_phone}; beş kilogram"
    assert result.complete == (policy == "fallback")
    assert result.fallback_used == (policy == "fallback")
    expected = []
    if prefix:
        expected.append((0, start, "verbatim", prefix, "source.verbatim"))
    expected.extend(
        [
            (
                start,
                end,
                "fallback" if policy == "fallback" else "unresolved",
                expected_phone,
                "source.fallback" if policy == "fallback" else "source.unresolved",
            ),
            (end, end + 2, "verbatim", "; ", "source.verbatim"),
            (end + 2, end + 6, "unit", "beş kilogram", "quantity"),
        ]
    )
    assert [dataclasses.astuple(segment) for segment in result.segments] == expected
    if policy == "fallback":
        assert result.issues == ()
        assert [dataclasses.astuple(record) for record in result.fallbacks] == [
            (
                start,
                end,
                "identifier",
                "invalid_form",
                "invalid_expression",
                "unicode_code_point" if strategy == "UnicodeCodePoint" else "literal",
            )
        ]
    else:
        assert result.fallbacks == ()
        assert [dataclasses.astuple(issue) for issue in result.issues] == [
            (
                start,
                end,
                "invalid_expression",
                "expression has an invalid value, grammar, or suffix",
            )
        ]
    assert (
        "".join(segment.text for segment in result.segments) == result.normalized_text
    )


@pytest.mark.parametrize("policy", ("preserve", "reject", "fallback"))
@pytest.mark.parametrize("prefix", ("", "O\u0308nce "))
@pytest.mark.parametrize(
    "start,end,kind",
    (
        (0, 11, "digits"),
        (12, 18, "cardinal"),
        (0, 18, "telephone"),
        (0, 18, "digits"),
    ),
)
def test_failed_phone_cannot_be_cut_or_legalized_by_a_hint(
    policy, prefix, start, end, kind
):
    offset = len(prefix.encode("utf-8"))
    with pytest.raises(NormalizationError) as raised:
        Normalizer().normalize(
            prefix + PHONE,
            ambiguity_policy=policy,
            hints=(Hint(offset + start, offset + end, kind),),
        )
    assert raised.value.code == "invalid_hint"
    assert raised.value.issues == () and raised.value.limit_kind is None


@pytest.mark.parametrize("policy", ("preserve", "reject", "fallback"))
@pytest.mark.parametrize(
    "phone,end",
    [
        ("0532 123 45-67'den", 8),
        ("0532 123 45(67)'den", 14),
        ("0532 123 45 (67'den)", 19),
        ("0532 123 45 67'd\u0308en", 11),
        ("0532 123 45(67)\u0301'den", 14),
    ],
)
def test_phone_punctuation_and_marks_do_not_allow_shortened_hints(policy, phone, end):
    prefix = "O\u0308nce "
    start = len(prefix.encode("utf-8"))
    with pytest.raises(NormalizationError) as raised:
        Normalizer().normalize(
            prefix + phone,
            ambiguity_policy=policy,
            hints=(Hint(start, start + end, "digits"),),
        )
    assert raised.value.code == "invalid_hint"
    assert raised.value.issues == () and raised.value.limit_kind is None


@pytest.mark.parametrize("policy", ("preserve", "reject", "fallback"))
def test_valid_phone_keeps_following_suffixed_cardinal_and_whole_hint(policy):
    source = "0532 123 45 67 5'ten"
    result = Normalizer().normalize(
        source, ambiguity_policy=policy, hints=(Hint(0, 14, "telephone"),)
    )
    assert result.complete and result.issues == result.fallbacks == ()
    assert [dataclasses.astuple(segment) for segment in result.segments] == [
        (
            0,
            14,
            "telephone",
            "sıfır beş yüz otuz iki yüz yirmi üç kırk beş altmış yedi",
            "telephone.tr",
        ),
        (14, 15, "verbatim", " ", "source.verbatim"),
        (15, 20, "cardinal", "beşten", "number"),
    ]


@pytest.mark.parametrize("policy", ("preserve", "reject", "fallback"))
def test_parenthesis_suffix_does_not_pair_with_a_later_telephone_apostrophe(policy):
    first = "0532 123 45(67)'den"
    second = "0532 123 45 67'"
    source = first + " ; " + second
    expected_ranges = [(0, len(first)), (len(first) + 3, len(source))]
    if policy == "reject":
        with pytest.raises(NormalizationError) as raised:
            Normalizer().normalize(source, ambiguity_policy=policy)
        assert raised.value.code == "unresolved"
        records = raised.value.issues
    else:
        result = Normalizer().normalize(source, ambiguity_policy=policy)
        records = result.fallbacks if policy == "fallback" else result.issues
        assert result.complete == (policy == "fallback")
        if policy == "preserve":
            assert result.normalized_text == source
        else:
            assert result.normalized_text == (
                "sıfır beş üç iki bir iki üç dört beş aç parantez altı yedi "
                "kapat parantez kesme de e ne ; sıfır beş üç iki bir iki üç "
                "dört beş altı yedi kesme"
            )
    assert [
        (record.start_byte, record.end_byte) for record in records
    ] == expected_ranges


@pytest.mark.parametrize("policy", ("preserve", "reject", "fallback"))
@pytest.mark.parametrize(
    "neighbor,kind,spoken",
    [
        ("AB12", "unresolved", "AB12"),
        ("5kg", "unit", "beş kilogram"),
        ("Claude", "pronunciation", "klod"),
        ("12", "cardinal", "on iki"),
    ],
)
def test_marked_phone_closer_keeps_the_following_owner(policy, neighbor, kind, spoken):
    prefix = "O\u0308nce "
    phone = "0532 123 45 (67'den)\u0301"
    source = prefix + phone + neighbor + "; 5 kg"
    start = len(prefix.encode("utf-8"))
    end = start + len(phone.encode("utf-8"))
    tail_end = end + len(neighbor)
    normalizer = Normalizer()
    if policy == "reject":
        with pytest.raises(NormalizationError) as raised:
            normalizer.normalize(source, ambiguity_policy=policy)
        assert raised.value.code == "unresolved"
        assert raised.value.issues == normalizer.normalize(source).issues
        return
    result = normalizer.normalize(source, ambiguity_policy=policy)
    assert result.complete == (policy == "fallback")
    records = result.fallbacks if policy == "fallback" else result.issues
    assert (records[0].start_byte, records[0].end_byte) == (start, end)
    assert len(records) == (2 if neighbor == "AB12" else 1)
    if neighbor == "AB12":
        assert (records[1].start_byte, records[1].end_byte) == (end, tail_end)
        if policy == "fallback":
            assert records[1].attempted_class == "identifier"
            assert records[1].reason == "protected_identifier"
    else:
        assert any(
            (s.start_byte, s.end_byte, s.kind, s.text) == (end, tail_end, kind, spoken)
            for s in result.segments
        )
    cursor = 0
    for segment in result.segments:
        assert segment.start_byte == cursor
        cursor = segment.end_byte
    assert cursor == len(source.encode("utf-8"))
    assert "".join(s.text for s in result.segments) == result.normalized_text
    assert result.segments[-1].kind == "unit"
    assert result.segments[-1].text == "beş kilogram"


@pytest.mark.parametrize("policy", ("preserve", "reject", "fallback"))
@pytest.mark.parametrize("tail", ("'de", "'d2", "'den'de", "'"))
def test_repeated_tail_after_phone_closer_cannot_hide_the_next_empty_tail(policy, tail):
    prefix = "O\u0308nce "
    first = "0532 123 45 (67'den)" + tail
    second = "0532 123 45 67'"
    source = prefix + first + " ; " + second + "; 5 kg"
    first_start = len(prefix.encode("utf-8"))
    first_end = first_start + len(first)
    second_start = first_end + 3
    second_end = second_start + len(second)
    normalizer = Normalizer()
    if policy == "reject":
        with pytest.raises(NormalizationError) as raised:
            normalizer.normalize(source, ambiguity_policy=policy)
        assert raised.value.code == "unresolved"
        records = raised.value.issues
    else:
        result = normalizer.normalize(source, ambiguity_policy=policy)
        assert result.complete == (policy == "fallback")
        records = result.fallbacks if policy == "fallback" else result.issues
        assert all(segment.kind != "telephone" for segment in result.segments)
        if policy == "preserve":
            assert (
                result.normalized_text
                == prefix + first + " ; " + second + "; beş kilogram"
            )
        assert "".join(s.text for s in result.segments) == result.normalized_text
    assert [(record.start_byte, record.end_byte) for record in records] == [
        (first_start, first_end),
        (second_start, second_end),
    ]
    with pytest.raises(NormalizationError) as raised:
        normalizer.normalize(
            source,
            ambiguity_policy=policy,
            hints=(Hint(second_start, second_end - 1, "telephone"),),
        )
    assert raised.value.code == "invalid_hint"


@pytest.mark.parametrize("policy", ("preserve", "reject", "fallback"))
@pytest.mark.parametrize(
    "prefix,closing",
    [("(", ")"), ("((", "))"), ("O\u0308nce (", ")"), ("‘", "’"), ('"', '"')],
)
def test_empty_tail_phone_compound_keeps_its_confirmed_outer_wrapper(
    policy, prefix, closing
):
    phone = "0532 123 45(67')'"
    source = prefix + phone + closing + "; 5 kg"
    start = len(prefix.encode("utf-8"))
    end = start + len(phone)
    normalizer = Normalizer()
    if policy == "reject":
        with pytest.raises(NormalizationError) as raised:
            normalizer.normalize(source, ambiguity_policy=policy)
        assert raised.value.code == "unresolved"
        assert [(i.start_byte, i.end_byte) for i in raised.value.issues] == [
            (start, end)
        ]
        return
    result = normalizer.normalize(source, ambiguity_policy=policy)
    records = result.fallbacks if policy == "fallback" else result.issues
    assert len(records) == 1 and (records[0].start_byte, records[0].end_byte) == (
        start,
        end,
    )
    expected_phone = (
        "sıfır beş üç iki bir iki üç dört beş aç parantez altı yedi kesme kapat parantez kesme"
        if policy == "fallback"
        else phone
    )
    assert (
        result.normalized_text == prefix + expected_phone + closing + "; beş kilogram"
    )
    assert result.complete == (policy == "fallback")
    assert "".join(s.text for s in result.segments) == result.normalized_text
