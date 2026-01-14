use crate::{executor, radio, tracker};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("{0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("step {0}: {1}")]
    Step(usize, String),
    #[error("{0}")]
    Validation(String),
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    #[allow(dead_code)]
    pub variables: HashMap<String, serde_yaml::Value>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub time: Option<TimeExpr>,
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeExpr {
    Relative(Duration),
    Absolute(DateTime<Utc>),
}

impl TimeExpr {
    #[allow(dead_code)]
    pub fn resolve(&self, start: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            TimeExpr::Relative(d) => start + *d,
            TimeExpr::Absolute(dt) => *dt,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Tracker(tracker::Command),
    Executor(executor::Command),
    Radio(radio::Command),
}

impl Schedule {
    pub fn from_str(yaml: &str) -> Result<Self, ParseError> {
        let root: serde_yaml::Value = serde_yaml::from_str(yaml)?;

        let variables: HashMap<String, serde_yaml::Value> = root
            .get("variables")
            .map(|v| serde_yaml::from_value(v.clone()))
            .transpose()?
            .unwrap_or_default();

        let start = parse_time_variable(&variables, "start")?;
        let end = parse_time_variable(&variables, "end")?;

        if end <= start {
            return Err(ParseError::Validation("'end' must be after 'start'".into()));
        }

        let steps = root
            .get("steps")
            .and_then(|v| v.as_sequence())
            .ok_or_else(|| ParseError::Step(0, "missing 'steps'".into()))?
            .iter()
            .enumerate()
            .map(|(i, v)| parse_step(i, v, &variables))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Schedule {
            start,
            end,
            variables,
            steps,
        })
    }
}

fn parse_step(
    i: usize,
    value: &serde_yaml::Value,
    vars: &HashMap<String, serde_yaml::Value>,
) -> Result<Step, ParseError> {
    let err = |msg: &str| ParseError::Step(i, msg.into());
    let map = value.as_mapping().ok_or_else(|| err("expected mapping"))?;

    let time = map
        .get("time")
        .map(|v| resolve_value(v, vars))
        .and_then(|v| v.as_str().map(String::from))
        .map(parse_time)
        .transpose()
        .map_err(|e| err(&e))?;

    // Find the command key (anything that isn't "time")
    let (module, value) = map
        .iter()
        .find(|(k, _)| k.as_str() != Some("time"))
        .ok_or_else(|| err("no command found"))?;

    let module = module
        .as_str()
        .ok_or_else(|| err("command must be string"))?;
    let value = resolve_value(value, vars);

    let command = match module {
        "tracker" => Command::Tracker(serde_yaml::from_value(value)?),
        "executor" => Command::Executor(serde_yaml::from_value(value)?),
        "radio" => Command::Radio(serde_yaml::from_value(value)?),
        _ => return Err(err(&format!("unknown module: {}", module))),
    };

    Ok(Step { time, command })
}

fn parse_time(s: String) -> Result<TimeExpr, String> {
    let s = s.trim();

    // Relative: T+10s, T-5m
    if s.to_lowercase().starts_with('t') {
        let rest = &s[1..];
        let (neg, rest) = match rest.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, rest.strip_prefix('+').unwrap_or(rest)),
        };
        let dur = parse_duration(rest)?;
        return Ok(TimeExpr::Relative(if neg { -dur } else { dur }));
    }

    // Absolute with offset: 2026-01-12T10:00:00Z - 10s
    if let Some(idx) = s.rfind(['+', '-']) {
        if idx > 10 {
            if let Ok(base) = DateTime::parse_from_rfc3339(s[..idx].trim()) {
                let offset = &s[idx..];
                let (neg, rest) = match offset.strip_prefix('-') {
                    Some(r) => (true, r),
                    None => (false, offset.strip_prefix('+').unwrap_or(offset)),
                };
                let dur = parse_duration(rest)?;
                return Ok(TimeExpr::Absolute(
                    base.with_timezone(&Utc) + if neg { -dur } else { dur },
                ));
            }
        }
    }

    // Plain absolute
    DateTime::parse_from_rfc3339(s)
        .map(|dt| TimeExpr::Absolute(dt.with_timezone(&Utc)))
        .map_err(|e| e.to_string())
}

fn parse_time_variable(
    variables: &HashMap<String, serde_yaml::Value>,
    name: &str,
) -> Result<DateTime<Utc>, ParseError> {
    let str_val = variables
        .get(name)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ParseError::Validation(format!("missing mandatory variable '{}'", name)))?;

    DateTime::parse_from_rfc3339(str_val)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ParseError::Validation(format!("invalid '{}' datetime: {}", name, e)))
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s.trim())
        .map_err(|e| e.to_string())
        .and_then(|d| Duration::from_std(d).map_err(|e| e.to_string()))
}

