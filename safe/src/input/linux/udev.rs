use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn sort_key(path: &Path) -> (String, Option<u32>, String) {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let split = name
        .rfind(|ch: char| !ch.is_ascii_digit())
        .map(|index| index + 1)
        .unwrap_or(0);
    let (prefix, suffix) = name.split_at(split);
    let numeric = (!suffix.is_empty())
        .then(|| suffix.parse::<u32>().ok())
        .flatten();
    (prefix.to_string(), numeric, name)
}

pub fn discover_device_nodes(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| {
                    name.starts_with("event")
                        || name.starts_with("js")
                        || name.starts_with("hidraw")
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|path| sort_key(path));
    Ok(entries)
}
