//! Independent scanner workloads; gates must preserve these unchanged-core outcomes.
use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackReason, FallbackStrategy, Issue, IssueCategory,
    LimitKind, MAX_CANDIDATES, MAX_INPUT_BYTES, NormalizeError, NormalizeOptions, NormalizeResult,
    SegmentKind, SourceRange,
};

use super::support::ScalingCase;

const SIZES: [usize; 3] = [4096, 16384, 32768];
const POLICIES: [AmbiguityPolicy; 3] = [
    AmbiguityPolicy::Preserve,
    AmbiguityPolicy::Reject,
    AmbiguityPolicy::Fallback,
];

#[derive(Clone, Copy)]
enum Family {
    NoCommaProse,
    SingleIdentifier,
    ManyIdentifiers,
    ApprovedMultiword,
    LateUnknown,
    CommaProse,
    ListLimit,
    PeriodProse,
    UrlNegative,
    DecimalNegative,
    ExplicitAddress,
    InferredAddress,
    PostalLabel,
    MalformedField,
    AddressLimit,
}

impl Family {
    const ALL: [Self; 15] = [
        Self::NoCommaProse,
        Self::SingleIdentifier,
        Self::ManyIdentifiers,
        Self::ApprovedMultiword,
        Self::LateUnknown,
        Self::CommaProse,
        Self::ListLimit,
        Self::PeriodProse,
        Self::UrlNegative,
        Self::DecimalNegative,
        Self::ExplicitAddress,
        Self::InferredAddress,
        Self::PostalLabel,
        Self::MalformedField,
        Self::AddressLimit,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::NoCommaProse => "list-no-comma-prose",
            Self::SingleIdentifier => "list-no-comma-single-identifier",
            Self::ManyIdentifiers => "list-no-comma-many-identifiers",
            Self::ApprovedMultiword => "list-approved-multiword",
            Self::LateUnknown => "list-late-unknown",
            Self::CommaProse => "list-comma-prose",
            Self::ListLimit => "list-candidate-limit",
            Self::PeriodProse => "address-period-prose",
            Self::UrlNegative => "address-url-negative",
            Self::DecimalNegative => "address-decimal-negative",
            Self::ExplicitAddress => "address-explicit",
            Self::InferredAddress => "address-inferred",
            Self::PostalLabel => "address-postal-unpunctuated",
            Self::MalformedField => "address-malformed-field",
            Self::AddressLimit => "address-candidate-limit",
        }
    }

    fn sizes(self) -> &'static [usize] {
        if self.is_limit() {
            &SIZES[SIZES.len() - 1..]
        } else {
            &SIZES
        }
    }

    fn is_limit(self) -> bool {
        matches!(self, Self::ListLimit | Self::AddressLimit)
    }

    fn pattern(self) -> &'static str {
        match self {
            Self::NoCommaProse => "Bu metin sakin bir anlatim sunar ve gunluk olaylari aktarir ",
            Self::SingleIdentifier => "AB",
            Self::ManyIdentifiers => "AB12                         ",
            Self::ApprovedMultiword => "GitHub Copilot,Claude; bu metin sakin ",
            Self::LateUnknown => "GitHub Copilot,Claude,UNKNOWN; ",
            Self::CommaProse => "Bu metin, sakin bir anlatim sunar ve gunluk olaylari aktarir ",
            Self::ListLimit => "CHP,AKP ",
            Self::PeriodProse => "Sözcük. ",
            Self::UrlNegative => "https://ornek.com/                        ",
            Self::DecimalNegative => "2,5                         ",
            Self::ExplicitAddress => {
                "adres: No:5 Kat:3; bu metin sakin bir anlatim sunar ve gunluk olaylari aktarir "
            }
            Self::InferredAddress => {
                "No:5 Kat:3; bu metin sakin bir anlatim sunar ve gunluk olaylari aktarir "
            }
            Self::PostalLabel => {
                "posta kodu 03456; bu metin sakin bir anlatim sunar ve gunluk olaylari aktarir "
            }
            Self::MalformedField => {
                "adres No:abc Kat:3; bu metin sakin bir anlatim sunar ve gunluk olaylari aktarir "
            }
            Self::AddressLimit => "adres No:1 ",
        }
    }

    fn input(self, bytes: usize) -> String {
        assert!(self.sizes().contains(&bytes));
        assert!(bytes <= MAX_INPUT_BYTES);
        if matches!(self, Self::SingleIdentifier) {
            return format!("AB{}", "7".repeat(bytes - self.pattern().len()));
        }
        let pattern = self.pattern();
        let mut text = pattern.repeat(bytes / pattern.len());
        text.push_str(&" ".repeat(bytes - text.len()));
        assert_eq!(text.len(), bytes);
        text
    }

    fn expected_block(self, policy: AmbiguityPolicy) -> &'static str {
        match self {
            Self::ManyIdentifiers if policy == AmbiguityPolicy::Fallback => {
                "a be bir iki                         "
            }
            Self::ApprovedMultiword => "git hab ko paylıt,klod; bu metin sakin ",
            Self::UrlNegative => {
                "ha te te pe es iki nokta eğik çizgi eğik çizgi ornek nokta kom eğik çizgi                        "
            }
            Self::DecimalNegative => "iki virgül beş                         ",
            Self::ExplicitAddress => {
                "adres: numara beş kat üç; bu metin sakin bir anlatim sunar ve gunluk olaylari aktarir "
            }
            Self::InferredAddress => {
                "numara beş kat üç; bu metin sakin bir anlatim sunar ve gunluk olaylari aktarir "
            }
            Self::PostalLabel => {
                "posta kodu sıfır üç dört beş altı; bu metin sakin bir anlatim sunar ve gunluk olaylari aktarir "
            }
            Self::MalformedField if policy == AmbiguityPolicy::Fallback => {
                "adres No iki nokta abc kat üç; bu metin sakin bir anlatim sunar ve gunluk olaylari aktarir "
            }
            Self::MalformedField => {
                "adres No:abc kat üç; bu metin sakin bir anlatim sunar ve gunluk olaylari aktarir "
            }
            _ => self.pattern(),
        }
    }
}

