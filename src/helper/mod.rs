/// Search the given directory and each parent directory for `file_name`.
/// Returns the full path to the first match, or `None` if nothing is found.
///
///
pub fn find_file_upwards(path: std::path::PathBuf, file_name: &str) -> Option<std::path::PathBuf> {
    // Where we start the search
    let mut dir = std::fs::canonicalize(&path).ok()?;
    if !dir.is_dir() {
        dir = dir.parent()?.to_path_buf();
    }

    loop {
        let candidate = dir.join(file_name);
        if candidate.is_file() {
            return Some(candidate);
        }

        // If `dir` has no parent, we’ve reached the filesystem root.
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break,
        }
    }

    None
}
