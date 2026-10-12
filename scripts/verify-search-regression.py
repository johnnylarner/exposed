#!/usr/bin/env python3
"""Launch this worktree's API on an isolated port and retain HTTP evidence."""
import argparse
import errno
from pathlib import Path
import re
import socket
import subprocess
import sys
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence_dir", type=Path, help="New directory for server log and HTTP evidence")
    args = parser.parse_args()
    root = Path.cwd()
    args.evidence_dir.mkdir(parents=True, exist_ok=False)
    guard = socket.socket()
    child = None
    try:
        guard.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            guard.bind(("0.0.0.0", 6999))
            guard.listen()
        except OSError as error:
            if error.errno != errno.EADDRINUSE:
                raise
        log_path = args.evidence_dir / "server.log"
        with log_path.open("wb") as log:
            child = subprocess.Popen([str(root / "target/debug/exposed"), "server"],
                                     cwd=root, stdout=log, stderr=subprocess.STDOUT)
            deadline = time.monotonic() + 30
            while True:
                if child.poll() is not None:
                    raise RuntimeError(f"API exited with code {child.returncode}; see {log_path}")
                match = re.search(r"Listening on http://localhost:(\d+)",
                                  log_path.read_text(errors="replace"))
                if match:
                    port = int(match.group(1))
                    if port == 6999:
                        raise RuntimeError(f"API used shared port 6999; see {log_path}")
                    break
                if time.monotonic() >= deadline:
                    raise RuntimeError(f"API startup timed out; see {log_path}")
                time.sleep(0.05)
            guard.close()
            base_url = f"http://127.0.0.1:{port}"
            (args.evidence_dir / "server.txt").write_text(f"PID {child.pid}\nURL {base_url}\n")
            print(f"API PID {child.pid}: {base_url}", flush=True)
            print(f"Evidence: {args.evidence_dir.resolve()}", flush=True)
            return subprocess.run([sys.executable, str(root / "scripts/check-search-regression.py"),
                                   base_url, str(args.evidence_dir / "http")], cwd=root).returncode
    finally:
        guard.close()
        if child is not None:
            if child.poll() is None:
                child.terminate()
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1)
