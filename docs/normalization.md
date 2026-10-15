# Normalization reference

This document describes the single built-in normalizer, not selectable reading
profiles. `NORMALIZER_ID` / `normalizer_id` is diagnostic package metadata.
`complete` means all recognized normalization work was resolved; it is not a
claim about native-language or speech quality.

This reference follows repository behavior. See the [changelog](../CHANGELOG.md)
for versioned changes and migration requirements.

## Exact numbers and morphology

Magnitudes are strictly below `1_000_000_000_000_000_000`. Comma decimals have
1-9 fractional digits, read individually: `12,05` -> `on iki virgül sıfır beş`.
Explicit signs, including negative zero, are retained. Dot grouping must use
valid three-digit groups; bare `1.234` remains ambiguous without intent.

Numbers, percentages and approved quantities accept a finite suffix order:
optional plural, optional possession (all six persons), optional case,
optional `ki` after locative, then optional `dır/dir/dur/dür` or its voiceless
counterpart. Each stage occurs at most once. Cases are accusative, dative,
locative, ablative and genitive. Examples: `%20'si` -> `yüzde yirmisi`,
`2026'daki` -> `iki bin yirmi altıdaki`, and `2026'dakidir` ->
`iki bin yirmi altıdakidir`. Other tenses, repeated stages and further
plural/case chains after `ki` are not inferred.

Existing explicit ordinals and percent `lik/lık/luk/lük` readings remain.
Written-form pronunciation and the target spoken stem are distinct:
`25 TL'den` becomes `yirmi beş Türk lirasından`, while `1.'nin` becomes
`birincinin`. Existing successful ordinary-case interpretations have explicit
precedence; other new analyses must agree on their final speech or remain
ambiguous. Emitted text is never reparsed to infer suffixes.

Percentages accept prefix, spaced-prefix and trailing `%` with a valid exact
number and the same supported suffix rules:
`%37,5'lik` -> `yüzde otuz yedi virgül beşlik`;
`%3,25'ten` -> `yüzde üç virgül iki beşten`;
`12,5%'lik` and `% 12,5'lik` -> `yüzde on iki virgül beşlik`.
Duplicate notation and incorrect allomorphs do not become valid by moving `%`.

## Currencies, units and ranges

Only these currencies are supported, with 0-2 fractional digits and no rounding,
conversion, exchange-rate lookup or currency catalog inference:

| Currency | Labels | Major / minor words |
|---|---|---|
| TRY | `TL`, `TRY`, `₺` | `Türk lirası` / `kuruş` |
| USD | `USD`, `$` | `dolar` / `sent` |
| EUR | `EUR`, `€` | `avro` / `sent` |
| GBP | `GBP`, `£` | `sterlin` / `peni` |

