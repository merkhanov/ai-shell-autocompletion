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
