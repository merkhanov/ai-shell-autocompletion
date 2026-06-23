use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct Options<'a> {
    temperature: f64,
    num_predict: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<&'a [&'a str]>,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    keep_alive: &'a str,
    raw: bool,
    options: Options<'a>,
}

/// Extract the `response` field from an Ollama JSON body; "" on any problem.
pub fn parse_response(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("response").and_then(|r| r.as_str()).map(str::to_string))
        .unwrap_or_default()
}

/// Build the `/api/generate` request body. Pure, so it can be unit-tested.
pub fn build_request_body(
    prompt: &str,
    model: &str,
    max_tokens: u32,
    keep_alive: &str,
    raw: bool,
    stop: Option<&[&str]>,
) -> String {
    let req = GenerateRequest {
        model,
        prompt,
        stream: false,
        keep_alive,
        raw,
        options: Options { temperature: 0.1, num_predict: max_tokens, stop },
    };
    serde_json::to_string(&req).unwrap_or_default()
}

/// POST the prompt to `{url}/api/generate` and return the parsed response text.
/// `raw=true` bypasses the chat template (required for FIM). Network/HTTP errors
/// propagate so the caller can treat any failure as "no suggestion."
pub fn query(
    prompt: &str,
    url: &str,
    model: &str,
    max_tokens: u32,
    keep_alive: &str,
    timeout: Duration,
    raw: bool,
    stop: Option<&[&str]>,
) -> Result<String, Box<dyn std::error::Error>> {
    let body = build_request_body(prompt, model, max_tokens, keep_alive, raw, stop);
    let resp = ureq::post(&format!("{url}/api/generate"))
        .timeout(timeout)
        .set("Content-Type", "application/json")
        .send_string(&body)?;
    let text = resp.into_string()?;
    Ok(parse_response(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extracts_field() {
        assert_eq!(parse_response(r#"{"response": "ckout main"}"#), "ckout main");
    }
    #[test]
    fn parse_empty_on_bad_json() {
        assert_eq!(parse_response("not json"), "");
    }
    #[test]
    fn parse_empty_on_missing_field() {
        assert_eq!(parse_response(r#"{"x": 1}"#), "");
    }
    #[test]
    fn body_includes_raw_and_stop() {
        let body = build_request_body("p", "m", 64, "30m", true, Some(&["\n"]));
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["raw"], true);
        assert_eq!(v["stream"], false);
        assert_eq!(v["model"], "m");
        assert_eq!(v["options"]["num_predict"], 64);
        assert_eq!(v["options"]["stop"][0], "\n");
    }
    #[test]
    fn body_omits_stop_when_none() {
        let body = build_request_body("p", "m", 64, "30m", false, None);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["raw"], false);
        assert!(v["options"].get("stop").is_none());
    }
}
