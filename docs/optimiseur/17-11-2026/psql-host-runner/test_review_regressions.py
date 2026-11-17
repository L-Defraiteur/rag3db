"""Contre-exemples source R9a3. NON JOUÉS ; aucun processus natif.

Faux transport/Popen seulement, vraies écritures dans TemporaryDirectory et
vrais raccords observe/consumer/capture. Future recette rouge séparée requise.
"""
import io
import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

from test_runner import runner, profile, rows, encode, fake_observe


class ReviewRegressions(unittest.TestCase):
    def test_first_capture_failure_survives_cleanup_failure(self):
        with tempfile.TemporaryDirectory() as temp:
            out=Path(temp)/'proof'
            class Session:
                def __call__(self,*,identity=None):
                    raise runner.CaptureRefused(dict(stdout=b'first-capture',stderr=b'first-error'))
                def close(self):raise OSError('owned cleanup failed')
            with self.assertRaises(ValueError):
                runner.observe(profile(),out,session_factory=lambda _:Session(),clock=lambda:100.,
                    phase_check=lambda:dict(active=True,first_red=False,common_end=120.))
            refusal=json.loads((out/'refusal.json').read_text())
            self.assertEqual(refusal.get('first_error'),'CaptureRefused')
            self.assertEqual(refusal.get('cleanup_error'),'OSError')
            self.assertEqual((out/'identity.stdout').read_bytes(),b'first-capture')

    def test_unexpected_capture_exception_keeps_record(self):
        # Pas de fork, pas de pidfd réel : échec d'API OS injecté avant lecture.
        class Child:
            pid=12345
            returncode=0
            stdin=io.BytesIO()
            stdout=io.BytesIO(b'captured')
            stderr=io.BytesIO()
            def poll(self):return self.returncode
        with patch.object(runner.subprocess,'Popen',return_value=Child()), \
             patch.object(runner.os,'pidfd_open',side_effect=OSError('injected pidfd failure')):
            try:
                runner.capture(['never-executed'],env={},stdin=b'',deadline=time.monotonic()+1)
            except OSError:
                self.fail('OS refusal escaped without a CaptureRefused record')
            except runner.CaptureRefused as error:
                self.assertEqual(error.record.get('first_error'),'OSError')
                self.assertIs(error.record['reaped'],True)
            else:self.fail('injected OS failure was accepted')

    def test_consumer_refuses_missing_completion_sidecar(self):
        with tempfile.TemporaryDirectory() as temp:
            out=Path(temp)/'proof';path,digest=fake_observe(out)
            (out/'observation.sha256').unlink()  # Ensemble incomplet construit hors vivant.
            with self.assertRaises(ValueError):
                runner.read_fixture_observation(path,digest,profile())

    def test_late_publication_without_refusal_is_not_consumable(self):
        with tempfile.TemporaryDirectory() as temp:
            out=Path(temp)/'proof';now=[100.];p=profile()
            def factory(_):
                def session(identity=None):
                    ro,row=rows(p,check=identity is not None,identity=identity)
                    before=now[0];now[0]+=.1
                    return dict(stdout=encode(ro,row),stderr=b'',returncode=0,reaped=True,
                        partial=False,truncated=False,timed_out=False,pid=901,
                        started_monotonic=before,ended_monotonic=now[0])
                return session
            actual_sync=runner.sync_directory;actual_write=runner.v1._write_exclusive
            def sync_then_expire(path):
                actual_sync(path)
                if (out/'observation.json').exists():now[0]=106.
            def refuse_marker_failure(path,raw):
                if Path(path).name=='refusal.json':raise OSError('injected refusal fsync failure')
                return actual_write(path,raw)
            with patch.object(runner,'sync_directory',side_effect=sync_then_expire), \
                 patch.object(runner.v1,'_write_exclusive',side_effect=refuse_marker_failure):
                try:
                    runner.observe(p,out,session_factory=factory,clock=lambda:now[0],
                        phase_check=lambda:dict(active=True,first_red=False,common_end=120.))
                except (ValueError,OSError):pass  # On juge ensuite LE consommateur.
                else:self.fail('late observation accepted')
            self.assertGreater(now[0],105.)  # Le vrai fsync après lien a été franchi.
            self.assertTrue((out/'observation.json').exists())
            self.assertFalse((out/'refusal.json').exists())
            raw=(out/'observation.json').read_bytes()
            digest=runner.hashlib.sha256(raw).hexdigest()
            with self.assertRaises(ValueError):
                runner.read_fixture_observation(out/'observation.json',digest,p)


class TerminalAdmission(unittest.TestCase):
    """Pièces terminales SYNTHÉTIQUES CPU ; aucune portée réelle admise."""
    def terminal(self,out,path,digest,**patches):
        body=dict(kind='psql-fixture-terminal-admission-1',artifact_path=str(path),artifact_sha256=digest,
            common_state='termine',common_verdict='VERT',common_code=0,common_remaining=[],
            common_cleaned=True,publication_complete_sha256=runner.sha(out/'publication-complete.json'),
            observed_monotonic=110.)
        body.update(patches)
        terminal=out/'terminal-admission.json';terminal.write_bytes(encode(body))
        return dict(terminal_receipt=terminal,terminal_sha256=runner.sha(terminal))

    def test_complete_observation_and_terminal_admission_are_readable(self):
        with tempfile.TemporaryDirectory() as temp:
            out=Path(temp)/'proof';path,digest=fake_observe(out)
            pins=self.terminal(out,path,digest)
            body=runner.read_fixture_observation(path,digest,profile(),**pins)
            self.assertEqual(body['identity'],rows()[1])
            self.assertIs(body['grant_authorized'],False)

    def test_red_terminal_scope_is_not_accepted(self):
        with tempfile.TemporaryDirectory() as temp:
            out=Path(temp)/'proof';path,digest=fake_observe(out)
            pins=self.terminal(out,path,digest,common_verdict='ROUGE',common_code=1)
            with self.assertRaises(ValueError):runner.read_fixture_observation(path,digest,profile(),**pins)

    def test_sidecar_required_even_with_valid_terminal(self):
        with tempfile.TemporaryDirectory() as temp:
            out=Path(temp)/'proof';path,digest=fake_observe(out)
            pins=self.terminal(out,path,digest)
            (out/'observation.sha256').unlink()
            with self.assertRaises(ValueError):runner.read_fixture_observation(path,digest,profile(),**pins)

    def test_completion_required_even_with_valid_terminal(self):
        with tempfile.TemporaryDirectory() as temp:
            out=Path(temp)/'proof';path,digest=fake_observe(out)
            pins=self.terminal(out,path,digest)
            (out/'publication-complete.json').unlink()
            with self.assertRaises(ValueError):runner.read_fixture_observation(path,digest,profile(),**pins)


if __name__=='__main__':unittest.main()
