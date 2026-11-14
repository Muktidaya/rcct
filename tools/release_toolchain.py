#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Inspect the project Rust pin or compare/update it from the official stable manifest.

--print is offline. --check and --update download metadata only from the fixed
Rust distribution host; --update changes only the project pin and provenance.
"""
import argparse
import difflib
import datetime
import hashlib
from pathlib import Path
import re
import tomllib
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
PIN = ROOT / 'rust-toolchain.toml'
PROVENANCE = ROOT / 'docs/toolchain-provenance.md'
BASE = 'https://static.rust-lang.org/dist/'
MANIFEST = 'channel-rust-stable.toml'
MAX_BYTES = 4 * 1024 * 1024


def valid_version(value):
    return isinstance(value, str) and re.fullmatch(
        r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)", value
    ) is not None


def read_pin():
    document = tomllib.loads(PIN.read_text(encoding='utf-8'))
    toolchain = document.get('toolchain')
    if not isinstance(toolchain, dict) or not valid_version(toolchain.get('channel')):
        raise ValueError('expected a stable numeric project toolchain channel')
    return toolchain['channel']


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise urllib.error.HTTPError(req.full_url, code, 'redirect rejected', headers, fp)


def download(name):
    opener = urllib.request.build_opener(NoRedirect())
    request = urllib.request.Request(BASE + name, headers={'User-Agent': 'rcct-release-toolchain/1'})
    with opener.open(request, timeout=20) as response:
        data = response.read(MAX_BYTES + 1)
    if len(data) > MAX_BYTES:
        raise ValueError('official metadata exceeds size limit')
    return data


def official():
    data = download(MANIFEST)
    checksum = download(MANIFEST + '.sha256').decode('ascii').strip()
    match = re.fullmatch(r'([0-9a-f]{64})\s+(?:\*)?channel-rust-stable\.toml', checksum)
    digest = hashlib.sha256(data).hexdigest()
    if not match or match.group(1) != digest:
        raise ValueError('official manifest checksum mismatch or malformed checksum')
    document = tomllib.loads(data.decode('utf-8'))
    date = document.get('date')
    if not isinstance(date, str):
        raise ValueError('missing or invalid stable release date')
    try:
        if datetime.date.fromisoformat(date).isoformat() != date:
            raise ValueError('noncanonical stable release date')
    except ValueError as error:
        raise ValueError('invalid stable release date') from error
    package = document.get('pkg')
    rust = package.get('rust') if isinstance(package, dict) else None
    version = rust.get('version') if isinstance(rust, dict) else None
    match = re.fullmatch(r'([^ ]+)(?: \([^()\n]+\))?', version) if isinstance(version, str) else None
    if not match or not valid_version(match.group(1)):
        raise ValueError('missing or invalid stable Rust version')
    return match.group(1), date, digest



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument('--print', action='store_true', dest='print_pin')
    action.add_argument('--check', action='store_true')
    action.add_argument('--update', action='store_true')
    args = parser.parse_args()
    pin = read_pin()
    if args.print_pin:
        print(pin)
        return
    stable, date, digest = official()
    print('Official stable:', stable, 'release date:', date, 'manifest sha256:', digest)
    if args.check:
        if pin != stable:
            raise SystemExit('project pin differs from official stable')
        return
    if tuple(map(int, stable.split('.'))) < tuple(map(int, pin.split('.'))):
        raise SystemExit('refusing to downgrade the project pin')
    old = PIN.read_text(encoding='utf-8')
    # Only edit an unambiguous canonical channel line inside [toolchain].
    section = re.search(r'^\[toolchain\][ \t]*$(.*?)(?=^\[|\Z)', old,
                        re.MULTILINE | re.DOTALL)
    line = re.compile(r'^channel = "' + re.escape(pin) + r'"$', re.MULTILINE)
    if not section or len(line.findall(section.group(1))) != 1:
        raise ValueError('pin formatting is not supported for safe update')
    body = line.sub('channel = "' + stable + '"', section.group(1), count=1)
    new = old[:section.start(1)] + body + old[section.end(1):]
    before = tomllib.loads(old)
    proposed = tomllib.loads(new)
    expected = tomllib.loads(old)
    expected['toolchain']['channel'] = stable
    if before['toolchain']['channel'] != pin or proposed != expected:
        raise ValueError('proposed pin update changes unexpected metadata')
    old_provenance = PROVENANCE.read_text(encoding='utf-8')
    new_provenance = ('# Rust toolchain provenance\n\n'
                      'Project pin: `' + stable + '`\n\n'
                      'Official stable manifest release date: `' + date + '`\n\n'
                      'Manifest SHA-256: `' + digest + '`\n\n'
                      'Source: https://static.rust-lang.org/dist/channel-rust-stable.toml\n\n'
                      'The checksum is fetched from the same official HTTPS origin; this is\n'
                      'transport and consistency evidence, not an independent signature.\n')
    for label, before, after in (('rust-toolchain.toml', old, new), ('docs/toolchain-provenance.md', old_provenance, new_provenance)):
        print(''.join(difflib.unified_diff(before.splitlines(True), after.splitlines(True), fromfile=label, tofile=label)), end='')
    PIN.write_text(new, encoding='utf-8')
    PROVENANCE.write_text(new_provenance, encoding='utf-8')
    print('Updated project pin and provenance; review the diff and run the required checks.')


if __name__ == '__main__':
    main()
