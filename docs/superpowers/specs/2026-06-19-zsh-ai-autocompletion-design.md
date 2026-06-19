# zsh AI Shell Autocompletion — Design

**Date:** 2026-06-19
**Status:** Approved

## Goal

A Cursor-style inline AI autocompletion for zsh: as you type a command, a local
LLM predicts the rest and shows it as greyed-out "ghost text." You accept with a
keystroke. Everything runs locally via Ollama; nothing leaves the machine.

## Core decisions

| Decision | Choice |
|---|---|
| Interaction model | Inline ghost text (Cursor-style) |
| AI backend | Local model via Ollama |
| Trigger | Debounced as you type (async) |
| Context | Rich (cwd, git, history, dir, OS) with hard caps |
| Architecture | zsh front-end + per-request Python helper |
| Ghost-text substrate | Build on `zsh-autosuggestions` custom strategy |

## Architecture

```
zsh-autosuggestions (dependency)
  - ghost-text render (POSTDISPLAY) + greying
  - accept widgets: -> / End (full), Ctrl-> (one word)
  - async forked-worker plumbing + stale-response discarding
        ^ registers strategy "ai"
our plugin: ai-autocomplete.plugin.zsh
  - _zsh_autosuggest_strategy_ai()  (strategy slot)
  - parent-side DEBOUNCE (the one custom mechanism)
  - config + on/off toggle keybind
        | spawns per request
Python helper: ai_suggest.py
  gather context -> build prompt -> call Ollama -> clean output
```

We build on `zsh-autosuggestions` rather than reimplementing the line-editor
internals. Those internals (fd lifecycle, killing/cleaning forked children,
repeated-callback leaks, POSTDISPLAY rendering, accept widgets) are the real
from-scratch risk; reusing a ubiquitous, well-maintained plugin removes it.

## zsh front-end

### Strategy hook
```zsh
ZSH_AUTOSUGGEST_STRATEGY=(ai)
ZSH_AUTOSUGGEST_USE_ASYNC=1

_zsh_autosuggest_strategy_ai() {
  local prefix="$1"                  # current BUFFER
  # ...debounce gate...
  local suffix="$(python3 .../ai_suggest.py -- "$prefix")"
  [[ -n $suffix ]] && typeset -g suggestion="${prefix}${suffix}"
}
```

**Gotcha:** zsh-autosuggestions expects `suggestion` to be the *full* line
(prefix included) and computes the greyed part as `${suggestion#$BUFFER}`
itself, even though our helper emits only the suffix. The strategy therefore
concatenates `${prefix}${suffix}`.

### Debounce (the only genuinely custom piece)
zsh-autosuggestions fires its strategy on every keystroke (history lookups are
instant, so it never needed throttling). For an LLM we must throttle or we'd
kick off a discarded inference on every character.

Approach: a line-edit hook stamps an "edit token" (`$EPOCHREALTIME`) on each
change. The async strategy captures the token, sleeps the debounce interval
(~200ms), re-reads it; if it changed, the user is still typing -> return empty
(no Ollama call); if unchanged -> fire the real suggestion.

**This mechanism is validated with a throwaway spike as the first implementation
step.** Everything else rides on proven zsh-autosuggestions machinery.

### Accept / dismiss
Inherited from zsh-autosuggestions: **-> or End** accepts the full suggestion;
**Ctrl->** accepts one word. Tab stays as normal completion (README documents how
to bind Tab to accept for the literal Cursor feel). Any edit dismisses + refetches.

## Python helper

- **Input:** partial command as an argument; cwd inherited from the shell.
- **Context gathering** (all with hard caps to keep the prompt small): cwd,
  OS/shell, git branch + short status, top dir entries, recent history (zsh
  passes accurate session history via env; falls back to reading `$HISTFILE`).
- **Prompt:** system instruction = "you are a zsh completion engine; output ONLY
  the text that continues the partial command; no prose/markdown/fences."
- **Ollama call:** `POST {url}/api/generate`, `stream:false`,
  `temperature:0.2`, `num_predict:~64`, `stop:["\n"]`, `keep_alive:"30m"` (keeps
  the model warm; a cold reload after idle is the "feels broken after lunch"
  trap). Short timeout.
- **Output cleaning:** strip fences/whitespace, take first line, strip the prefix
  if the model echoes it, return empty if the result equals the buffer.

### Rich-context vs. latency
A big prompt on every debounced call slows a small (1.5b/3b) model and can
degrade its output. Hard caps on every context source keep this a known dial,
not a surprise.

## Config (env vars, override in `.zshrc`)

| Var | Default | Meaning |
|---|---|---|
| `AI_AC_MODEL` | `qwen2.5-coder:3b` | Ollama model |
| `AI_AC_OLLAMA_URL` | `http://localhost:11434` | Ollama endpoint |
| `AI_AC_DEBOUNCE` | `0.2` | Debounce seconds |
| `AI_AC_MIN_CHARS` | `3` | Don't fire below this buffer length |
| `AI_AC_MAX_TOKENS` | `64` | `num_predict` cap |
| `AI_AC_KEEP_ALIVE` | `30m` | Ollama keep_alive |
| `AI_AC_DEBUG` | `0` | `1` -> log to `/tmp/ai-ac.log` |

Plus a keybind to toggle the feature on/off.

## Error handling

Any failure (Ollama down, timeout, bad JSON, missing python) -> silently no
suggestion. **Never break or block the prompt.** All work happens in the async
child.

## Testing

- Python logic is unit-tested: prompt building, output cleaning (prefix-strip,
  fence removal), Ollama response parsing (mocked HTTP). No pip deps — stdlib
  only, so tests run anywhere.
- zsh side: the debounce spike + a manual test checklist.

## File layout

```
ai-autocomplete.plugin.zsh     # strategy + debounce + config
ai_suggest.py                  # thin entry point
ailib/  context.py  prompt.py  ollama.py  clean.py
tests/  test_prompt.py  test_clean.py  test_ollama.py
install.sh   README.md
```

## Setup / dependencies

- Ollama installed + `ollama pull qwen2.5-coder:3b`
- `zsh-autosuggestions` installed
- Python 3 (stdlib only)
- `install.sh` checks these and adds the source line to `.zshrc`.

## Out of scope (v1, YAGNI)

- Long-running daemon (per-request helper is enough; Ollama keeps the model warm).
- Cloud/pluggable backends.
- NL->command hotkey mode.
- Suggestion caching layer.
