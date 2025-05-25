//! Prototypical module for translation
//! This module is completely prototypical and will be completely rewritten in the future
//!
//!

use std::io::Write;

use crate::{
    helper::{divide_into_chunks, extract_translated_from_response, read_string_file},
    Language,
};
use google_genai::datatypes::{Content, GenerateContentParameters, Part};
use tokio::runtime::Runtime;

fn get_default_prompt() -> String {
    read_string_file("/Users/dobbikov/Desktop/stage/prompts/prompt3")
}

pub(crate) fn put_lang_into_prompt(prompt: &str, lang: &Language) -> String {
    let lang_str: &str = (*lang).clone().into();

    prompt.replace("[TARGET_LANGUAGE]", lang_str)
}

pub fn translate_file_to_file(
    from_path: impl Into<std::path::PathBuf>,
    to_path: impl Into<std::path::PathBuf>,
    tgt_lang: &Language,
) -> std::io::Result<()> {
    let contents = translate_file(from_path, tgt_lang);
    let to_path: std::path::PathBuf = to_path.into();

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(to_path)?;

    file.write_fmt(format_args!("{}", contents))?;
    Ok(())
}

pub fn translate_file(path: impl Into<std::path::PathBuf>, tgt_lang: &Language) -> String {
    let path: std::path::PathBuf = path.into();
    let contents = read_string_file(path);
    translate_contents(&contents, tgt_lang)
}

pub fn translate_contents(contents: &str, tgt_lang: &Language) -> String {
    let mut res = String::new();

    const LINES_PER_CHUNK: usize = 100;

    let chunks = divide_into_chunks(contents.to_string(), LINES_PER_CHUNK);
    for chunk in chunks {
        let tr_ch = translate_chunk(&chunk, tgt_lang);
        res.push_str(&tr_ch);
    }
    res
}

pub fn translate_chunk(contents: &str, tgt_lang: &Language) -> String {
    let mut fin_mess = String::new();
    let prompt = put_lang_into_prompt(&get_default_prompt(), tgt_lang);
    fin_mess.push_str(&prompt);
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
