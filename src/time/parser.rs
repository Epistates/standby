use crate::errors::{Result, StandbyError};
use crate::time::Duration;
use std::time::Duration as StdDuration;

const NANOS_PER_SEC: u128 = 1_000_000_000;

/// Parses a duration such as `90`, `2.5s`, `1h30m` or `infinity`.
///
/// See the [module documentation](crate::time) for the format.
///
/// # Errors
///
/// Returns [`StandbyError::InvalidDuration`] when the text is empty, negative,
/// has an unknown unit, or mixes several numbers without giving each a unit.
pub fn parse_duration(input: &str) -> Result<Duration> {
    let invalid = |reason: String| StandbyError::InvalidDuration {
        input: input.to_string(),
        reason,
    };

    let text = input.trim();
    if text.starts_with('-') {
        return Err(invalid("a duration cannot be negative".to_string()));
    }
    let text = text.strip_prefix('+').unwrap_or(text);
    if text.eq_ignore_ascii_case("inf") || text.eq_ignore_ascii_case("infinity") {
        return Ok(Duration::Infinite);
    }
    if text.is_empty() {
        return Err(invalid("expected a number".to_string()));
    }

    // `None` once the total no longer fits, which reads as infinite.
    let mut total_nanos = Some(0u128);
    let mut first_segment = true;
    let mut rest = text;
    while !rest.is_empty() {
        let (number, after_number) = Decimal::parse_prefix(rest)
            .ok_or_else(|| invalid(format!("expected a number at '{rest}'")))?;
        let number_text = &rest[..rest.len() - after_number.len()];

        let unit_len = after_number
            .find(|c: char| c.is_ascii_digit() || c == '.' || c.is_whitespace())
            .unwrap_or(after_number.len());
        let (unit, after_unit) = after_number.split_at(unit_len);
        let after_unit = after_unit.trim_start();

        let unit_nanos = match unit {
            "" if first_segment && after_unit.is_empty() => NANOS_PER_SEC,
            "" => return Err(invalid(format!("missing unit after '{number_text}'"))),
            unit => unit_nanos(unit).ok_or_else(|| invalid(format!("unknown unit '{unit}'")))?,
        };

        total_nanos = total_nanos.and_then(|total| total.checked_add(number.times(unit_nanos)?));
        first_segment = false;
        rest = after_unit;
    }

    Ok(total_nanos.map_or(Duration::Infinite, from_nanos))
}

fn unit_nanos(unit: &str) -> Option<u128> {
    Some(match unit {
        "ns" => 1,
        // Both the micro sign (U+00B5) and the Greek letter mu (U+03BC).
        "us" | "\u{b5}s" | "\u{3bc}s" => 1_000,
        "ms" => 1_000_000,
        "s" => NANOS_PER_SEC,
        "m" => 60 * NANOS_PER_SEC,
        "h" => 3_600 * NANOS_PER_SEC,
        "d" => 86_400 * NANOS_PER_SEC,
        _ => return None,
    })
}

fn from_nanos(nanos: u128) -> Duration {
    match u64::try_from(nanos / NANOS_PER_SEC) {
        // The remainder is below one billion, so it fits in a u32.
        Ok(secs) => Duration::Finite(StdDuration::new(secs, (nanos % NANOS_PER_SEC) as u32)),
        Err(_) => Duration::Infinite,
    }
}

/// A non-negative decimal number, `mantissa × 10^exponent`.
///
/// Durations are computed from this in integer arithmetic, so `0.1` is
/// exactly 100 ms and never picks up binary floating-point error. The
/// mantissa keeps 19 significant digits; a longer number rounds up.
struct Decimal {
    mantissa: u64,
    exponent: i32,
    /// Non-zero digits beyond what `mantissa` holds were dropped, so the true
    /// value is slightly larger.
    truncated: bool,
}

impl Decimal {
    /// Parses a number from the start of `text` and returns it with the rest.
    ///
    /// Accepts `12`, `1.5`, `.5`, `5.` and an exponent such as `1e3` or
    /// `2.5E-3`. Returns `None` when `text` does not start with a digit or
    /// a fraction.
    fn parse_prefix(text: &str) -> Option<(Self, &str)> {
        let bytes = text.as_bytes();
        let mut number = Self {
            mantissa: 0,
            exponent: 0,
            truncated: false,
        };
        let mut full = false;
        let mut pos = 0;

        while let Some(digit) = digit_at(bytes, pos) {
            if full || !number.push(digit) {
                // No room for this digit: keep the magnitude, lose the digit.
                full = true;
                number.exponent = number.exponent.saturating_add(1);
                number.truncated |= digit != 0;
            }
            pos += 1;
        }
        let integer_digits = pos;

        let mut fraction_digits = 0;
        if bytes.get(pos) == Some(&b'.') {
            pos += 1;
            while let Some(digit) = digit_at(bytes, pos) {
                if !full && number.push(digit) {
                    number.exponent = number.exponent.saturating_sub(1);
                } else {
                    full = true;
                    number.truncated |= digit != 0;
                }
                pos += 1;
                fraction_digits += 1;
            }
        }

        if integer_digits + fraction_digits == 0 {
            return None;
        }

        // An exponent counts only when digits follow it; otherwise the `e`
        // belongs to whatever comes next and is reported as an unknown unit.
        if matches!(bytes.get(pos), Some(b'e' | b'E')) {
            let mut end = pos + 1;
            let negative = match bytes.get(end) {
                Some(b'-') => {
                    end += 1;
                    true
                }
                Some(b'+') => {
                    end += 1;
                    false
                }
                _ => false,
            };
            let digits_start = end;
            let mut exponent = 0i32;
            while let Some(digit) = digit_at(bytes, end) {
                exponent = exponent.saturating_mul(10).saturating_add(i32::from(digit));
                end += 1;
            }
            if end > digits_start {
                let exponent = if negative { -exponent } else { exponent };
                number.exponent = number.exponent.saturating_add(exponent);
                pos = end;
            }
        }

        Some((number, &text[pos..]))
    }

