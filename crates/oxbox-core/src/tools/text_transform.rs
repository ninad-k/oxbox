//! Tool: **Transform Text** (`transform-text`).
//!
//! Reference implementation of a *pure* tool: no I/O, no OS calls, fully unit-tested.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextCase {
    /// `HELLO WORLD`
    Upper,
    /// `hello world`
    Lower,
    /// `Hello World`
    Title,
    /// `Hello world. Next sentence.`
    Sentence,
    /// `helloWorld`
    Camel,
    /// `HelloWorld`
    Pascal,
    /// `hello_world`
    Snake,
    /// `hello-world`
    Kebab,
    /// `HELLO_WORLD`
    Constant,
}

/// Convert `input` to the requested case.
pub fn transform(input: &str, case: TextCase) -> String {
    use TextCase::*;
    match case {
        Upper => input.to_uppercase(),
        Lower => input.to_lowercase(),
        Title => title_case(input),
        Sentence => sentence_case(input),
        Camel => join_camel(&words(input), false),
        Pascal => join_camel(&words(input), true),
        Snake => join_lower(&words(input), "_"),
        Kebab => join_lower(&words(input), "-"),
        Constant => words(input)
            .iter()
            .map(|w| w.to_uppercase())
            .collect::<Vec<_>>()
            .join("_"),
    }
}

/// Split text into words for programmer cases.
///
/// Splits on any non-alphanumeric character and on case boundaries:
/// `parseHTTPResponse` → `parse`, `HTTP`, `Response`; `version2Update` → `version2`, `Update`.
pub fn words(input: &str) -> Vec<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut current = String::new();

    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            continue;
        }
        if !current.is_empty() && c.is_uppercase() {
            // `current` is non-empty, so chars[i - 1] exists and is alphanumeric.
            let prev = chars[i - 1];
            let next_is_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if prev.is_lowercase() || prev.is_numeric() || (prev.is_uppercase() && next_is_lower) {
                out.push(std::mem::take(&mut current));
            }
        }
        current.push(c);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}

fn title_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut at_word_start = true;
    for c in input.chars() {
        if c.is_alphanumeric() {
            if at_word_start {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            at_word_start = false;
        } else {
            out.push(c);
            // Keep "don't" as "Don't", not "Don'T".
            at_word_start = c != '\'' && c != '\u{2019}';
        }
    }
    out
}

fn sentence_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut capitalize_next = true;
    for c in input.chars() {
        if c.is_alphanumeric() {
            if capitalize_next {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            capitalize_next = false;
        } else {
            out.push(c);
            if matches!(c, '.' | '!' | '?') {
                capitalize_next = true;
            }
        }
    }
    out
}

fn join_camel(words: &[String], pascal: bool) -> String {
    words
        .iter()
        .enumerate()
        .map(|(i, w)| {
            if i == 0 && !pascal {
                w.to_lowercase()
            } else {
                capitalize(w)
            }
        })
        .collect()
}

fn join_lower(words: &[String], sep: &str) -> String {
    words
        .iter()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join(sep)
}

#[cfg(test)]
mod tests {
    use super::TextCase::*;
    use super::*;

    #[test]
    fn upper_and_lower() {
        assert_eq!(transform("Hello World", Upper), "HELLO WORLD");
        assert_eq!(transform("Hello World", Lower), "hello world");
    }

    #[test]
    fn title_keeps_separators() {
        assert_eq!(
            transform("the quick-brown FOX", Title),
            "The Quick-Brown Fox"
        );
        assert_eq!(transform("  two  spaces", Title), "  Two  Spaces");
    }

    #[test]
    fn apostrophes_do_not_start_words() {
        assert_eq!(transform("don't stop", Title), "Don't Stop");
    }

    #[test]
    fn sentence_capitalises_after_terminators() {
        assert_eq!(
            transform("HELLO THERE. how are you? fine!  ok", Sentence),
            "Hello there. How are you? Fine!  Ok"
        );
    }

    #[test]
    fn words_split_on_separators_and_case_boundaries() {
        assert_eq!(
            words("hello_world-foo bar"),
            ["hello", "world", "foo", "bar"]
        );
        assert_eq!(words("parseHTTPResponse"), ["parse", "HTTP", "Response"]);
        assert_eq!(words("version2Update"), ["version2", "Update"]);
        assert!(words("  --  ").is_empty());
    }

    #[test]
    fn programmer_cases() {
        let s = "Parse HTTP response_code";
        assert_eq!(transform(s, Camel), "parseHttpResponseCode");
        assert_eq!(transform(s, Pascal), "ParseHttpResponseCode");
        assert_eq!(transform(s, Snake), "parse_http_response_code");
        assert_eq!(transform(s, Kebab), "parse-http-response-code");
        assert_eq!(transform(s, Constant), "PARSE_HTTP_RESPONSE_CODE");
    }

    #[test]
    fn unicode_is_handled() {
        assert_eq!(transform("élan vital", Title), "Élan Vital");
        assert_eq!(transform("straße", Upper), "STRASSE");
    }

    #[test]
    fn empty_input_is_empty_output() {
        for case in [
            Upper, Lower, Title, Sentence, Camel, Pascal, Snake, Kebab, Constant,
        ] {
            assert_eq!(transform("", case), "");
        }
    }

    #[test]
    fn case_uses_kebab_case_on_the_wire() {
        // This is the contract with the UI (`src/lib/api.ts`).
        assert_eq!(serde_json::to_string(&Constant).unwrap(), "\"constant\"");
        let parsed: TextCase = serde_json::from_str("\"pascal\"").unwrap();
        assert_eq!(parsed, Pascal);
    }
}
