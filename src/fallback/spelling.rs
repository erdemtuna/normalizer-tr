//! Small explicit name inventory, adapted from compatible PR #1 data.

pub(super) fn letter_name(letter: char) -> Option<&'static str> {
    Some(match letter {
        'a' | 'A' => "a",
        'b' | 'B' => "be",
        'c' | 'C' => "ce",
        'ç' | 'Ç' => "çe",
        'd' | 'D' => "de",
        'e' | 'E' => "e",
        'f' | 'F' => "fe",
        'g' | 'G' => "ge",
        'ğ' | 'Ğ' => "yumuşak ge",
        'h' | 'H' => "he",
        'ı' | 'I' => "ı",
        'i' | 'İ' => "i",
        'j' | 'J' => "je",
        'k' | 'K' => "ke",
        'l' | 'L' => "le",
        'm' | 'M' => "me",
        'n' | 'N' => "ne",
        'o' | 'O' => "o",
        'ö' | 'Ö' => "ö",
        'p' | 'P' => "pe",
        'q' | 'Q' => "kü",
        'r' | 'R' => "re",
        's' | 'S' => "se",
        'ş' | 'Ş' => "şe",
        't' | 'T' => "te",
        'u' | 'U' => "u",
        'ü' | 'Ü' => "ü",
        'v' | 'V' => "ve",
        'w' | 'W' => "çift ve",
        'x' | 'X' => "iks",
        'y' | 'Y' => "ye",
        'z' | 'Z' => "ze",
        _ => return None,
    })
}

pub(super) fn symbol_name(symbol: char) -> Option<&'static str> {
    Some(match symbol {
        '.' => "nokta",
        ',' => "virgül",
        ':' => "iki nokta",
        ';' => "noktalı virgül",
        '/' => "eğik çizgi",
        '\\' => "ters eğik çizgi",
        '-' | '–' | '—' => "tire",
        '+' => "artı",
        '−' => "eksi",
        '=' => "eşittir",
        '×' => "çarpı işareti",
        '*' => "yıldız",
        '÷' => "bölme işareti",
        '%' => "yüzde",
        '&' => "ve",
        '@' => "et",
        '#' => "kare",
        '<' => "küçüktür işareti",
        '>' => "büyüktür işareti",
        '_' => "alt çizgi",
        '(' => "aç parantez",
        ')' => "kapat parantez",
        '[' => "aç köşeli parantez",
        ']' => "kapat köşeli parantez",
        '{' => "aç süslü parantez",
        '}' => "kapat süslü parantez",
        '\'' | '’' => "kesme",
        '"' | '“' | '”' => "tırnak",
        '!' => "ünlem",
        '?' => "soru işareti",
        '|' | '¦' => "dikey çizgi",
        '`' => "ters kesme",
        '~' => "tilde",
        '^' => "şapka işareti",
        '…' => "üç nokta",
        '°' => "derece işareti",
        '₺' => "Türk lirası işareti",
        '$' => "dolar işareti",
        '€' => "avro işareti",
        '£' => "sterlin işareti",
        '¥' => "yen işareti",
        '©' => "telif işareti",
        '®' => "tescilli marka işareti",
        '™' => "marka işareti",
        '•' | '·' | '◦' | '▪' => "nokta işareti",
        '→' | '⇒' => "sağ ok",
        '←' => "sol ok",
        '↑' => "yukarı ok",
        '↓' => "aşağı ok",
        '✓' | '✔' => "onay işareti",
        '★' | '☆' | '⭐' => "yıldız",
        '≈' => "yaklaşık işareti",
        '≠' => "eşit değil işareti",
        '≤' => "küçük eşit işareti",
        '≥' => "büyük eşit işareti",
        '±' => "artı eksi işareti",
        '∞' => "sonsuzluk işareti",
        '‰' => "binde işareti",
        '√' => "karekök işareti",
        'π' => "pi",
        '🙂' => "gülümseyen yüz",
        _ => return None,
    })
}

/// Ordinary prose punctuation is retained for sentence structure, not dropped.
pub(super) fn prose_punctuation(symbol: char) -> bool {
    matches!(
        symbol,
        '.' | ','
            | ';'
            | ':'
            | '!'
            | '?'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '"'
            | '\''
            | '’'
            | '“'
            | '”'
            | '…'
            | '-'
            | '–'
            | '—'
    )
}