Zero minor units are omitted. Unknown or malformed currency expressions remain
whole unresolved spans; see the
[reviewed boundary regressions](#reviewed-boundary-regressions-050).

Explicit possession replaces the compound ending in `Türk lirası`:
`25 TL'm` -> `yirmi beş Türk liram` and `25 TL'lerim` ->
`yirmi beş Türk liralarım`. Fractional amounts attach morphology to the last
spoken minor unit: `25,50 TL'm` -> `yirmi beş Türk lirası elli kuruşum`.
Likewise `600 TL'dir` reads `altı yüz Türk lirasıdır`, while `600,05 TL'dir`
ends in `beş kuruştur`. Input suffixes are validated against the approved
currency-label reading, not the expanded output.

Supported currency labels may be compact or spaced: `25TL`, `25 TL`, `$25`,
`$ 25` and `25$`. Currency context also permits space-grouped money such as
`1 234,50 TL`, `$1 234,50` and `1 234,50TL`. The first group has 1-3 digits
without a leading zero; continuation groups have exactly three digits. Use one
consistent single separator: ordinary space (U+0020), nonbreaking space (U+00A0)
or narrow nonbreaking space (U+202F). Mixed separators, invalid group widths and
excess precision stay whole unresolved amounts. Tabs/newlines are not grouping
separators, and bare numbers or measurements do not gain space-grouping intent.
No global whitespace rewriting is performed.

Approved unit labels are exactly `kg`, `g`, `km`, `m`, `cm`, `mm`, `L`, `mL`,
`mg`, `µg`, `μg`, `gr`, `ml`, `lt`, `dk`, `sn`, `sa`, `m²`, `cm²`, `km²`, `m³`,
`°C`, `V`, `kW`, `kWh` and `GB`.
Aliases are explicit: there is no arbitrary case-folding or NFKC expansion.
Rates are only `km/sa`, `km/h` and `m/s`. They use written unit order:
`5 km/sa` -> `beş kilometre saat`, `5 m/s` -> `beş metre saniye`.
The denominator supplies the suffix tail: `5 km/sa'tir` ->
`beş kilometre saattir`. This deliberately changes earlier `saatte` /
`saniyede` readings; see the [0.7.0 migration notes](../CHANGELOG.md#070---2026-10-11).
Number/label adjacency is supported: `5kg` -> `beş kilogram`.

| Label | Reading in numeric quantity context |
|---|---|
| `°C` | `derece Santigrat` |
| `V` | `volt` |
| `kW` | `kilovat` |
| `kWh` | `kilovat saat` |
| `GB` | `gigabayt` |

Bare symbols do not become unit readings. Supported case families are validated
against the approved spoken names: `75 kW'tan` -> `yetmiş beş kilovattan`;
`10 kWh'ten` -> `on kilovat saatten`. `KW`/`kw`, arbitrary suffix chains and
unapproved units remain unsupported.

Numeric ranges keep exact endpoint order and use compact or spaced `-` / `–` plus an
approved unit or `kişi`, `adet`, `gün`, `yaş` context, or a whole Range hint. Bare `10-15`,
mathematical subtraction/equations and date/currency ranges are not guessed.
`10 - 15 kişi` -> `on ila on beş kişi`.

## Dates, clocks and abbreviations

Gregorian dates accept dotted day-month-year with a four-digit nonzero year,
fixed ISO `YYYY-MM-DD`, `DD/MM/YYYY`, and `YYYY/MM/DD`. Valid year-first slash
dates establish their own intent. Day-first dates retain their bounded
`tarih` / `tarihi` cue, supported colon form or Date hint; exact following
`tarihinde` / `tarihine` also supplies intent.
`tarih 03/04/2026` -> `tarih üç Nisan iki bin yirmi altı`.
Bare day-first slash notation remains unresolved; Fallback reads its written components
literally, not as a preferred/surface calendar date. US-order, two-digit-year
and clock-seconds interpretation are unsupported.

All date formats accept five cases, locative-plus-`ki` and a final copula, but
not whole-date possession/plural. Validate against the last written component
and generate against the final spoken year: `tarih 2026-12-31'de` ->
`tarih otuz bir Aralık iki bin yirmi altıda`. Invalid calendars and wrong
source allomorphs remain unresolved.

Clocks are valid 24-hour values using `:` or `.` with a `saat` cue or Time hint.
A valid locative suffix also establishes dotted-clock intent:
`14.30'da` -> `on dört otuzda`. Single bare dotted forms remain conservative.
Clock locative follows the minute, or hour when zero minutes are elided:
`14.00'te` is valid; `14.00'da` is not. The existing bounded
date-locative + `saat` + clock-locative frame remains locative-only.

Valid clock ranges establish intent with either separator and a compact or
spaced hyphen/en dash. `13:00-17:00` reads `on üç ila on yedi`;
before a following `arasında`, use `ile`. A whole Range hint uses the same rule.
Equal/decreasing endpoints retain written order, without guessing a duration,
next day or timetable validity. Seconds remain unsupported.
Grouped numeric ranges such as `2.000-3.000 kişi` and a date followed by a
prose dash are not clock ranges. Clock-shaped but invalid endpoints remain
whole unresolved ranges.
`Toplam 25.` is not silently treated as an ordinal.

### Multiplication

Flat chains use `x`, `*` or `×` and are read, not calculated:
`12*2*833,25 TL` -> `on iki çarpı iki çarpı sekiz yüz otuz üç Türk lirası
yirmi beş kuruş`. Letter `x` requires surrounding whitespace; explicit
symbols accept compact/spaced forms. Validated multiplication establishes
grouping intent for `1.234 x 2`, without changing bare grouped numbers.

Numeric-only chains use exact numbers. A count-quantity chain contains one
approved money, measurement or rate operand and otherwise nonnegative integer
counts. Grouping and labels belong to their whole operand. Parentheses, equations,
other operators and multiple dimensional quantity operands are not inferred.
Identifiers, phones and electronic text are not operands just because they
contain digits. Failed chains own their whole source.

### Address fields

An explicit address cue with recognizable fields, or a reviewed multi-field
pattern, establishes address context. Only there, `Mah./Mh.`, `Cad./Cd.`,
`Sk./Sok.`, `No:`, `Kat:` and `D:/Daire:` have address meanings, including
reviewed lower/title/uppercase spellings. Attached and spaced fields are equivalent.
Unknown or malformed fields are handled atomically; valid neighbors can still
normalize. This is neither geocoding nor a general address validator.

Without another apartment field, `No:12/4` reads `bina numarası on iki,
daire numarası dört`. With an explicit apartment field, preserve the compound
as `numara on iki bölü dört` and read that separate field. Dotted numbered
streets are ordinal: `245. Sk.` -> `iki yüz kırk beşinci sokak`.

Identified five-digit postal codes read individually with leading zeros.
Add `posta kodu` to a bare identified code, without duplicating a written label.
An ordinary five-digit number outside the address context remains cardinal.
Locality separators become a pause: `Kadıköy/İstanbul` -> `Kadıköy, İstanbul`.
Names/order remain unchanged; no location dictionary or assignment lookup is used.
These address-specific slash rules do not change email/URL readings.

### Abbreviations and initialisms

The explicit abbreviation inventory includes `Dr.` / `dr.` / `DR.`,
`Prof.` / `prof.` / `PROF.`, `Doç.` / `doç.` / `DOÇ.`, `vb.`, `TBMM`,
`PTT`, `NATO`, `IBAN` and `KDV`. Initialisms retain spaced letter readings.
Suffix support is bounded per entry, including approved `KDV'den` ->
`katma değer vergisinden`. Arbitrary abbreviation meanings are not inferred.

The [approved catalog](../src/domain/lexicon/abbreviations.rs) adds
65 exact uppercase initialisms, bringing the inventory to 80 spellings including
existing aliases. Examples include `CHP`, `TRT`, `SGK`, `GPU`, `USB` and
`PDF`. They use explicit Turkish letter readings, not full-name expansions.
`SGK` defaults to `se ge ka`; the approved international entries `API`, `IP`
and `HDMI` read `I` as `i`. This does not change literal Turkish `I`, Roman intent
or source casing.

Only accusative, dative, locative, ablative and genitive case families are
accepted for the new initialisms. Suffixes select an approved
pronunciation rather than being repaired:
`SGK'ya` -> `se ge kaya`, `SGK'ye` -> `se ge keye`;
`PDF'ten` -> `pe de eften`, `PDF'den` -> `pe de feden`.
The `ke`/`ka` alternatives are limited to `SGK`, `BDDK`, `BTK`, `KVKK`, `SPK`,
`TCK`, `TDK`, `TSK`, `YSK` and `SSK`; the `fe`/`ef` alternative is PDF-only.
Other K-ending defaults use `ke`. Invalid or multiple suffixes stay whole
unresolved spans. Word-read acronyms, dotted aliases and other pronunciations
are not inferred from the new catalog.

Unknown uppercase alphabetic prose such as `BUGÜN`, `İSTANBUL` and `ABC`
remains verbatim without an uppercase-only issue or automatic letter spelling,
in every policy. Recognized Roman-looking notation and protected identifiers
retain their distinct intent and fallback rules. This is not an acronym or
proper-name detector.

## Approved foreign names

The [name catalog](../src/domain/lexicon/pronunciations.rs) contains 35 reviewed
readings with 47 exact keys in thematic [AI](../src/domain/lexicon/pronunciations/ai.rs),
[developer](../src/domain/lexicon/pronunciations/developer.rs) and
[consumer](../src/domain/lexicon/pronunciations/consumer.rs) groups.
These are user-approved Turkish-readable **text aliases**, not IPA, phonemes or
official/universal pronunciations. No TTS/audio model or voice quality was tested.

| Exact source | Primary output |
|---|---|
| `Claude` | `klod` |
| `ChatGPT` | `çet ci pi ti` |
| `OpenAI` | `opın ey ay` |
| `GitHub Copilot` | `git hab ko paylıt` |
| `Hugging Face` | `haging feys` |
| `Visual Studio Code` | `vijuıl stüdyo kod` |
| `EMA Lightning`, `ema lightning`, `ema-lightning` | `ema laytning` |
| `Instagram` | `instagram` |
| `WhatsApp`, `Whatsapp` | `vatsap` |

The Instagram and WhatsApp readings above apply from **0.6.0**. Each name has
one chosen reading. See the [migration notes](../CHANGELOG.md#060---2026-10-11)
for changes from 0.5.0 and spellings that no longer validate.

Recognition is automatic under Preserve, Reject and Fallback, without a name
hint or selector. Exact keys such as `Apple`, `Rust`, `Python` and `React`
receive their aliases even in ordinary prose; lowercase `apple`, `rust`,
`python` and `react` remain verbatim. Casing aliases are individually authored
(for example `ChatGPT`, `CHATGPT`, `chatgpt`), not inferred by case-folding.
Unknown words and unapproved spellings remain prose, not guessed names.

Multiword keys use exact single ordinary spaces and at most three words.
The longest approved phrase owns one source span: `GitHub Copilot` is not two
name segments. Tabs, newlines and doubled spaces do not match a phrase;
independently approved words may still resolve on either side. Partial phrase
words such as `Face`, `Studio` and `Code` gain no standalone alias.

Straight or smart apostrophes introduce a validated tail based on the approved
spoken stem, not the last written English letter:
`Claude'un` -> `klodun`, `ChatGPT'ye` -> `çet ci pi tiye`,
`GitHub Copilot'ın` -> `git hab ko paylıtın`.

### Nominal suffixes

Available from 0.6.0.

Every exact approved name accepts suffixes in the order **plural, possessive,
case**. Each stage is optional and can appear at most once. A tail after an
apostrophe must be nonempty and match completely. Cases are accusative, dative,
locative, ablative and genitive. Possession covers all six person and number
combinations:

| Possessor | Example | Primary output |
|---|---|---|
| First person singular | `iPhone'um` | `ayfonum` |
| Second person singular | `iPhone'un` | `ayfonun` |
| Third person singular | `iPhone'u` | `ayfonu` |
| First person plural | `iPhone'umuz` | `ayfonumuz` |
| Second person plural | `iPhone'unuz` | `ayfonunuz` |
| Third person plural | `iPhone'ları` | `ayfonları` |

Each suffix follows the stem produced by the previous stage. Suffixes that can
use `ı`, `i`, `u` or `ü` use `ı` after `lar` and `i` after `ler`.
For example, `YouTube'larımı` reads `yu tublarımı`, not
`yu tublarumu`. A longer combination, `iPhone'larımıza`, reads `ayfonlarımıza`.

Third person possessive forms take a linking `n` before a following case
suffix: `iPhone'una` -> `ayfonuna`. First and second person possessive forms
end in a consonant and attach the case directly: `iPhone'umdan` -> `ayfonumdan`.
Noun plural and third person plural possession share one `lar` or `ler`,
so the grammar accepts `iPhone'ları`, not `iPhone'larları`.

Different grammatical meanings can produce the same speech. The accusative
and possessive forms of `iPhone'u` both read `ayfonu`; the normalizer returns
that shared reading without selecting a grammatical meaning.

Pronunciations ending in a vowel use their own forms: `ChatGPT'm` ->
`çet ci pi tim`, `ChatGPT'sinden` -> `çet ci pi tisinden`.
`ChatGPT'im` is invalid because the spoken stem `ti` already ends in a vowel.
The written acronym letters do not override that stem.
Phrase ownership and exact casing are unchanged.

Suffixes follow the chosen spoken reading: `Instagram'a` -> `instagrama`,
`Instagram'da` -> `instagramda` and `WhatsApp'tan` -> `vatsaptan`.
Spellings based on the old readings, such as `Instagram'e` and `WhatsApp'ten`,
are invalid; they do not select another pronunciation.

A wrong allomorph (`Claude'ye`), empty tail, repeated stage, multiple
apostrophes, derivation or trailing unconsumed text owns the whole
recognized name span: Preserve retains it with an issue, Reject fails, and
Fallback renders the source literally rather than repairing the alias.
Other suffix families are not inferred. This grammar does not expand suffix
coverage for numbers, quantities, units or initialisms.

Successful segments have Rust kind `SegmentKind::Pronunciation`, Python kind
`"pronunciation"` and rule ID `pronunciation.name`, without fallback records.
Hint kinds are unchanged; hints cannot cut a recognized name/phrase.
URL/email interiors, protected identifiers and versioned names such as
`ChatGPT4o` or `Claude-3.5` do not acquire a pronunciation alias; their existing
notation/identifier policy still applies.

## Quotation and list boundaries

Smart double quotes and clearly paired smart/straight single quotes around
quantities stay verbatim: `‘25 TL’` -> `‘yirmi beş Türk lirası’`.
Apostrophes in supported suffixes remain part of their whole expression.
Ambiguous/unmatched forms are not forcibly treated as paired quotations.

**Since 0.8.0:** additional externally isolated quote pairs allow whitespace just
inside either edge and unambiguous nested straight/smart single pairs in either
direction. Whitespace and wrappers stay exact. These frames may also make
valid generic expressions resolve: `' SGK'ya'` -> `' se ge kaya'`, and
`' 25 TL'` -> `' yirmi beş Türk lirası'`.
The closing wrapper is punctuation, not a failed suffix. A genuinely invalid
contained expression still owns its whole content: in `' 01 V'` the quantity
issue/fallback range is `[2,6)`, excluding the quote. Correcting this framing
can change a previously swallowed quote's issue category and literal fallback;
it does not repair the contained value or weaken queued address-field owners.

Semicolon lists and clear comma boundaries after approved quantity labels are
separated without inventing spaces: `25kg,30kg` ->
`yirmi beş kilogram,otuz kilogram`.
Decimal commas remain numeric: `25,30kg` ->
`yirmi beş virgül üç sıfır kilogram`. This is not global splitting on commas.
URL/address punctuation and genuine identifier interiors stay protected.

No-space abbreviation-only comma lists split only when every member has an
approved uppercase base: `CHP,AKP` -> `ce he pe,a ke pe`. Existing forms such as
`TBMM,PTT` use the same rule. A malformed suffix on a known member retains its
own whole-span diagnostic. Mixed unknown lists such as `CHP,UNKNOWN` are not
partially interpreted.
Standalone approved abbreviations also preserve a trailing colon or sentence
period: `TRT:` -> `te re te:`, `LCD.` -> `le ce de.`.
Canonical Roman notation, `GPU:123`, opaque identifiers and electronic interiors
retain their existing ownership and intent rules.

The same narrow comma-list rule accepts exact approved name members:
`ChatGPT,Claude` -> `çet ci pi ti,klod`; `Claude,UNKNOWN` stays verbatim.
Complete phrases participate (`Hugging Face,Claude`), but their last words do
not independently qualify (`Face,Claude`). Approved names also retain paired
quotes and ordinary trailing colon/period punctuation.
Unknown, protected or empty members block successful catalog rewrites throughout
a tight group. Genuine unresolved findings and literal fallback remain available.
Comma-separated spaced prose is normalized independently; a tight group ends
at a space after the comma or an established semicolon boundary. See
[reviewed boundary regressions](#reviewed-boundary-regressions-050).

## Phones, IBANs and Romans

Turkish national phones use `0 + 3 + 3 + 2 + 2` grouping. International `+90`
forms say `artı doksan` followed by the ten written national digits; no trunk
zero is invented. Significant leading zeros in groups remain digitwise.
Plain ten digits require Telephone intent or a clear phone cue. Digits hints
remain digitwise, not telephone prosody. Subscriber assignment is not verified.

Telephone suffixes are unsupported. Once telephone intent and isolated source
membership are established, an attached tail stays with the whole invalid
phone: `0532 123 45 67'den` remains unchanged with one issue over `[0,18)`.
Malformed group widths, incomplete runs, empty/repeated apostrophe tails and
digit-bearing tails do not release numeric fragments. Existing phone-shaped
ownership also includes invalid whitespace separators such as tabs/newlines
or nonbreaking spaces; this does not make those separators valid telephones.
Mixed-space/hyphen groups and adjoining phone parentheses retain their attached
tails too, for example `0532 123 45-67'den` and `0532 123 45(67)'den`.
Non-composing combining marks in an invalid tail stay in that same owner.
Phone-local closing parentheses are included; surrounding prose wrappers
remain outside it.
Repeated or empty tails attached after an owned closer remain in that same
invalid owner, for example `0532 123 45 (67'den)'de`. Such suffix apostrophes
do not open prose quotes across another phone or field.
Numeric parenthesis compounds with apostrophe tails retain their internal
syntax while respecting external semicolons and sentence punctuation; an
invalid phone does not absorb that separator or its independently readable
neighbor. Unsuffixed numeric-parenthesis admission retains its existing path.
For empty-tail parenthesis compounds, a confirmed outer wrapper stays outside
the phone owner too. Unmatched invalid local closers are not stripped to make
that distinction; source pairing is checked only when the raw compound needs it.
The scanner keeps a marked closing-parenthesis grapheme separate from its
following token, so a phone claim cannot skip an adjacent identifier, quantity,
name or number. This also makes standalone `)\u030112` retain its closing
grapheme and read `12` as a Cardinal; marked semicolon/comma numeric compounds
retain their existing atomic behavior.

A successful unsuffixed phone keeps a following independent number:
`0532 123 45 67 5'ten` retains the Telephone plus `beşten`. Protected identifier,
electronic and tight-list interiors are not extracted as telephone tails, and
independent quantities and address fields retain their own ranges.
Hints cannot cut the corrected invalid owner: on the suffixed example,
`Hint(0,11,"digits")` and `Hint(12,18,"cardinal")` now fail with `invalid_hint`,
just like a whole invalid Telephone or Digits hint. All coordinates remain
original UTF-8 bytes, including after decomposed-Unicode prefixes.

IBANs require the full uppercase `TR` + two check digits + 22 numeric BBAN
characters, canonical space grouping and modulo-97 validity. The public
`TR330006100519786457841326` fixture is a checksum example, not a verified
account. Fragments and malformed expressions remain protected. Boundary rules
keep following prose, abbreviations and independently valid quantities separate.

Romans must be canonical uppercase values 1-3999. A whole Roman hint or the
bounded ordinal context `Dünya Savaşı` / `yüzyıl` establishes reading intent.
Bare `IV`, `I.` and list markers are not guessed. `II. Abdülhamit` can use an
explicit hint; there is no general proper-name detector.

## Electronic text and symbols

### Standalone uppercase letters

**Introduced in 0.8.0.**

A supported uppercase letter occupying a complete whitespace-delimited source
atom receives its Turkish letter name as a primary Symbol segment in every
policy, with rule `letter.name`. For example, `P ve S dalgaları` becomes
`pe ve se dalgaları`; `N tipi` becomes `ne tipi`; `V yakalı` becomes `ve yakalı`.
No following noun, suffix, coordination or technical context is required.
Unmarked initials (`S Yılmaz`), ordinary capitals (`O geldi`) and longer
space-separated letter lists deliberately follow the same rule.

Default Preserve does not disable these readings: it preserves unresolved
spans, not text that now has a primary reading. See the
[before/after migration examples](../CHANGELOG.md#behavior-and-migration) for
changes to strict-mode success, segment kinds and fallback diagnostics.

The inventory is the Turkish uppercase alphabet plus `Q`, `W` and `X`, named
`kü`, `çift ve` and `iks`. `I` names `ı`, while `İ` names `i`. Lowercase letters,
unknown uppercase words and unsupported alphabets are not automatically spelled.
Approved initialisms keep their established readings and Abbreviation segments.

Every scanner whitespace separator, including tabs and line breaks, establishes
independent atoms and stays exact. Zero-width characters are not separators.
Confirmed straight single/double or smart single/double quote pairs may wrap
one letter or a phrase: `“P” ve ‘S’` becomes `“pe” ve ‘se’`, and
`"P ve S dalgaları"` becomes `"pe ve se dalgaları"`. Wrappers must be externally
isolated by whitespace or input boundaries. Unambiguous different-style nested
pairs work; unmatched, mismatched or ambiguous pairs and other quote styles do
not provide transparency. Whitespace just inside a confirmed pair is allowed,
including tabs/line breaks. Quotes stay verbatim outside the letter's source range.

Punctuation trimming alone never establishes this intent: `S.`, `S)`, `(S)`,
`P,S`, `S/P`, `S’nin` and `"S"X` do not gain letter readings. In `P, S`, only
the independently isolated `S` gains a name. This is source recognition, not
a pass over emitted text or arbitrary identifier interiors.

Existing primary readings and failed whole-expression owners retain precedence:
`5 V` stays `beş volt`; invalid `01 V` stays one unresolved quantity, and a
failed product or address field never releases a letter to this rule.
Only an eligible bare single Roman letter's missing-intent finding is replaced.
`V` now names `ve` without issues or fallbacks, but `IV`, `C.` and list-marker
forms keep existing intent/policy behavior. `V. yüzyıl` and valid explicit Roman
hints retain numeral readings. `V 5` names V independently of the following
cardinal; it does not acquire a new prefix-unit interpretation.

Recognition uses one supported NFC scalar on its whole original grapheme:
decomposed `I` + U+0307 names `i` with the full original three-byte range.
Other combining marks are not discarded to expose a supported base.
Names, including already word-shaped `A`/`O`, have non-Verbatim Symbol segments;
completion remains a statement about recognized work, not audio quality.
Repeated singleton input can now reach candidate/result budgets rather than
remaining one cheap Verbatim span.

### Electronic source

The bounded grammar supports ASCII unquoted email, HTTP/HTTPS (scheme case
insensitive), `www`, and a bare domain with a `web` / `site` cue or Electronic
hint. Final TLDs are `com`, `net`, `org`, `tr`, `gov`, `edu`, `app`.

Alphabetic chunks keep source case. Schemes, ports, paths, query strings,
fragments, escaped bytes and digit order are spoken without fetching, decoding
or dropping components. `@` in a URI path is not authority userinfo. Quoted or
Unicode email, IDN, authority userinfo, unknown schemes/TLDs and bad escapes
remain unresolved as whole expressions. Prose hashtags and ampersands have
role-aware readings.

## Hints, coordinates and errors

Hints are Cardinal, Digits, Date, Time, Ordinal, Roman, Range, Telephone and
Electronic. They use nonoverlapping, grapheme-safe **original UTF-8 byte ranges**
over whole expressions. Cardinal intent permits explicit zero-padding. Digits
accepts optional initial plus and the supported space/dot/slash/hyphen/
parenthesis separators. A hint cannot legalize an invalid date, checksum or
malformed grammar, or cut a detected compound.

Already-NFC source is borrowed with its grapheme boundaries. Other input uses
NFC recognition with a mapping back to original graphemes. Verbatim/unresolved
text remains exact; no global lowercasing or Unicode compatibility rewrite.
Segments partition the original input contiguously and their text concatenates
to `normalized_text`.

Default Preserve returns useful partial work, `complete = false` and structured
issues. Reject returns `NormalizeError::Unresolved` with the same diagnostics
and no normalized result. Invalid input/hints/configuration, cancellation,
deadlines, resource limits and internal failures remain real errors in every policy.

| Limit | Bound |
|---|---|
| Original input | 32 KiB UTF-8 |
| Hints | 256 |
| Candidate records | 4096 |
| Logical result allocation | 512 KiB |

The allocation budget includes owned segment and final text plus result/
diagnostic structures. Empty/whitespace-only input, Bidi_Control and unsupported
controls are invalid. Cancellation/deadlines are cooperative between bounded
steps, not hard wall-clock guarantees. Outputs contain input text, but the core
does not log it; issue explanations contain no copied input values.

## Reviewed boundary regressions (0.5.0)

The [full-PR review](https://github.com/erdemtuna/normalizer-tr/pull/2#pullrequestreview-5466509996)
and [pronunciation review](https://github.com/erdemtuna/normalizer-tr/pull/2#pullrequestreview-5467492566)
identified four implementation bugs in the
[`76eb8b4` source head](https://github.com/erdemtuna/normalizer-tr/commit/76eb8b454cb3b8079df8df94270f76497963fd71).
The fixes restore the intended contract and are covered by independent
cross-policy regression tests:

- [Numeric-comma scanning](https://github.com/erdemtuna/normalizer-tr/pull/2#discussion_r4227273249):
  quantity-boundary signals advance incrementally, and fallback numeric-run
  boundaries are discovered once. Repeated `1,` or `1.` does not cause
  growing-prefix/remainder rescans. Controls remain cooperative, not a hard SLA.
- [Duplicate attached currencies](https://github.com/erdemtuna/normalizer-tr/pull/2#discussion_r4227273255):
  `$1 234,50TL`, same-currency duplicates and compact symbol variants have one
  invalid whole owner. Reject does not accept successful money fragments.
- [Mixed quantity/abbreviation lists](https://github.com/erdemtuna/normalizer-tr/pull/2#discussion_r4227273258):
  `25kg;CHP,AKP` and the reverse order recognize each safely separated member.
  A quantity's digits do not suppress a later approved initialism/suffix list.
- [Multiword-name comma lists](https://github.com/erdemtuna/normalizer-tr/pull/2#discussion_r4228033170):
  `Claude,Hugging Face,UNKNOWN` stays unchanged: complete tight-group admission
  precedes any successful catalog rewrite. Unknown, protected or empty members
  prevent primary name/abbreviation changes, while genuine identifier/errors and
  literal fallback remain available. Spaced prose such as
  `Claude, Hugging Face, UNKNOWN` still normalizes approved names independently.

Neither `complete=true` nor a passing frozen performance cohort proves universal
coverage or pronunciation quality. Malformed-token/list scaling checks remain
separate from the short/medium latency target.
