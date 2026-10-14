import dataclasses
from importlib.metadata import version

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
PHONE_LITERAL = "sıfır beş üç iki bir iki üç dört beş altı yedi kesme de e ne"


@pytest.mark.parametrize("policy", ("preserve", "reject", "fallback"))
@pytest.mark.parametrize("prefix", ("", "O\u0308nce ", "ö\r\n", "tel "))
def test_failed_phone_has_one_original_owner_and_independent_unit(policy, prefix):
    source = f"{prefix}{PHONE}; 5 kg"
    start = len(prefix.encode("utf-8"))
    end = start + len(PHONE)
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
    expected_phone = PHONE_LITERAL if policy == "fallback" else PHONE
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
            (start, end, "identifier", "invalid_form", "invalid_expression", "literal")
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
