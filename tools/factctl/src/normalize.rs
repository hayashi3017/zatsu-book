use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;

const JAPANESE_PUNCTUATION: &[char] = &[
    '、', '。', '・', '：', '；', '？', '！', '「', '」', '『', '』', '（', '）', '［', '］', '｛',
    '｝', '〈', '〉', '《', '》', '【', '】', '〔', '〕', '…', 'ー', '〜', '～', '，', '．', '／',
    '＼', '“', '”', '‘', '’',
];

pub fn normalize_claim(value: &str) -> String {
    normalize_text(value)
}

pub fn normalize_text(value: &str) -> String {
    let normalized = value.nfkc().collect::<String>();
    let mut out = String::with_capacity(normalized.len());
    let mut pending_space = false;

    for ch in normalized.chars() {
        let lowered = ch.to_lowercase().collect::<String>();
        for lowered in lowered.chars() {
            let folded = fold_katakana_to_hiragana(lowered);
            if keep_char(folded) {
                if pending_space && !out.is_empty() {
                    out.push(' ');
                }
                out.push(folded);
                pending_space = false;
            } else if folded.is_whitespace() || is_separator(folded) {
                pending_space = !out.is_empty();
            }
        }
    }

    out.trim().to_owned()
}

pub fn trigram_jaccard(left: &str, right: &str) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    if left == right {
        return 1.0;
    }

    let left = trigrams(left);
    let right = trigrams(right);
    let intersection = left.intersection(&right).count() as f64;
    let union = left.union(&right).count() as f64;

    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn fold_katakana_to_hiragana(ch: char) -> char {
    if ('\u{30A1}'..='\u{30F6}').contains(&ch) {
        char::from_u32(ch as u32 - 0x60).unwrap_or(ch)
    } else {
        ch
    }
}

fn keep_char(ch: char) -> bool {
    ch.is_alphanumeric()
        || matches!(
            ch,
            '\u{3041}'..='\u{3096}'
                | '\u{3400}'..='\u{4DBF}'
                | '\u{4E00}'..='\u{9FFF}'
                | '\u{F900}'..='\u{FAFF}'
        )
}

fn is_separator(ch: char) -> bool {
    ch.is_ascii_punctuation() || JAPANESE_PUNCTUATION.contains(&ch)
}

fn trigrams(value: &str) -> BTreeSet<String> {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return BTreeSet::new();
    }
    if chars.len() < 3 {
        return [chars.iter().collect::<String>()].into_iter().collect();
    }

    chars
        .windows(3)
        .map(|window| window.iter().collect::<String>())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_text_for_similarity() {
        assert_eq!(normalize_text("ネタ　ＡＢＣ１２３！？"), "ねた abc123");
    }

    #[test]
    fn computes_trigram_similarity() {
        let score = trigram_jaccard("same claim", "same claim");
        assert_eq!(score, 1.0);
    }
}
