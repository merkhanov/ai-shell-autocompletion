// tests/parity.rs
use ai_suggest::context::Context;
use ai_suggest::{clean, ollama, prompt};
use serde_json::Value;

fn golden() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/parity.json");
    let data = std::fs::read_to_string(path).expect("read tests/golden/parity.json");
    serde_json::from_str(&data).expect("parse golden fixtures")
}

fn ctx_from_json(v: &Value) -> Context {
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let opt = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    let vec = |k: &str| {
        v.get(k)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|e| e.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    Context {
        cwd: s("cwd"),
        os: s("os"),
        shell: s("shell"),
        git_branch: opt("git_branch"),
        git_status: opt("git_status"),
        dir_entries: vec("dir_entries"),
        history: vec("history"),
    }
}

#[test]
fn build_prompt_matches_python_byte_for_byte() {
    let g = golden();
    for case in g["build_prompt"].as_array().unwrap() {
        let prefix = case["prefix"].as_str().unwrap();
        let ctx = ctx_from_json(&case["ctx"]);
        let expected = case["expected"].as_str().unwrap();
        assert_eq!(prompt::build_prompt(prefix, &ctx), expected, "prefix={prefix:?}");
    }
}

#[test]
fn clean_matches_python() {
    let g = golden();
    for case in g["clean"].as_array().unwrap() {
        let raw = case["raw"].as_str().unwrap();
        let prefix = case["prefix"].as_str().unwrap();
        let expected = case["expected"].as_str().unwrap();
        assert_eq!(clean::clean_suggestion(raw, prefix), expected, "raw={raw:?}");
    }
}

#[test]
fn parse_response_matches_python() {
    let g = golden();
    for case in g["parse_response"].as_array().unwrap() {
        let body = case["body"].as_str().unwrap();
        let expected = case["expected"].as_str().unwrap();
        assert_eq!(ollama::parse_response(body), expected, "body={body:?}");
    }
}
