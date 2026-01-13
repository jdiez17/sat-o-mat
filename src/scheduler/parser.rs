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
}

#[derive(Debug, Clone)]
pub struct Schedule {
    #[allow(dead_code)]
    pub variables: HashMap<String, serde_yaml::Value>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub time: Option<TimeExpr>,
    pub command: Command,
}

#[derive(Debug, Clone)]
pub enum TimeExpr {
    Relative(Duration),
    Absolute(DateTime<Utc>),
}

impl TimeExpr {
    pub fn resolve(&self, start: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            TimeExpr::Relative(d) => start + *d,
            TimeExpr::Absolute(dt) => *dt,
        }
    }
}

#[derive(Debug, Clone)]
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

        let steps = root
            .get("steps")
            .and_then(|v| v.as_sequence())
            .ok_or_else(|| ParseError::Step(0, "missing 'steps'".into()))?
            .iter()
            .enumerate()
            .map(|(i, v)| parse_step(i, v, &variables))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Schedule { variables, steps })
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
        "tracker" => {
            Command::Tracker(serde_yaml::from_value(value).map_err(|e| err(&e.to_string()))?)
        }
        "executor" => {
            Command::Executor(serde_yaml::from_value(value).map_err(|e| err(&e.to_string()))?)
        }
        "radio" => Command::Radio(serde_yaml::from_value(value).map_err(|e| err(&e.to_string()))?),
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