fn resolve_value(
    value: &serde_yaml::Value,
    vars: &HashMap<String, serde_yaml::Value>,
) -> serde_yaml::Value {
    match value {
        serde_yaml::Value::String(s) => {
            // Direct reference: "$var"
            let t = s.trim();
            if t.starts_with('$') && !t.contains(' ') {
                if let Some(v) = vars.get(&t[1..]) {
                    return v.clone();
                }
            }
            // Inline substitution
            let mut result = s.clone();
            for (name, val) in vars {
                let pattern = format!("${}", name);
                if let Some(rep) = simple_to_string(val) {
                    result = result.replace(&pattern, &rep);
                }
            }
            serde_yaml::Value::String(result)
        }
        serde_yaml::Value::Mapping(m) => serde_yaml::Value::Mapping(
            m.iter()
                .map(|(k, v)| (k.clone(), resolve_value(v, vars)))
                .collect(),
        ),
        serde_yaml::Value::Sequence(s) => {
            serde_yaml::Value::Sequence(s.iter().map(|v| resolve_value(v, vars)).collect())
        }
        other => other.clone(),
    }
}

fn simple_to_string(v: &serde_yaml::Value) -> Option<String> {
    match v {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_schedule() {
        let yaml = include_str!("../../examples/schedules/basic.yml");
        let schedule = Schedule::from_str(yaml).expect("failed to parse basic schedule");

        assert_eq!(schedule.steps.len(), 5);

        // Step 0: tracker.run with relative time (T+10 seconds)
        assert_eq!(
            schedule.steps[0].time,
            Some(TimeExpr::Relative(Duration::seconds(10)))
        );
        assert!(matches!(
            &schedule.steps[0].command,
            Command::Tracker(tracker::Command::Run { rotator, tle, .. })
            if rotator.as_deref() == Some("uhf1") && tle.contains("ISS (ZARYA)")
        ));

        // Step 1: executor.run_shell (immediate)
        assert_eq!(
            schedule.steps[1].command,
            Command::Executor(executor::Command::RunShell {
                cmd: "python -c \"print('hello world from pre_script')\"".to_string(),
                on_fail: executor::OnFail::Continue,
            })
        );

        // Step 2: radio.run with variable resolution (immediate)
        assert_eq!(
            schedule.steps[2].command,
            Command::Radio(radio::Command::Run {
                radio: "sdr1".to_string(),
                bandwidth: "100 KHz".to_string(),
                out: Some(radio::Output {
                    udp: Some(radio::UdpOutput {
                        send: "127.0.0.1:817817".to_string(),
                        format: "cs16".to_string(),
                    })
                }),
                web_fft: true,
            })
        );

        // Step 3: tracker.rotator_park with absolute time ($end - 10 seconds = 2026-01-12T10:09:50Z)
        assert_eq!(
            // This checks variable resolution and a dynamic time expression
            schedule.steps[3].time,
            Some(TimeExpr::Absolute(
                DateTime::parse_from_rfc3339("2026-01-12T10:09:50Z")
                    .unwrap()
                    .with_timezone(&Utc)
            ))
        );
        assert_eq!(
            schedule.steps[3].command,
            Command::Tracker(tracker::Command::RotatorPark {
                rotator: "uhf1".to_string(),
            })
        );

        // Step 4: executor.run_shell (immediate)
        assert_eq!(
            schedule.steps[4].command,
            Command::Executor(executor::Command::RunShell {
                cmd: "echo \"hello from post_script\"".to_string(),
                on_fail: executor::OnFail::Abort,
            })
        );
    }

    #[test]
    fn test_missing_start_variable() {
        let yaml = "variables: {end: '2026-01-12T10:10:00Z'}\nsteps: []";
        assert!(matches!(
            Schedule::from_str(yaml),
            Err(ParseError::Validation(msg)) if msg.contains("start")
        ));
    }

    #[test]
    fn test_missing_end_variable() {
        let yaml = "variables: {start: '2026-01-12T10:00:00Z'}\nsteps: []";
        assert!(matches!(
            Schedule::from_str(yaml),
            Err(ParseError::Validation(msg)) if msg.contains("end")
        ));
    }

    #[test]
    fn test_invalid_start_datetime() {
        let yaml = "variables: {start: 'not-a-date', end: '2026-01-12T10:10:00Z'}\nsteps: []";
        assert!(matches!(
            Schedule::from_str(yaml),
            Err(ParseError::Validation(msg)) if msg.contains("start")
        ));
    }

    #[test]
    fn test_end_before_start() {
        let yaml =
            "variables: {start: '2026-01-12T10:10:00Z', end: '2026-01-12T10:00:00Z'}\nsteps: []";
        assert!(matches!(
            Schedule::from_str(yaml),
            Err(ParseError::Validation(msg)) if msg.contains("must be after")
        ));
    }

    #[test]
    fn test_end_equal_to_start() {
        let yaml =
            "variables: {start: '2026-01-12T10:00:00Z', end: '2026-01-12T10:00:00Z'}\nsteps: []";
        assert!(matches!(
            Schedule::from_str(yaml),
            Err(ParseError::Validation(msg)) if msg.contains("must be after")
        ));
    }
}
