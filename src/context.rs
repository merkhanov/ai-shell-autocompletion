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
