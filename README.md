# ai-shell-autocompletion

Cursor-style **inline AI autocompletion for zsh**. As you type a command, a
local [Ollama](https://ollama.com) model predicts the rest and shows it as
greyed-out ghost text. Press `→` to accept. Everything runs locally — nothing
leaves your machine.

It's built on top of
[`zsh-autosuggestions`](https://github.com/zsh-users/zsh-autosuggestions): we
register a custom suggestion *strategy* that calls a small Python helper, which
gathers context (cwd, git, history, directory, OS), prompts Ollama, and returns
the completion.

Completions use **fill-in-middle (FIM)**, the mode `qwen2.5-coder` is trained
for — it produces clean continuations instead of the echoes/markdown you get
from instruction-style prompts, so even the tiny 1.5b model works well.

## Requirements

- [Ollama](https://ollama.com) installed and running
- A `qwen2.5-coder` model pulled (default `qwen2.5-coder:1.5b`): `ollama pull qwen2.5-coder:1.5b`
  - The FIM prompt is `qwen2.5-coder`-specific; other models need a different prompt.
- [`zsh-autosuggestions`](https://github.com/zsh-users/zsh-autosuggestions) installed and sourced in your `.zshrc`
- Python 3 (standard library only — no pip packages)

## Install

```bash
git clone <this-repo> ~/ai-shell-autocompletion
cd ~/ai-shell-autocompletion
./install.sh        # checks deps, pulls the model, adds a source line to ~/.zshrc
exec zsh            # reload
```

That's the whole footprint — `install.sh` appends a single self-contained line
to your `~/.zshrc`:

```zsh
source "/path/to/ai-shell-autocompletion/ai-autocomplete.plugin.zsh"
```

The plugin loads `zsh-autosuggestions` itself if it isn't already, so order
doesn't matter and one line is enough. If you already source
`zsh-autosuggestions` elsewhere in your `~/.zshrc`, that line is now redundant
and can be removed.

## Usage

- Start typing a command and pause briefly — greyed ghost text appears.
- **`→` or `End`** — accept the whole suggestion.
- **`Ctrl-→`** — accept one word.

It runs as an invisible layer: no on-screen messages, and it binds no keys of
its own by default (so nothing you already use changes). To add an on/off
toggle, set a key — e.g. `AI_AC_TOGGLE_KEY='^G'` before the source line.

### Make `Tab` accept (literal Cursor feel)

`Tab` defaults to normal zsh completion. To make it accept the suggestion when
one is showing, add after the source line:

```zsh
bindkey '^I' autosuggest-accept
```

## Configuration

Set any of these before the source line in `~/.zshrc`:

| Variable | Default | Meaning |
|---|---|---|
| `AI_AC_MODEL` | `qwen2.5-coder:1.5b` | Ollama model to use |
| `AI_AC_OLLAMA_URL` | `http://localhost:11434` | Ollama endpoint |
| `AI_AC_DEBOUNCE` | `0.2` | Seconds to wait after you stop typing |
| `AI_AC_MIN_CHARS` | `3` | Minimum buffer length before suggesting |
| `AI_AC_MAX_TOKENS` | `64` | Max tokens the model may generate |
| `AI_AC_KEEP_ALIVE` | `30m` | How long Ollama keeps the model warm |
| `AI_AC_HISTORY_LINES` | `30` | Recent history lines sent as context |
| `AI_AC_TIMEOUT` | `5` | Request timeout (seconds) |
| `AI_AC_ENABLED` | `1` | Start enabled (`0` to start off) |
| `AI_AC_TOGGLE_KEY` | _(none)_ | Key to toggle on/off, e.g. `'^G'`. Empty = bind nothing |
| `AI_AC_DEBUG` | `0` | `1` → log to `/tmp/ai-ac.log` |

A larger model (e.g. `qwen2.5-coder:3b` or `:7b`) gives better suggestions at
the cost of latency. Tune `AI_AC_DEBOUNCE` up if your machine struggles, down
for snappier suggestions.

## How it works

```
zsh-autosuggestions  →  strategy "ai"  →  (debounce)  →  python3 ai_suggest.py
   renders ghost text                                      gather context
   accept / discard widgets                                build prompt
   async forked worker                                     POST Ollama /api/generate
                                                           clean → print suffix
```

The strategy returns the *full* line (`prefix + suffix`); zsh-autosuggestions
greys the part after what you typed. Debounce is content-based: the worker
sleeps, then only calls Ollama if the buffer hasn't changed — so rapid typing
doesn't flood the model.

The helper prompts Ollama in **fill-in-middle** mode: context is rendered as
shell comments, then the partial command, with an empty suffix — so the model
fills the gap (the rest of the command). This is why a 1.5b model gives clean
completions: it's continuing text, not answering a question.

## Troubleshooting

- **No suggestions appear.** Is Ollama running (`ollama list`)? Is the model
  pulled? Is `zsh-autosuggestions` loaded *before* this plugin? Set
  `AI_AC_DEBUG=1`, type a command, then check `/tmp/ai-ac.log`.
- **First suggestion after a break is slow.** Ollama unloaded the model; raise
  `AI_AC_KEEP_ALIVE`.
- **Too many model calls / high CPU.** Raise `AI_AC_DEBOUNCE` or `AI_AC_MIN_CHARS`.

## Development

Run the unit tests (stdlib only, no deps):

```bash
python3 -m unittest discover -s tests -v
```

The Python logic — prompt building, output cleaning, Ollama parsing, context —
lives in `ailib/` and is unit-tested. The zsh front-end is in
`ai-autocomplete.plugin.zsh`.
