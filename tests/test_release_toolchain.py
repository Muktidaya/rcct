#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Offline unit tests for release pin metadata handling; no network or tools."""
import contextlib
import hashlib
import importlib.util
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

PATH = Path(__file__).resolve().parents[1] / 'tools/release_toolchain.py'
SPEC = importlib.util.spec_from_file_location('release_toolchain', PATH)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ReleaseToolchainTests(unittest.TestCase):
    def test_official_metadata_and_checksum(self):
        data = b'date = "2026-11-12"\n[pkg.rust]\nversion = "1.100.0 (abc 2026-11-12)"\n'
        checksum = (hashlib.sha256(data).hexdigest() + '  channel-rust-stable.toml\n').encode()
        with patch.object(MODULE, 'download', side_effect=[data, checksum]):
            self.assertEqual(MODULE.official(), ('1.100.0', '2026-11-12', hashlib.sha256(data).hexdigest()))
        with patch.object(MODULE, 'download', side_effect=[data, b'0' * 64 + b'  channel-rust-stable.toml\n']):
            with self.assertRaises(ValueError):
                MODULE.official()
        with patch.object(MODULE, 'download', side_effect=[data, b'invalid']):
            with self.assertRaises(ValueError):
                MODULE.official()

    def test_print_and_check_do_not_write(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pin = root / 'rust-toolchain.toml'
            provenance = root / 'provenance.md'
            pin.write_text('[toolchain]\nchannel = "1.100.0"\n', encoding='utf-8')
            provenance.write_text('unchanged\n', encoding='utf-8')
            with patch.object(MODULE, 'PIN', pin), patch.object(MODULE, 'PROVENANCE', provenance):
                with patch.object(MODULE, 'official', side_effect=AssertionError('network called')), patch.object(sys, 'argv', ['tool', '--print']), contextlib.redirect_stdout(io.StringIO()):
                    MODULE.main()
                with patch.object(MODULE, 'official', return_value=('1.101.0', '2026-12-01', 'a' * 64)), patch.object(sys, 'argv', ['tool', '--check']), contextlib.redirect_stdout(io.StringIO()):
                    with self.assertRaises(SystemExit):
                        MODULE.main()
            self.assertIn('1.100.0', pin.read_text(encoding='utf-8'))
            self.assertEqual(provenance.read_text(encoding='utf-8'), 'unchanged\n')

    def test_update_scope_and_downgrade_refusal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pin = root / 'rust-toolchain.toml'
            provenance = root / 'provenance.md'
            untouched = root / 'Cargo.toml'
            pin.write_text('[toolchain]\nchannel = "1.100.0"\n', encoding='utf-8')
            provenance.write_text('old\n', encoding='utf-8')
            untouched.write_text('rust-version = "1.100.0"\n', encoding='utf-8')
            with patch.object(MODULE, 'PIN', pin), patch.object(MODULE, 'PROVENANCE', provenance), patch.object(sys, 'argv', ['tool', '--update']), contextlib.redirect_stdout(io.StringIO()):
                with patch.object(MODULE, 'official', return_value=('1.99.1', '2026-10-15', 'a' * 64)):
                    with self.assertRaises(SystemExit):
                        MODULE.main()
                with patch.object(MODULE, 'official', return_value=('1.101.0', '2026-12-01', 'b' * 64)):
                    MODULE.main()
            self.assertIn('1.101.0', pin.read_text(encoding='utf-8'))
            self.assertIn('b' * 64, provenance.read_text(encoding='utf-8'))
            self.assertEqual(untouched.read_text(encoding='utf-8'), 'rust-version = "1.100.0"\n')


if __name__ == '__main__':
    unittest.main()
