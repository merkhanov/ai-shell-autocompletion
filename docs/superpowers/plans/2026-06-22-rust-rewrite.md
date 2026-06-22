# Rust Rewrite of the AI-Suggest Helper — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Python suggestion helper (`ai_suggest.py` + `ailib/`) with a single drop-in Rust binary `ai-suggest` that keeps the exact same I/O contract.

**Architecture:** A lib+bin Cargo crate at the repo root. Pure modules (`clean`, `context`, `prompt`, `ollama`, `config`) live in `src/` behind a `lib.rs` so both the binary (`main.rs`) and integration tests can call them. The zsh front-end is unchanged except the one line that invokes the helper. Parity with the outgoing Python is verified by differential/golden tests before the Python is deleted.

**Tech Stack:** Rust (edition 2021), `ureq` (blocking HTTP), `serde` + `serde_json`. No async runtime, no `anyhow`.

## Global Constraints

- **Never break the shell:** on *any* failure (and on panic) print nothing and exit 0.
- **Output is the suffix only**, written to stdout with **no trailing newline**.
- **Parity contract (verbatim):** invocation `ai-suggest -- "<prefix>"`; config from `AI_AC_*` env vars; defaults — `AI_AC_MIN_CHARS=3`, `AI_AC_HISTORY_LINES=30`, `AI_AC_OLLAMA_URL=http://localhost:11434`, `AI_AC_MODEL=qwen2.5-coder:3b`, `AI_AC_MAX_TOKENS=64`, `AI_AC_KEEP_ALIVE=30m`, `AI_AC_TIMEOUT=5`, `AI_AC_DEBUG=0`.
- **Dir/history caps are first-`n`** (not last-n): `dir_entries` ≤ 50, `history` ≤ `history_lines`.
- **History lines kept verbatim:** drop only blank/whitespace-only lines; never trim a kept line's content.
- **`MIN_CHARS` is a char count** (`prefix.chars().count()`), not bytes.
- **OS string** is mapped to `platform.system()` values: `macos→Darwin`, `linux→Linux`, `windows→Windows`.
- **Ollama options:** `temperature: 0.1`, `num_predict: max_tokens`, `stop` included only when provided, `raw: true`, `stream: false`.
- **Do NOT delete the Python until Task 8's differential tests pass** — it is the parity oracle.
- Dependency versions: `ureq = "2"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`.

---

### Task 1: Project scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs` (empty module root)
- Create: `src/main.rs` (compiling stub)
- Modify: `.gitignore`

**Interfaces:**
- Produces: a crate that builds and whose test runner works. Package name `ai-suggest` ⇒ binary `ai-suggest`, library `ai_suggest` (hyphen→underscore).

- [ ] **Step 1: Write `Cargo.toml`**

```toml
[package]
name = "ai-suggest"
version = "0.1.0"
edition = "2021"

[dependencies]
ureq = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Write the empty lib root**

```rust
// src/lib.rs
// Modules are added here task-by-task.
```

- [ ] **Step 3: Write the binary stub**

```rust
// src/main.rs
fn main() {}
```

- [ ] **Step 4: Add `/target` to `.gitignore`**

Append the line `/target` to `.gitignore` (keep existing contents).

- [ ] **Step 5: Verify build + test runner**

Run: `cargo build && cargo test`
Expected: compiles; test run reports `0 passed` (no tests yet).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/main.rs .gitignore
git commit -m "feat(rust): scaffold ai-suggest crate"
```

---

### Task 2: `clean` module

**Files:**
- Create: `src/clean.rs`
- Modify: `src/lib.rs` (add `pub mod clean;`)

