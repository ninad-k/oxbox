//! Explicit, bounded plain-text cleanup. No clipboard access or persistence.

use crate::{CoreError, CoreResult, jobs::CancelToken};

pub const MAX_INPUT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub trim_lines: bool,
    pub collapse_whitespace: bool,
    pub remove_empty_lines: bool,
    pub normalize_line_endings: bool,
}

/// Clean explicitly supplied plain text without reading/writing the clipboard.
/// Horizontal whitespace never merges separate lines. With no options selected,
/// all Unicode text, CR/LF/CRLF separators and edge whitespace are preserved.
pub fn clean(input: &str, options: &Options, cancel: &CancelToken) -> CoreResult<String> {
    cancel.check()?;
    if input.len() > MAX_INPUT_BYTES {
        return Err(CoreError::InvalidInput(
            "text cleanup is limited to 1 MiB".into(),
        ));
    }
    if input.contains('\0') {
        return Err(CoreError::InvalidInput(
            "plain text cannot contain NUL characters".into(),
        ));
    }
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while cursor < input.len() {
        cancel.check()?;
        let rest = &input[cursor..];
        let end = rest.find(['\r', '\n']).unwrap_or(rest.len());
        let mut line = &rest[..end];
        let tail = &rest[end..];
        let separator = if tail.starts_with("\r\n") {
            "\r\n"
        } else if tail.starts_with('\r') {
            "\r"
        } else if tail.starts_with('\n') {
            "\n"
        } else {
            ""
        };
        cursor += end + separator.len();
        if options.trim_lines {
            line = line.trim();
        }
        if options.remove_empty_lines && line.trim().is_empty() {
            continue;
        }
        if options.collapse_whitespace {
            let mut whitespace = false;
            for (index, value) in line.char_indices() {
                if index % 4096 == 0 {
                    cancel.check()?;
                }
                if value.is_whitespace() {
                    if !whitespace {
                        output.push(' ');
                    }
                    whitespace = true;
                } else {
                    output.push(value);
                    whitespace = false;
                }
            }
        } else {
            output.push_str(line);
        }
        output.push_str(if options.normalize_line_endings && !separator.is_empty() {
            "\n"
        } else {
            separator
        });
    }
    cancel.check()?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::CancelToken;

    #[test]
    fn no_options_preserve_unicode_line_endings_and_horizontal_whitespace() {
        let input = "  café\t\t東京  \r\n\rnext\n";
        assert_eq!(
            clean(input, &Options::default(), &CancelToken::default()).unwrap(),
            input
        );
    }

    #[test]
    fn cleanup_options_combine_without_merging_distinct_lines() {
        let options = Options {
            trim_lines: true,
            collapse_whitespace: true,
            remove_empty_lines: true,
            normalize_line_endings: true,
        };
        let input = "  café\t\t東京  \r\n\r\n \t\rNext\u{2003}  line\n";
        let output = clean(input, &options, &CancelToken::default()).unwrap();
        assert_eq!(output, "café 東京\nNext line\n");
        assert_eq!(
            clean(&output, &options, &CancelToken::default()).unwrap(),
            output
        );
    }

    #[test]
    fn collapse_whitespace_keeps_one_edge_space_unless_trim_is_selected() {
        let options = Options {
            collapse_whitespace: true,
            ..Default::default()
        };
        assert_eq!(
            clean(" \ta\t  b  \t\r\n c ", &options, &CancelToken::default()).unwrap(),
            " a b \r\n c "
        );
        let trim = Options {
            trim_lines: true,
            ..Default::default()
        };
        assert_eq!(
            clean(" \ta\t  b  \t\r\n c ", &trim, &CancelToken::default()).unwrap(),
            "a\t  b\r\nc"
        );
    }

    #[test]
    fn empty_cleanup_is_valid_and_bounds_errors_never_include_input() {
        assert_eq!(
            clean("", &Options::default(), &CancelToken::default()).unwrap(),
            ""
        );
        let input = "GENERATED_PRIVATE_SENTINEL".repeat(50_000);
        let error = clean(&input, &Options::default(), &CancelToken::default()).unwrap_err();
        assert_eq!(error.code(), "invalid_input");
        assert!(!error.to_string().contains("GENERATED_PRIVATE_SENTINEL"));
        assert!(
            clean(
                "generated\0text",
                &Options::default(),
                &CancelToken::default()
            )
            .is_err()
        );
    }

    #[test]
    fn cancelled_cleanup_returns_no_result() {
        let cancel = CancelToken::default();
        cancel.cancel();
        assert_eq!(
            clean("Generated text", &Options::default(), &cancel)
                .unwrap_err()
                .code(),
            "cancelled"
        );
    }
}
