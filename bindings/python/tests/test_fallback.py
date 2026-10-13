"""Public installed binding contract for opt-in, source-faithful fallback."""

from dataclasses import FrozenInstanceError, asdict, astuple

import pytest

from normalizer_tr import CancellationToken, Hint, NormalizationError, Normalizer

POLICIES = ("preserve", "reject", "fallback")
MELTING_FACE = "\U0001fae0"
MELTING_FACE_SPEECH = "unikod u artı bir fe a e sıfır"
FAMILY = "\U0001f468\u200d\U0001f469\u200d\U0001f467"
FAMILY_SPEECH = (
    "unikod u artı bir fe dört altı sekiz "
    "unikod u artı iki sıfır sıfır de "
    "unikod u artı bir fe dört altı dokuz "
    "unikod u artı iki sıfır sıfır de "
    "unikod u artı bir fe dört altı yedi"
)
VARIATION = "\U0001f642\ufe0f"
VARIATION_SPEECH = "unikod u artı bir fe altı dört iki unikod u artı fe e sıfır fe"


def assert_partition(source, result):
    encoded = source.encode("utf-8")
    cursor = 0
    for segment in result.segments:
        assert segment.start_byte == cursor
        cursor = segment.end_byte
        assert segment.text
        encoded[:cursor].decode("utf-8")
        if segment.kind == "verbatim":
            assert segment.text == encoded[segment.start_byte : cursor].decode("utf-8")
    assert cursor == len(encoded)
    assert (
        "".join(segment.text for segment in result.segments) == result.normalized_text
    )


def assert_symbol_owner(
    source, result, start, end, expected, strategy="unicode_code_point"
):
    assert result.normalized_text == expected
    assert result.complete and not result.issues and result.fallback_used
    assert len(result.fallbacks) == 1
    assert asdict(result.fallbacks[0]) == {
        "start_byte": start,
        "end_byte": end,
        "attempted_class": "symbol",
        "reason": "unhandled_symbol",
        "original_category": None,
        "strategy": strategy,
    }
    owners = [segment for segment in result.segments if segment.kind == "fallback"]
    assert len(owners) == 1
    assert (owners[0].start_byte, owners[0].end_byte) == (start, end)
    assert owners[0].rule_id == "source.fallback"
    assert_partition(source, result)


@pytest.mark.parametrize("policy", POLICIES)
@pytest.mark.parametrize(
    ("source", "spoken"),
    [
        (MELTING_FACE, MELTING_FACE_SPEECH),
        (FAMILY, FAMILY_SPEECH),
        (VARIATION, VARIATION_SPEECH),
        ("\u0301", "unikod u artı sıfır üç sıfır bir"),
        ("\U0010ffff", "unikod u artı bir sıfır fe fe fe fe"),
        (
            FAMILY + VARIATION + MELTING_FACE,
            f"{FAMILY_SPEECH} {VARIATION_SPEECH} {MELTING_FACE_SPEECH}",
        ),
    ],
)
def test_exact_unicode_scalar_sequences_and_owner(source, spoken, policy):
    result = Normalizer().normalize(source, ambiguity_policy=policy)
    if policy == "fallback":
        assert_symbol_owner(source, result, 0, len(source.encode("utf-8")), spoken)
    else:
        assert result.normalized_text == source
        assert result.complete and not result.issues and not result.fallback_used
        assert len(result.segments) == 1 and result.segments[0].kind == "verbatim"
        assert_partition(source, result)


@pytest.mark.parametrize(
    ("source", "spoken"),
    [
        ("Ж12", "unikod u artı sıfır dört bir altı bir iki"),
        ("A\u030a12", "unikod u artı sıfır sıfır ce beş bir iki"),
        ("\u00c512", "unikod u artı sıfır sıfır ce beş bir iki"),
    ],
)
def test_nfc_spelling_keeps_original_identifier_diagnostics(source, spoken):
    normalizer = Normalizer()
    end = len(source.encode("utf-8"))
    preserved = normalizer.normalize(source)
    assert preserved.normalized_text == source and not preserved.complete
    assert not preserved.fallback_used
    assert len(preserved.issues) == 1
    assert (preserved.issues[0].start_byte, preserved.issues[0].end_byte) == (0, end)
    assert preserved.issues[0].category == "protected_identifier"
    assert_partition(source, preserved)
    with pytest.raises(NormalizationError) as captured:
        normalizer.normalize(source, ambiguity_policy="reject")
    assert captured.value.code == "unresolved"
    assert captured.value.issues == preserved.issues
    assert captured.value.limit_kind is None
    assert not hasattr(captured.value, "normalized_text")
    result = normalizer.normalize(source, ambiguity_policy="fallback")
    assert result.normalized_text == spoken
    assert result.complete and not result.issues and result.fallback_used
    assert len(result.segments) == 1
    assert result.segments[0].kind == "fallback"
    assert result.segments[0].rule_id == "source.fallback"
    assert len(result.fallbacks) == 1
    assert asdict(result.fallbacks[0]) == {
        "start_byte": 0,
        "end_byte": end,
        "attempted_class": "identifier",
        "reason": "protected_identifier",
        "original_category": "protected_identifier",
        "strategy": "unicode_code_point",
    }
    assert_partition(source, result)


