import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

BACKENDS = ("server", "migrate", "cli")
VARIANTS = ("server", "app", "migrate", "cli")
DIGEST = re.compile(r"sha256:[0-9a-f]{64}")
SOURCE = re.compile(r"[0-9a-f]{40}")
VERSION = re.compile(r"[0-9]{4}(?:\.[0-9]{2}){4}")


def run(*args):
    return subprocess.check_output(args, stderr=subprocess.PIPE)


def digest(value):
    if not isinstance(value, str) or not DIGEST.fullmatch(value):
        raise ValueError("Invalid image digest")
    return value


def plan(repository, source, event):
    build = {"mode": "build", "backend": {}}
    if event != "push":
        return build
    try:
        release = json.loads(run("gh", "api", f"repos/{repository}/releases/latest"))
        tag = release["tag_name"]
        if not isinstance(tag, str) or not VERSION.fullmatch(tag):
            raise ValueError("Invalid release tag")
        with tempfile.TemporaryDirectory() as directory:
            run("gh", "release", "download", tag, "--repo", repository,
                "--pattern", "images.json", "--dir", directory)
            manifest = json.loads((Path(directory) / "images.json").read_text())
        baseline = manifest["source"]
        if (manifest["version"] != tag or not isinstance(baseline, str)
                or not SOURCE.fullmatch(baseline)):
            raise ValueError("Invalid release source or version")
        prefix = f"ghcr.io/{repository.lower()}@"
        images = manifest["images"]
        digests = {}
        for variant in VARIANTS:
            ref = images[variant]
            if not isinstance(ref, str) or not ref.startswith(prefix):
                raise ValueError("Invalid image repository")
            digests[variant] = digest(ref[len(prefix):])
        run("git", "merge-base", "--is-ancestor", baseline, source)
        paths = run("git", "diff", "--no-renames", "--name-only", "-z", baseline, source).split(b"\0")
        if any(path and not (path.startswith(b"frontend/") or path == b"docker/Dockerfile.frontend")
               for path in paths):
            return build
        return {"mode": "reuse", "source": baseline,
                "backend": {variant: digests[variant] for variant in BACKENDS}}
    except (subprocess.CalledProcessError, OSError, ValueError, KeyError, TypeError):
        return build


def assemble(mode, backend):
    if mode == "build":
        selected = {variant: os.environ[f"{variant.upper()}_DIGEST"] for variant in BACKENDS}
    elif mode == "reuse":
        selected = json.loads(backend)
        if set(selected) != set(BACKENDS):
            raise ValueError("Incomplete reused backend set")
    else:
        raise ValueError("Invalid build plan")
    return {variant: digest(os.environ["APP_DIGEST"] if variant == "app" else selected[variant])
            for variant in VARIANTS}


def main():
    parser = argparse.ArgumentParser(description="Plan backend reuse and assemble complete immutable release artifacts.")
    parser.add_argument("command", choices=("plan", "assemble", "validate"))
    args = parser.parse_args()
    if args.command == "plan":
        result = plan(os.environ["GITHUB_REPOSITORY"], os.environ["GITHUB_SHA"], os.environ["GITHUB_EVENT_NAME"])
        print(json.dumps(result))
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            output.write(f"build-backend={'true' if result['mode'] == 'build' else 'false'}\n")
            output.write(f"backend={json.dumps(result['backend'], separators=(',', ':'))}\n")
    elif args.command == "assemble":
        mode = {"true": "build", "false": "reuse"}[os.environ["BUILD_BACKEND"]]
        result = assemble(mode, os.environ["BACKEND"])
        Path("amd64.json").write_text(json.dumps(result) + "\n")
    else:
        result = json.loads(Path("digests/amd64.json").read_text())
        if set(result) != set(VARIANTS):
            raise ValueError("Incomplete release image set")
        for value in result.values():
            digest(value)


if __name__ == "__main__":
    main()
