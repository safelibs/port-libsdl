use std::path::PathBuf;

pub fn parse_device_hint(value: &str) -> Vec<PathBuf> {
    value
        .split(':')
        .filter(|segment| !segment.is_empty())
        .map(PathBuf::from)
        .collect()
}
