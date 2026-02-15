/// Text tokenizer for scoring: splits on whitespace, camelCase, snake_case,
/// removes stop words, and normalizes to lowercase.
pub struct Tokenizer;

const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "do", "for", "from", "had", "has",
    "have", "he", "her", "his", "how", "i", "if", "in", "into", "is", "it", "its", "just", "me",
    "my", "no", "not", "of", "on", "or", "our", "out", "so", "than", "that", "the", "their",
    "them", "then", "there", "these", "they", "this", "to", "up", "us", "was", "we", "were",
    "what", "when", "which", "who", "will", "with", "would", "you", "your",
];

impl Tokenizer {
    /// Tokenize a string into normalized terms.
    pub fn tokenize(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();

        // Split on whitespace and common separators
        for word in input.split(|c: char| c.is_whitespace() || c == '/' || c == '.' || c == '-') {
            if word.is_empty() {
                continue;
            }

            // Split on underscores (snake_case)
            for part in word.split('_') {
                if part.is_empty() {
                    continue;
                }
                // Split camelCase / PascalCase
                let sub_tokens = split_camel_case(part);
                for token in sub_tokens {
                    let lower = token.to_lowercase();
                    if lower.len() >= 2 && !is_stop_word(&lower) {
                        tokens.push(lower);
                    }
                }
            }
        }

        tokens
    }
}

/// Split a string on camelCase / PascalCase boundaries.
///
/// Examples:
///   "insertBreak" -> ["insert", "Break"]
///   "FileInfo" -> ["File", "Info"]
///   "parseHTTPResponse" -> ["parse", "HTTP", "Response"]
fn split_camel_case(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let bytes = s.as_bytes();
    let mut start = 0;

    for i in 1..bytes.len() {
        let prev_upper = bytes[i - 1].is_ascii_uppercase();
        let curr_upper = bytes[i].is_ascii_uppercase();
        let curr_lower = bytes[i].is_ascii_lowercase();

        // Split at lowercase -> uppercase transition (camelCase)
        let split_camel = !prev_upper && curr_upper;

        // Split at uppercase -> lowercase transition when preceded by multiple uppercase (acronyms)
        // e.g., "HTTPResponse" -> split before 'R' so we get "HTTP" + "Response"
        let split_acronym = prev_upper && curr_lower && i >= 2 && bytes[i - 2].is_ascii_uppercase();

        if split_camel {
            if start < i {
                parts.push(&s[start..i]);
            }
            start = i;
        } else if split_acronym {
            if start < i - 1 {
                parts.push(&s[start..i - 1]);
            }
            start = i - 1;
        }
    }

    if start < s.len() {
        parts.push(&s[start..]);
    }

    parts
}

fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.binary_search(&word).is_ok()
}
