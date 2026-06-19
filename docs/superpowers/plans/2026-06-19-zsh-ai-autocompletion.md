# zsh AI Shell Autocompletion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cursor-style inline AI autocompletion for zsh, powered by a local Ollama model, built on top of `zsh-autosuggestions`.

**Architecture:** A thin zsh plugin registers a custom `zsh-autosuggestions` strategy that, after a parent-side debounce, shells out to a per-request Python helper. The helper gathers rich shell context, builds a prompt, calls Ollama, and prints the completion suffix.

**Tech Stack:** zsh + zsh-autosuggestions, Python 3 (stdlib only), Ollama HTTP API.

## Global Constraints

- Python: **stdlib only** — no pip packages (urllib for HTTP, json, subprocess, os, platform).
- The helper must **never raise to the shell**: any failure prints nothing and exits 0.
- The helper prints the **suffix only** (no trailing newline); empty output = no suggestion.
- All context sources have **hard caps** to keep prompts small.
- Defaults: model `qwen2.5-coder:3b`, debounce `0.2`s, min chars `3`, max tokens `64`, keep_alive `30m`, Ollama `http://localhost:11434`.

---

### Task 1: Output cleaning module

**Files:**
- Create: `ailib/__init__.py` (empty)
- Create: `ailib/clean.py`
- Test: `tests/test_clean.py`

**Interfaces:**
- Produces: `clean_suggestion(raw: str, prefix: str) -> str` — turns raw model output into the suffix to append after `prefix`; returns `""` when unusable.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_clean.py
from ailib.clean import clean_suggestion

def test_strips_prefix_when_model_echoes_full_command():
    assert clean_suggestion("git checkout main", "git che") == "ckout main"

def test_returns_suffix_directly_when_not_echoed():
    assert clean_suggestion("ckout main", "git che") == "ckout main"

def test_takes_first_line_only():
    assert clean_suggestion("ckout main\nrm -rf /", "git che") == "ckout main"

def test_strips_code_fences():
    assert clean_suggestion("```\nckout main\n```", "git che") == "ckout main"

def test_empty_when_equals_prefix():
    assert clean_suggestion("git che", "git che") == ""

def test_empty_when_blank():
    assert clean_suggestion("   ", "git che") == ""
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 -m pytest tests/test_clean.py -v` (or `python3 -m unittest`)
Expected: FAIL — `ModuleNotFoundError: ailib.clean`

- [ ] **Step 3: Write minimal implementation**

```python
# ailib/clean.py
def clean_suggestion(raw: str, prefix: str) -> str:
    """Turn raw model output into the suffix that follows `prefix`."""
    if not raw or not raw.strip():
        return ""
    s = raw.strip()
    if s.startswith("```"):
        lines = s.splitlines()[1:]
        if lines and lines[-1].strip().startswith("```"):
            lines = lines[:-1]
        s = "\n".join(lines).strip()
    # first non-empty line only
    s = next((ln for ln in s.splitlines() if ln.strip()), "").strip()
    s = s.strip("`").strip()
    if not s:
        return ""
    suffix = s[len(prefix):] if s.startswith(prefix) else s
    if not suffix or suffix == prefix:
        return ""
    return suffix
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 -m pytest tests/test_clean.py -v`
Expected: PASS (6 passed)

- [ ] **Step 5: Commit**

```bash
git add ailib/__init__.py ailib/clean.py tests/test_clean.py
git commit -m "feat: add output cleaning module"
```

---

### Task 2: Prompt builder

**Files:**
- Create: `ailib/prompt.py`
- Test: `tests/test_prompt.py`

**Interfaces:**
- Consumes: nothing.
- Produces: `build_prompt(prefix: str, ctx: dict) -> str`. `ctx` keys (all optional): `cwd`, `os`, `shell`, `git_branch`, `git_status`, `dir_entries` (list[str]), `history` (list[str]).

- [ ] **Step 1: Write the failing test**

```python
# tests/test_prompt.py
from ailib.prompt import build_prompt

CTX = {
    "cwd": "/home/u/proj", "os": "Darwin", "shell": "zsh",
    "git_branch": "main", "git_status": "M file.py",
    "dir_entries": ["src/", "README.md"], "history": ["cd proj", "ls"],
}

def test_includes_prefix_and_context():
    p = build_prompt("git che", CTX)
    assert "git che" in p
    assert "/home/u/proj" in p
    assert "main" in p
    assert "README.md" in p
    assert "cd proj" in p

def test_handles_missing_context_keys():
    p = build_prompt("ls", {})          # must not raise
    assert "ls" in p

