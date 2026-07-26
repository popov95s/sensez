import unittest
from pathlib import Path

from sensez_action.config import (
    ActionEnvironment,
    AnnotationLevel,
    Config,
    ConfigError,
    FailureLevel,
)


class ConfigTests(unittest.TestCase):
    def test_parses_typed_levels_and_resolves_relative_path(self) -> None:
        env = ActionEnvironment(
            {
                "GITHUB_WORKSPACE": "/repo",
                "INPUT_PATH": "src",
                "INPUT_LEVEL": "error",
                "INPUT_FAIL_ON_NEW": "must-fix",
            }
        )

        config = Config.from_env(env)

        self.assertEqual(config.path, Path("/repo/src"))
        self.assertIs(config.level, AnnotationLevel.ERROR)
        self.assertIs(config.fail_on_new, FailureLevel.MUST_FIX)

    def test_rejects_unknown_annotation_level(self) -> None:
        with self.assertRaises(ConfigError):
            Config.from_env(ActionEnvironment({"INPUT_LEVEL": "urgent"}))

    def test_defaults_to_warning_annotations_without_failure(self) -> None:
        config = Config.from_env(ActionEnvironment({}))

        self.assertIs(config.level, AnnotationLevel.WARNING)
        self.assertIs(config.fail_on_new, FailureLevel.DISABLED)
