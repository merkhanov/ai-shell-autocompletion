import unittest

from ailib.context import _truncate, gather_context


class TestContext(unittest.TestCase):
    def test_truncate_caps_list(self):
        self.assertEqual(_truncate([1, 2, 3, 4], 2), [1, 2])

    def test_gather_context_never_raises_and_has_keys(self):
        ctx = gather_context(history_lines=5, max_dir_entries=10, history=["ls"])
        self.assertIsInstance(ctx, dict)
        self.assertEqual(ctx["history"], ["ls"])
        self.assertIn("cwd", ctx)
        self.assertIn("os", ctx)


if __name__ == "__main__":
    unittest.main()
