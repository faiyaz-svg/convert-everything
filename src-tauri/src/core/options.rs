//! Options a user can set for a conversion, and helpers to read them.

use serde::Serialize;
use serde_json::{Map, Value, json};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Select,
    Number,
    Text,
    Toggle,
}

#[derive(Serialize, Clone, Debug)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OptionSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice>,
    pub default: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
}

impl OptionSpec {
    fn base(key: &'static str, label: &'static str, kind: Kind, default: Value) -> Self {
        OptionSpec { key, label, kind, choices: vec![], default, min: None, max: None, unit: None, help: None, placeholder: None }
    }

    pub fn select(key: &'static str, label: &'static str, choices: &[(&str, &str)], default: &str) -> Self {
        let mut o = Self::base(key, label, Kind::Select, json!(default));
        o.choices = choices.iter().map(|(v, l)| Choice { value: v.to_string(), label: l.to_string() }).collect();
        o
    }

    pub fn number(key: &'static str, label: &'static str, default: Value, min: f64, max: f64, unit: Option<&'static str>) -> Self {
        let mut o = Self::base(key, label, Kind::Number, default);
        o.min = Some(min);
        o.max = Some(max);
        o.unit = unit;
        o
    }

    pub fn text(key: &'static str, label: &'static str, placeholder: &'static str) -> Self {
        let mut o = Self::base(key, label, Kind::Text, json!(""));
        o.placeholder = Some(placeholder);
        o
    }

    pub fn toggle(key: &'static str, label: &'static str, default: bool) -> Self {
        Self::base(key, label, Kind::Toggle, json!(default))
    }

    pub fn help(mut self, help: &'static str) -> Self {
        self.help = Some(help);
        self
    }
}

/// The values a user picked. Missing or empty values fall back to "not set".
#[derive(Clone, Debug, Default)]
pub struct Opts(pub Map<String, Value>);

impl Opts {
    pub fn new(map: Map<String, Value>) -> Self {
        Opts(map)
    }

    pub fn str(&self, key: &str) -> Option<String> {
        match self.0.get(key)? {
            Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    pub fn str_or(&self, key: &str, default: &str) -> String {
        self.str(key).unwrap_or_else(|| default.to_string())
    }

    pub fn num(&self, key: &str) -> Option<f64> {
        match self.0.get(key)? {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        match self.0.get(key) {
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => s == "true",
            _ => default,
        }
    }
}

/// Parses "90", "1:30", "01:02:03.5" into seconds.
pub fn parse_time(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut total = 0.0;
    for part in s.split(':') {
        total = total * 60.0 + part.trim().parse::<f64>().ok()?;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times() {
        assert_eq!(parse_time("90"), Some(90.0));
        assert_eq!(parse_time("1:30"), Some(90.0));
        assert_eq!(parse_time("01:00:02.5"), Some(3602.5));
        assert_eq!(parse_time(""), None);
        assert_eq!(parse_time("abc"), None);
    }
}
