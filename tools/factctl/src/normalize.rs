use crate::model::{Fact, SourceKind};
use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;
use url::Url;

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

pub fn normalize_primary_source_url(fact: &Fact) -> Option<String> {
    let primary = fact
        .sources
        .iter()
        .find(|source| matches!(source.kind, SourceKind::Official | SourceKind::Primary))
        .or_else(|| fact.sources.first())?;

    normalize_url(&primary.url)
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

fn normalize_url(value: &str) -> Option<String> {
    let mut url = Url::parse(value).ok()?;
    url.set_fragment(None);
    match (url.scheme(), url.port()) {
        ("http", Some(80)) | ("https", Some(443)) => {
            let _ = url.set_port(None);
        }
        _ => {}
    }

    if url.path() != "/" {
        let trimmed = url.path().trim_end_matches('/').to_owned();
        url.set_path(&trimmed);
    }

    Some(url.to_string())
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
    use crate::model::{Fact, FactStatus, Source};
    use chrono::NaiveDate;

    fn sample_fact(url: &str) -> Fact {
        Fact {
            id: "money-001-sample".to_owned(),
            title: "Title".to_owned(),
            primary_genre: "money".to_owned(),
            genres: vec!["money".to_owned()],
            tags: Vec::new(),
            summary: "Summary".to_owned(),
            claim: "Claim".to_owned(),
            explanation: None,
            sources: vec![Source {
                id: "source-1".to_owned(),
                url: url.to_owned(),
                title: "Source".to_owned(),
                publisher: "Publisher".to_owned(),
                kind: SourceKind::Official,
                accessed_at: NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
                quoted_fact: None,
            }],
            status: FactStatus::Published,
            created_at: NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
            updated_at: NaiveDate::from_ymd_opt(2026, 3, 15).expect("valid date"),
            revision: 1,
            aliases: Vec::new(),
            duplicate_of: None,
            supersedes: None,
            canonical: true,
            importance: None,
            editorial: None,
        }
    }

    #[test]
    fn normalizes_text_for_similarity() {
        assert_eq!(normalize_text("ネタ　ＡＢＣ１２３！？"), "ねた abc123");
    }

    #[test]
    fn normalizes_primary_source_url_conservatively() {
        let normalized =
            normalize_primary_source_url(&sample_fact("HTTPS://Example.com/path/?q=1#fragment"))
                .expect("url should normalize");

        assert_eq!(normalized, "https://example.com/path?q=1");
    }

    #[test]
    fn computes_trigram_similarity() {
        let score = trigram_jaccard("same claim", "same claim");
        assert_eq!(score, 1.0);
    }
}
