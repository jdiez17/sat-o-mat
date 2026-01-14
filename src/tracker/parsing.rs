use crate::tracker::TrackerError;

pub fn parse_tle_lines(tle: &str) -> Result<(Option<String>, String, String), TrackerError> {
    let lines: Vec<String> = tle
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    match lines.len() {
        2 => Ok((None, lines[0].clone(), lines[1].clone())),
        3 => Ok((Some(lines[0].clone()), lines[1].clone(), lines[2].clone())),
        _ => Err(TrackerError::InvalidTleFormat),
    }
}

#[allow(dead_code)]
pub fn parse_frequency_hz(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    let mut parts = trimmed.split_whitespace();
    let value_str = parts.next()?;
    let unit = parts.next().unwrap_or("hz").to_lowercase();
    let value: f64 = value_str.parse().ok()?;
    let multiplier = match unit.as_str() {
        "hz" => 1.0,
        "khz" => 1e3,
        "mhz" => 1e6,
        "ghz" => 1e9,
        _ => 1.0,
    };
    Some(value * multiplier)
}