def test_decomposed_neighbors_padding_and_crlf_keep_original_symbol_owner():
    prefix, suffix = "o\u0308", "eve\r\n"
    source = f"{prefix}{FAMILY}{suffix}"
    result = Normalizer().normalize(source, ambiguity_policy="fallback")
    start = len(prefix.encode("utf-8"))
    end = start + len(FAMILY.encode("utf-8"))
    assert_symbol_owner(
        source, result, start, end, f"{prefix} {FAMILY_SPEECH} {suffix}"
    )


@pytest.mark.parametrize("policy", POLICIES)
def test_unicode_controls_and_whole_owner_hints_remain_errors(policy):
    normalizer = Normalizer()
    for source, hint in [
        ("o\u0308" + FAMILY, Hint(0, 1, "digits")),
        (FAMILY, Hint(0, len("\U0001f468".encode("utf-8")), "digits")),
        ("Ж12", Hint(len("Ж".encode("utf-8")), len("Ж12".encode("utf-8")), "digits")),
    ]:
        with pytest.raises(NormalizationError) as captured:
            normalizer.normalize(source, ambiguity_policy=policy, hints=(hint,))
        assert captured.value.code == "invalid_hint"
        assert captured.value.limit_kind is None and not captured.value.issues
    for source in ("\0", "a\u202eb", "\ud800"):
        with pytest.raises(NormalizationError) as captured:
            normalizer.normalize(source, ambiguity_policy=policy)
        assert captured.value.code == "invalid_input"
    token = CancellationToken()
    token.cancel()
    with pytest.raises(NormalizationError) as captured:
        normalizer.normalize(
            MELTING_FACE * 8192, ambiguity_policy=policy, cancellation=token
        )
    assert captured.value.code == "cancelled"
    assert captured.value.limit_kind is None and not captured.value.issues
    assert not hasattr(captured.value, "normalized_text")


@pytest.mark.parametrize("policy", POLICIES)
def test_unicode_negative_controls_and_primary_hints_keep_all_policies(policy):
    normalizer = Normalizer()
    for source in ("25 TL; saat 09:30", "q\u0301", "προ Hello"):
        expected = normalizer.normalize(source)
        result = normalizer.normalize(source, ambiguity_policy=policy)
        assert result == expected and result.complete and not result.fallback_used
    source = "hello🙂world"
    result = normalizer.normalize(source, ambiguity_policy=policy)
    if policy == "fallback":
        assert_symbol_owner(
            source,
            result,
            len("hello".encode("utf-8")),
            len("hello🙂".encode("utf-8")),
            "hello gülümseyen yüz world",
            "literal",
        )
    else:
        assert result.normalized_text == source and result.complete
        assert not result.fallback_used and not result.issues
    hinted = normalizer.normalize(
        "IV", ambiguity_policy=policy, hints=(Hint(0, 2, "roman"),)
    )
    assert hinted.normalized_text == "dört" and hinted.complete
    assert not hinted.fallback_used and not hinted.issues


@pytest.mark.parametrize(("prefix", "suffix"), [("", ""), ("o\u0308", "eve\r\n")])
def test_installed_target_result_boundary_neighbors(prefix, suffix):
    normalizer = Normalizer()
    maximum_count = (32768 - len((prefix + suffix).encode("utf-8"))) // len(
        MELTING_FACE.encode("utf-8")
    )

    def normalize_count(count):
        source = f"{prefix}{MELTING_FACE * count}{suffix}"
        assert len(source.encode("utf-8")) <= 32768
        try:
            return normalizer.normalize(source, ambiguity_policy="fallback")
        except NormalizationError as error:
            assert error.code == "limit_exceeded"
            assert error.limit_kind == "result" and not error.issues
            assert not hasattr(error, "normalized_text")
            return None

    low, high = 1, maximum_count
    assert normalize_count(low) is not None
    assert normalize_count(high) is None
    while high - low > 1:
        middle = (low + high) // 2
        if normalize_count(middle) is None:
            high = middle
        else:
            low = middle
    for count in (low - 1, low):
        source = f"{prefix}{MELTING_FACE * count}{suffix}"
        expected = (
            prefix
            + (" " if prefix else "")
            + " ".join([MELTING_FACE_SPEECH] * count)
            + (" " if suffix else "")
            + suffix
        )
        result = normalize_count(count)
        assert result is not None
        assert_symbol_owner(
            source,
            result,
            len(prefix.encode("utf-8")),
            len(source.encode("utf-8")) - len(suffix.encode("utf-8")),
            expected,
        )
    assert high == low + 1 and normalize_count(high) is None


