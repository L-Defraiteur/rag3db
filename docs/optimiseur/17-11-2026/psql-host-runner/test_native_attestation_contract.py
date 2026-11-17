"""Valid synthetic parent attestation + isolated mutants, zero native process.

Actual PsqlSession.__call__, explicit __new__ double, constructor bypassed.
No fixture path is opened. The capture callback is entirely fake and supplies
an in-memory held-process attestation; this is not an OS collector qualification.
"""
import copy
import hashlib
import unittest
from unittest.mock import patch

import test_native_attestation_red as initial
from test_runner import runner, rows, encode


class NativeAttestationContract(unittest.TestCase):
    def fixture(self,*,postcheck=False):
        obj=initial.NativeAttestationRegressions().session()
        files={str(obj.loader):dict(sha256=obj.loader_sha256,device_major=8,device_minor=1,inode=101),
            str(obj.program):dict(sha256=obj.program_sha256,device_major=8,device_minor=1,inode=102),
            str(obj.bundle/'lib/libpq.so.5'):dict(sha256='d'*64,device_major=8,device_minor=1,inode=103)}
        obj.native_map_files=copy.deepcopy(files)
        obj.native_required_map_shas={x['sha256'] for x in files.values()}
        obj.native_cgroup='/fixture/private-common'
        original=rows(obj.profile)[1]
        ro,row=rows(obj.profile,check=postcheck,identity=original if postcheck else None)
        maps=''.join(f'{0x100000+i*0x1000:x}-{0x101000+i*0x1000:x} r-xp 00000000 08:01 {meta["inode"]} {path}\n'
            for i,(path,meta) in enumerate(files.items()))
        maps+='7fff0000-7fff1000 rw-p 00000000 00:00 0 [stack]\n'
        def make_record(env):
            proof=dict(schema_version=1,stage='held-after-readonly',pid=12345,start='11',
                cgroup=obj.native_cgroup,kernel_entry=dict(path=str(obj.loader),**files[str(obj.loader)]),
                maps=maps,map_files=[dict(path=p,**copy.deepcopy(m)) for p,m in files.items()],
                environment=copy.deepcopy(env),readonly_stdout_sha256=hashlib.sha256(encode(ro).rstrip(b'\n')).hexdigest(),
                readonly_backend_pid=ro['backend_pid'],observed_monotonic=100.,alive_before=True,alive_after=True)
            return dict(stdout=encode(ro,row),stderr=b'',returncode=0,reaped=True,
                partial=False,truncated=False,timed_out=False,pid=12345,start='11',
                started_monotonic=100.,ended_monotonic=100.,exe=str(obj.loader),
                observed_unknown=False,maps=maps,native_attestation=proof)
        return obj,make_record,original

    def run_fixture(self,mutate=None,*,postcheck=False):
        obj,make_record,identity=self.fixture(postcheck=postcheck)
        def capture(argv,*,env,stdin,deadline,clock):
            self.assertIn('--no-password',argv)
            self.assertNotIn(obj.secret.decode(),repr(argv))
            self.assertNotIn(obj.secret,stdin)
            record=make_record(env)
            if mutate is not None:mutate(obj,record)
            return record
        with patch.object(runner,'capture',side_effect=capture) as mocked:
            result=obj(identity=identity if postcheck else None)
            mocked.assert_called_once()
        return obj,result

    def refused(self,mutate,reason):
        with self.assertRaises(runner.CaptureRefused) as ctx:self.run_fixture(mutate)
        self.assertIn(reason,ctx.exception.record.get('native_refusal',''))
        self.assertIsInstance(ctx.exception.__cause__,ValueError)

    def test_valid_identity_attestation_is_accepted(self):
        obj,record=self.run_fixture()
        self.assertIs(record.get('native_attestation_checked'),True)
        self.assertEqual(record['kernel_entry_requested'],str(obj.loader))
        self.assertEqual(record['program_requested'],str(obj.program))
        self.assertNotEqual(record['kernel_entry_sha256'],record['program_sha256'])

    def test_valid_postcheck_attestation_is_accepted(self):
        _,record=self.run_fixture(postcheck=True)
        self.assertIs(record.get('native_attestation_checked'),True)
        self.assertTrue(record['native_attestation']['environment']['PGAPPNAME'].endswith(':check'))

    def test_unknown_client_identity_refused(self):
        self.refused(lambda o,r:r.update(observed_unknown=True),'native identity')

    def test_wrong_attested_client_pid_refused(self):
        self.refused(lambda o,r:r['native_attestation'].update(pid=12346),'native identity')

    def test_wrong_attested_client_start_refused(self):
        self.refused(lambda o,r:r['native_attestation'].update(start='12'),'native identity')

    def test_wrong_kernel_sha_refused(self):
        self.refused(lambda o,r:r['native_attestation']['kernel_entry'].update(sha256='0'*64),'native kernel')

    def test_foreign_cgroup_refused(self):
        self.refused(lambda o,r:r['native_attestation'].update(cgroup='/fixture/foreign'),'native cgroup')

    def test_foreign_effective_home_refused(self):
        self.refused(lambda o,r:r['native_attestation']['environment'].update(HOME='/fixture/foreign'),'native environment')

    def test_extra_ld_preload_refused(self):
        self.refused(lambda o,r:r['native_attestation']['environment'].update(LD_PRELOAD='/fixture/foreign.so'),'native environment')

    def test_preexec_attestation_refused(self):
        self.refused(lambda o,r:r['native_attestation'].update(stage='before-exec'),'native stage')

    def test_numeric_alive_is_not_true(self):
        self.refused(lambda o,r:r['native_attestation'].update(alive_after=1),'native stage')

    def test_attestation_after_capture_end_refused(self):
        self.refused(lambda o,r:r['native_attestation'].update(observed_monotonic=101.),'native timing')

    def test_wrong_readonly_frame_sha_refused(self):
        self.refused(lambda o,r:r['native_attestation'].update(readonly_stdout_sha256='0'*64),'native readonly binding')

    def test_missing_program_mapping_refused(self):
        def mutate(o,r):
            a=r['native_attestation'];a['maps']=''.join(x+'\n' for x in a['maps'].splitlines() if not x.endswith(str(o.program)))
            a['map_files']=[x for x in a['map_files'] if x['path']!=str(o.program)]
        self.refused(mutate,'native required maps')

    def test_map_inode_disagreement_refused(self):
        self.refused(lambda o,r:r['native_attestation']['map_files'][1].update(inode=999),'native map identity')

    def test_map_hash_disagreement_refused(self):
        self.refused(lambda o,r:r['native_attestation']['map_files'][1].update(sha256='0'*64),'native map identity')

    def test_foreign_file_mapping_refused(self):
        def mutate(o,r):
            a=r['native_attestation'];a['maps']=a['maps'].replace('7fff0000-',
                '400000-401000 r-xp 00000000 08:01 999 /fixture/foreign.so\n7fff0000-')
            a['map_files'].append(dict(path='/fixture/foreign.so',sha256='e'*64,device_major=8,device_minor=1,inode=999))
        self.refused(mutate,'native map outside')

    def test_boolean_schema_version_refused(self):
        self.refused(lambda o,r:r['native_attestation'].update(schema_version=True),'native schema')

    def test_public_canary_in_metadata_is_not_exported(self):
        def mutate(o,r):r['native_attestation']['environment']['UNEXPECTED']=o.secret.decode()
        with self.assertRaises(runner.CaptureRefused) as ctx:self.run_fixture(mutate)
        self.assertNotIn('f'*64,repr(ctx.exception.record))
        self.assertIs(ctx.exception.record['partial'],True)


if __name__=='__main__':unittest.main()
