#!/usr/bin/env python3
import datetime
import importlib.util
import pathlib
import subprocess
import sys
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("release", pathlib.Path(__file__).with_name("publish-release.py"))
release = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = release
spec.loader.exec_module(release)

SHA = "a" * 40
DIGEST = "sha256:" + "b" * 64
IMAGE = "ghcr.io/owner/exposed"
TUPLE = release.Release("42", SHA, "2026.10.14.12.30", IMAGE, DIGEST)


class ReleaseTests(unittest.TestCase):
    def test_unrelated_json_annotation_is_ignored(self):
        refs = [{"object": {"type": "tag", "sha": SHA}}]
        with patch.object(release, "pages", return_value=refs), \
                patch.object(release, "api", return_value={"message": "[]"}):
            self.assertEqual(list(release.reservations("owner/exposed")), [])

    def test_retry_recovers_exact_tuple_without_creating_tag(self):
        with patch.object(release, "existing", return_value=TUPLE), patch.object(release, "api") as api:
            self.assertEqual(release.reserve("owner/exposed", "42", SHA, IMAGE, DIGEST), TUPLE)
            api.assert_not_called()

    def test_retry_rejects_different_digest(self):
        with patch.object(release, "existing", return_value=TUPLE), patch.object(release, "api") as api:
            with self.assertRaisesRegex(ValueError, "replace"):
                release.reserve("owner/exposed", "42", SHA, IMAGE, "sha256:" + "c" * 64)
            api.assert_not_called()

    def test_collision_waits_for_real_clock_then_reserves_next_minute(self):
        clock = datetime.datetime
        moments = [clock(2026, 10, 14, 12, 30, tzinfo=datetime.timezone.utc),
                   clock(2026, 10, 14, 12, 30, 59, tzinfo=datetime.timezone.utc),
                   clock(2026, 10, 14, 12, 31, tzinfo=datetime.timezone.utc),
                   clock(2026, 10, 14, 12, 31, tzinfo=datetime.timezone.utc)]
        conflict = subprocess.CalledProcessError(1, "gh", stderr="Reference already exists")
        responses = [{"sha": "annotation-1"}, conflict,
                     [{"ref": "refs/tags/2026.10.14.12.30"}], {"sha": "annotation-2"}, {}]
        with patch.object(release, "existing", return_value=None), patch.object(release, "api", side_effect=responses) as api, \
                patch.object(release.datetime, "datetime") as date, patch.object(release.time, "sleep") as sleep:
            date.now.side_effect = moments
            result = release.reserve("owner/exposed", "42", SHA, IMAGE, DIGEST)
        self.assertEqual(result.tag, "2026.10.14.12.31")
        self.assertEqual(api.call_args_list[-1].args[1]["ref"], "refs/tags/2026.10.14.12.31")
        sleep.assert_called_once_with(1)

    def test_api_failure_does_not_masquerade_as_collision(self):
        failure = subprocess.CalledProcessError(1, "gh", stderr="Forbidden")
        with patch.object(release, "existing", return_value=None), \
                patch.object(release, "api", side_effect=[{"sha": "annotation"}, failure, []]):
            with self.assertRaises(subprocess.CalledProcessError):
                release.reserve("owner/exposed", "42", SHA, IMAGE, DIGEST)

    def test_registry_tag_cannot_be_overwritten(self):
        with patch.object(release, "manifest_digest", side_effect=[DIGEST, "sha256:" + "c" * 64]), \
                patch.object(release, "command") as command:
            with self.assertRaisesRegex(ValueError, "overwrite"):
                release.publish("owner/exposed", TUPLE)
            command.assert_not_called()

    def test_published_retry_checks_asset_and_does_not_republish(self):
        body = (f"Source: https://github.com/owner/exposed/commit/{SHA}\n\n"
                f"Image: `{IMAGE}:{TUPLE.tag}`\n\nImmutable image: `{TUPLE.reference}`\n\n"
                f"```sh\ndocker run --rm --env DATABASE_URL -p 6999:6999 {TUPLE.reference}\n```\n\n"
                "Apply database migrations separately before starting the server.\n")
        state = [{"tag_name": TUPLE.tag, "body": body, "target_commitish": SHA,
                  "draft": False, "assets": [{"name": "image.txt"}]}]
        with patch.object(release, "manifest_digest", return_value=DIGEST), \
                patch.object(release, "pages", return_value=state), \
                patch.object(release, "command", return_value=TUPLE.reference) as command:
            release.publish("owner/exposed", TUPLE)
            self.assertEqual(command.call_count, 1)
            self.assertEqual(command.call_args.args[:3], ("gh", "release", "download"))

    def test_release_rejects_invalid_boundary_values(self):
        with self.assertRaises(ValueError):
            release.Release("42", SHA, TUPLE.tag, IMAGE, "latest")

    def test_interrupted_draft_upload_recovers_before_publication(self):
        body = (f"Source: https://github.com/owner/exposed/commit/{SHA}\n\n"
                f"Image: `{IMAGE}:{TUPLE.tag}`\n\nImmutable image: `{TUPLE.reference}`\n\n"
                f"```sh\ndocker run --rm --env DATABASE_URL -p 6999:6999 {TUPLE.reference}\n```\n\n"
                "Apply database migrations separately before starting the server.\n")
        state = [{"tag_name": TUPLE.tag, "body": body, "target_commitish": SHA,
                  "draft": True, "assets": []}]
        failure = subprocess.CalledProcessError(1, "gh", stderr="Upload interrupted")
        with patch.object(release, "manifest_digest", return_value=DIGEST), \
                patch.object(release, "pages", return_value=state), \
                patch.object(release, "command", side_effect=failure) as command:
            with self.assertRaises(subprocess.CalledProcessError):
                release.publish("owner/exposed", TUPLE)
            self.assertEqual(command.call_count, 1)
            self.assertEqual(command.call_args.args[:3], ("gh", "release", "upload"))
        with patch.object(release, "manifest_digest", return_value=DIGEST), \
                patch.object(release, "pages", return_value=state), \
                patch.object(release, "command") as command:
            release.publish("owner/exposed", TUPLE)
            self.assertEqual([call.args[:3] for call in command.call_args_list],
                             [("gh", "release", "upload"), ("gh", "release", "edit")])


if __name__ == "__main__":
    unittest.main()
