//! Prototypical module for translation
//! This module is completely prototypical and will be completely rewritten in the future
//!
//!

use crate::helper::{divide_into_chunks, extract_translated_from_response, read_string_file};
use google_genai::datatypes::{Content, GenerateContentParameters, Part};
use tokio::runtime::Runtime;

fn get_prompt() -> String {
    read_string_file("/Users/dobbikov/Desktop/stage/prompts/prompt3")
}

pub fn translate_file(path: impl Into<std::path::PathBuf>) -> String {
    let path: std::path::PathBuf = path.into();
    let contents = read_string_file(path);
    translate_contents(&contents)
}

pub fn translate_contents(contents: &str) -> String {
    let mut res = String::new();

    const LINES_PER_CHUNK: usize = 100;

    let chunks = divide_into_chunks(contents.to_string(), LINES_PER_CHUNK);
    for chunk in chunks {
        let tr_ch = translate_chunk(&chunk);
        res.push_str(&tr_ch);
    }
    res
}

pub fn translate_chunk(contents: &str) -> String {
    let mut fin_mess = String::new();
    fin_mess.push_str(&get_prompt());
    fin_mess.push_str("<document>");
    fin_mess.push_str(contents);
    fin_mess.push_str("\n</document>");

    let rt = Runtime::new().unwrap();
    let gen_resp = rt.block_on(async { ask_gemini_model(fin_mess).await });

    let translated = extract_translated_from_response(gen_resp);
    translated
}

pub async fn ask_gemini_model(message: String) -> String {
    let api_key =
        std::env::var("GOOGLE_API_KEY").expect("GOOGLEAI_API_KEY environment variable must be set");

    let params = GenerateContentParameters::default()
        .contents(vec![Content {
            parts: Some(vec![Part::default().text(message)]),
            role: Some("user".to_string()),
        }])
        .model("gemini-2.0-flash");

    let request = google_genai::datatypes::GenerateContentReq::default()
        .contents(params.contents.unwrap())
        .model(params.model.unwrap());

    let response = google_genai::generate_content(&api_key, request)
        .await
        .unwrap();
    let text = response
        .candidates // Option<Vec<Candidate>>
        .as_ref() // Option<&Vec<Candidate>>
        .and_then(|v| v.first())
        .and_then(|cand| cand.content.as_ref())
        .and_then(|cnt| cnt.parts.as_ref())
        .and_then(|v| v.first())
        .and_then(|part| part.text.as_ref())
        .cloned() // we finally need an owned String
        .unwrap_or_default(); // or .ok_or(MyError::MissingText)? for Result<T,E>

    return text;
    String::new()
}
