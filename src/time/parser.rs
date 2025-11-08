use crate::errors::{StandbyError, Result};
use crate::time::Duration;

/// Parses a time duration string in various formats
///
/// Supported formats:
/// - Integer seconds: "5", "10"
/// - Floating point: "5.5", "1.234"
/// - With unit suffix: "5s", "1m", "1h", "1d", "5.5m"
/// - Special value: "infinity"
///
/// Examples:
/// - "5" -> 5 seconds
/// - "5.5" -> 5.5 seconds
/// - "5s" -> 5 seconds
/// - "1m30s" -> 90 seconds
/// - "1h" -> 3600 seconds
/// - "1d" -> 86400 seconds
/// - "infinity" -> maximum duration
pub fn parse_duration(input: &str) -> Result<Duration> {
    let input = input.trim();

    // Handle special case: infinity
    if input.eq_ignore_ascii_case("infinity") {
        return Ok(Duration {
            secs: u64::MAX,
            nanos: 0,
        });
    }

    // Try to parse as simple number (seconds)
    if let Ok(secs) = input.parse::<f64>() {
        return Duration::from_secs_f64(secs);
    }

    // Try to parse with unit suffix
    parse_duration_with_units(input)
}

fn parse_duration_with_units(input: &str) -> Result<Duration> {
    let input = input.trim();

    // Check for compound format like "1h30m45s"
    if contains_multiple_units(input) {
        return parse_compound_duration(input);
    }

    // Single unit format
    let (number_str, unit) = split_number_and_unit(input)?;
    let value = number_str
        .parse::<f64>()
        .map_err(|_| StandbyError::InvalidTimeFormat(format!("Invalid number: {}", number_str)))?;

    let seconds = match unit {
        "s" | "" => value,
        "ms" => value / 1000.0,
        "m" => value * 60.0,
        "h" => value * 3600.0,
        "d" => value * 86400.0,
        _ => {
            return Err(StandbyError::InvalidTimeFormat(format!(
                "Unknown time unit: {}",
                unit
            )))
        }
    };

    Duration::from_secs_f64(seconds)
}

fn contains_multiple_units(input: &str) -> bool {
    let units = ["s", "ms", "m", "h", "d"];
    let mut found_units = 0;

    for unit in &units {
        if input.contains(unit) {
            found_units += 1;
            if found_units > 1 {
                return true;
            }
        }
    }

    false
}

fn parse_compound_duration(input: &str) -> Result<Duration> {
    let mut total_seconds = 0.0;
    let mut remaining = input;

    // Parse in order of largest units first: d, h, m, s
    let units_in_order = [("d", 86400.0), ("h", 3600.0), ("m", 60.0), ("s", 1.0)];

    for (unit_str, multiplier) in &units_in_order {
        if let Some(pos) = remaining.find(unit_str) {
            let number_part = &remaining[..pos];

            // Find the start of this number (after previous unit)
            let start = if total_seconds == 0.0 {
                0
            } else {
                // Skip whitespace and find where the number starts
                number_part.rfind(|c: char| c.is_whitespace() || !c.is_numeric() && c != '.')
                    .map(|p| p + 1)
                    .unwrap_or(0)
            };

            let number_str = number_part[start..].trim();

            if !number_str.is_empty() {
                let value = number_str
                    .parse::<f64>()
                    .map_err(|_| StandbyError::InvalidTimeFormat(
                        format!("Invalid number in compound duration: {}", number_str),
                    ))?;

                total_seconds += value * multiplier;
                remaining = &remaining[pos + unit_str.len()..];
            }
        }
    }

    if total_seconds == 0.0 {
        return Err(StandbyError::InvalidTimeFormat(
            "No valid time units found".to_string(),
        ));
    }

    Duration::from_secs_f64(total_seconds)
}

fn split_number_and_unit(input: &str) -> Result<(&str, &str)> {
    let input = input.trim();

    // Find where the unit starts (first non-digit, non-dot character)
    let mut unit_start = input.len();

    for (i, c) in input.chars().enumerate() {
        if !c.is_numeric() && c != '.' {
            unit_start = i;
            break;
        }
    }

    let number_part = &input[..unit_start].trim();
    let unit_part = input[unit_start..].trim();

    if number_part.is_empty() {
        return Err(StandbyError::InvalidTimeFormat(format!(
            "Missing number in: {}",
            input
        )));
    }

    Ok((number_part, unit_part))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integer_seconds() {
        let d = parse_duration("5").unwrap();
        assert_eq!(d.secs, 5);
        assert_eq!(d.nanos, 0);
    }

    #[test]
    fn test_parse_float_seconds() {
        let d = parse_duration("5.5").unwrap();
        assert_eq!(d.secs, 5);
        assert_eq!(d.nanos, 500_000_000);
    }

    #[test]
    fn test_parse_seconds_with_suffix() {
        let d = parse_duration("5s").unwrap();
        assert_eq!(d.secs, 5);
    }

    #[test]
    fn test_parse_minutes() {
        let d = parse_duration("1m").unwrap();
        assert_eq!(d.secs, 60);
    }

    #[test]
    fn test_parse_minutes_float() {
        let d = parse_duration("1.5m").unwrap();
        assert_eq!(d.secs, 90);
    }

    #[test]
    fn test_parse_hours() {
        let d = parse_duration("1h").unwrap();
        assert_eq!(d.secs, 3600);
    }

    #[test]
    fn test_parse_days() {
        let d = parse_duration("1d").unwrap();
        assert_eq!(d.secs, 86400);
    }

    #[test]
    fn test_parse_infinity() {
        let d = parse_duration("infinity").unwrap();
        assert!(d.is_infinite());
    }

    #[test]
    fn test_parse_infinity_case_insensitive() {
        let d = parse_duration("INFINITY").unwrap();
        assert!(d.is_infinite());
    }

    #[test]
    fn test_parse_compound_duration() {
        let d = parse_duration("1m30s").unwrap();
        assert_eq!(d.secs, 90);
    }

    #[test]
    fn test_parse_complex_compound() {
        let d = parse_duration("1h30m45s").unwrap();
        assert_eq!(d.secs, 5445);
    }

    #[test]
    fn test_invalid_duration() {
        assert!(parse_duration("invalid").is_err());
    }

    #[test]
    fn test_invalid_unit() {
        assert!(parse_duration("5x").is_err());
    }
}
