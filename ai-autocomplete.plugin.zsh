# ai-autocomplete.plugin.zsh
# Cursor-style inline AI autocompletion for zsh — an invisible layer over the
# shell. Self-contained: it loads zsh-autosuggestions itself if needed, so a
# single `source` line (in any order) is all that's required.

typeset -g _AI_AC_DIR="${0:A:h}"

# --- config defaults (override before sourcing) ---
# Default to qwen2.5-coder:1.5b — fast, and excellent via FIM completion.
# Bump to :3b or :7b for higher-quality suggestions at the cost of latency.
: ${AI_AC_MODEL:=qwen2.5-coder:1.5b}
: ${AI_AC_OLLAMA_URL:=http://localhost:11434}
: ${AI_AC_DEBOUNCE:=0.2}
: ${AI_AC_MIN_CHARS:=3}
: ${AI_AC_MAX_TOKENS:=64}
: ${AI_AC_KEEP_ALIVE:=30m}
: ${AI_AC_HISTORY_LINES:=30}
: ${AI_AC_TIMEOUT:=5}
: ${AI_AC_DEBUG:=0}
: ${AI_AC_ENABLED:=1}
: ${AI_AC_TOGGLE_KEY:=}        # empty = no keybind (stay invisible); e.g. '^G'
export AI_AC_MODEL AI_AC_OLLAMA_URL AI_AC_MIN_CHARS AI_AC_MAX_TOKENS \
       AI_AC_KEEP_ALIVE AI_AC_HISTORY_LINES AI_AC_TIMEOUT AI_AC_DEBUG

# --- appearance: subtle, native-looking ghost text (respect user override) ---
: ${ZSH_AUTOSUGGEST_HIGHLIGHT_STYLE:=fg=8}

# --- self-contained: load zsh-autosuggestions if it isn't already ------------
if (( ! ${+functions[_zsh_autosuggest_fetch_suggestion]} )); then
  for _za in \
    /opt/homebrew/share/zsh-autosuggestions/zsh-autosuggestions.zsh \
    /usr/local/share/zsh-autosuggestions/zsh-autosuggestions.zsh \
    /usr/share/zsh-autosuggestions/zsh-autosuggestions.zsh \
    "${ZSH_CUSTOM:-$HOME/.oh-my-zsh/custom}/plugins/zsh-autosuggestions/zsh-autosuggestions.zsh"; do
    [[ -r "$_za" ]] && { source "$_za"; break; }
  done
  unset _za
fi

# --- wire into zsh-autosuggestions ---
ZSH_AUTOSUGGEST_STRATEGY=(ai)
ZSH_AUTOSUGGEST_USE_ASYNC=1

# --- debounce: track the live buffer in a temp file -------------------------
# The async strategy runs in a forked worker that cannot see new keystrokes, so
# we compare its captured prefix against the latest buffer (written here) after
# sleeping. Comparing content (not timestamps) avoids false aborts from redraws.
typeset -g _AI_AC_BUF_FILE="${TMPDIR:-/tmp}/ai-ac-buf.$$"
typeset -g _AI_AC_LAST_BUF=""

_ai_ac_write_buf() {
  [[ "$AI_AC_ENABLED" == "1" ]] || return
  if [[ "$BUFFER" != "$_AI_AC_LAST_BUF" ]]; then
    _AI_AC_LAST_BUF="$BUFFER"
    print -rn -- "$BUFFER" > "$_AI_AC_BUF_FILE" 2>/dev/null
  fi
}
autoload -Uz add-zle-hook-widget
add-zle-hook-widget line-pre-redraw _ai_ac_write_buf

# --- the suggestion strategy ------------------------------------------------
_zsh_autosuggest_strategy_ai() {
  emulate -L zsh
  [[ "$AI_AC_ENABLED" == "1" ]] || return
  local prefix="$1"
  [[ ${#prefix} -ge $AI_AC_MIN_CHARS ]] || return

  # debounce: wait, then bail if the buffer changed while we slept
  sleep "$AI_AC_DEBOUNCE"
  local current; current="$(cat "$_AI_AC_BUF_FILE" 2>/dev/null)"
  [[ "$current" == "$prefix" ]] || return

  local hist; hist="$(fc -ln -$AI_AC_HISTORY_LINES 2>/dev/null)"
  local suffix
  suffix="$(AI_AC_HISTORY=$hist python3 "$_AI_AC_DIR/ai_suggest.py" -- "$prefix" 2>/dev/null)"
  [[ -n $suffix ]] && typeset -g suggestion="${prefix}${suffix}"
}

# --- toggle on/off (silent; keybind only if AI_AC_TOGGLE_KEY is set) --------
_ai_ac_toggle() {
  [[ "$AI_AC_ENABLED" == "1" ]] && AI_AC_ENABLED=0 || AI_AC_ENABLED=1
}
zle -N _ai_ac_toggle
[[ -n "$AI_AC_TOGGLE_KEY" ]] && bindkey "$AI_AC_TOGGLE_KEY" _ai_ac_toggle