pub(super) fn cases() -> Vec<ScalingCase> {
    let mut cases = Vec::new();
    for family in Family::ALL {
        for &bytes in family.sizes() {
            let text = family.input(bytes);
            for ambiguity_policy in POLICIES {
                cases.push(ScalingCase {
                    id: family.id(),
                    text: text.clone(),
                    options: NormalizeOptions {
                        ambiguity_policy,
                        ..Default::default()
                    },
                });
            }
        }
    }
    cases
}

struct ExpectedSegment {
    range: SourceRange,
    kind: SegmentKind,
    text: String,
    rule: &'static str,
}

struct ExpectedFinding {
    range: SourceRange,
    category: IssueCategory,
    class: FallbackClass,
    reason: FallbackReason,
}

fn owner(
    segments: &mut Vec<ExpectedSegment>,
    range: SourceRange,
    kind: SegmentKind,
    text: impl Into<String>,
    rule: &'static str,
) {
    segments.push(ExpectedSegment {
        range,
        kind,
        text: text.into(),
        rule,
    });
}

fn block_owner(
    segments: &mut Vec<ExpectedSegment>,
    pattern: &str,
    offset: usize,
    source: &str,
    kind: SegmentKind,
    text: &str,
    rule: &'static str,
) {
    let start = offset + pattern.find(source).expect("owner belongs to its pattern");
    owner(
        segments,
        SourceRange::new(start, start + source.len()),
        kind,
        text,
        rule,
    );
}

fn unresolved_owner(
    segments: &mut Vec<ExpectedSegment>,
    findings: &mut Vec<ExpectedFinding>,
    finding: ExpectedFinding,
    policy: AmbiguityPolicy,
    source: &str,
    fallback: impl Into<String>,
) {
    let range = finding.range;
    findings.push(finding);
    if policy == AmbiguityPolicy::Fallback {
        owner(
            segments,
            range,
            SegmentKind::Fallback,
            fallback,
            "source.fallback",
        );
    } else {
        owner(
            segments,
            range,
            SegmentKind::Unresolved,
            source,
            "source.unresolved",
        );
    }
}

