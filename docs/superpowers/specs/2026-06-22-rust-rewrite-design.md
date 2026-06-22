# Rust Rewrite of the AI-Suggest Helper — Design

**Date:** 2026-06-22
**Status:** Approved

## Goal

Rewrite the Python suggestion helper (`ai_suggest.py` + `ailib/`) as a single,
self-contained **Rust binary** `ai-suggest`. The primary motivation is a clean
learning rewrite of the helper in Rust; a real side benefit is latency — the
per-keystroke ~50ms Python interpreter startup drops to ~1–2ms for a compiled
binary.

The zsh front-end (`ai-autocomplete.plugin.zsh`) and its `zsh-autosuggestions`
integration are **kept as-is**. The line editor (ZLE), ghost-text rendering,
keybindings, debounce, and the history-first strategy all stay in zsh — those
cannot move to Rust and already work well. This rewrite is scoped to the helper
that the strategy shells out to.

## Core decisions

| Decision | Choice |
|---|---|
| Scope | Drop-in replacement for `ai_suggest.py` + `ailib/` only |
| Language | Rust (single binary `ai-suggest`) |
| HTTP | `ureq` (blocking, no async runtime) |
| JSON | `serde` + `serde_json` |
| Error model | std-only `Result` / `Box<dyn Error>`; never break the shell |
| Crate location | Repo root (`Cargo.toml` + `src/`) |
| Python | Deleted only after differential tests confirm parity |
| zsh front-end | Unchanged except the one line that invokes the helper |

## Architecture

```
zsh-autosuggestions (dependency, unchanged)
  - ghost-text render + accept widgets + async forked worker
        ^ registers strategy "ai"
ai-autocomplete.plugin.zsh (unchanged except one line)
  - history-first strategy, debounce, config, toggle
        | spawns per request
ai-suggest (NEW Rust binary, replaces ai_suggest.py + ailib/)
  main -> config -> context -> prompt(FIM) -> ollama -> clean -> stdout suffix
```

The boundary is the same process seam as today: the zsh strategy spawns the
helper once per (debounced) suggestion and reads the suffix from stdout. Only the
helper changes language.

## The parity contract

The binary reproduces `ai_suggest.py`'s I/O contract exactly, so the zsh plugin
changes a single line.

- **Invocation:** `ai-suggest -- "<prefix>"` — optional leading `--`, then the
  partial command as the first positional arg. No arg ⇒ empty prefix.
- **Config:** read from the same `AI_AC_*` environment variables, same defaults:

  | Var | Default | Type |
  |---|---|---|
  | `AI_AC_MIN_CHARS` | `3` | usize (char count) |
  | `AI_AC_HISTORY` | _(empty)_ | newline-joined history string |
  | `AI_AC_HISTORY_LINES` | `30` | usize |
  | `AI_AC_OLLAMA_URL` | `http://localhost:11434` | string |
  | `AI_AC_MODEL` | `qwen2.5-coder:3b` | string |
  | `AI_AC_MAX_TOKENS` | `64` | u32 |
  | `AI_AC_KEEP_ALIVE` | `30m` | string |
  | `AI_AC_TIMEOUT` | `5` | f64 seconds |
  | `AI_AC_DEBUG` | `0` | `"1"` enables logging |

  (The plugin exports `AI_AC_MODEL=qwen2.5-coder:1.5b`; the `:3b` default only
  applies when the var is unset, matching the Python helper's own default.)

- **Output:** write the suffix to stdout with **no trailing newline**.
- **Failure mode:** on *any* failure, print nothing and exit 0. It is an
  invisible layer and must never break or block the shell.
- **Debug log:** when `AI_AC_DEBUG=1`, best-effort append to `/tmp/ai-ac.log`
  (the `query failed: …` and `prefix=… suffix=…` lines), swallowing log errors.

Plugin diff (the only change to the zsh side):

```diff
- suffix="$(AI_AC_HISTORY=$hist python3 "$_AI_AC_DIR/ai_suggest.py" -- "$prefix" 2>/dev/null)"
+ suffix="$(AI_AC_HISTORY=$hist "$_AI_AC_DIR/target/release/ai-suggest" -- "$prefix" 2>/dev/null)"
```

If the binary is missing, the existing `2>/dev/null` + `[[ -n $suffix ]]` guard
already degrades gracefully to "no suggestion."

## Module layout

```
Cargo.toml
src/
  main.rs      # entry: argv -> orchestrate -> print suffix -> never crash
  config.rs    # Config struct from AI_AC_* env vars (+ defaults)
  context.rs   # gather_context() -> Context struct
  prompt.rs    # FIM prompt builder + STOP_TOKENS
  ollama.rs    # query() POST /api/generate + parse_response()
  clean.rs     # clean_suggestion(raw, prefix) -> suffix
```

Modules map 1:1 to `ailib/` so the mental model carries over directly.

## Data flow (unchanged from Python)

1. Parse argv → `prefix`.
2. If `prefix.chars().count() < MIN_CHARS` → print nothing, exit.
3. Read `AI_AC_HISTORY` → split into lines, **drop only blank/whitespace-only
   lines, keep each kept line's content verbatim** (no per-line trim — `fc -ln`
   emits significant leading whitespace) → `Option<Vec<String>>`. Matches
   `[h for h in hist.splitlines() if h.strip()]` in `ai_suggest.py`.
