#!/usr/bin/env python3
from pathlib import Path
import os
import socket
import subprocess
import sys
import tempfile
import unittest

DRIVER = Path(__file__).with_name("verify-search-regression.py").resolve()


class PortIsolationTest(unittest.TestCase):
    def test_occupied_address_uses_owned_ephemeral_api(self):
        with socket.socket() as old_listener:
            old_listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            old_listener.bind(("127.0.0.1", 0))
            old_listener.listen()
            occupied_port = old_listener.getsockname()[1]
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "target/debug").mkdir(parents=True)
                (root / "scripts").mkdir()
                driver = root / "driver.py"
                driver.write_text(DRIVER.read_text().replace("6999", str(occupied_port)))
                api = root / "target/debug/exposed"
                api.write_text(f'''#!{sys.executable}
import socket
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
import os
class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.end_headers()
        self.wfile.write(str(os.getpid()).encode())
server = HTTPServer(("0.0.0.0", 0), Handler, bind_and_activate=False)
server.socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
try:
    server.socket.bind(("0.0.0.0", {occupied_port}))
except OSError:
    server.socket.bind(("0.0.0.0", 0))
server.server_address = server.socket.getsockname()
server.server_activate()
Path("child.pid").write_text(str(os.getpid()))
print(f"Listening on http://localhost:{{server.server_address[1]}}", flush=True)
server.serve_forever()
''')
                api.chmod(0o755)
                checker = root / "scripts/check-search-regression.py"
                checker.write_text(f'''import sys
from pathlib import Path
from urllib.request import urlopen
assert not sys.argv[1].endswith(":{occupied_port}")
assert urlopen(sys.argv[1], timeout=2).read().decode() == Path("child.pid").read_text()
Path(sys.argv[2]).mkdir()
print("PASS owned API")
raise SystemExit(7)
''')
                evidence = root / "evidence"
                result = subprocess.run([sys.executable, str(driver), str(evidence)],
                                        cwd=root, capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, 7, result.stdout + result.stderr)
                self.assertIn("PASS owned API", result.stdout)
                self.assertTrue((evidence / "server.log").exists())
                self.assertTrue((evidence / "server.txt").exists())
                pid = int((root / "child.pid").read_text())
                with self.assertRaises(ProcessLookupError):
                    os.kill(pid, 0)
                with socket.create_connection(("127.0.0.1", occupied_port), timeout=2):
                    connection, _ = old_listener.accept()
                    connection.close()


if __name__ == "__main__":
    unittest.main()