fn block_expectations(
    family: Family,
    offset: usize,
    policy: AmbiguityPolicy,
    segments: &mut Vec<ExpectedSegment>,
    findings: &mut Vec<ExpectedFinding>,
) {
    let pattern = family.pattern();
    match family {
        Family::ManyIdentifiers => {
            unresolved_owner(
                segments,
                findings,
                ExpectedFinding {
                    range: SourceRange::new(offset, offset + "AB12".len()),
                    category: IssueCategory::ProtectedIdentifier,
                    class: FallbackClass::Identifier,
                    reason: FallbackReason::ProtectedIdentifier,
                },
                policy,
                "AB12",
                "a be bir iki",
            );
        }
        Family::ApprovedMultiword => {
            block_owner(
                segments,
                pattern,
                offset,
                "GitHub Copilot",
                SegmentKind::Pronunciation,
                "git hab ko paylıt",
                "pronunciation.name",
            );
            block_owner(
                segments,
                pattern,
                offset,
                "Claude",
                SegmentKind::Pronunciation,
                "klod",
                "pronunciation.name",
            );
        }
        Family::UrlNegative => block_owner(
            segments,
            pattern,
            offset,
            "https://ornek.com/",
            SegmentKind::Electronic,
            "ha te te pe es iki nokta eğik çizgi eğik çizgi ornek nokta kom eğik çizgi",
            "electronic.ascii",
        ),
        Family::DecimalNegative => block_owner(
            segments,
            pattern,
            offset,
            "2,5",
            SegmentKind::Decimal,
            "iki virgül beş",
            "number",
        ),
        Family::ExplicitAddress | Family::InferredAddress => {
            for (source, kind, text, rule) in [
                ("No:", SegmentKind::Abbreviation, "numara ", "address.field"),
                ("5", SegmentKind::Cardinal, "beş", "number"),
                ("Kat:", SegmentKind::Abbreviation, "kat ", "address.field"),
                ("3", SegmentKind::Cardinal, "üç", "number"),
            ] {
                block_owner(segments, pattern, offset, source, kind, text, rule);
            }
        }
        Family::PostalLabel => {
            block_owner(
                segments,
                pattern,
                offset,
                "posta kodu ",
                SegmentKind::Abbreviation,
                "posta kodu ",
                "address.field",
            );
            block_owner(
                segments,
                pattern,
                offset,
                "03456",
                SegmentKind::Digits,
                "sıfır üç dört beş altı",
                "address.field",
            );
        }
        Family::MalformedField => {
            let start = offset + pattern.find("No:abc").unwrap();
            unresolved_owner(
                segments,
                findings,
                ExpectedFinding {
                    range: SourceRange::new(start, start + "No:abc".len()),
                    category: IssueCategory::InvalidExpression,
                    class: FallbackClass::Number,
                    reason: FallbackReason::InvalidForm,
                },
                policy,
                "No:abc",
                "No iki nokta abc",
            );
            for (source, kind, text, rule) in [
                ("Kat:", SegmentKind::Abbreviation, "kat ", "address.field"),
                ("3", SegmentKind::Cardinal, "üç", "number"),
            ] {
                block_owner(segments, pattern, offset, source, kind, text, rule);
            }
        }
        _ => {}
    }
}

fn check_issues(issues: &[Issue], expected: &[ExpectedFinding]) {
    assert_eq!(issues.len(), expected.len());
    for (issue, finding) in issues.iter().zip(expected) {
        assert_eq!(issue.range(), finding.range);
        assert_eq!(issue.category(), finding.category);
        assert_eq!(
            issue.explanation(),
            match finding.category {
                IssueCategory::ProtectedIdentifier => "structured identifier is preserved in full",
                IssueCategory::InvalidExpression => {
                    "expression has an invalid value, grammar, or suffix"
                }
                _ => unreachable!("scanner workload has a known finding category"),
            }
        );
    }
}