def test_instructs_suffix_only():
    p = build_prompt("ls", CTX).lower()
    assert "only" in p and "continue" in p
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 -m pytest tests/test_prompt.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Write minimal implementation**

```python
# ailib/prompt.py
SYSTEM = (
    "You are a zsh command-line autocompletion engine. "
    "Given a partial command and shell context, output ONLY the text that "
    "should continue the partial command. Do not repeat what the user typed. "
    "No explanation, no markdown, no code fences. Output a single line."
)

def build_prompt(prefix: str, ctx: dict) -> str:
    lines = [SYSTEM, ""]
    if ctx.get("cwd"):    lines.append(f"cwd: {ctx['cwd']}")
    if ctx.get("os"):     lines.append(f"os: {ctx['os']}  shell: {ctx.get('shell','zsh')}")
    if ctx.get("git_branch"):
        lines.append(f"git branch: {ctx['git_branch']}")
    if ctx.get("git_status"):
        lines.append(f"git status:\n{ctx['git_status']}")
    if ctx.get("dir_entries"):
        lines.append("files: " + ", ".join(ctx["dir_entries"]))
    if ctx.get("history"):
        lines.append("recent commands:\n" + "\n".join(ctx["history"]))
    lines += ["", f"partial command: {prefix}", "completion:"]
    return "\n".join(lines)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 -m pytest tests/test_prompt.py -v`
Expected: PASS (3 passed)

- [ ] **Step 5: Commit**

```bash
git add ailib/prompt.py tests/test_prompt.py
git commit -m "feat: add prompt builder"
```

---

### Task 3: Ollama client

**Files:**
- Create: `ailib/ollama.py`
- Test: `tests/test_ollama.py`

**Interfaces:**
- Produces:
  - `parse_response(body: str) -> str` — extract `.response` from an Ollama JSON body; `""` on any problem.
  - `query_ollama(prompt, *, url, model, max_tokens, keep_alive, timeout) -> str` — POST to `{url}/api/generate`, return raw response text; raises on network/HTTP error (caller catches).

- [ ] **Step 1: Write the failing test**

```python
# tests/test_ollama.py
from unittest import mock
from ailib.ollama import parse_response, query_ollama

def test_parse_response_extracts_field():
    assert parse_response('{"response": "ckout main"}') == "ckout main"

def test_parse_response_empty_on_bad_json():
    assert parse_response("not json") == ""
    assert parse_response('{"x": 1}') == ""

def test_query_ollama_builds_request_and_parses():
    fake = mock.MagicMock()
    fake.read.return_value = b'{"response": "ckout main"}'
    fake.__enter__ = lambda s: fake
    fake.__exit__ = lambda s, *a: False
    with mock.patch("ailib.ollama.urllib.request.urlopen", return_value=fake) as uo:
        out = query_ollama("p", url="http://x:1", model="m",
                           max_tokens=64, keep_alive="30m", timeout=5)
    assert out == "ckout main"
    req = uo.call_args[0][0]
    assert req.full_url == "http://x:1/api/generate"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 -m pytest tests/test_ollama.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Write minimal implementation**

```python
# ailib/ollama.py
import json
import urllib.request

def parse_response(body: str) -> str:
    try:
        return str(json.loads(body).get("response", "") or "")
    except Exception:
        return ""

