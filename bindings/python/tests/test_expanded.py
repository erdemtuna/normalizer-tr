import json
import re
from pathlib import Path

import pytest
from normalizer_tr import Hint, NormalizationError, Normalizer

CASES = json.loads(
    (
        Path(__file__).resolve().parents[3]
        / "tests"
        / "fixtures"
        / "expanded-coverage.json"
    ).read_text(encoding="utf-8")
)


def label(value):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", value).lower()


@pytest.mark.parametrize("case", CASES, ids=lambda case: case["id"])
@pytest.mark.parametrize("policy", ("preserve", "reject", "fallback"))
def test_shared_public_policy_contract(case, policy):
    text = case["text"]
    hint = case.get("hint")
    hints = () if hint is None else (Hint(hint["start"], hint["end"], hint["kind"]),)
    n = Normalizer()
    if "category" in case and policy == "reject":
        with pytest.raises(NormalizationError) as error:
            n.normalize(text, ambiguity_policy=policy, hints=hints)
        assert error.value.code == "unresolved"
        assert error.value.issues == n.normalize(text, hints=hints).issues
        return
    result = n.normalize(text, ambiguity_policy=policy, hints=hints)
    if "category" in case:
        expected = case["fallback"] if policy == "fallback" else text
        records = result.fallbacks if policy == "fallback" else result.issues
        assert len(records) == 1
        record = records[0]
        assert (record.start_byte, record.end_byte) == (0, len(text.encode("utf-8")))
        if policy == "fallback":
            assert record.attempted_class == label(case["fallback_class"])
            assert record.strategy == label(case["strategy"])
            assert record.original_category == label(case["category"])
        else:
            assert record.category == label(case["category"])
    else:
        expected = case["expected"]
        assert result.issues == result.fallbacks == ()
        if "kind" in case:
            assert len(result.segments) == 1
            assert result.segments[0].kind == label(case["kind"])
    assert result.normalized_text == expected
    assert result.complete == ("category" not in case or policy == "fallback")
    assert result.fallback_used == bool(result.fallbacks)
    source = text.encode("utf-8")
    cursor = 0
    for segment in result.segments:
        assert segment.start_byte == cursor
        cursor = segment.end_byte
        if segment.kind in ("verbatim", "unresolved"):
            assert segment.text == source[segment.start_byte : cursor].decode("utf-8")
    assert cursor == len(source)
    assert "".join(segment.text for segment in result.segments) == expected
