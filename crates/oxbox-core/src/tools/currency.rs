//! Dated ECB reference rates. Import and conversion work offline; downloads require explicit opt-in.
use crate::{CoreError, CoreResult};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
pub const MAX_DOCUMENT: usize = 1024 * 1024;
pub const PROVIDER_URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateBook {
    pub date: String,
    pub fetched_at: u64,
    pub rates: BTreeMap<String, f64>,
}
fn invalid(text: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(text.into())
}
fn attributes(mut text: &str) -> CoreResult<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    loop {
        text = text.trim().trim_end_matches('/').trim();
        if text.is_empty() {
            break;
        }
        let end = text
            .find('=')
            .ok_or_else(|| invalid("malformed rate attribute"))?;
        let name = text[..end].trim();
        if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphabetic()) {
            return Err(invalid("invalid rate attribute"));
        }
        text = text[end + 1..].trim_start();
        let quote = text
            .chars()
            .next()
            .ok_or_else(|| invalid("missing rate attribute"))?;
        if quote != '\'' && quote != '"' {
            return Err(invalid("rate attributes must be quoted"));
        }
        text = &text[1..];
        let end = text
            .find(quote)
            .ok_or_else(|| invalid("unterminated rate attribute"))?;
        if result.insert(name.into(), text[..end].into()).is_some() {
            return Err(invalid("duplicate rate attribute"));
        }
        text = &text[end + 1..];
    }
    Ok(result)
}
pub fn parse_ecb(xml: &str, fetched_at: u64) -> CoreResult<RateBook> {
    if xml.len() > MAX_DOCUMENT || xml.contains("<!") || xml.contains('&') {
        return Err(invalid(
            "rate document is too large or contains unsupported declarations/entities",
        ));
    }
    let mut rates = BTreeMap::from([("EUR".into(), 1.0)]);
    let mut date = None;
    let mut elements: Vec<(String, bool)> = Vec::new();
    let mut root_seen = false;
    let mut parts = xml.split('<');
    if !parts
        .next()
        .unwrap_or_default()
        .trim_start_matches('\u{feff}')
        .trim()
        .is_empty()
    {
        return Err(invalid("invalid XML prefix"));
    }
    for (index, part) in parts.enumerate() {
        if index > 1024 || elements.len() > 16 {
            return Err(invalid("rate document nesting or entry limit exceeded"));
        }
        let end = part
            .find('>')
            .ok_or_else(|| invalid("truncated rate document"))?;
        let tag = part[..end].trim();
        if tag.starts_with("?xml ") && tag.ends_with('?') && !root_seen {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            let previous = elements
                .pop()
                .ok_or_else(|| invalid("unbalanced rate document"))?;
            if previous.0 != name.trim() {
                return Err(invalid("unbalanced rate document"));
            }
        } else {
            let self_closing = tag.ends_with('/');
            let body = tag.trim_end_matches('/').trim();
            let name = body
                .split_ascii_whitespace()
                .next()
                .ok_or_else(|| invalid("missing XML element"))?;
            if !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b":_-".contains(&c))
            {
                return Err(invalid("invalid XML element name"));
            }
            if elements.is_empty() {
                if root_seen {
                    return Err(invalid("expected one XML root"));
                }
                root_seen = true;
            }
            let mut dated = false;
            if name == "Cube" {
                let attrs = attributes(&body[name.len()..])?;
                if let Some(value) = attrs.get("time") {
                    if attrs.len() != 1 || self_closing || elements.iter().any(|(_, dated)| *dated)
                    {
                        return Err(invalid("invalid dated rate context"));
                    }
                    NaiveDate::parse_from_str(value, "%Y-%m-%d")
                        .map_err(|_| invalid("invalid rate date"))?;
                    if date.replace(value.clone()).is_some() {
                        return Err(invalid("expected one daily rate table"));
                    }
                    dated = true;
                } else if let Some(currency) = attrs.get("currency") {
                    if attrs.len() != 2
                        || !self_closing
                        || !elements.iter().any(|(_, dated)| *dated)
                    {
                        return Err(invalid(
                            "currency entries must be inside the dated rate table",
                        ));
                    }
                    if currency.len() != 3 || !currency.bytes().all(|c| c.is_ascii_uppercase()) {
                        return Err(invalid("invalid currency code"));
                    }
                    let rate = attrs
                        .get("rate")
                        .and_then(|v| v.parse::<f64>().ok())
                        .filter(|v| v.is_finite() && *v > 0.0 && *v <= 1e12)
                        .ok_or_else(|| invalid("invalid currency rate"))?;
                    if rates.insert(currency.clone(), rate).is_some() {
                        return Err(invalid("duplicate currency code"));
                    }
                    if rates.len() > 512 {
                        return Err(invalid("too many currency rates"));
                    }
                } else if !attrs.is_empty() {
                    return Err(invalid("unsupported currency attributes"));
                }
            }
            if !self_closing {
                elements.push((name.to_string(), dated));
            }
        }
        if elements.is_empty() && !part[end + 1..].trim().is_empty() {
            return Err(invalid("unexpected text outside the rate document"));
        }
    }
    if !elements.is_empty() || !root_seen {
        return Err(invalid("unbalanced rate document"));
    }
    if rates.len() < 2 {
        return Err(invalid("no currency rates found"));
    }
    Ok(RateBook {
        date: date.ok_or_else(|| invalid("missing rate date"))?,
        fetched_at,
        rates,
    })
}
pub fn convert(amount: f64, from: &str, to: &str, book: &RateBook) -> CoreResult<f64> {
    if !amount.is_finite() {
        return Err(invalid("amount must be finite"));
    }
    let rate = |code| {
        book.rates
            .get(code)
            .copied()
            .filter(|v| v.is_finite() && *v > 0.0)
            .ok_or_else(|| invalid("currency is not in the cached rate table"))
    };
    let result = amount / rate(from)? * rate(to)?;
    if !result.is_finite() {
        return Err(invalid("converted amount exceeds numeric range"));
    }
    Ok(result)
}
pub fn needs_refresh(last: u64, now: u64) -> bool {
    now < last || now.saturating_sub(last) >= 86400
}
pub fn import(path: &Path, now: u64) -> CoreResult<RateBook> {
    let mut input = std::fs::File::open(path)?;
    let mut buffer = [0u8; 8192];
    let mut bytes = Vec::new();
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        if bytes.len() + count > MAX_DOCUMENT {
            return Err(invalid("rate file exceeds 1 MiB"));
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    parse_ecb(
        std::str::from_utf8(&bytes).map_err(|_| invalid("rate file must be UTF-8 XML"))?,
        now,
    )
}
/// No shell, credentials, redirects, uploads, or user-selected URLs.
pub fn download(approved: bool) -> CoreResult<RateBook> {
    if !approved {
        return Err(invalid(
            "enable ECB network access explicitly before refreshing rates",
        ));
    }
    let program = if cfg!(windows) {
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
            .join("System32/curl.exe")
    } else {
        PathBuf::from("/usr/bin/curl")
    };
    if !program.is_absolute() {
        return Err(invalid("the system curl path is unavailable"));
    }
    let mut child = Command::new(program)
        .args([
            "--disable",
            "--fail",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--max-time",
            "20",
            "--max-filesize",
            "1048576",
            PROVIDER_URL,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let result = (|| -> CoreResult<Vec<u8>> {
        let mut output = child
            .stdout
            .take()
            .ok_or_else(|| invalid("currency provider output unavailable"))?;
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            let count = output.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            if bytes.len() + count > MAX_DOCUMENT {
                return Err(invalid("provider response exceeds 1 MiB"));
            }
            bytes.extend_from_slice(&buffer[..count]);
        }
        Ok(bytes)
    })();
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait()?;
    let bytes = result?;
    if !status.success() {
        return Err(invalid(
            "ECB refresh failed; the previous cache is unchanged",
        ));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |v| v.as_secs());
    parse_ecb(
        std::str::from_utf8(&bytes).map_err(|_| invalid("invalid provider response encoding"))?,
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    const XML: &str = "<Envelope><Cube><Cube time='2026-11-20'><Cube currency='USD' rate='1.10'/><Cube currency='GBP' rate='0.85'/></Cube></Cube></Envelope>";
    #[test]
    fn imports_daily_rates_and_converts_via_euro() {
        let rates = parse_ecb(XML, 100).unwrap();
        assert_eq!(rates.date, "2026-11-20");
        assert!((convert(110.0, "USD", "GBP", &rates).unwrap() - 85.0).abs() < 1e-9);
        assert_eq!(convert(1.0, "EUR", "USD", &rates).unwrap(), 1.1);
    }
    #[test]
    fn rejects_untrusted_or_invalid_rate_documents() {
        for text in [
            "<!DOCTYPE x><Cube time='2026-11-20'/>",
            "<Cube time='bad'><Cube currency='USD' rate='1'/></Cube>",
            "<Cube time='2026-11-20'><Cube currency='USD' rate='NaN'/></Cube>",
            "<Cube time='2026-11-20'><Cube currency='USD' rate='0'/></Cube>",
        ] {
            assert!(parse_ecb(text, 0).is_err());
        }
        let rates = parse_ecb(XML, 100).unwrap();
        assert!(convert(f64::INFINITY, "EUR", "USD", &rates).is_err());
        assert!(convert(1.0, "BAD", "USD", &rates).is_err());
        assert!(parse_ecb(&"x".repeat(MAX_DOCUMENT + 1), 0).is_err());
    }
    #[test]
    fn rejects_unbalanced_or_undated_rate_contexts() {
        for document in [
            "<Cube time='2026-11-20'><Cube currency='USD' rate='1.1'/>",
            "<Cube currency='USD' rate='1.1'/><Cube time='2026-11-20'></Cube>",
            "<Cube time='2026-11-20'></Cube><Cube currency='USD' rate='1.1'/>",
            "</Cube><Cube time='2026-11-20'><Cube currency='USD' rate='1.1'/></Cube>",
        ] {
            assert!(parse_ecb(document, 0).is_err(), "{document}");
        }
    }
    #[test]
    fn opt_in_and_freshness_are_explicit() {
        assert!(!needs_refresh(100, 101));
        assert!(needs_refresh(100, 100 + 86400));
        assert!(download(false).is_err());
    }
}