def query_ollama(prompt, *, url, model, max_tokens, keep_alive, timeout):
    payload = json.dumps({
        "model": model,
        "prompt": prompt,
        "stream": False,
        "keep_alive": keep_alive,
        "options": {"temperature": 0.2, "num_predict": max_tokens, "stop": ["\n"]},
    }).encode()
    req = urllib.request.Request(
        f"{url}/api/generate", data=payload,
        headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return parse_response(resp.read().decode())
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 -m pytest tests/test_ollama.py -v`
Expected: PASS (3 passed)

- [ ] **Step 5: Commit**

```bash
git add ailib/ollama.py tests/test_ollama.py
git commit -m "feat: add ollama client"
```

---

### Task 4: Context gathering

**Files:**
- Create: `ailib/context.py`
- Test: `tests/test_context.py`

**Interfaces:**
- Produces: `gather_context(*, history_lines: int, max_dir_entries: int, history: list[str] | None) -> dict` — returns the `ctx` dict consumed by `build_prompt`. All gathering is best-effort; never raises.

- [ ] **Step 1: Write the failing test** (only the pure, deterministic parts)

```python
# tests/test_context.py
from ailib.context import _truncate, gather_context

def test_truncate_caps_list():
    assert _truncate([1, 2, 3, 4], 2) == [1, 2]

def test_gather_context_never_raises_and_has_keys():
    ctx = gather_context(history_lines=5, max_dir_entries=10, history=["ls"])
    assert isinstance(ctx, dict)
    assert ctx["history"] == ["ls"]
    assert "cwd" in ctx and "os" in ctx
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 -m pytest tests/test_context.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Write minimal implementation**

```python
# ailib/context.py
import os
import platform
import subprocess

def _truncate(items, n):
    return list(items)[:n]

def _run(cmd, timeout=0.5):
    try:
        out = subprocess.run(cmd, capture_output=True, text=True,
                             timeout=timeout)
        return out.stdout.strip()
    except Exception:
        return ""

def gather_context(*, history_lines, max_dir_entries, history=None):
    cwd = os.getcwd()
    ctx = {
        "cwd": cwd,
        "os": platform.system(),
        "shell": os.environ.get("SHELL", "zsh"),
    }
    branch = _run(["git", "branch", "--show-current"])
    if branch:
        ctx["git_branch"] = branch
        status = _run(["git", "status", "--porcelain"])
        if status:
            ctx["git_status"] = "\n".join(_truncate(status.splitlines(), 10))
    try:
        entries = sorted(os.listdir(cwd))
        ctx["dir_entries"] = _truncate(
            [e + "/" if os.path.isdir(os.path.join(cwd, e)) else e
             for e in entries if not e.startswith(".")],
            max_dir_entries)
    except Exception:
        pass
    if history:
        ctx["history"] = _truncate(history, history_lines)
    return ctx
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 -m pytest tests/test_context.py -v`
Expected: PASS (2 passed)

- [ ] **Step 5: Commit**

```bash
git add ailib/context.py tests/test_context.py
git commit -m "feat: add context gathering"
```

---

### Task 5: Entry point `ai_suggest.py`

**Files:**
- Create: `ai_suggest.py`
- Test: manual (integration against real Ollama).

**Interfaces:**
- Consumes: all four `ailib` modules.
- CLI: `python3 ai_suggest.py -- "<partial command>"`. Reads config from env (`AI_AC_*`). Prints suffix (no newline). History passed via `AI_AC_HISTORY` env (newline-separated), optional.

- [ ] **Step 1: Write the implementation**

```python
#!/usr/bin/env python3
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ailib.context import gather_context
from ailib.prompt import build_prompt
from ailib.ollama import query_ollama
from ailib.clean import clean_suggestion

def _env(name, default):
    return os.environ.get(name, default)

def _log(msg):
    if _env("AI_AC_DEBUG", "0") == "1":
        try:
            with open("/tmp/ai-ac.log", "a") as f:
                f.write(msg + "\n")
        except Exception:
            pass

def main():
    args = sys.argv[1:]
    if args and args[0] == "--":
        args = args[1:]
    prefix = args[0] if args else ""
    if len(prefix) < int(_env("AI_AC_MIN_CHARS", "3")):
        return
    hist = _env("AI_AC_HISTORY", "")
    history = [h for h in hist.splitlines() if h.strip()] or None
    ctx = gather_context(history_lines=int(_env("AI_AC_HISTORY_LINES", "30")),
                         max_dir_entries=50, history=history)
    prompt = build_prompt(prefix, ctx)
    try:
        raw = query_ollama(
            prompt,
            url=_env("AI_AC_OLLAMA_URL", "http://localhost:11434"),
            model=_env("AI_AC_MODEL", "qwen2.5-coder:3b"),
            max_tokens=int(_env("AI_AC_MAX_TOKENS", "64")),
            keep_alive=_env("AI_AC_KEEP_ALIVE", "30m"),
            timeout=float(_env("AI_AC_TIMEOUT", "5")))
    except Exception as e:
        _log(f"query failed: {e}")
        return
    suffix = clean_suggestion(raw, prefix)
    _log(f"prefix={prefix!r} suffix={suffix!r}")
    sys.stdout.write(suffix)

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run the full unit suite**

Run: `python3 -m pytest tests/ -v`
Expected: PASS (all tests)

- [ ] **Step 3: Manual integration test** (requires Ollama running + model pulled)

Run: `python3 ai_suggest.py -- "git che"`
Expected: prints something like `ckout main` (no newline). If Ollama is down, prints nothing and exits 0.

- [ ] **Step 4: Commit**

```bash
git add ai_suggest.py
git commit -m "feat: add ai_suggest entry point"
```

---

### Task 6: zsh plugin — strategy + config (no debounce yet)

**Files:**
- Create: `ai-autocomplete.plugin.zsh`
- Test: manual.

**Interfaces:**
- Consumes: `ai_suggest.py`.
- Produces: `_zsh_autosuggest_strategy_ai()` (zsh-autosuggestions strategy slot); sets `ZSH_AUTOSUGGEST_STRATEGY=(ai)` and `ZSH_AUTOSUGGEST_USE_ASYNC=1`.

- [ ] **Step 1: Write the plugin (strategy only, no debounce)**

```zsh
# ai-autocomplete.plugin.zsh
typeset -g _AI_AC_DIR="${0:A:h}"

# --- config defaults (override before sourcing) ---
: ${AI_AC_MODEL:=qwen2.5-coder:3b}
: ${AI_AC_OLLAMA_URL:=http://localhost:11434}
: ${AI_AC_DEBOUNCE:=0.2}
: ${AI_AC_MIN_CHARS:=3}
: ${AI_AC_MAX_TOKENS:=64}
: ${AI_AC_KEEP_ALIVE:=30m}
: ${AI_AC_HISTORY_LINES:=30}
: ${AI_AC_ENABLED:=1}
export AI_AC_MODEL AI_AC_OLLAMA_URL AI_AC_MIN_CHARS AI_AC_MAX_TOKENS \
       AI_AC_KEEP_ALIVE AI_AC_HISTORY_LINES

# --- wire into zsh-autosuggestions ---
ZSH_AUTOSUGGEST_STRATEGY=(ai)
ZSH_AUTOSUGGEST_USE_ASYNC=1

_zsh_autosuggest_strategy_ai() {
  emulate -L zsh
  [[ "$AI_AC_ENABLED" == "1" ]] || return
  local prefix="$1"
  [[ ${#prefix} -ge $AI_AC_MIN_CHARS ]] || return
  local hist; hist="$(fc -ln -$AI_AC_HISTORY_LINES 2>/dev/null)"
  local suffix
  suffix="$(AI_AC_HISTORY=$hist python3 "$_AI_AC_DIR/ai_suggest.py" -- "$prefix" 2>/dev/null)"
  [[ -n $suffix ]] && typeset -g suggestion="${prefix}${suffix}"
}
```

- [ ] **Step 2: Manual test** (zsh-autosuggestions + Ollama installed)

```bash
# In a zsh shell with zsh-autosuggestions loaded:
source ./ai-autocomplete.plugin.zsh
# type "git che" and pause — greyed ghost text should appear; press -> to accept.
```
Expected: ghost text appears after a brief pause. (It fires per keystroke here — debounce comes next.)

- [ ] **Step 3: Commit**

```bash
git add ai-autocomplete.plugin.zsh
git commit -m "feat: add zsh plugin with ai strategy"
```

---

### Task 7: Debounce (spike, then implement)

**Files:**
- Modify: `ai-autocomplete.plugin.zsh`
- Test: manual + spike.

**Approach (content-comparison debounce, robust to event ordering):**
On each buffer change, the parent writes the current `$BUFFER` to a temp file. The
async strategy sleeps the debounce interval, then re-reads the file; if its
content differs from the strategy's `prefix`, the user kept typing → return
without calling Ollama. Comparing *content* (not timestamps) avoids false aborts
from redraw events.

- [ ] **Step 1: Spike — confirm the mechanism in a throwaway script**

Create `/tmp/spike.zsh`:
```zsh
typeset -g _AI_AC_BUF_FILE="${TMPDIR:-/tmp}/ai-ac-buf.$$"
_ai_ac_write_buf() {
  if [[ "$BUFFER" != "$_AI_AC_LAST_BUF" ]]; then
    _AI_AC_LAST_BUF="$BUFFER"
    print -rn -- "$BUFFER" > "$_AI_AC_BUF_FILE"
  fi
}
autoload -Uz add-zle-hook-widget
add-zle-hook-widget line-pre-redraw _ai_ac_write_buf
```
Source it, type a few chars, and `cat $_AI_AC_BUF_FILE` from another pane (or add a debug log) to confirm the file tracks the buffer and only updates on content change.

- [ ] **Step 2: Implement debounce in the plugin**

Add near the top (after config):
```zsh
typeset -g _AI_AC_BUF_FILE="${TMPDIR:-/tmp}/ai-ac-buf.$$"
typeset -g _AI_AC_LAST_BUF=""

_ai_ac_write_buf() {
  if [[ "$BUFFER" != "$_AI_AC_LAST_BUF" ]]; then
    _AI_AC_LAST_BUF="$BUFFER"
    print -rn -- "$BUFFER" > "$_AI_AC_BUF_FILE" 2>/dev/null
  fi
}
autoload -Uz add-zle-hook-widget
add-zle-hook-widget line-pre-redraw _ai_ac_write_buf
```

Then add the debounce gate inside `_zsh_autosuggest_strategy_ai`, right after the
min-chars check:
```zsh
  # debounce: wait, then bail if the buffer changed while we slept
  sleep "$AI_AC_DEBOUNCE"
  local current; current="$(cat "$_AI_AC_BUF_FILE" 2>/dev/null)"
  [[ "$current" == "$prefix" ]] || return
```

- [ ] **Step 3: Manual test**

Type quickly: no flood of model calls (check `AI_AC_DEBUG=1` log — one query when you pause). Type and pause: suggestion appears.
Expected: one Ollama call per pause, not per keystroke.

- [ ] **Step 4: Commit**

```bash
git add ai-autocomplete.plugin.zsh
git commit -m "feat: add parent-side debounce"
```

---

### Task 8: install.sh + README + toggle keybind

**Files:**
- Create: `install.sh`
- Modify: `ai-autocomplete.plugin.zsh` (toggle widget + keybind)
- Modify: `README.md`

- [ ] **Step 1: Add toggle widget + keybind to the plugin**

```zsh
# --- toggle on/off (Ctrl-G) ---
_ai_ac_toggle() {
  if [[ "$AI_AC_ENABLED" == "1" ]]; then AI_AC_ENABLED=0
  else AI_AC_ENABLED=1; fi
  zle -M "AI autocomplete: $AI_AC_ENABLED"
}
zle -N _ai_ac_toggle
bindkey '^G' _ai_ac_toggle
```

- [ ] **Step 2: Write `install.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
MODEL="${AI_AC_MODEL:-qwen2.5-coder:3b}"

command -v ollama >/dev/null || { echo "Install Ollama first: https://ollama.com"; exit 1; }
command -v python3 >/dev/null || { echo "python3 required"; exit 1; }
ollama list | grep -q "${MODEL%%:*}" || { echo "Pulling $MODEL..."; ollama pull "$MODEL"; }

if [ ! -d "${ZSH_CUSTOM:-$HOME/.oh-my-zsh/custom}/plugins/zsh-autosuggestions" ] \
   && ! grep -rq zsh-autosuggestions "$HOME/.zshrc" 2>/dev/null; then
  echo "WARNING: zsh-autosuggestions not detected. Install it:"
  echo "  https://github.com/zsh-users/zsh-autosuggestions"
fi

LINE="source \"$DIR/ai-autocomplete.plugin.zsh\""
grep -qF "$LINE" "$HOME/.zshrc" 2>/dev/null || {
  echo "$LINE" >> "$HOME/.zshrc"
  echo "Added source line to ~/.zshrc"
}
echo "Done. Restart zsh or run: source ~/.zshrc"
```

Run: `chmod +x install.sh`

- [ ] **Step 3: Write README** (replace the stub)

Cover: what it is, dependencies (Ollama, model, zsh-autosuggestions, python3), `./install.sh`, the source line, config env vars table, keybinds (→/End accept, Ctrl-→ word, Ctrl-G toggle), how to bind Tab to accept, troubleshooting (`AI_AC_DEBUG=1` → `/tmp/ai-ac.log`).

- [ ] **Step 4: Run full test suite + commit**

```bash
python3 -m pytest tests/ -v   # all pass
chmod +x install.sh
git add install.sh README.md ai-autocomplete.plugin.zsh
git commit -m "feat: add installer, toggle keybind, and docs"
```

---

## Self-Review

- **Spec coverage:** strategy substrate (T6), debounce (T7), helper context/prompt/ollama/clean (T1–T4), entry point + config (T5), errors=silent (T5 try/except + `2>/dev/null`), keep_alive (T3/T5), accept keys + toggle + install + docs (T8). All spec sections mapped.
- **Placeholders:** none — every code step has real code. README content (T8.3) is described by required sections, not code, which is acceptable for a docs step.
- **Type consistency:** `build_prompt(prefix, ctx)`, `clean_suggestion(raw, prefix)`, `query_ollama(prompt, *, url, model, max_tokens, keep_alive, timeout)`, `gather_context(*, history_lines, max_dir_entries, history)` — used consistently across T2/T3/T4/T5. `ctx` keys produced by T4 match those consumed by T2.
