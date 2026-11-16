"""Persistent observations; transport bytes and current rendered image are distinct."""
import base64
import json
import os
from pathlib import Path
import threading
import time


def screen_contains(raw, rendered, needle):
    # raw stays evidence, never the authority for a reconstructed screen.
    return rendered is not None and needle in rendered


def approval_and_end(posts, mailbox, state):
    expected = [line for line in mailbox if line.get('kind') == 'approval']
    if len(expected) != 1:
        return False
    approval_id = expected[0].get('id')
    if not isinstance(approval_id, str) or not approval_id:
        return False
    approved = [p for p in posts if p.get('chemin') == '/ide/teammate/mailbox'
                and p.get('corps', {}).get('kind') == 'answer'
                and p['corps'].get('approved') is True
                and p['corps'].get('reply_to') == approval_id]
    ended = any(line.get('kind') == 'ended' for line in mailbox)
    return bool(approved) and ended and state == 'ended'


class PhaseLog:
    def __init__(self, path):
        self.path = Path(path)
        self.lock = threading.RLock()
        self.screen_count = 0
        self.record('phase-created', {}, sync=True)

    def record(self, kind, data, sync=False):
        row = dict(monotonic=time.monotonic(), wall=time.time(), kind=kind, data=data)
        with self.lock, (self.path / 'events.jsonl').open('a', encoding='utf8') as stream:
            stream.write(json.dumps(row, ensure_ascii=False) + '\n')
            stream.flush()
            if sync:
                os.fsync(stream.fileno())

    def output(self, terminal, data):
        self.record('pty-output', dict(terminal=terminal, bytes_b64=base64.b64encode(data).decode()))

    def input(self, terminal, data, written):
        self.record('pty-input-written', dict(terminal=terminal, written=written,
                    bytes_b64=base64.b64encode(data[:written]).decode()))

    def screen(self, terminal, text, started, rc):
        with self.lock:
            self.screen_count += 1
            name = f'screen-{self.screen_count:06}.txt'
            with (self.path / name).open('x', encoding='utf8') as stream:
                stream.write(text)
        self.record('screen-capture', dict(terminal=terminal, file=name, started=started, rc=rc))

    def checkpoint(self, reason):
        self.record('checkpoint', dict(reason=reason), sync=True)
        # Fake-host writes are closed per POST; synchronize the phase's current proof files.
        for path in self.path.iterdir():
            if path.is_file() and not path.is_symlink():
                with path.open('rb') as stream:
                    os.fsync(stream.fileno())
        fd = os.open(self.path, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)


class ObservationArchive:
    def __init__(self, root):
        self.root = Path(root) / 'observations'
        self.root.mkdir(mode=0o700)  # refuse reuse, never truncate an older attempt
        self.counter = 0

    def new_phase(self):
        self.counter += 1
        path = self.root / f'phase-{self.counter:04}'
        path.mkdir(mode=0o700)
        return PhaseLog(path)
