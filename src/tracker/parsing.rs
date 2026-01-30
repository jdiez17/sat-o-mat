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
