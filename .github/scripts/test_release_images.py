import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('release_images.py').resolve()
VARIANTS = ('server', 'app', 'migrate', 'cli')
DIGESTS = {name: 'sha256:' + str(index) * 64 for index, name in enumerate(VARIANTS, 1)}


class ReleaseImagesTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.env = dict(os.environ, GITHUB_REPOSITORY='Owner/Repo', GITHUB_EVENT_NAME='push',
                        GITHUB_OUTPUT=str(self.root / 'output'), FAKE_ROOT=str(self.root))
        self.git('init', '-q')
        self.git('config', 'user.email', 'test@example.com')
        self.git('config', 'user.name', 'Test')
        self.change('exposed/main.rs')
        self.baseline = self.git('rev-parse', 'HEAD').strip()
        self.manifest = dict(version='2026.10.26.12.00', source=self.baseline,
                             images={name: 'ghcr.io/owner/repo@' + value for name, value in DIGESTS.items()})
        (self.root / 'release.json').write_text(json.dumps({'tag_name': self.manifest['version']}))
        self.save_manifest()
        binary = self.root / 'bin'
        binary.mkdir()
        gh = binary / 'gh'
        gh.write_text('''#!/usr/bin/env python3
import os, pathlib, shutil, sys
root = pathlib.Path(os.environ['FAKE_ROOT'])
if (root / 'fail').exists(): sys.exit(1)
if sys.argv[1] == 'api': print((root / 'release.json').read_text())
else: shutil.copyfile(root / 'manifest.json', pathlib.Path(sys.argv[sys.argv.index('--dir') + 1]) / 'images.json')
''')
        gh.chmod(0o755)
        self.env['PATH'] = str(binary) + os.pathsep + self.env['PATH']

    def git(self, *args):
        return subprocess.check_output(['git', *args], cwd=self.root, stderr=subprocess.PIPE).decode()

    def change(self, path):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(file.read_text() + 'change\n' if file.exists() else 'change\n')
        self.git('add', path)
        self.git('commit', '-qm', path)

    def save_manifest(self):
        (self.root / 'manifest.json').write_text(json.dumps(self.manifest))

    def invoke(self, command, **env):
        return subprocess.run([sys.executable, str(SCRIPT), command], cwd=self.root,
                              env=dict(self.env, **env), capture_output=True, text=True, check=True)

    def plan(self, event='push'):
        return json.loads(self.invoke('plan', GITHUB_SHA=self.git('rev-parse', 'HEAD').strip(),
                                      GITHUB_EVENT_NAME=event).stdout)

    def test_frontend_reuses_exact_backend_and_assembles_complete_artifact(self):
        self.change('frontend/src/page.svelte')
        self.change('docker/Dockerfile.frontend')
        plan = self.plan()
        self.assertEqual(plan, {'mode': 'reuse', 'source': self.baseline,
                               'backend': {name: DIGESTS[name] for name in ('server', 'migrate', 'cli')}})
        self.invoke('assemble', BUILD_BACKEND='false', BACKEND=json.dumps(plan['backend']), APP_DIGEST=DIGESTS['app'])
        self.assertEqual(json.loads((self.root / 'amd64.json').read_text()), DIGESTS)

    def test_backend_and_shared_changes_build(self):
        for path in ('exposed/main.rs', 'db/migrations/new.up.sql', 'Cargo.lock', 'Cargo.toml',
                     '.dockerignore', 'docker/Dockerfile', '.github/workflows/release.yml', 'README.md'):
            with self.subTest(path=path):
                self.git('reset', '--hard', self.baseline)
                self.change(path)
                self.change('frontend/app.js')
                self.assertEqual(self.plan()['mode'], 'build')

    def test_workflow_gates_all_backends_and_keeps_app_unconditional(self):
        action = SCRIPT.parents[1] / 'actions/build-images/action.yml'
        text = action.read_text()
        self.assertIn("  build-backend:\n    description: Build all three backend targets\n    default: 'true'", text)
        for name in ('Build backend server', 'Build migrate', 'Build data'):
            self.assertIn(f"    - name: {name}\n      if: inputs.build-backend == 'true'", text)
        app = text.split('    - name: Build frontend-only app\n')[1].split('    - name: Build migrate')[0]
        self.assertNotIn('      if:', app)
        self.assertIn('scope=exposed-frontend-linux-', app)
        self.assertNotIn('scope=exposed-backend-linux-', app)
        self.assertNotIn('scope=exposed-linux-', text)
        release = (SCRIPT.parents[1] / 'workflows/release.yml').read_text()
        self.assertIn('fetch-depth: 0', release)
        self.assertEqual(release.count('release_images.py plan'), 1)
        self.assertIn('build-backend: ${{ needs.prepare.outputs.build-backend }}', release)
        self.assertIn('release_images.py validate', release)
        arm = (SCRIPT.parents[1] / 'workflows/arm-test.yml').read_text()
        self.assertNotIn('build-backend:', arm)

    def test_backend_deletion_builds(self):
        self.git('rm', 'exposed/main.rs')
        self.git('commit', '-qm', 'delete')
        self.assertEqual(self.plan()['mode'], 'build')

    def test_failed_backend_release_is_included_in_comparison(self):
        self.change('exposed/main.rs')
        self.change('frontend/app.js')
        self.assertEqual(self.plan()['mode'], 'build')

    def test_backend_renamed_into_frontend_builds(self):
        (self.root / 'frontend').mkdir()
        self.git('mv', 'exposed/main.rs', 'frontend/main.rs')
        self.git('commit', '-qm', 'rename')
        self.assertEqual(self.plan()['mode'], 'build')

    def test_invalid_metadata_builds(self):
        self.change('frontend/app.js')
        original = json.dumps(self.manifest)
        mutations = [dict(source='f' * 40), dict(source='bad'), dict(source=['bad']), dict(version='wrong'), dict(images={}),
                     dict(images={**self.manifest['images'], 'app': 'ghcr.io/other/repo@' + DIGESTS['app']}),
                     dict(images={**self.manifest['images'], 'cli': 'ghcr.io/owner/repo:latest'})]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                self.manifest = dict(json.loads(original), **mutation)
                self.save_manifest()
                self.assertEqual(self.plan()['mode'], 'build')
        (self.root / 'manifest.json').write_text('{')
        self.assertEqual(self.plan()['mode'], 'build')
        (self.root / 'manifest.json').unlink()
        self.assertEqual(self.plan()['mode'], 'build')

    def test_first_release_and_network_failure_build(self):
        (self.root / 'fail').touch()
        self.assertEqual(self.plan()['mode'], 'build')
        (self.root / 'fail').unlink()
        (self.root / 'release.json').write_text('{}')
        self.assertEqual(self.plan()['mode'], 'build')

    def test_nonancestor_builds(self):
        self.change('frontend/other.js')
        self.manifest['source'] = self.git('rev-parse', 'HEAD').strip()
        self.save_manifest()
        self.git('reset', '--hard', self.baseline)
        self.change('frontend/app.js')
        self.assertEqual(self.plan()['mode'], 'build')

    def test_manual_builds_all_and_assembly_rejects_partial_set(self):
        self.change('frontend/app.js')
        self.assertEqual(self.plan('workflow_dispatch')['mode'], 'build')
        env = {name.upper() + '_DIGEST': value for name, value in DIGESTS.items()}
        self.invoke('assemble', BUILD_BACKEND='true', BACKEND='{}', **env)
        self.assertEqual(json.loads((self.root / 'amd64.json').read_text()), DIGESTS)
        with self.assertRaises(subprocess.CalledProcessError):
            self.invoke('assemble', BUILD_BACKEND='false', BACKEND='{}', APP_DIGEST=DIGESTS['app'])
        directory = self.root / 'digests'
        directory.mkdir()
        (directory / 'amd64.json').write_text(json.dumps(DIGESTS))
        self.invoke('validate')
        (directory / 'amd64.json').write_text(json.dumps({**DIGESTS, 'cli': ''}))
        with self.assertRaises(subprocess.CalledProcessError):
            self.invoke('validate')


if __name__ == '__main__':
    unittest.main()
