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
