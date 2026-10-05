#!/usr/bin/env python3
"""Create .env.compose with a worktree project name and three free host ports."""

import hashlib
import re
import socket
import subprocess
import sys
from contextlib import ExitStack
from pathlib import Path


def main() -> int:
    root = Path(
        subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=Path(__file__).resolve().parent,
            text=True,
        ).strip()
    )
    destination = root / ".env.compose"
    if destination.exists():
        print(
            f"{destination} already exists. Reuse it, or stop its Compose deployment "
            "and remove the file before you select new ports.",
            file=sys.stderr,
        )
        return 1

    branch = subprocess.check_output(
        ["git", "branch", "--show-current"], cwd=root, text=True
    ).strip() or "detached"
    slug = re.sub(r"[^a-z0-9_-]+", "-", f"{root.name}-{branch}".lower())
    path_hash = hashlib.sha256(str(root).encode()).hexdigest()[:8]
    project = f"exposed-{slug[:60]}-{path_hash}"

    # Keep all sockets open until selection is complete so the ports are distinct.
    with ExitStack() as stack:
        ports: dict[str, int] = {}
        for name in ("FRONTEND_PORT", "API_PORT", "POSTGRES_PORT"):
            listener = stack.enter_context(socket.socket(socket.AF_INET, socket.SOCK_STREAM))
            listener.bind(("127.0.0.1", 0))
            ports[name] = listener.getsockname()[1]

        with destination.open("x", encoding="utf-8") as output:
            output.write(f"COMPOSE_PROJECT_NAME={project}\n")
            for name, port in ports.items():
                output.write(f"{name}={port}\n")

    print(f"Created {destination}")
    print(f"Frontend: http://localhost:{ports['FRONTEND_PORT']}")
    print(f"API: http://localhost:{ports['API_PORT']}")
    print("Set DATABASE_URL in .env (or ingest/.env for the Python importer) to:")
    print(
        "DATABASE_URL=postgresql://exposed:exposed_local_dev@localhost:"
        f"{ports['POSTGRES_PORT']}/exposed?sslmode=disable"
    )
    print("Use: docker compose --env-file .env.compose up -d")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"Could not create .env.compose: {error}", file=sys.stderr)
        sys.exit(1)
