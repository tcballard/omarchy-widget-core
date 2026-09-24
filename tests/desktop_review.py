#!/usr/bin/env python3
"""Evidence integrity tests; no systemd or desktop required."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('review', Path(__file__).resolve().parents[1] / 'tools/desktop-review.py')
review = importlib.util.module_from_spec(spec)
spec.loader.exec_module(review)


class EvidenceTests(unittest.TestCase):
    def test_portable_suite_overrides_desktop_qt_backend(self):
        with patch.dict(review.os.environ, {'QT_QPA_PLATFORM': 'wayland;xcb',
                                            'QT_QPA_PLATFORMTHEME': 'gtk3'}), \
             patch.object(review, 'OUT', Path('/unused-evidence'), create=True), \
             patch.object(review, 'command', return_value={'status': 'pass'}) as command:
            list(review.suite(Path('/clock'), Path('/older-core'), Path('/proxy')))
            self.assertGreater(len(command.call_args_list), 40)
            for call in command.call_args_list:
                self.assertEqual(call.kwargs['env']['QT_QPA_PLATFORM'], 'offscreen')
                self.assertEqual(call.kwargs['env']['QT_QPA_PLATFORMTHEME'], 'basic')
            self.assertEqual(review.os.environ['QT_QPA_PLATFORM'], 'wayland;xcb')

    def test_restart_and_counter_reset_are_not_zero_cpu(self):
        first = {'identity': 12, 'cpu_usec': 100}
        self.assertIsNone(review.cpu_percent(first, {'identity': 13, 'cpu_usec': 500}, 1))
        self.assertIsNone(review.cpu_percent(first, {'identity': 12, 'cpu_usec': 50}, 1))
        self.assertEqual(review.cpu_percent(first, {'identity': 12, 'cpu_usec': 10100}, 1), 1)

    def test_missing_desktop_is_blocked(self):
        with patch.object(review, 'units', side_effect=RuntimeError('no session')):
            result = review.capture(1)
        self.assertEqual(result['status'], 'blocked')
        self.assertEqual(result['summary'], {})

    def test_unit_churn_invalidates_measurement(self):
        row = {'identity': 1, 'cpu_usec': 0, 'memory_current_bytes': 1,
               'process_count': 1, 'pss_bytes': 1}
        with patch.object(review, 'units', side_effect=[{review.HOST: Path('/fake')}, {}]), \
             patch.object(review, 'sample_group', return_value=row), \
             patch.object(review.time, 'sleep'):
            result = review.capture(1)
        self.assertEqual(result['status'], 'blocked')
        self.assertIn('Unit set changed', result['errors'][0])

    def test_disk_hardlinks_and_symlinks_not_double_counted(self):
        import os
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            (base / 'a').write_bytes(b'1234')
            os.link(base / 'a', base / 'b')
            (base / 'c').symlink_to('/etc/passwd')
            result = review.manifest(base)
            self.assertEqual(result['bytes'], 4)
            self.assertEqual(len(result['files']), 1)

    def test_failed_commands_and_timeout_survive_reporting(self):
        import sys
        self.assertEqual(review.command([sys.executable, '-c', 'raise SystemExit(2)'])['status'], 'fail')
        self.assertEqual(review.command(['/no-such-widget-test'])['status'], 'blocked')
        self.assertEqual(review.command([sys.executable, '-c', 'import time; time.sleep(5)'], timeout=.02)['status'], 'timeout')
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp) / 'evidence'
            review.initialize(base)
            review.event(base, 'failed-test', {'status': 'fail'})
            self.assertIn('**fail**', (base / 'report.md').read_text())
            self.assertTrue(all(v['status'] == 'not-run' for v in json.loads((base / 'checks.json').read_text()).values()))
            with self.assertRaises(FileExistsError):
                review.initialize(base)


if __name__ == '__main__':
    unittest.main()
