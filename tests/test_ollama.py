import unittest
from unittest import mock

from ailib.ollama import parse_response, query_ollama


class TestParseResponse(unittest.TestCase):
    def test_extracts_field(self):
        self.assertEqual(parse_response('{"response": "ckout main"}'), "ckout main")

    def test_empty_on_bad_json(self):
        self.assertEqual(parse_response("not json"), "")

    def test_empty_on_missing_field(self):
        self.assertEqual(parse_response('{"x": 1}'), "")


class TestQueryOllama(unittest.TestCase):
    def test_builds_request_and_parses(self):
        fake = mock.MagicMock()
        fake.read.return_value = b'{"response": "ckout main"}'
        fake.__enter__ = lambda s: fake
        fake.__exit__ = lambda s, *a: False
        with mock.patch("ailib.ollama.urllib.request.urlopen", return_value=fake) as uo:
            out = query_ollama(
                "p", url="http://x:1", model="m",
                max_tokens=64, keep_alive="30m", timeout=5,
            )
        self.assertEqual(out, "ckout main")
        req = uo.call_args[0][0]
        self.assertEqual(req.full_url, "http://x:1/api/generate")


if __name__ == "__main__":
    unittest.main()