4. `gather_context(history_lines, max_dir_entries=50, history)` → `Context`.
5. `build_prompt(prefix, &ctx)` → FIM string.
6. `ollama::query(...)` → raw response. On error → debug-log + silent exit.
7. `clean_suggestion(&raw, &prefix)` → suffix.
8. Debug-log `prefix`/`suffix`; write suffix to stdout.

## Module details

### config.rs
A `Config` struct with one field per `AI_AC_*` var, populated by a small helper
that reads an env var or falls back to a default and parses the type. Parse
failures fall back to the default (never abort).

### context.rs
`gather_context()` is best-effort and never returns an error — it builds a
`Context` struct:

```rust
struct Context {
    cwd: String,
    os: String,            // "Darwin"/"Linux"/"Windows" — see parity note
    shell: String,
    git_branch: Option<String>,
    git_status: Option<String>,   // up to 10 porcelain lines, joined by '\n'
    dir_entries: Vec<String>,     // non-dotfiles, sorted, dirs suffixed '/', first 50
    history: Vec<String>,         // first history_lines (not last)
}
```

- **cwd:** `std::env::current_dir()`.
- **os:** map `std::env::consts::OS` (`"macos"`/`"linux"`/`"windows"`) to Python's
  `platform.system()` strings (`"Darwin"`/`"Linux"`/`"Windows"`) so prompt output
  is byte-identical to the Python version.
- **shell:** `$SHELL` or `"zsh"`.
- **git:** `git branch --show-current`; if non-empty, also `git status
  --porcelain` (first 10 lines). Both run through `run_with_timeout(cmd, 0.5s)`.
- **dir_entries:** `read_dir(cwd)`, drop names starting with `.`, sort, append
  `/` to directories, take the **first** 50. (Python `_truncate` is `items[:n]`
  — first-n, not last-n; same for `history`.)

**`run_with_timeout` (the one genuinely new mechanism):** `std::process::Command`
has no timeout. Implement a std-only helper that spawns the child, then polls
`try_wait()` against a deadline, killing the child on overrun and returning `""`.
Mirrors Python's `subprocess.run(..., timeout=0.5)`; no extra dependency.

### prompt.rs
Port `FIM_PREFIX/SUFFIX/MIDDLE`, `STOP_TOKENS`, and the context-comment renderer.
`build_prompt` assembles `{FIM_PREFIX}{comment head}{prefix}{FIM_SUFFIX}{FIM_MIDDLE}`.
Comment lines (`# cwd:`, `# os:`, `# git branch:`, `# changed files:`, `# files:`,
`# recent:`) match the Python output exactly.

### ollama.rs
- Build the request body with `serde_json` (`temperature: 0.1`,
  `num_predict: max_tokens`, `stop: STOP_TOKENS`, plus `model`, `prompt`,
  `stream: false`, `keep_alive`, `raw: true`).
- `POST {url}/api/generate` via `ureq` with a read timeout from `AI_AC_TIMEOUT`.
- `parse_response(body)` extracts the `"response"` field as a string; returns
  `""` on any parse error. HTTP/network errors propagate as `Err` so `main`
  logs and exits silently.