fn check_result(
    case: &ScalingCase,
    result: &NormalizeResult,
    text: &str,
    segments: &[ExpectedSegment],
    findings: &[ExpectedFinding],
) {
    let fallback = case.options.ambiguity_policy == AmbiguityPolicy::Fallback;
    assert_eq!(result.normalized_text(), text, "{}", case.id);
    assert_eq!(result.locale(), "tr-TR");
    assert_eq!(result.complete(), findings.is_empty() || fallback);
    assert_eq!(result.fallback_used(), fallback && !findings.is_empty());
    if fallback {
        assert!(result.issues().is_empty());
        assert_eq!(result.fallbacks().len(), findings.len());
        for (diagnostic, finding) in result.fallbacks().iter().zip(findings) {
            assert_eq!(diagnostic.range(), finding.range);
            assert_eq!(diagnostic.attempted_class(), finding.class);
            assert_eq!(diagnostic.reason(), finding.reason);
            assert_eq!(diagnostic.original_category(), Some(finding.category));
            assert_eq!(diagnostic.strategy(), FallbackStrategy::Literal);
        }
    } else {
        check_issues(result.issues(), findings);
        assert!(result.fallbacks().is_empty());
    }
    let mut cursor = 0;
    let mut nonverbatim = result
        .segments()
        .iter()
        .filter(|segment| segment.kind() != SegmentKind::Verbatim);
    for expected in segments {
        let actual = nonverbatim.next().expect("expected owner was emitted");
        assert_eq!(actual.range(), expected.range, "{}", case.id);
        assert_eq!(actual.kind(), expected.kind, "{}", case.id);
        assert_eq!(actual.text(), expected.text, "{}", case.id);
        assert_eq!(actual.rule_id(), expected.rule, "{}", case.id);
    }
    assert!(nonverbatim.next().is_none(), "extra owner: {}", case.id);
    for segment in result.segments() {
        let range = segment.range();
        assert_eq!(range.start(), cursor);
        assert!(range.end() > cursor);
        assert!(case.text.is_char_boundary(range.start()));
        assert!(case.text.is_char_boundary(range.end()));
        if segment.kind() == SegmentKind::Verbatim {
            assert_eq!(segment.text(), &case.text[range.start()..range.end()]);
            assert_eq!(segment.rule_id(), "source.verbatim");
        }
        cursor = range.end();
    }
    assert_eq!(cursor, case.text.len());
    assert_eq!(
        result
            .segments()
            .iter()
            .map(|segment| segment.text())
            .collect::<String>(),
        text
    );
}

pub(super) fn check(case: &ScalingCase, result: &Result<NormalizeResult, NormalizeError>) {
    let family = Family::ALL
        .into_iter()
        .find(|family| family.id() == case.id)
        .expect("registered scanner family");
    assert!(case.options.hints.is_empty());
    assert_eq!(case.text, family.input(case.text.len()));
    if family.is_limit() {
        let blocks = case.text.len() / family.pattern().len();
        assert!(blocks * 2 > MAX_CANDIDATES);
        assert_eq!(
            *result,
            Err(NormalizeError::LimitExceeded(LimitKind::Candidates)),
            "{}",
            case.id
        );
        return;
    }
    let policy = case.options.ambiguity_policy;
    let mut segments = Vec::new();
    let mut findings = Vec::new();
    let text = if matches!(family, Family::SingleIdentifier) {
        let spoken = format!(
            "a be {}",
            vec!["yedi"; case.text.len() - family.pattern().len()].join(" ")
        );
        unresolved_owner(
            &mut segments,
            &mut findings,
            ExpectedFinding {
                range: SourceRange::new(0, case.text.len()),
                category: IssueCategory::ProtectedIdentifier,
                class: FallbackClass::Identifier,
                reason: FallbackReason::ProtectedIdentifier,
            },
            policy,
            &case.text,
            &spoken,
        );
        if policy == AmbiguityPolicy::Fallback {
            spoken
        } else {
            case.text.clone()
        }
    } else {
        let pattern = family.pattern();
        let blocks = case.text.len() / pattern.len();
        for block in 0..blocks {
            block_expectations(
                family,
                block * pattern.len(),
                policy,
                &mut segments,
                &mut findings,
            );
        }
        let mut text = family.expected_block(policy).repeat(blocks);
        text.push_str(&case.text[blocks * pattern.len()..]);
        text
    };
    if policy == AmbiguityPolicy::Reject && !findings.is_empty() {
        match result {
            Err(NormalizeError::Unresolved(issues)) => check_issues(issues, &findings),
            other => panic!(
                "expected complete Reject diagnostics for {}: {other:?}",
                case.id
            ),
        }
    } else {
        check_result(
            case,
            result.as_ref().expect("successful scanner throughput case"),
            &text,
            &segments,
            &findings,
        );
    }
}
