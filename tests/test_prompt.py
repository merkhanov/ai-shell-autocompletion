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

    def test_uses_fim_format(self):
        p = build_prompt("ls", CTX)
        self.assertTrue(p.startswith("<|fim_prefix|>"))
        self.assertTrue(p.endswith("<|fim_middle|>"))
        self.assertIn("<|fim_suffix|>", p)

    def test_partial_command_sits_just_before_suffix_marker(self):
        # the model must complete the command, so it ends the prefix region
        p = build_prompt("git che", CTX)
        self.assertIn("git che<|fim_suffix|>", p)

    def test_handles_missing_context_keys(self):
        p = build_prompt("ls", {})  # must not raise
        self.assertIn("ls", p)


if __name__ == "__main__":
    unittest.main()