**Interfaces:**
- Produces: `clean::clean_suggestion(raw: &str, prefix: &str) -> String` — first line only; reject a stray ``` fence; `trim_end` (keep leading whitespace); strip an echoed `prefix`; `""` when blank or equal to `prefix`. Ports `ailib/clean.py`.

- [ ] **Step 1: Write the failing tests**

```rust
// src/clean.rs  (tests first)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_prefix_when_model_echoes_full_command() {
        assert_eq!(clean_suggestion("git checkout main", "git che"), "ckout main");
    }
    #[test]
    fn returns_suffix_directly_when_not_echoed() {
        assert_eq!(clean_suggestion("ckout main", "git che"), "ckout main");
    }
    #[test]
    fn takes_first_line_only() {
        assert_eq!(clean_suggestion("ckout main\nrm -rf /", "git che"), "ckout main");
    }
    #[test]
    fn preserves_significant_leading_space() {
        assert_eq!(clean_suggestion(" \"pattern\" .", "grep -r"), " \"pattern\" .");
    }
    #[test]
    fn rejects_code_fence() {
        assert_eq!(clean_suggestion("```bash", "git che"), "");
    }
    #[test]
    fn empty_when_equals_prefix() {
        assert_eq!(clean_suggestion("git che", "git che"), "");
    }
    #[test]
    fn empty_when_blank() {
        assert_eq!(clean_suggestion("   ", "git che"), "");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib clean`
Expected: FAIL — `cannot find function clean_suggestion` (and `clean` module not declared).

- [ ] **Step 3: Write the implementation + declare the module**

Add to `src/lib.rs`:
```rust
pub mod clean;
```

Prepend to `src/clean.rs` (above the `#[cfg(test)]` block):
```rust
/// Turn raw FIM model output into the suffix that follows `prefix`.
/// First line only, reject stray ``` fences, trim only the tail (leading
/// whitespace is significant), strip an echoed prefix. Ports ailib/clean.py.
pub fn clean_suggestion(raw: &str, prefix: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let first_line = raw.split('\n').next().unwrap_or("");
    if first_line.trim_start().starts_with("```") {
        return String::new();
    }
    let trimmed = first_line.trim_end(); // keep leading space; trim the tail
    let body = trimmed.strip_prefix(prefix).unwrap_or(trimmed);
    if body.trim().is_empty() || body == prefix {
        return String::new();
    }
    body.to_string()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib clean`
Expected: PASS (7 passed).

- [ ] **Step 5: Commit**

```bash
git add src/clean.rs src/lib.rs
git commit -m "feat(rust): port clean_suggestion"
```

---

### Task 3: `context` module

**Files:**
- Create: `src/context.rs`
- Modify: `src/lib.rs` (add `pub mod context;`)

**Interfaces:**
- Produces:
  - `pub struct Context { cwd, os, shell: String, git_branch, git_status: Option<String>, dir_entries, history: Vec<String> }` (all fields `pub`, derives `Debug, Default, Clone`).
  - `context::gather_context(history_lines: usize, max_dir_entries: usize, history: Option<Vec<String>>) -> Context` — best-effort, never panics.
  - `context::run_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> String` — stdout trimmed, or `""` on error/timeout.

- [ ] **Step 1: Write the failing tests**

```rust
// src/context.rs  (tests first)
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn run_with_timeout_captures_output() {
        assert_eq!(run_with_timeout("echo", &["hello"], Duration::from_secs(2)), "hello");
    }
    #[test]
    fn run_with_timeout_empty_on_missing_command() {
        assert_eq!(run_with_timeout("no-such-cmd-xyzzy", &[], Duration::from_secs(2)), "");
    }
    #[test]
    fn gather_sets_basics_and_keeps_history_verbatim() {
        let ctx = gather_context(5, 10, Some(vec!["  ls -la".to_string()]));
        assert_eq!(ctx.history, vec!["  ls -la".to_string()]); // leading space preserved
        assert!(!ctx.os.is_empty());
    }
    #[test]
    fn history_truncates_to_first_n() {
        let h = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let ctx = gather_context(2, 10, Some(h));
        assert_eq!(ctx.history, vec!["a".to_string(), "b".to_string()]);
    }
    #[test]
    fn os_is_mapped_to_platform_system_value() {
        // On macOS/Linux dev/CI hosts the raw const is remapped, never passed through.
        let name = os_name();
        assert!(!name.is_empty());
        assert_ne!(name, std::env::consts::OS); // "macos"->"Darwin", "linux"->"Linux"
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib context`
Expected: FAIL — module/functions not found.

- [ ] **Step 3: Write the implementation + declare the module**

Add to `src/lib.rs`:
```rust
pub mod context;
```

Prepend to `src/context.rs` (above the `#[cfg(test)]` block):
```rust
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Default, Clone)]
pub struct Context {
    pub cwd: String,
    pub os: String,
    pub shell: String,
    pub git_branch: Option<String>,
    pub git_status: Option<String>,
    pub dir_entries: Vec<String>,
    pub history: Vec<String>,
}

/// Map Rust's OS const to Python's `platform.system()` strings for prompt parity.
pub fn os_name() -> String {
    match std::env::consts::OS {
        "macos" => "Darwin",
        "linux" => "Linux",
        "windows" => "Windows",
        other => other,
    }
    .to_string()
}

/// Run a command best-effort with a wall-clock timeout; return trimmed stdout or
/// "" on spawn error / timeout / non-UTF8. std-only (a reader thread + channel
/// recv_timeout) — `std::process::Command` has no built-in timeout.
pub fn run_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> String {
    let child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let child = match child {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        _ => String::new(), // timeout or error — best-effort empty
    }
}

/// Collect best-effort shell context. Never panics. Ports ailib/context.py.
pub fn gather_context(
    history_lines: usize,
    max_dir_entries: usize,
    history: Option<Vec<String>>,
) -> Context {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut ctx = Context {
        cwd: cwd.clone(),
        os: os_name(),
        shell: std::env::var("SHELL").unwrap_or_else(|_| "zsh".to_string()),
        ..Default::default()
    };

    let branch = run_with_timeout("git", &["branch", "--show-current"], Duration::from_millis(500));
    if !branch.is_empty() {
        ctx.git_branch = Some(branch);
        let status = run_with_timeout("git", &["status", "--porcelain"], Duration::from_millis(500));
        if !status.is_empty() {
            let first_ten: Vec<&str> = status.lines().take(10).collect();
            ctx.git_status = Some(first_ten.join("\n"));
        }
    }

    if let Ok(entries) = std::fs::read_dir(&cwd) {
        // Sort by RAW name first (like Python's sorted(os.listdir)), THEN append
        // "/" to dirs — appending before sorting would reorder entries.
        let mut names: Vec<(String, bool)> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    return None;
                }
                let is_dir = e.path().is_dir();
                Some((name, is_dir))
            })
            .collect();
        names.sort_by(|a, b| a.0.cmp(&b.0));
        let mut rendered: Vec<String> = names
            .into_iter()
            .map(|(name, is_dir)| if is_dir { format!("{name}/") } else { name })
            .collect();
        rendered.truncate(max_dir_entries);
        ctx.dir_entries = rendered;
    }

    if let Some(mut h) = history {
        h.truncate(history_lines);
        ctx.history = h;
    }

    ctx
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib context`
Expected: PASS (5 passed).

- [ ] **Step 5: Commit**

```bash
git add src/context.rs src/lib.rs
git commit -m "feat(rust): port context gathering with std-only command timeout"
```

---

### Task 4: `prompt` module

**Files:**
- Create: `src/prompt.rs`
- Modify: `src/lib.rs` (add `pub mod prompt;`)

**Interfaces:**
- Consumes: `context::Context`.
- Produces:
  - `pub const FIM_PREFIX/FIM_SUFFIX/FIM_MIDDLE: &str` and `pub const STOP_TOKENS: &[&str]`.
  - `prompt::build_prompt(prefix: &str, ctx: &Context) -> String`.

- [ ] **Step 1: Write the failing tests**

```rust
// src/prompt.rs  (tests first)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;

    fn ctx() -> Context {
        Context {
            cwd: "/home/u/proj".into(),
            os: "Darwin".into(),
            shell: "zsh".into(),
            git_branch: Some("main".into()),
            git_status: Some("M file.py".into()),
            dir_entries: vec!["src/".into(), "README.md".into()],
            history: vec!["cd proj".into(), "ls".into()],
        }
    }

    #[test]
    fn includes_prefix_and_context() {
        let p = build_prompt("git che", &ctx());
        for needle in ["git che", "/home/u/proj", "main", "README.md", "cd proj"] {
            assert!(p.contains(needle), "missing {needle}");
        }
    }
    #[test]
    fn uses_fim_format() {
        let p = build_prompt("ls", &ctx());
        assert!(p.starts_with("<|fim_prefix|>"));
        assert!(p.ends_with("<|fim_middle|>"));
        assert!(p.contains("<|fim_suffix|>"));
    }
    #[test]
    fn partial_command_sits_just_before_suffix_marker() {
        let p = build_prompt("git che", &ctx());
        assert!(p.contains("git che<|fim_suffix|>"));
    }
    #[test]
    fn handles_empty_context() {
        let p = build_prompt("ls", &Context::default());
        assert!(p.contains("ls"));
        assert_eq!(p, "<|fim_prefix|>ls<|fim_suffix|><|fim_middle|>");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib prompt`
Expected: FAIL — module/function not found.

- [ ] **Step 3: Write the implementation + declare the module**

Add to `src/lib.rs`:
```rust
pub mod prompt;
```

Prepend to `src/prompt.rs` (above the `#[cfg(test)]` block):
```rust
use crate::context::Context;

pub const FIM_PREFIX: &str = "<|fim_prefix|>";
pub const FIM_SUFFIX: &str = "<|fim_suffix|>";
pub const FIM_MIDDLE: &str = "<|fim_middle|>";

/// Stop at end of line or any FIM/EOT control token.
pub const STOP_TOKENS: &[&str] = &[
    "\n",
    "<|endoftext|>",
    "<|fim_pad|>",
    "<|file_sep|>",
    FIM_PREFIX,
    FIM_SUFFIX,
    FIM_MIDDLE,
];

/// Render context as shell-comment lines a coder model understands.
/// Empty strings / None / empty vecs are skipped (Python truthiness parity).
fn context_comments(ctx: &Context) -> Vec<String> {
    let mut out = Vec::new();
    if !ctx.cwd.is_empty() {
        out.push(format!("# cwd: {}", ctx.cwd));
    }
    if !ctx.os.is_empty() {
        let shell = if ctx.shell.is_empty() { "zsh" } else { ctx.shell.as_str() };
        out.push(format!("# os: {} shell: {}", ctx.os, shell));
    }
    if let Some(branch) = ctx.git_branch.as_deref().filter(|b| !b.is_empty()) {
        out.push(format!("# git branch: {branch}"));
        if let Some(status) = ctx.git_status.as_deref().filter(|s| !s.is_empty()) {
            out.push(format!("# changed files: {}", status.replace('\n', ", ")));
        }
    }
    if !ctx.dir_entries.is_empty() {
        out.push(format!("# files: {}", ctx.dir_entries.join(", ")));
    }
    if !ctx.history.is_empty() {
        out.push(format!("# recent: {}", ctx.history.join("; ")));
    }
    out
}

/// Assemble a FIM prompt: context comments + the partial command, with an empty
/// suffix so the model completes to end of line. Ports ailib/prompt.py.
pub fn build_prompt(prefix: &str, ctx: &Context) -> String {
    let mut head = context_comments(ctx).join("\n");
    if !head.is_empty() {
        head.push('\n');
    }
    format!("{FIM_PREFIX}{head}{prefix}{FIM_SUFFIX}{FIM_MIDDLE}")
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib prompt`
Expected: PASS (4 passed).

- [ ] **Step 5: Commit**

```bash
git add src/prompt.rs src/lib.rs
git commit -m "feat(rust): port FIM prompt builder"
```

---

### Task 5: `ollama` module

**Files:**
- Create: `src/ollama.rs`
- Modify: `src/lib.rs` (add `pub mod ollama;`)

**Interfaces:**
- Produces:
  - `ollama::parse_response(body: &str) -> String` — extract `response`; `""` on error.
  - `ollama::build_request_body(prompt: &str, model: &str, max_tokens: u32, keep_alive: &str, raw: bool, stop: Option<&[&str]>) -> String` — the JSON payload (pure, testable).
  - `ollama::query(prompt, url, model, max_tokens, keep_alive, timeout: Duration, raw, stop) -> Result<String, Box<dyn std::error::Error>>` — POST `{url}/api/generate`, return parsed response.

- [ ] **Step 1: Write the failing tests**

```rust
// src/ollama.rs  (tests first)
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib ollama`
Expected: FAIL — module/functions not found.

- [ ] **Step 3: Write the implementation + declare the module**

Add to `src/lib.rs`:
```rust
pub mod ollama;
```

Prepend to `src/ollama.rs` (above the `#[cfg(test)]` block):
```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib ollama`
Expected: PASS (5 passed).

- [ ] **Step 5: Commit**

```bash
git add src/ollama.rs src/lib.rs
git commit -m "feat(rust): port ollama client (ureq + serde)"
```

---

### Task 6: `config` module

**Files:**
- Create: `src/config.rs`
- Modify: `src/lib.rs` (add `pub mod config;`)

**Interfaces:**
- Produces: `pub struct Config { min_chars, history_lines: usize, ollama_url, model, keep_alive: String, max_tokens: u32, timeout: f64, debug: bool }` and `Config::from_env() -> Config`.

- [ ] **Step 1: Write the failing tests**

```rust
// src/config.rs  (tests first)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_parse_falls_back_when_unset() {
        // A name that is never set, so this is deterministic under parallel tests.
        assert_eq!(env_parse::<usize>("AI_AC_NEVER_SET_PARSE_Q", 3), 3);
    }
    #[test]
    fn env_or_falls_back_when_unset() {
        assert_eq!(env_or("AI_AC_NEVER_SET_OR_Q", "def"), "def");
    }
    #[test]
    fn env_parse_falls_back_on_garbage() {
        std::env::set_var("AI_AC_GARBAGE_INT_Q", "not-a-number");
        assert_eq!(env_parse::<u32>("AI_AC_GARBAGE_INT_Q", 64), 64);
        std::env::remove_var("AI_AC_GARBAGE_INT_Q");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib config`
Expected: FAIL — module/functions not found.

- [ ] **Step 3: Write the implementation + declare the module**

Add to `src/lib.rs`:
```rust
pub mod config;
```

Prepend to `src/config.rs` (above the `#[cfg(test)]` block):
```rust
fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn env_parse<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[derive(Debug, Clone)]
pub struct Config {
    pub min_chars: usize,
    pub history_lines: usize,
    pub ollama_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub keep_alive: String,
    pub timeout: f64,
    pub debug: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            min_chars: env_parse("AI_AC_MIN_CHARS", 3),
            history_lines: env_parse("AI_AC_HISTORY_LINES", 30),
            ollama_url: env_or("AI_AC_OLLAMA_URL", "http://localhost:11434"),
            model: env_or("AI_AC_MODEL", "qwen2.5-coder:3b"),
            max_tokens: env_parse("AI_AC_MAX_TOKENS", 64),
            keep_alive: env_or("AI_AC_KEEP_ALIVE", "30m"),
            timeout: env_parse("AI_AC_TIMEOUT", 5.0),
            debug: env_or("AI_AC_DEBUG", "0") == "1",
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib config`
Expected: PASS (3 passed).

- [ ] **Step 5: Commit**

```bash
git add src/config.rs src/lib.rs
git commit -m "feat(rust): add Config from AI_AC_* env vars"
```

---

### Task 7: `main` orchestration

**Files:**
- Modify: `src/main.rs`
- Test: manual integration (requires Ollama running + model pulled).

**Interfaces:**
- Consumes: `clean`, `config::Config`, `context`, `ollama`, `prompt`.
- CLI: `ai-suggest -- "<prefix>"`. Reads `AI_AC_*` env (history via `AI_AC_HISTORY`, newline-separated). Prints suffix (no newline). Silent + exit 0 on any failure or panic.

- [ ] **Step 1: Write the full binary**

```rust
// src/main.rs
use std::io::Write;
use std::time::Duration;

use ai_suggest::config::Config;
use ai_suggest::{clean, context, ollama, prompt};

fn log(debug: bool, msg: &str) {
    if !debug {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/ai-ac.log")
    {
        let _ = writeln!(f, "{msg}");
    }
}

fn run() {
    let cfg = Config::from_env();

    // argv: optional leading "--", then the prefix as the first positional.
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s == "--").unwrap_or(false) {
        args.remove(0);
    }
    let prefix = args.into_iter().next().unwrap_or_default();

    if prefix.chars().count() < cfg.min_chars {
        return;
    }

    // History via env: drop blank lines, keep each kept line verbatim.
    let history: Option<Vec<String>> = match std::env::var("AI_AC_HISTORY") {
        Ok(raw) => {
            let lines: Vec<String> = raw
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.to_string())
                .collect();
            if lines.is_empty() { None } else { Some(lines) }
        }
        Err(_) => None,
    };

    let ctx = context::gather_context(cfg.history_lines, 50, history);
    let prompt_str = prompt::build_prompt(&prefix, &ctx);

    let raw = match ollama::query(
        &prompt_str,
        &cfg.ollama_url,
        &cfg.model,
        cfg.max_tokens,
        &cfg.keep_alive,
        Duration::from_secs_f64(cfg.timeout),
        true,
        Some(prompt::STOP_TOKENS),
    ) {
        Ok(r) => r,
        Err(e) => {
            log(cfg.debug, &format!("query failed: {e}"));
            return;
        }
    };

    let suffix = clean::clean_suggestion(&raw, &prefix);
    log(cfg.debug, &format!("prefix={prefix:?} suffix={suffix:?}"));
    print!("{suffix}");
    let _ = std::io::stdout().flush();
}

fn main() {
    // Invisible layer: silence panics and swallow them so the shell is never
    // disrupted. Any failure path prints nothing and exits 0.
    std::panic::set_hook(Box::new(|_| {}));
    let _ = std::panic::catch_unwind(run);
}
```

- [ ] **Step 2: Build and run the whole unit suite**

Run: `cargo test`
Expected: PASS (all lib tests green).

- [ ] **Step 3: Build the release binary**

Run: `cargo build --release`
Expected: produces `target/release/ai-suggest`.

- [ ] **Step 4: Manual integration test** (Ollama running + `qwen2.5-coder` pulled)

Run: `AI_AC_MODEL=qwen2.5-coder:1.5b ./target/release/ai-suggest -- "git che"`
Expected: prints a plausible suffix like `ckout main` (no trailing newline).

Run with Ollama stopped: `AI_AC_OLLAMA_URL=http://localhost:1 ./target/release/ai-suggest -- "git che"; echo "exit=$?"`
Expected: prints nothing, `exit=0`.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat(rust): orchestrate ai-suggest binary"
```

---

### Task 8: Differential (golden) parity tests

**Files:**
- Create: `scripts/gen_golden.py` (one-time generator; imports the still-present `ailib/`)
- Create: `tests/golden/parity.json` (generated, committed)
- Create: `tests/parity.rs` (integration test)

**Interfaces:**
- Consumes: `ai_suggest::{prompt, clean, ollama}` and `ai_suggest::context::Context`.
- The Python is the oracle: golden values are generated FROM `ailib/`, not hand-written, so the Rust is diffed against the real Python output.

- [ ] **Step 1: Write the generator**

```python
# scripts/gen_golden.py — run once to snapshot Python outputs as the parity oracle.
import json, os, sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from ailib.prompt import build_prompt
from ailib.clean import clean_suggestion
from ailib.ollama import parse_response

CTX = {
    "cwd": "/home/u/proj", "os": "Darwin", "shell": "zsh",
    "git_branch": "main", "git_status": "M file.py\nA new.py",
    "dir_entries": ["src/", "README.md"], "history": ["cd proj", "ls"],
}
build_cases = [
    {"prefix": "git che", "ctx": CTX},
    {"prefix": "ls", "ctx": {}},
    {"prefix": "grep -r", "ctx": {"cwd": "/tmp"}},
    {"prefix": "docker ", "ctx": {"cwd": "/srv", "os": "Linux", "shell": "zsh",
                                   "git_branch": "dev"}},
]
for c in build_cases:
    c["expected"] = build_prompt(c["prefix"], c["ctx"])

clean_cases = [
    {"raw": "git checkout main", "prefix": "git che"},
    {"raw": "ckout main\nrm -rf /", "prefix": "git che"},
    {"raw": ' "pattern" .', "prefix": "grep -r"},
    {"raw": "```bash", "prefix": "git che"},
    {"raw": "git che", "prefix": "git che"},
    {"raw": "   ", "prefix": "git che"},
]
for c in clean_cases:
    c["expected"] = clean_suggestion(c["raw"], c["prefix"])

parse_cases = [
    {"body": '{"response": "ckout main"}'},
    {"body": "not json"},
    {"body": '{"x": 1}'},
    {"body": '{"response": ""}'},
]
for c in parse_cases:
    c["expected"] = parse_response(c["body"])

print(json.dumps(
    {"build_prompt": build_cases, "clean": clean_cases, "parse_response": parse_cases},
    indent=2,
))
```

- [ ] **Step 2: Generate and commit the golden fixtures**

Run:
```bash
mkdir -p tests/golden
python3 scripts/gen_golden.py > tests/golden/parity.json
```
Expected: `tests/golden/parity.json` contains `expected` strings for every case.

- [ ] **Step 3: Write the integration parity test**

```rust
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
```

- [ ] **Step 4: Run the parity tests**

Run: `cargo test --test parity`
Expected: PASS (3 passed). If `build_prompt_matches_python_byte_for_byte` fails, the Rust prompt diverges from Python — fix the Rust until byte-identical (do NOT edit the golden file by hand).

- [ ] **Step 5: Commit**

```bash
git add scripts/gen_golden.py tests/golden/parity.json tests/parity.rs
git commit -m "test(rust): differential parity tests vs Python oracle"
```

---

### Task 9: Wire-up, docs, and Python removal

**Files:**
- Modify: `ai-autocomplete.plugin.zsh` (one line)
- Modify: `install.sh`
- Modify: `README.md`
- Delete: `ai_suggest.py`, `ailib/` (4 files + `__init__.py`), `tests/__init__.py`, `tests/test_clean.py`, `tests/test_context.py`, `tests/test_ollama.py`, `tests/test_prompt.py`, `scripts/gen_golden.py`

**Interfaces:**
- The plugin now invokes the Rust binary instead of `python3`. `tests/golden/parity.json` is KEPT (it is the committed oracle snapshot; the Rust parity test still uses it after Python is gone).

- [ ] **Step 1: Point the plugin at the binary**

In `ai-autocomplete.plugin.zsh`, change the one strategy line:
```zsh
  suffix="$(AI_AC_HISTORY=$hist "$_AI_AC_DIR/target/release/ai-suggest" -- "$prefix" 2>/dev/null)"
```
(Replaces the `python3 "$_AI_AC_DIR/ai_suggest.py" -- "$prefix"` invocation. Everything else in the plugin is unchanged.)

- [ ] **Step 2: Update `install.sh`**

Replace the `python3` dependency check + the message with a Rust toolchain check, and build the binary. Change:
```bash
command -v python3 >/dev/null || { echo "python3 required"; exit 1; }
```
to:
```bash
command -v cargo >/dev/null || { echo "Rust toolchain required: https://rustup.rs"; exit 1; }
```
and add, right after the model-pull block:
```bash
echo "Building ai-suggest (release) ..."
( cd "$DIR" && cargo build --release )
```

- [ ] **Step 3: Update `README.md`**

- Requirements: replace "Python 3 (standard library only — no pip packages)" with "Rust toolchain (`cargo`) — builds a small binary, `ureq` + `serde` only."
- Install: note `./install.sh` runs `cargo build --release`.
- How it works: the strategy shells out to the `ai-suggest` **binary** (not Python).
- Development: replace the unittest line with:
  ```bash
  cargo test            # unit + differential parity tests
  ```

- [ ] **Step 4: Run the full Rust suite once more**

Run: `cargo test`
Expected: PASS (all unit + parity tests). This is the gate — parity is confirmed.

- [ ] **Step 5: Remove the Python (oracle no longer needed live)**

Run:
```bash
git rm ai_suggest.py ailib/__init__.py ailib/clean.py ailib/context.py \
       ailib/ollama.py ailib/prompt.py \
       tests/__init__.py tests/test_clean.py tests/test_context.py \
       tests/test_ollama.py tests/test_prompt.py \
       scripts/gen_golden.py
```
(Keep `tests/golden/parity.json`.)

- [ ] **Step 6: Verify nothing references the deleted Python**

Run: `grep -rn "ai_suggest.py\|ailib\|python3" ai-autocomplete.plugin.zsh install.sh README.md`
Expected: no matches (the plugin/install/docs all point at the binary).

Run: `cargo test`
Expected: PASS (parity test still green using the kept golden fixtures).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(rust): switch plugin/installer/docs to ai-suggest binary; remove Python"
```

---

## Self-Review

**1. Spec coverage:**
- Drop-in binary + parity contract → Tasks 1–7 (modules) + Task 9 (plugin/install switch).
- `config / context / prompt / ollama / clean / main` modules → Tasks 6/3/4/5/2/7.
- `ureq` + `serde`/`serde_json`, no async/anyhow → Task 1 `Cargo.toml`, Task 5.
- Never-crash + silent-exit-0 + panic hook → Task 7 `main`.
- `run_with_timeout` for git → Task 3.
- OS-string mapping, first-`n` truncation, raw-name-sort-then-slash, verbatim history → Tasks 3 + 7, locked by Task 8 differential `build_prompt`.
- Differential/golden parity tests; delete Python only after they pass → Tasks 8 then 9 (gate at 9.4, deletion at 9.5).
- install.sh (cargo), README, `/target` ignore → Tasks 1 + 9.
- Manual seam check → Task 7 Step 4.

**2. Placeholder scan:** every code step contains complete code; `tests/golden/parity.json` is generated output (exact generator + command given), not a placeholder.

**3. Type consistency:** `Context` (fields + types) defined in Task 3 is used identically in Tasks 4 (`build_prompt(&Context)`), 7, and 8 (`ctx_from_json`). `clean_suggestion(&str,&str)->String`, `parse_response(&str)->String`, `build_request_body(...)->String`, `query(...,Duration,bool,Option<&[&str]>)->Result<String,Box<dyn Error>>`, `gather_context(usize,usize,Option<Vec<String>>)->Context`, `Config::from_env()->Config`, `STOP_TOKENS: &[&str]` — all used consistently across producing and consuming tasks.