### clean.rs
Pure string port of `clean_suggestion`: take the first line; reject a stray
```` ``` ```` fence; `trim_end` only (leading whitespace is significant — e.g.
the space before `"pattern"`); strip an echoed `prefix`; return `""` when the
result is blank or equals the prefix.

## Error handling

"Invisible layer — never break the shell." Internally use `Result` with
`Box<dyn std::error::Error>` (std-only; no `anyhow`, keeping the dependency set
minimal). `main` swallows every `Err` into a silent exit 0, and a defensive
panic hook does the same for any unexpected panic. `gather_context` is
infallible by construction, exactly like the Python version.

## Dependencies

```toml
[dependencies]
ureq = "2"          # blocking HTTP, no async runtime
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

No async runtime, no `anyhow`. `ureq` 2.x is chosen for its simple blocking
`post(url).timeout(..).send_string(..)` API; exact patch versions are resolved by
Cargo during implementation.

## Testing

Two layers: ported unit tests (Rust behaves sensibly) and **differential tests
against the still-present Python** (Rust behaves *identically*). The spec's
central claim is byte-parity, and hand-written Rust assertions cannot verify that
— two implementations can both pass re-stated expectations yet diverge. The
Python is the oracle, so we diff against it while it still exists.

**Ported unit tests** — idiomatic inline `#[cfg(test)] mod tests`:

- **clean:** first-line only, fence rejection, leading-space preservation,
  echoed-prefix strip, blank/equals-prefix → `""`.
- **prompt:** FIM envelope structure, each context-comment line, stop-token set.
- **ollama:** `parse_response` extracts `response`; returns `""` on malformed JSON.
- **context:** returns cwd/os/shell; dir filtering + truncation; never panics.

**Differential (golden) tests on the deterministic surfaces** — the real parity
oracle. End-to-end can't be diffed (Ollama is non-deterministic and needs a
server), so we diff the pure, deterministic functions:

- **`build_prompt`** over a fixed `Context` (fixed cwd, os, shell, git fields,
  dir entries, history) → assert the Rust prompt equals the Python `build_prompt`
  output **byte-for-byte**. Highest-value oracle: the prompt *is* what reaches
  Ollama.
- **`clean_suggestion`** and **`parse_response`** are pure → run Python and Rust
  on shared fixture inputs and assert identical outputs.

Implementation note: capture the Python outputs as committed golden fixtures
(generated once from the current `ailib/`), so the Rust test suite diffs against
them without needing Python at `cargo test` time.

`cargo test` replaces `python3 -m unittest`.

**Manual seam check** (team memory flags the zsh↔helper seam as the silent
failure point): run `ai-suggest -- "git "` by hand and confirm a plausible
suffix on stdout; then source the plugin and confirm ghost text still renders and
accept still works.

**Sequencing:** delete the Python only **after** the differential tests pass —
it is the parity oracle, so it must not be discarded before it has been used to
verify the port.

## Install & docs

- **install.sh:** replace the `python3` check with a `cargo` check; run
  `cargo build --release` in the repo dir; keep the Ollama check, model pull, and
  the `~/.zshrc` source-line append.
- **README.md:** Rust toolchain replaces Python 3 in Requirements; `cargo test`
  in the Development section; the binary noted in "How it works."
- **.gitignore:** add `/target`.

## File layout (after rewrite)

```
Cargo.toml
src/  main.rs  config.rs  context.rs  prompt.rs  ollama.rs  clean.rs
ai-autocomplete.plugin.zsh     # unchanged except the helper-invocation line
install.sh   README.md
docs/...
# removed: ai_suggest.py, ailib/, tests/*.py
```

## Out of scope (YAGNI)

- Long-running daemon / unix-socket protocol (per-invocation binary is enough;
  it's already ~1–2ms to start).
- Moving debounce / history-first strategy / ghost-text rendering into Rust
  (the "own-the-stack" path) — kept as a possible future phase, not built now.
- `anyhow`, `tokio`/`reqwest`, or any async stack.
- Cloud/pluggable backends; caching layer.
