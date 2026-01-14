/// Converts a serde_yaml::Value to a string representation.
///
/// This handles all YAML value types including scalars, sequences, and mappings.
/// For complex types (sequences/mappings), returns YAML-formatted string.
pub fn yaml_value_to_str(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::Null => Some(String::from("null")),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Sequence(seq) => serde_yaml::to_string(seq).ok(),
        serde_yaml::Value::Mapping(map) => serde_yaml::to_string(map).ok(),
        serde_yaml::Value::Tagged(tagged) => yaml_value_to_str(&tagged.value),
    }
}
