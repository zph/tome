use gitignored::Gitignore;

use super::script::Script;
use std::{fs::read_dir, io, path::{Path, PathBuf}};

/// scan a directory for all files,
/// consuming each one as a script.
/// returns the invocation
pub fn scan_directory(
    root: &str,
    previous_commands: &mut Vec<String>,
) -> io::Result<Vec<(String, Script)>> {
    let mut result = vec![];
    let paths: Vec<_> = read_dir(root).unwrap().map(|r| r.unwrap()).collect();
    // paths.sort_by_key(|f| f.path());
    for entry in paths {
        let path = entry.path();
        previous_commands.push(
            path.file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .to_string(),
        );
        if path.is_dir() {
            if is_tome_script_directory(&path) {
                result.extend(scan_directory(
                    path.as_path().to_str().unwrap_or_default(),
                    previous_commands,
                )?);
            }
        } else {
            result.push((
                previous_commands.join(" "),
                Script::load(path.as_path().to_str().unwrap_or_default())?,
            ));
        }
        previous_commands.pop();
    }
    Ok(result)
}

pub fn is_tomeignored(root: &str, path: PathBuf) -> bool {
    // Gitignore is particular about the structuring of the root and requires no trailing slash
    let root = root.trim_end_matches("/");
    let mut ig = Gitignore::new(root, true, true);
    let tomeignore_location =  PathBuf::from(&format!("{}/{}", root, ".tomeignore"));
    let binding = std::fs::read_to_string(tomeignore_location.clone()).unwrap_or_default();
    let lines: Vec<&str> = binding.lines().collect();
    if ig.ignores(&lines, path.to_str().unwrap_or_default()) {
        return true;
    }
    return false;
}

/// returns if this directory should be considered by tome
pub fn is_tome_script_directory(dir: &Path) -> bool {
    let mut tomeignore_location = dir.to_path_buf();
    // ignore dot directories
    if tomeignore_location
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .starts_with('.')
    {
        return false;
    }
    tomeignore_location.push(".tomeignore");
    !tomeignore_location.exists()
}
