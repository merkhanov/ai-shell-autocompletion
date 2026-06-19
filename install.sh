#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
MODEL="${AI_AC_MODEL:-qwen2.5-coder:1.5b}"

command -v ollama >/dev/null || { echo "Install Ollama first: https://ollama.com"; exit 1; }
command -v python3 >/dev/null || { echo "python3 required"; exit 1; }

if ! ollama list 2>/dev/null | grep -q "${MODEL%%:*}"; then
  echo "Pulling $MODEL ..."
  ollama pull "$MODEL"
fi

if [ ! -d "${ZSH_CUSTOM:-$HOME/.oh-my-zsh/custom}/plugins/zsh-autosuggestions" ] \
   && ! grep -rsq zsh-autosuggestions "$HOME/.zshrc"; then
  echo "WARNING: zsh-autosuggestions not detected. Install it first:"
  echo "  https://github.com/zsh-users/zsh-autosuggestions"
fi

LINE="source \"$DIR/ai-autocomplete.plugin.zsh\""
if grep -qF "$LINE" "$HOME/.zshrc" 2>/dev/null; then
  echo "Already sourced in ~/.zshrc"
else
  echo "$LINE" >> "$HOME/.zshrc"
  echo "Added source line to ~/.zshrc"
fi

echo "Done. Restart zsh or run: source ~/.zshrc"