@pytest.mark.parametrize(
    ("source", "spoken", "strategy"),
    [
        ("1.234", "bin iki yüz otuz dört", "preferred_number"),
        ("00042", "sıfır sıfır sıfır dört iki", "literal"),
        ("AB12", "a be bir iki", "literal"),
        ("IV", "ı ve", "literal"),
        ("Toplam 25.", "Toplam yirmi beş.", "preferred_number"),
        ("10-15", "on tire on beş", "literal"),
        ("40.03.2026", "kırk Mart iki bin yirmi altı", "surface_date"),
        ("12:30", "on iki otuz", "preferred_time"),
        ("hello🙂world", "hello gülümseyen yüz world", "literal"),
        ("🫠", "unikod u artı bir fe a e sıfır", "unicode_code_point"),
    ],
)
def test_source_faithful_fallback(source: str, spoken: str, strategy: str) -> None:
    result = Normalizer().normalize(source, ambiguity_policy="fallback")
    assert result.normalized_text == spoken
    assert result.complete and not result.issues and result.fallback_used
    assert result.fallbacks[0].strategy == strategy
    assert "".join(segment.text for segment in result.segments) == spoken
    assert any(segment.kind == "fallback" for segment in result.segments)
    assert "fully_rendered" not in asdict(result)
    with pytest.raises(FrozenInstanceError):
        setattr(result.fallbacks[0], "reason", "something")


def test_fallback_provenance_and_primary_precedence() -> None:
    normalizer = Normalizer()
    result = normalizer.normalize("hello🙂", ambiguity_policy="fallback")
    record = result.fallbacks[0]
    assert (record.start_byte, record.end_byte) == (5, 9)
    assert record.attempted_class == "symbol"
    assert record.reason == "unhandled_symbol"
    assert record.original_category is None
    primary = normalizer.normalize("25 TL; saat 09:30")
    fallback = normalizer.normalize("25 TL; saat 09:30", ambiguity_policy="fallback")
    assert fallback == primary and not fallback.fallback_used
    hinted = normalizer.normalize(
        "IV", ambiguity_policy="fallback", hints=(Hint(0, 2, "roman"),)
    )
    assert hinted.normalized_text == "dört" and not hinted.fallback_used
    preserved = normalizer.normalize("1.234")
    assert not preserved.complete and not preserved.fallback_used


def test_decomposed_crlf_policy_owner_uses_original_bytes() -> None:
    normalizer = Normalizer()
    prefix = "o\u0308\r\n"
    source = prefix + "1.234; word"
    preserved = normalizer.normalize(source)
    assert preserved.normalized_text == source and not preserved.complete
    assert len(preserved.issues) == 1 and not preserved.fallbacks
    issue = preserved.issues[0]
    assert (issue.start_byte, issue.end_byte, issue.category) == (5, 10, "ambiguous")
    with pytest.raises(NormalizationError) as error:
        normalizer.normalize(source, ambiguity_policy="reject")
    assert error.value.code == "unresolved"
    assert error.value.issues == preserved.issues
    assert not hasattr(error.value, "normalized_text")
    fallback = normalizer.normalize(source, ambiguity_policy="fallback")
    assert fallback.normalized_text == prefix + "bin iki yüz otuz dört; word"
    assert fallback.complete and not fallback.issues and fallback.fallback_used
    assert len(fallback.fallbacks) == 1
    assert astuple(fallback.fallbacks[0]) == (
        5,
        10,
        "number",
        "missing_intent",
        "ambiguous",
        "preferred_number",
    )
    for result in (preserved, fallback):
        encoded = source.encode("utf-8")
        boundaries = set(range(len(encoded) + 1)) - {1, 2, 4}
        cursor = 0
        for segment in result.segments:
            assert segment.start_byte == cursor
            assert segment.start_byte < segment.end_byte
            assert segment.start_byte in boundaries and segment.end_byte in boundaries
            if segment.kind in ("verbatim", "unresolved"):
                assert (
                    segment.text
                    == encoded[segment.start_byte : segment.end_byte].decode()
                )
            cursor = segment.end_byte
        assert cursor == len(encoded)
        assert "".join(s.text for s in result.segments) == result.normalized_text


def test_fallback_does_not_mask_errors_or_limits() -> None:
    normalizer = Normalizer()
    for source, code in [
        ("", "invalid_input"),
        ("a\u202eb", "invalid_input"),
        ("🫠" * 8192, "limit_exceeded"),
    ]:
        with pytest.raises(NormalizationError) as captured:
            normalizer.normalize(source, ambiguity_policy="fallback")
        assert captured.value.code == code
        if code == "limit_exceeded":
            assert captured.value.limit_kind == "result"
            assert not captured.value.issues
            assert not hasattr(captured.value, "normalized_text")
    with pytest.raises(NormalizationError) as captured:
        normalizer.normalize(
            "AB12", ambiguity_policy="fallback", hints=(Hint(2, 4, "cardinal"),)
        )
    assert captured.value.code == "invalid_hint"
    token = CancellationToken()
    token.cancel()
    with pytest.raises(NormalizationError) as captured:
        normalizer.normalize("🙂", ambiguity_policy="fallback", cancellation=token)
    assert captured.value.code == "cancelled"
    with pytest.raises(TypeError):
        normalizer.normalize("🙂", ambiguity_policy="fallback", deadline_ms=True)
    with pytest.raises(NormalizationError) as captured:
        normalizer.normalize("\ud800", ambiguity_policy="fallback")
    assert captured.value.code == "invalid_input"
