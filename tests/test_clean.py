import unittest

from ailib.clean import clean_suggestion


class TestCleanSuggestion(unittest.TestCase):
    def test_strips_prefix_when_model_echoes_full_command(self):
        self.assertEqual(clean_suggestion("git checkout main", "git che"), "ckout main")

    def test_returns_suffix_directly_when_not_echoed(self):
        self.assertEqual(clean_suggestion("ckout main", "git che"), "ckout main")

    def test_takes_first_line_only(self):
        self.assertEqual(clean_suggestion("ckout main\nrm -rf /", "git che"), "ckout main")

    def test_preserves_significant_leading_space(self):
        self.assertEqual(clean_suggestion(' "pattern" .', "grep -r"), ' "pattern" .')

    def test_rejects_code_fence(self):
        self.assertEqual(clean_suggestion("```bash", "git che"), "")

    def test_empty_when_equals_prefix(self):
        self.assertEqual(clean_suggestion("git che", "git che"), "")

    def test_empty_when_blank(self):
        self.assertEqual(clean_suggestion("   ", "git che"), "")


if __name__ == "__main__":
    unittest.main()