    /// Appends a digit to the mantissa. Returns `false` when it does not fit.
    fn push(&mut self, digit: u8) -> bool {
        match self
            .mantissa
            .checked_mul(10)
            .and_then(|mantissa| mantissa.checked_add(u64::from(digit)))
        {
            Some(mantissa) => {
                self.mantissa = mantissa;
                true
            }
            None => false,
        }
    }

    /// Multiplies by a whole number of nanoseconds, rounding up to the next
    /// nanosecond. Returns `None` when the result does not fit in a `u128`.
    fn times(&self, unit_nanos: u128) -> Option<u128> {
        // Dropped digits make the true value larger, so take the upper bound.
        let mantissa = u128::from(self.mantissa) + u128::from(self.truncated);
        // At most 2^64 times one day of nanoseconds: far below u128::MAX.
        let scaled = mantissa * unit_nanos;
        if scaled == 0 {
            return Some(0);
        }
        let power = 10u128.checked_pow(self.exponent.unsigned_abs());
        if self.exponent >= 0 {
            power.and_then(|factor| scaled.checked_mul(factor))
        } else {
            // With no divisor that large, the value is below a nanosecond
            // and still not zero.
            Some(power.map_or(1, |divisor| scaled.div_ceil(divisor)))
        }
    }
}

