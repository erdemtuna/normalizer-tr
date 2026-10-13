import concurrent.futures
import dataclasses
import inspect
import sys
import threading
import time
from pathlib import Path

import pytest
import normalizer_tr
from normalizer_tr import CancellationToken, Hint, NormalizationError, Normalizer

FIXTURES = Path(__file__).resolve().parents[3] / "tests" / "fixtures"


def test_public_exports_call_defaults_and_record_fields():
    assert set(normalizer_tr.__all__) == {
        "Normalizer",
        "NormalizationError",
        "Hint",
        "Issue",
        "Segment",
        "NormalizeResult",
        "FallbackDiagnostic",
        "CancellationToken",
        "NORMALIZER_ID",
        "BUILD_VERSION",
    }
    assert list(inspect.signature(Normalizer).parameters) == []
    parameters = inspect.signature(Normalizer.normalize).parameters
    assert list(parameters) == [
        "self",
        "text",
        "ambiguity_policy",
        "hints",
        "cancellation",
        "deadline_ms",
    ]
    assert parameters["ambiguity_policy"].default == "preserve"
    assert parameters["hints"].default == ()
    assert parameters["cancellation"].default is None
    assert parameters["deadline_ms"].default is None
    assert all(
        parameters[name].kind == inspect.Parameter.KEYWORD_ONLY
        for name in ("ambiguity_policy", "hints", "cancellation", "deadline_ms")
    )
    for name, expected in {
        "Hint": ("start_byte", "end_byte", "kind"),
        "Issue": ("start_byte", "end_byte", "category", "explanation"),
        "Segment": ("start_byte", "end_byte", "kind", "text", "rule_id"),
        "FallbackDiagnostic": (
            "start_byte",
            "end_byte",
            "attempted_class",
            "reason",
            "original_category",
            "strategy",
        ),
        "NormalizeResult": (
            "normalized_text",
            "locale",
            "normalizer_id",
            "complete",
            "segments",
            "issues",
            "fallbacks",
        ),
    }.items():
        cls = getattr(normalizer_tr, name)
        assert tuple(field.name for field in dataclasses.fields(cls)) == expected
        assert cls.__dataclass_params__.frozen
    assert dataclasses.fields(normalizer_tr.NormalizeResult)[-1].default == ()


def test_golden_and_exact_original_ranges():
    text = (FIXTURES / "financial-input.txt").read_text(encoding="utf-8").rstrip("\r\n")
    expected = (
        (FIXTURES / "financial-output.txt").read_text(encoding="utf-8").rstrip("\r\n")
    )
    for policy in ("preserve", "reject"):
        result = Normalizer().normalize(text, ambiguity_policy=policy)
        assert result.normalized_text == expected
        assert result.complete and result.issues == ()
        assert result.locale == "tr-TR"
        assert [
            (s.kind, s.start_byte, s.end_byte)
            for s in result.segments
            if s.kind != "verbatim"
        ] == [
            ("date", 0, 13),
            ("time", 19, 27),
            ("percent", 51, 60),
            ("abbreviation", 70, 73),
            ("money", 80, 95),
        ]
        assert "".join(s.text for s in result.segments) == expected
        with pytest.raises(dataclasses.FrozenInstanceError):
            result.complete = False
        with pytest.raises(dataclasses.FrozenInstanceError):
            result.segments[0].text = "changed"
    assert "torch" not in sys.modules and "huggingface_hub" not in sys.modules


@pytest.mark.parametrize("policy", ["preserve", "reject", "fallback"])
def test_crlf_and_decomposed_hints_keep_original_coordinates(policy):
    normalizer = Normalizer()
    source = "o\u0308\r\n1;\r\n2;\t3"
    result = normalizer.normalize(
        source,
        ambiguity_policy=policy,
        hints=(
            Hint(12, 13, "cardinal"),
            Hint(5, 6, "cardinal"),
            Hint(9, 10, "cardinal"),
        ),
    )
    assert result.normalized_text == "o\u0308\r\nbir;\r\niki;\tüç"
    assert result.complete and not result.issues and not result.fallbacks
    assert [
        (s.start_byte, s.end_byte, s.kind, s.rule_id)
        for s in result.segments
        if s.kind != "verbatim"
    ] == [
        (5, 6, "cardinal", "number"),
        (9, 10, "cardinal", "number"),
        (12, 13, "cardinal", "number"),
    ]
    encoded = source.encode("utf-8")
    boundaries = set(range(len(encoded) + 1)) - {1, 2, 4, 8}
    cursor = 0
    for segment in result.segments:
        assert segment.start_byte == cursor
        assert segment.start_byte < segment.end_byte
        assert segment.start_byte in boundaries and segment.end_byte in boundaries
        if segment.kind == "verbatim":
            original = encoded[segment.start_byte : segment.end_byte].decode()
            assert segment.text == original
        cursor = segment.end_byte
    assert cursor == len(encoded)
    assert "".join(s.text for s in result.segments) == result.normalized_text


