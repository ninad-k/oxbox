//! Dated ECB reference rates. Import and conversion work offline; downloads require explicit opt-in.
use crate::jobs::CancelToken;
use crate::{CoreError, CoreResult};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
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
    download_with_cancel(approved, &CancelToken::default())
}
pub fn download_with_cancel(approved: bool, cancel: &CancelToken) -> CoreResult<RateBook> {
    download_using(approved, cancel, provider_bytes)
}
fn download_using(
    approved: bool,
    cancel: &CancelToken,
    fetch: impl FnOnce(&Path, &[String], &CancelToken, Duration, usize) -> CoreResult<Vec<u8>>,
) -> CoreResult<RateBook> {
    if !approved {
        return Err(invalid(
            "enable ECB network access explicitly before refreshing rates",
        ));
    }
    cancel.check()?;
    let program = if cfg!(windows) {
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
            .join("System32/curl.exe")
    } else {
        PathBuf::from("/usr/bin/curl")
    };
    if !program.is_absolute() {
        return Err(invalid("the system curl path is unavailable"));
    }
    let args = [
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
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    let bytes = fetch(
        &program,
        &args,
        cancel,
        Duration::from_secs(20),
        MAX_DOCUMENT,
    )?;
    cancel.check()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |v| v.as_secs());
    parse_ecb(
        std::str::from_utf8(&bytes).map_err(|_| invalid("invalid provider response encoding"))?,
        now,
    )
}
fn provider_bytes(
    program: &Path,
    args: &[String],
    cancel: &CancelToken,
    timeout: Duration,
    max_bytes: usize,
) -> CoreResult<Vec<u8>> {
    super::media_provider::run_bytes(program, args, cancel, timeout, max_bytes)
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
    #[test]
    fn refresh_cancel_before_start_skips_the_transport() {
        let cancel = CancelToken::default();
        cancel.cancel();
        let mut invoked = false;
        let result = download_using(true, &cancel, |_, _, _, _, _| {
            invoked = true;
            Ok(XML.as_bytes().to_vec())
        });
        assert!(matches!(result, Err(CoreError::Cancelled)));
        assert!(!invoked, "cancelled refresh still started its provider");
    }
    #[test]
    fn refresh_active_cancel_interrupts_generated_provider_without_network() {
        let cancel = CancelToken::default();
        let signal = cancel.clone();
        let stopper = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            signal.cancel();
        });
        let began = std::time::Instant::now();
        let result = download_using(true, &cancel, |_, _, token, timeout, max_bytes| {
            provider_bytes(
                &std::env::current_exe().unwrap(),
                &[
                    "--exact".into(),
                    "tools::media_provider::tests::provider_fixture_descendant".into(),
                    "--nocapture".into(),
                ],
                token,
                timeout,
                max_bytes,
            )
        });
        stopper.join().unwrap();
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "Cancel waited {:?} for the generated provider",
            began.elapsed()
        );
        assert!(
            matches!(result, Err(CoreError::Cancelled)),
            "unexpected result: {result:?}"
        );
    }
    #[test]
    fn refresh_preserves_fixed_request_and_strict_utf8_policy() {
        let cancel = CancelToken::default();
        let book = download_using(true, &cancel, |_, args, _, timeout, limit| {
            assert_eq!(args.first().map(String::as_str), Some("--disable"));
            assert_eq!(args.last().map(String::as_str), Some(PROVIDER_URL));
            assert!(args.windows(2).any(|pair| pair == ["--proto", "=https"]));
            assert!(!args.iter().any(|arg| arg == "--location"));
            assert_eq!(timeout, Duration::from_secs(20));
            assert_eq!(limit, MAX_DOCUMENT);
            Ok(XML.as_bytes().to_vec())
        })
        .unwrap();
        assert_eq!(book.rates["USD"], 1.1);
        assert!(download_using(true, &cancel, |_, _, _, _, _| Ok(vec![0xff])).is_err());
    }
}
