"""Source-only counterexamples of native attestation; no native client.

The actual PsqlSession.__call__ is used on an explicit test double created
WITHOUT __init__. No bundle/secret/provider lookup. capture is entirely fake.
Valid synthetic SQL rows must never replace an owned-process attestation.
"""
from pathlib import Path
import unittest
from unittest.mock import patch

from test_runner import runner, profile, rows, encode


class NativeAttestationRegressions(unittest.TestCase):
    def session(self):
        # Intentionally bypass real constructor: no actual credential access.
        obj=runner.PsqlSession.__new__(runner.PsqlSession)
        obj.profile=profile();obj.clock=lambda:100.;obj.deadline=105.
        obj.check_authority=lambda:None  # FAKE authority, never a runtime grant.
        obj.bundle=Path('/fixture/private-bundle/bundle')
        obj.loader=obj.bundle/'loader/ld-musl-x86_64.so.1'
        obj.program=obj.bundle/'bin/psql'
        obj.home=Path('/fixture/private-home');obj.tmp=Path('/fixture/private-tmp')
        obj.pgpass=Path('/fixture/private-passfile')
        obj.loader_sha256='38d022ce7425ff105ccfb53598f606e6e5f5f0a34bfbc793d65e6f34c9d72806'
        obj.program_sha256='c4e48eba81b1b9e2a8ab11347afdce0ffc9eb8df41c87e4e694dce3ab79cadb5'
        obj.secret=b'f'*64  # Synthetic public canary, no real passfile exists.
        return obj

    def bad_record(self,**changes):
        obj=self.session();ro,row=rows(obj.profile)
        record=dict(stdout=encode(ro,row),stderr=b'',returncode=0,reaped=True,
            partial=False,truncated=False,timed_out=False,pid=12345,start='11',
            started_monotonic=100.,ended_monotonic=100.,exe=str(obj.loader),
            observed_unknown=False,maps='fixture loader + psql + private dependencies',
            observed_env=dict(HOME=str(obj.home),PGPASSFILE=str(obj.pgpass)),
            attestation_stage='while-held-after-readonly',attested_while_alive=True)
        record.update(changes)
        with patch.object(runner,'capture',return_value=record) as capture:
            with self.assertRaises(ValueError):obj(identity=None)
            capture.assert_called_once()

    def test_unknown_execution_is_not_attested_by_sql_rows(self):
        self.bad_record(exe=None,observed_unknown=True)

    def test_foreign_kernel_entry_is_not_attested_by_requested_argv(self):
        self.bad_record(exe='/fixture/foreign-loader')

    def test_missing_program_mapping_is_not_attested_by_ledger(self):
        self.bad_record(maps='',attestation_stage='before-exec')

    def test_foreign_effective_environment_is_not_attested_by_config(self):
        self.bad_record(observed_env=dict(HOME='/fixture/foreign-home',PGPASSFILE='/fixture/foreign-passfile'))


if __name__=='__main__':unittest.main()
