//! A module with helper functions. Most of functions aim to work with files and text.
use std::io::{Read, Write};

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

/// Takes a text, divides it into chunks (each chunk containing at most precised
/// _lines\_per\_chunk_ number of lines) and returns the vector of such chunks
pub fn divide_into_chunks(text: String, lines_per_chunk: usize) -> Vec<String> {
    let mut res = Vec::<String>::new();

    if !text.contains("\n") || lines_per_chunk == 0 {
        return vec![text];
    }

    let parts = text.split("\n");
    let mut temp_res = String::new();
    for (id, part) in parts.enumerate() {
        temp_res.push_str(part);
        temp_res.push('\n');

        if (id > 0 && id % lines_per_chunk == 0) {
            res.push(temp_res);
            temp_res = String::new();
        }
    }
    if !temp_res.is_empty() {
        res.push(temp_res);
    }

    res
}

/// Takes a text into parameter and returns the content written in the `<document>` tag.
pub fn extract_translated_from_response(message: String) -> String {
    if !message.contains("<output>") {
        return String::new();
    }
    let mut res = String::new();
    let mut chunks_iter = message.split("<output>");
    let _ = chunks_iter.next();
    while let Some(chunk) = chunks_iter.next() {
        let chunk = chunk.strip_prefix("\n").unwrap_or(chunk);
        let mut chunk_string = chunk;

        if chunk_string.contains("</output>") {
            chunk_string = chunk_string.split("</output>").next().unwrap();
        }
        res.push_str(chunk_string);
    }
    res
}

/// Reads file and returns its contents in the String format
pub fn read_string_file(path: impl Into<std::path::PathBuf>) -> String {
    let mut contents = String::new();
    let path: std::path::PathBuf = path.into();
    let mut file = std::fs::File::open(path).expect("Couldn't open file");
    let _ = file.read_to_string(&mut contents);
    contents
}
