"""Public installed binding contract for opt-in, source-faithful fallback."""

from dataclasses import FrozenInstanceError, asdict

import pytest

from normalizer_tr import CancellationToken, Hint, NormalizationError, Normalizer


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
