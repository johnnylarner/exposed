#!/usr/bin/env python3
"""Bind each Actions run to one immutable source, UTC minute, and image digest."""

import argparse
import dataclasses
import datetime
import json
import os
import pathlib
import re
import subprocess
import tempfile
import time


def command(*args, input=None):
    return subprocess.run(args, input=input, text=True, check=True, capture_output=True).stdout.strip()


def api(path, payload=None):
    args = ["gh", "api", path]
    if payload is not None:
        args += ["--method", "POST", "--input", "-"]
    return json.loads(command(*args, input=json.dumps(payload) if payload is not None else None))


def pages(path):
    return [item for page in json.loads(command("gh", "api", "--paginate", "--slurp", path)) for item in page]


@dataclasses.dataclass(frozen=True)
class Release:
    run_id: str
    sha: str
    tag: str
    image: str
    digest: str

    def __post_init__(self):
        if not re.fullmatch(r"[0-9]+", self.run_id):
            raise ValueError("Invalid Actions run ID")
        if not re.fullmatch(r"[0-9a-f]{40}", self.sha):
            raise ValueError("Invalid source SHA")
        datetime.datetime.strptime(self.tag, "%Y.%m.%d.%H.%M")
        if not re.fullmatch(r"sha256:[0-9a-f]{64}", self.digest):
            raise ValueError("Invalid image digest")
        if not re.fullmatch(r"ghcr.io/[a-z0-9_.-]+/[a-z0-9_./-]+", self.image):
            raise ValueError("Invalid GHCR image")

    @property
    def reference(self):
        return f"{self.image}@{self.digest}"


def reservations(repo):
    refs = pages(f"repos/{repo}/git/matching-refs/tags/20")
    for ref in refs:
        if ref["object"]["type"] != "tag":
            continue
        annotation = api(f"repos/{repo}/git/tags/{ref['object']['sha']}")
        try:
            value = json.loads(annotation["message"])
        except json.JSONDecodeError:
            continue
        if not isinstance(value, dict) or value.get("kind") != "exposed-release-v1":
            continue
        release = Release(**{key: value[key] for key in ("run_id", "sha", "tag", "image", "digest")})
        if annotation["object"]["type"] != "commit" or annotation["object"]["sha"] != release.sha:
            raise ValueError("Reservation points to a different commit")
        if ref["ref"] != f"refs/tags/{release.tag}":
            raise ValueError("Reservation tag does not match its annotation")
        yield release


def existing(repo, run_id, sha, image):
    matches = [release for release in reservations(repo) if release.run_id == run_id]
    if len(matches) > 1:
        raise ValueError("Actions run has multiple release reservations")
    if matches and (matches[0].sha != sha or matches[0].image != image):
        raise ValueError("Actions run reservation conflicts with source or image")
    return matches[0] if matches else None


def reserve(repo, run_id, sha, image, digest):
    while True:
        prior = existing(repo, run_id, sha, image)
        if prior:
            if prior.digest != digest:
                raise ValueError("Refusing to replace the reserved image digest")
            return prior
        tag = datetime.datetime.now(datetime.timezone.utc).strftime("%Y.%m.%d.%H.%M")
        release = Release(run_id, sha, tag, image, digest)
        annotation = api(f"repos/{repo}/git/tags", {
            "tag": tag,
            "message": json.dumps({"kind": "exposed-release-v1", **dataclasses.asdict(release)}, sort_keys=True),
            "object": sha,
            "type": "commit",
        })
        try:
            api(f"repos/{repo}/git/refs", {"ref": f"refs/tags/{tag}", "sha": annotation["sha"]})
            return release
        except subprocess.CalledProcessError:
            refs = api(f"repos/{repo}/git/matching-refs/tags/{tag}")
            if not any(ref["ref"] == f"refs/tags/{tag}" for ref in refs):
                raise
            prior = existing(repo, run_id, sha, image)
            if prior:
                if prior.digest != digest:
                    raise ValueError("Concurrent run reserved a different digest")
                return prior
            while datetime.datetime.now(datetime.timezone.utc).strftime("%Y.%m.%d.%H.%M") == tag:
                time.sleep(1)


def manifest_digest(reference):
    manifest = json.loads(command("docker", "buildx", "imagetools", "inspect", reference, "--format", "{{json .Manifest}}"))
    return manifest["digest"]


def publish(repo, release):
    reference = f"{release.image}:{release.tag}"
    if manifest_digest(release.reference) != release.digest:
        raise ValueError("Source manifest digest mismatch")
    try:
        current_digest = manifest_digest(reference)
    except subprocess.CalledProcessError as error:
        if not any(message in error.stderr.lower() for message in ("not found", "manifest unknown")):
            raise
        current_digest = None
    if current_digest is not None and current_digest != release.digest:
        raise ValueError("Refusing to overwrite an existing image tag")
    if current_digest is None:
        command("docker", "buildx", "imagetools", "create", "--prefer-index=false", "--tag", reference, release.reference)
    if manifest_digest(reference) != release.digest:
        raise ValueError("Published image digest mismatch")
    body = (f"Source: https://github.com/{repo}/commit/{release.sha}\n\n"
            f"Image: `{reference}`\n\nImmutable image: `{release.reference}`\n\n"
            f"```sh\ndocker run --rm --env DATABASE_URL -p 6999:6999 {release.reference}\n```\n\n"
            "Apply database migrations separately before starting the server.\n")
    releases = pages(f"repos/{repo}/releases?per_page=100")
    found = next((item for item in releases if item["tag_name"] == release.tag), None)
    if found and (found["body"] != body or found["target_commitish"] != release.sha):
        raise ValueError("Existing GitHub release conflicts with reservation")
    with tempfile.TemporaryDirectory() as directory:
        notes = pathlib.Path(directory, "notes.md")
        notes.write_text(body)
        asset = pathlib.Path(directory, "image.txt")
        asset.write_text(release.reference + "\n")
        if not found:
            command("gh", "release", "create", release.tag, "--repo", repo, "--verify-tag", "--target", release.sha,
                    "--title", release.tag, "--notes-file", str(notes), "--draft")
            found = json.loads(command("gh", "release", "view", release.tag, "--repo", repo, "--json", "isDraft,assets"))
        assets = found.get("assets", [])
        if any(item["name"] == "image.txt" for item in assets):
            actual = command("gh", "release", "download", release.tag, "--repo", repo, "--pattern", "image.txt", "--output", "-")
            if actual != release.reference:
                raise ValueError("Existing image.txt conflicts with reservation")
        else:
            command("gh", "release", "upload", release.tag, str(asset), "--repo", repo)
        if found.get("draft", found.get("isDraft", False)):
            command("gh", "release", "edit", release.tag, "--repo", repo, "--draft=false")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["prepare", "publish"])
    parser.add_argument("--digest")
    args = parser.parse_args()
    repo, run_id, sha = (os.environ[key] for key in ("GITHUB_REPOSITORY", "GITHUB_RUN_ID", "GITHUB_SHA"))
    image = f"ghcr.io/{repo.lower()}"
    if args.operation == "prepare":
        prior = existing(repo, run_id, sha, image)
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(f"image={image}\ndigest={prior.digest if prior else ''}\n")
    else:
        if not args.digest:
            parser.error("publish requires --digest")
        release = reserve(repo, run_id, sha, image, args.digest)
        publish(repo, release)
        print(f"Released {release.tag}: {release.reference}")


if __name__ == "__main__":
    main()
