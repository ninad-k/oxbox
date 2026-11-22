//! Explicit local launcher actions; no shell, I/O, clipboard or background effects.
use crate::{CoreError, CoreResult, tools::unit_convert};

#[derive(Debug, Clone, PartialEq)]
pub enum QuickAction {
    Calculate {
        expression: String,
        result: f64,
    },
    Convert {
        value: f64,
        from: String,
        to: String,
        result: f64,
    },
    Timer {
        seconds: u64,
    },
}
fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.into())
}
pub fn parse(input: &str) -> CoreResult<Option<QuickAction>> {
    if input.len() > 1024 {
        return Err(invalid("Quick actions accept at most 1024 bytes."));
    }
    let input = input.trim();
    if let Some(expression) = input.strip_prefix('=') {
        if expression.len() > 256 {
            return Err(invalid(
                "Use an arithmetic expression of at most 256 bytes.",
            ));
        }
        let mut parser = Arithmetic {
            bytes: expression.as_bytes(),
            at: 0,
            steps: 0,
        };
        let result = parser.expression(0)?;
        parser.whitespace();
        if parser.at != parser.bytes.len() || !result.is_finite() {
            return Err(invalid("Use finite numbers and +, -, *, / or parentheses."));
        }
        return Ok(Some(QuickAction::Calculate {
            expression: expression.trim().into(),
            result,
        }));
    }
    let tokens = input.split_whitespace().collect::<Vec<_>>();
    if tokens
        .first()
        .is_some_and(|t| t.eq_ignore_ascii_case("timer"))
        && tokens.len() > 1
    {
        let first = tokens[1].as_bytes().first().copied();
        if first.is_some_and(|c| c.is_ascii_digit() || c == b'-' || c == b'.')
            || tokens[1].to_lowercase().starts_with("nan")
            || tokens[1].to_lowercase().starts_with("inf")
        {
            return timer(&tokens[1..].join(""))
                .map(|seconds| Some(QuickAction::Timer { seconds }));
        }
    }
    let tokens = if tokens
        .first()
        .is_some_and(|t| t.eq_ignore_ascii_case("convert"))
    {
        &tokens[1..]
    } else {
        &tokens[..]
    };
    if tokens.len() == 4 && tokens[2].eq_ignore_ascii_case("to") {
        let value = tokens[0]
            .parse::<f64>()
            .map_err(|_| invalid("Enter a finite value before the units."))?;
        let from = unit_id(tokens[1]).ok_or_else(|| {
            invalid("Unknown source unit. Use a unit ID such as in, cm, kg or lb.")
        })?;
        let to = unit_id(tokens[3]).ok_or_else(|| invalid("Unknown destination unit."))?;
        let result = unit_convert::convert(value, from, to)?;
        return Ok(Some(QuickAction::Convert {
            value,
            from: from.into(),
            to: to.into(),
            result,
        }));
    }
    Ok(None)
}
fn unit_id(value: &str) -> Option<&'static str> {
    let value = value.to_ascii_lowercase();
    let alias = match value.as_str() {
        "inch" | "inches" => "in",
        "feet" | "foot" => "ft",
        "pound" | "pounds" => "lb",
        "ounce" | "ounces" => "oz",
        "seconds" | "second" => "s",
        "minutes" | "minute" => "min",
        "hours" | "hour" => "h",
        other => other,
    };
    unit_convert::units()
        .iter()
        .find(|u| {
            u.id.eq_ignore_ascii_case(alias)
                || u.label.eq_ignore_ascii_case(alias)
                || format!("{}s", u.label).eq_ignore_ascii_case(alias)
        })
        .map(|u| u.id)
}
fn timer(value: &str) -> CoreResult<u64> {
    let mut rest = value;
    let mut total = 0.0;
    let mut segments = 0;
    while !rest.is_empty() {
        segments += 1;
        if segments > 3 {
            return Err(invalid("Use a duration such as 90s, 10m or 1h30m."));
        }
        let end = rest
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(rest.len());
        if end == 0 {
            return Err(invalid("Use positive numeric timer durations."));
        }
        let number = rest[..end]
            .parse::<f64>()
            .map_err(|_| invalid("Invalid timer duration."))?;
        rest = &rest[end..];
        let (factor, used) = match rest.as_bytes().first().copied() {
            Some(b'h' | b'H') => (3600.0, 1),
            Some(b'm' | b'M') => (60.0, 1),
            Some(b's' | b'S') => (1.0, 1),
            None if segments == 1 => (60.0, 0),
            _ => (0.0, 0),
        };
        if factor == 0.0 {
            return Err(invalid("Use s, m or h for timer units."));
        }
        total += number * factor;
        rest = &rest[used..];
    }
    if !total.is_finite()
        || !(1.0..=604800.0).contains(&total)
        || (total - total.round()).abs() > 1e-8
    {
        return Err(invalid(
            "Choose a whole-second duration from 1 second to 7 days.",
        ));
    }
    Ok(total.round() as u64)
}
struct Arithmetic<'a> {
    bytes: &'a [u8],
    at: usize,
    steps: usize,
}
impl Arithmetic<'_> {
    fn whitespace(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn expression(&mut self, depth: usize) -> CoreResult<f64> {
        let mut value = self.term(depth)?;
        loop {
            self.whitespace();
            let op = self.bytes.get(self.at).copied();
            if !matches!(op, Some(b'+' | b'-')) {
                break;
            }
            self.at += 1;
            let rhs = self.term(depth)?;
            value = if op == Some(b'+') {
                value + rhs
            } else {
                value - rhs
            };
            if !value.is_finite() {
                return Err(invalid("The result must be finite."));
            }
        }
        Ok(value)
    }
    fn term(&mut self, depth: usize) -> CoreResult<f64> {
        let mut value = self.factor(depth)?;
        loop {
            self.whitespace();
            let op = self.bytes.get(self.at).copied();
            if !matches!(op, Some(b'*' | b'/')) {
                break;
            }
            self.at += 1;
            let rhs = self.factor(depth)?;
            if op == Some(b'/') && rhs == 0.0 {
                return Err(invalid("Division by zero is not allowed."));
            }
            value = if op == Some(b'*') {
                value * rhs
            } else {
                value / rhs
            };
            if !value.is_finite() {
                return Err(invalid("The result must be finite."));
            }
        }
        Ok(value)
    }
    fn factor(&mut self, depth: usize) -> CoreResult<f64> {
        self.steps += 1;
        if depth > 16 || self.steps > 64 {
            return Err(invalid("The expression is too complex."));
        }
        self.whitespace();
        if matches!(self.bytes.get(self.at), Some(b'+' | b'-')) {
            let negative = self.bytes[self.at] == b'-';
            self.at += 1;
            let value = self.factor(depth + 1)?;
            return Ok(if negative { -value } else { value });
        }
        if self.bytes.get(self.at) == Some(&b'(') {
            self.at += 1;
            let value = self.expression(depth + 1)?;
            self.whitespace();
            if self.bytes.get(self.at) != Some(&b')') {
                return Err(invalid("Close every parenthesis."));
            }
            self.at += 1;
            return Ok(value);
        }
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
        {
            self.at += 1;
        }
        if matches!(self.bytes.get(self.at), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.bytes.get(self.at), Some(b'+' | b'-')) {
                self.at += 1;
            }
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        let token = std::str::from_utf8(&self.bytes[start..self.at])
            .map_err(|_| invalid("Use numeric arithmetic only."))?;
        let value = token
            .parse::<f64>()
            .map_err(|_| invalid("Use numeric arithmetic only."))?;
        if !value.is_finite() {
            return Err(invalid("Numbers must be finite."));
        }
        Ok(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quick_actions_arithmetic_is_bounded_and_uses_precedence() {
        match parse("= 2 + 3 * (4 - 1)").unwrap().unwrap() {
            QuickAction::Calculate { result, .. } => assert_eq!(result, 11.0),
            _ => panic!("not arithmetic"),
        }
        assert!(parse("=1/0").is_err());
        assert!(parse("=1e308*1e308").is_err());
        assert!(parse("=open('file')").is_err());
        assert!(parse(&format!("={}1{}", "(".repeat(40), ")".repeat(40))).is_err());
        assert!(parse(&"x".repeat(1025)).is_err());
    }
    #[test]
    fn quick_actions_reuse_units_and_parse_only_explicit_timer_requests() {
        match parse("12 inches to cm").unwrap().unwrap() {
            QuickAction::Convert {
                result,
                ref from,
                ref to,
                ..
            } => {
                assert!((result - 30.48).abs() < 1e-9);
                assert_eq!(from, "in");
                assert_eq!(to, "cm");
            }
            _ => panic!("not conversion"),
        }
        assert_eq!(
            parse("timer 1h30m").unwrap(),
            Some(QuickAction::Timer { seconds: 5400 })
        );
        assert_eq!(
            parse("timer 90s").unwrap(),
            Some(QuickAction::Timer { seconds: 90 })
        );
        for invalid in [
            "timer 0s",
            "timer 999999h",
            "timer NaNm",
            "timer 1.1s",
            "12 kg to cm",
        ] {
            assert!(parse(invalid).is_err(), "{invalid}");
        }
        for name in [
            "Timer",
            "Convert Audio / Video",
            "shutdown now",
            "rm -rf",
            "Screenshot",
        ] {
            assert!(parse(name).unwrap().is_none(), "{name}");
        }
    }
}