@pytest.mark.parametrize("policy", ["preserve", "reject", "fallback"])
@pytest.mark.parametrize(
    ("source", "start", "end"),
    [
        ("1\r\n2", 0, 2),
        ("1\r\n2", 2, 4),
        ("1\r\n2", 0, 5),
        ("1\r\n2", 0, (1 << 63) - 1),
        ("1\r\n2", 3, 3),
        ("1\r\n2", 4, 3),
        ("o\u0308\r\n2", 0, 1),
        ("25 TL\r\n2", 0, 2),
        ("AB12\r\n2", 2, 4),
    ],
)
def test_invalid_original_hints_remain_errors_in_all_policies(
    policy, source, start, end
):
    with pytest.raises(NormalizationError) as error:
        Normalizer().normalize(
            source, ambiguity_policy=policy, hints=(Hint(start, end, "cardinal"),)
        )
    assert error.value.code == "invalid_hint"
    assert error.value.issues == () and error.value.limit_kind is None
    assert not hasattr(error.value, "normalized_text")


def test_partial_strict_and_unicode():
    normalizer = Normalizer()
    partial = normalizer.normalize("25 TL; 1.234")
    assert not partial.complete
    assert partial.normalized_text == "yirmi beş Türk lirası; 1.234"
    with pytest.raises(NormalizationError) as error:
        normalizer.normalize("25 TL; 1.234", ambiguity_policy="reject")
    assert error.value.code == "unresolved"
    assert error.value.issues == partial.issues
    assert not hasattr(error.value, "normalized_text")
    with pytest.raises(AttributeError):
        error.value.issues = ()
    text = "ü 4’u\u0308n"
    result = normalizer.normalize(text)
    assert result.normalized_text == "ü dördün"
    assert result.segments[-1].end_byte == len(text.encode("utf-8"))
    assert normalizer.normalize("TBMM’ye").normalized_text == "te be me meye"
    assert (
        normalizer.normalize("ö 00042", hints=[Hint(3, 8, "digits")]).normalized_text
        == "ö sıfır sıfır sıfır dört iki"
    )


@pytest.mark.parametrize("value", [True, False, -1, 1.5, "2", 1 << 80])
def test_hint_integer_types(value):
    with pytest.raises((TypeError, ValueError)):
        Hint(value, 3, "digits")


@pytest.mark.parametrize("deadline", [True, False, 0, -1, 60001, 1.5, "1", 1 << 80])
def test_deadline_types(deadline):
    with pytest.raises((TypeError, ValueError)):
        Normalizer().normalize("1", deadline_ms=deadline)


def test_real_errors_and_control():
    n = Normalizer()
    for text, code in [
        ("", "invalid_input"),
        ("\ud800", "invalid_input"),
        ("a\u202e12", "invalid_input"),
        ("a" * 32769, "limit_exceeded"),
    ]:
        with pytest.raises(NormalizationError) as error:
            n.normalize(text)
        assert error.value.code == code
        if code == "limit_exceeded":
            assert error.value.limit_kind == "input"
    with pytest.raises(TypeError):
        Normalizer("latest")
    with pytest.raises(NormalizationError) as error:
        n.normalize("ü12", hints=[Hint(1, 4, "cardinal")])
    assert error.value.code == "invalid_hint"
    token = CancellationToken()
    token.cancel()
    with pytest.raises(NormalizationError) as error:
        n.normalize("1", cancellation=token, deadline_ms=60000)
    assert error.value.code == "cancelled"
    for value in (1, b"1", None):
        with pytest.raises(TypeError):
            n.normalize(value)


def test_deadline_is_a_real_monotonic_core_error():
    n = Normalizer()
    errors = []
    for _ in range(5):
        try:
            n.normalize("a" + "\u0315\u0300" * 8000, deadline_ms=1)
        except NormalizationError as error:
            errors.append(error.code)
    assert "cancelled" in errors


def test_actual_concurrent_calls_and_other_thread_cancellation():
    n = Normalizer()
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        results = list(pool.map(lambda _: n.normalize("25 TL"), range(100)))
    assert all(result.normalized_text == "yirmi beş Türk lirası" for result in results)
    token = CancellationToken()
    began = threading.Event()

    def work():
        began.set()
        for _ in range(10000):
            try:
                n.normalize("123 " * 3000, cancellation=token)
            except NormalizationError as exc:
                return exc.code
        return "not_cancelled"

    with concurrent.futures.ThreadPoolExecutor() as pool:
        task = pool.submit(work)
        assert began.wait(2)
        time.sleep(0.002)
        token.cancel()
        assert task.result(timeout=5) == "cancelled"
