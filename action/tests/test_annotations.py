import unittest

from sensez_action.annotations import _escape_message, _escape_property


class AnnotationTests(unittest.TestCase):
    def test_escapes_command_property_values(self) -> None:
        self.assertEqual(
            _escape_property("src/a,b.py:10%\n"),
            "src/a%2Cb.py%3A10%25%0A",
        )

    def test_escapes_command_message_values(self) -> None:
        self.assertEqual(_escape_message("100%\nready"), "100%25%0Aready")
