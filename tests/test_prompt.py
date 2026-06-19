import unittest

from ailib.prompt import build_prompt

CTX = {
    "cwd": "/home/u/proj", "os": "Darwin", "shell": "zsh",
    "git_branch": "main", "git_status": "M file.py",
    "dir_entries": ["src/", "README.md"], "history": ["cd proj", "ls"],
}


class TestBuildPrompt(unittest.TestCase):
    def test_includes_prefix_and_context(self):
        p = build_prompt("git che", CTX)
        for needle in ("git che", "/home/u/proj", "main", "README.md", "cd proj"):
            self.assertIn(needle, p)

    def test_handles_missing_context_keys(self):
        p = build_prompt("ls", {})  # must not raise
        self.assertIn("ls", p)

    def test_instructs_suffix_only(self):
        p = build_prompt("ls", CTX).lower()
        self.assertIn("only", p)
        self.assertIn("continue", p)


if __name__ == "__main__":
    unittest.main()