fn digit_at(bytes: &[u8], pos: usize) -> Option<u8> {
    bytes
        .get(pos)
        .filter(|byte| byte.is_ascii_digit())
        .map(|byte| byte - b'0')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nanos(input: &str) -> u128 {
        match parse_duration(input) {
            Ok(Duration::Finite(duration)) => duration.as_nanos(),
            other => panic!("expected a finite duration for {input:?}, got {other:?}"),
        }
    }

    fn reason(input: &str) -> String {
        match parse_duration(input) {
            Err(StandbyError::InvalidDuration { reason, .. }) => reason,
            other => panic!("expected an error for {input:?}, got {other:?}"),
        }
    }

    const SEC: u128 = NANOS_PER_SEC;

    #[test]
    fn bare_numbers_are_seconds() {
        assert_eq!(nanos("5"), 5 * SEC);
        assert_eq!(nanos("0"), 0);
        assert_eq!(nanos("5.5"), 5 * SEC + SEC / 2);
        assert_eq!(nanos(".5"), SEC / 2);
        assert_eq!(nanos("5."), 5 * SEC);
        assert_eq!(nanos("+5"), 5 * SEC);
        assert_eq!(nanos("  5  "), 5 * SEC);
    }

    #[test]
    fn every_unit_is_recognised() {
        assert_eq!(nanos("7ns"), 7);
        assert_eq!(nanos("7us"), 7_000);
        assert_eq!(nanos("7µs"), 7_000);
        assert_eq!(nanos("7μs"), 7_000);
        assert_eq!(nanos("7ms"), 7_000_000);
        assert_eq!(nanos("7s"), 7 * SEC);
        assert_eq!(nanos("7m"), 420 * SEC);
        assert_eq!(nanos("7h"), 25_200 * SEC);
        assert_eq!(nanos("7d"), 604_800 * SEC);
    }

    /// 0.2.0 read `5ms` as five minutes.
    #[test]
    fn milliseconds_are_not_minutes() {
        assert_eq!(nanos("5ms"), 5_000_000);
        assert_eq!(nanos("1500ms"), SEC + SEC / 2);
    }

    #[test]
    fn fractions_apply_to_the_unit() {
        assert_eq!(nanos("1.5m"), 90 * SEC);
        assert_eq!(nanos("0.5h"), 1_800 * SEC);
        assert_eq!(nanos("0.5ms"), 500_000);
    }

    #[test]
    fn segments_add_up_in_any_order() {
        assert_eq!(nanos("1m30s"), 90 * SEC);
        assert_eq!(nanos("1h30m45s"), 5_445 * SEC);
        assert_eq!(nanos("30s1m"), 90 * SEC);
        assert_eq!(nanos("1m 30s"), 90 * SEC);
        assert_eq!(nanos("1s500ms"), SEC + SEC / 2);
        assert_eq!(nanos("1m1m"), 120 * SEC);
        assert_eq!(nanos("1d2h3m4s5ms6us7ns"), 93_784 * SEC + 5_006_007);
    }

    #[test]
    fn decimal_fractions_are_exact() {
        assert_eq!(nanos("0.1"), 100_000_000);
        assert_eq!(nanos("0.3"), 300_000_000);
        assert_eq!(nanos("0.000000001"), 1);
        assert_eq!(nanos("1.000000001"), SEC + 1);
        assert_eq!(nanos("4.35"), 4_350_000_000);
    }

    #[test]
    fn anything_below_a_nanosecond_rounds_up() {
        assert_eq!(nanos("0.0000000001"), 1);
        assert_eq!(nanos("0.1234567891"), 123_456_790);
        assert_eq!(nanos("0.5ns"), 1);
        assert_eq!(nanos("1e-30"), 1);
        assert_eq!(nanos("1e-99999999999"), 1);
        // More digits than the mantissa holds still round up, not down.
        assert_eq!(nanos("0.10000000000000000000000001"), 100_000_001);
        assert_eq!(nanos("0.00000000000000000000000000000000000000000001"), 1);
    }

    #[test]
    fn zero_stays_zero_however_it_is_written() {
        assert_eq!(nanos("0.0"), 0);
        assert_eq!(nanos("0s"), 0);
        assert_eq!(nanos("0e99"), 0);
        assert_eq!(nanos("0e-99"), 0);
        assert_eq!(nanos("0.00000000000000000000000000000000000000000000"), 0);
        assert_eq!(nanos("0m0s"), 0);
    }

    #[test]
    fn exponents_scale_the_number() {
        assert_eq!(nanos("1e3"), 1_000 * SEC);
        assert_eq!(nanos("1E3"), 1_000 * SEC);
        assert_eq!(nanos("1.5e1"), 15 * SEC);
        assert_eq!(nanos("2.5e-3"), 2_500_000);
        assert_eq!(nanos("1e+2ms"), 100_000_000);
    }

    #[test]
    fn infinity_is_spelled_several_ways() {
        for input in [
            "infinity",
            "INFINITY",
            "Infinity",
            "inf",
            "INF",
            "+inf",
            " infinity ",
        ] {
            assert_eq!(parse_duration(input), Ok(Duration::Infinite), "{input}");
        }
    }

    #[test]
    fn values_too_large_to_represent_are_infinite() {
        assert_eq!(parse_duration("1e30"), Ok(Duration::Infinite));
        assert_eq!(parse_duration("1e99999999999"), Ok(Duration::Infinite));
        assert_eq!(
            parse_duration("99999999999999999999999999999999d"),
            Ok(Duration::Infinite)
        );
        assert_eq!(
            parse_duration("18446744073709551616"),
            Ok(Duration::Infinite)
        );
        assert_eq!(parse_duration("1e30d1s"), Ok(Duration::Infinite));
    }

    #[test]
    fn the_largest_finite_value_is_kept() {
        assert_eq!(
            parse_duration("18446744073709551615"),
            Ok(Duration::from_secs(u64::MAX))
        );
    }

    #[test]
    fn long_integers_keep_their_magnitude() {
        // 25 digits: more than a u64 mantissa holds, well within range once
        // counted in nanoseconds.
        assert_eq!(
            nanos("1000000000000000000000000ns"),
            1_000_000_000_000_000 * SEC
        );
    }

    #[test]
    fn rejects_text_that_is_not_a_duration() {
        assert_eq!(reason(""), "expected a number");
        assert_eq!(reason("   "), "expected a number");
        assert_eq!(reason("+"), "expected a number");
        assert_eq!(reason("invalid"), "expected a number at 'invalid'");
        assert_eq!(reason("."), "expected a number at '.'");
        assert_eq!(reason("s"), "expected a number at 's'");
        assert_eq!(reason("nan"), "expected a number at 'nan'");
        assert_eq!(reason("1m+30s"), "unknown unit 'm+'");
    }

    #[test]
    fn rejects_negative_durations() {
        assert_eq!(reason("-5"), "a duration cannot be negative");
        assert_eq!(reason("-inf"), "a duration cannot be negative");
    }

    #[test]
    fn rejects_unknown_units() {
        assert_eq!(reason("5x"), "unknown unit 'x'");
        assert_eq!(reason("5S"), "unknown unit 'S'");
        assert_eq!(reason("5sec"), "unknown unit 'sec'");
        assert_eq!(reason("1e"), "unknown unit 'e'");
        assert_eq!(reason("1h5w"), "unknown unit 'w'");
    }

    #[test]
    fn rejects_a_number_without_a_unit_among_several() {
        assert_eq!(reason("1m30"), "missing unit after '30'");
        assert_eq!(reason("1 30s"), "missing unit after '1'");
        assert_eq!(reason("5 s"), "missing unit after '5'");
    }

    #[test]
    fn errors_quote_the_original_input() {
        let err = parse_duration(" 5x ").unwrap_err();
        assert_eq!(err.to_string(), "invalid duration ' 5x ': unknown unit 'x'");
    }
}
