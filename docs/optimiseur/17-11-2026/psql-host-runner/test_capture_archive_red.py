"""Source counterexamples: fake capture + fake IO failure, zero real child.

These tests call the actual helper on private TemporaryDirectory only. A
missing original cause on an OSError is an assertion FAILURE, not swallowed
infrastructure. Import or any other unexpected exception remains ERROR.
"""
import os
from pathlib import Path
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import capture_evidence as evidence
from test_runner import runner


class ArchiveCauseRegressions(unittest.TestCase):
    def exercise(self,mode):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)/'proof'
            record=dict(stdout=b'original-capture-out',stderr=b'original-capture-err',
                pid=12345,start='11',exe=str(Path(sys.executable).resolve()),observed_unknown=False,
                reaped=True,partial=True,returncode=-9,first_error='original-refusal')
            original=runner.CaptureRefused(record)
            injected=OSError('injected '+mode+' archive failure')
            def capture(*args,**kwargs):raise original
            fake=SimpleNamespace(capture=capture,CaptureRefused=runner.CaptureRefused)
            actual_write=evidence.write_new;actual_sync=evidence.sync;actual_fd=evidence.fd_snapshot
            counts=dict(sync=0,fd=0)
            def write(path,raw):
                if mode=='write' and Path(path).name=='stdout':raise injected
                return actual_write(path,raw)
            def sync(path):
                counts['sync']+=1
                if mode=='sync' and counts['sync']==2:raise injected
                return actual_sync(path)
            def fd_snapshot():
                counts['fd']+=1
                if mode=='fd' and counts['fd']==2:raise injected
                return actual_fd()
            def proc_state(*_):
                if mode=='proc':raise injected
                return dict(original='ABSENT',state=None)
            env=dict(PSQL_CPU_CAPTURE_PROOFS=str(root),
                PSQL_CPU_PYTHON_SHA256=evidence.hashlib.sha256(Path(sys.executable).resolve().read_bytes()).hexdigest())
            with patch.dict(os.environ,env),patch.object(evidence,'write_new',side_effect=write), \
                 patch.object(evidence,'sync',side_effect=sync), \
                 patch.object(evidence,'fd_snapshot',side_effect=fd_snapshot), \
                 patch.object(evidence,'proc_state',side_effect=proc_state):
                with self.assertRaises(OSError) as ctx:
                    evidence.archived_capture(fake,'fixture_'+mode,['never-executed'],
                        env={},stdin=b'',deadline=time.monotonic()+2)
            error=ctx.exception
            self.assertIs(getattr(error,'first_error',None),original)
            self.assertIs(getattr(error,'record',None),record)
            self.assertIs(getattr(error,'archive_error',None),injected)
            self.assertIs(error.__cause__,original)

    def test_write_error_keeps_first_capture_record(self):self.exercise('write')
    def test_sync_error_keeps_first_capture_record(self):self.exercise('sync')
    def test_proc_metadata_error_keeps_first_capture_record(self):self.exercise('proc')
    def test_fd_metadata_error_keeps_first_capture_record(self):self.exercise('fd')


if __name__=='__main__':unittest.main()
