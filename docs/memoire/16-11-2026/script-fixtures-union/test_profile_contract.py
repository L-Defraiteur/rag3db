"""Private file fixtures and fake metadata subprocess only; no Cargo/proc/signals."""
from pathlib import Path
import hashlib,json,os,shutil,tempfile,types,unittest
from unittest.mock import patch
import profile_contract as c
import source_proof

def digest(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()

class Profile(unittest.TestCase):
    def setUp(self):
        cache=Path.home()/'.cache/rag3weaver-build';self.directory=tempfile.TemporaryDirectory(prefix='p-',dir=cache);self.root=Path(self.directory.name)
    def tearDown(self):self.directory.cleanup()
    def test_union_admission_declares_distinct_3600_and_never_uses_prior_end(self):
        text=Path(__file__).with_name('qualify.py').read_text()
        self.assertIn("'--delai','3600'",text);self.assertNotIn('contract.remaining(',text)
    def test_short_directory_is_exclusive_private_on_disk_and_length_bounded(self):
        with patch.object(c.subprocess,'check_output',return_value='btrfs\n'),patch.object(c.secrets,'token_hex',return_value='1234567890'):
            record=c.create_short(self.root);c.check_same_short(record)
            self.assertEqual(record['uid'],os.getuid());self.assertLessEqual(record['longest_socket_bytes'],107)
            self.assertEqual(os.stat(record['path']).st_mode&0o777,0o700)
            with self.assertRaises(FileExistsError):c.create_short(self.root)
    def test_ram_and_symlink_roots_refused(self):
        with patch.object(c.subprocess,'check_output',return_value='tmpfs\n'),self.assertRaises(RuntimeError):c.create_short(self.root)
        link=self.root/'link';link.symlink_to(self.root)
        with self.assertRaises(RuntimeError):c.create_short(link)
    def test_changed_identity_or_permissions_refused(self):
        with patch.object(c.subprocess,'check_output',return_value='btrfs\n'):record=c.create_short(self.root)
        os.chmod(record['path'],0o755)
        with self.assertRaises(RuntimeError):c.check_same_short(record)
    def test_utf8_byte_bound_refuses_long_root_even_when_character_count_is_short(self):
        path=self.root/('é'*20);path.mkdir(mode=0o700)
        with self.assertRaises(RuntimeError):c.check_short(path)

class Carry(unittest.TestCase):
    def setUp(self):
        self.directory=tempfile.TemporaryDirectory(prefix='carry-fixture-');self.r=Path(self.directory.name);self.app='new-source-exact';self.old='old-source-exact';self.w=self.r/'worktree';self.w.mkdir();self.t=self.r/'target';self.t.mkdir();(self.t/'.arbre-du-target').write_text(str(self.w/'extension/rag3weaver'))
        self.p=self.r/'cx-memoire-script-fixtures-qualification-v3';self.p.mkdir();self.q=self.r/'cx-memoire-script-fixtures-controller-v3';self.q.mkdir();self.a=self.r/'cx-proofs-memoire-script-fixtures-green-v3';self.a.mkdir()
        for n in ['client','controller','supervisor']:(self.a/n).mkdir()
        self.scope=self.r/'scope';self.scope.mkdir();auth=self.scope/'autorite.json';auth.write_text(json.dumps(dict(etat='termine',echeance=1186772.020708913)))
        verdict=self.scope/'verdict.json';verdict.write_text(json.dumps(dict(verdict='VERT',code=0,restes=[],nettoye=True,prive=str(self.r/'absent-bus'))))
        (self.p/'authority-path.json').write_text(json.dumps(dict(path=str(auth))))
        for n in ['composed-result.json','sdk-exec-profile.json','profile.json']:(self.p/n).write_text('{}')
        prior=self.r/'cx-memoire-script-fixtures-qualification-v2/official';prior.mkdir(parents=True);(prior/'resume.log').write_text('prior qualification\nVERT\n')
        (self.p/'composed-result.json').write_text(json.dumps(dict(status='GREEN',source=self.old,pilot=c.OLD_PILOT)))
        (self.q/'configuration.json').write_text(json.dumps(dict(app=self.old,pilot=c.OLD_PILOT)))
        mapping=[(self.p/n,'client/'+n) for n in ['composed-result.json','sdk-exec-profile.json','profile.json']]+[(self.q/'configuration.json','controller/configuration.json'),(verdict,'supervisor/verdict.json')]
        entries=[]
        for file,name in mapping:shutil.copy2(file,self.a/name);entries.append(dict(path=name,sha256=digest(file)))
        canonical=prior/'resume.log';(self.a/'carried-canonical/official').mkdir(parents=True);shutil.copy2(canonical,self.a/'carried-canonical/official/resume.log');entries.append(dict(path='carried-canonical/official/resume.log',sha256=digest(canonical)))
        self.binary=self.t/'tests';self.binary.write_bytes(b'\x7fELFfixture');shutil.copy2(self.binary,self.a/'client/large-unit-tests')
        ledger=self.p/'ledger-before-second-lib.json';ledger.write_text(json.dumps(dict(binary=str(self.binary),sha256=digest(self.binary),source=self.old,pilot=c.OLD_PILOT,canonical_summary_sha256=digest(canonical),first=dict(passed=2596,failed=0,ignored=72,filtered=0))))
        shutil.copy2(ledger,self.a/'client/ledger-before-second-lib.json');entries.append(dict(path='client/ledger-before-second-lib.json',sha256=digest(ledger)))
        (self.a/'manifest.json').write_text(json.dumps(entries))
        fp=self.t/'debug/.fingerprint/local';fp.mkdir(parents=True);self.fp=fp/'lib-local';self.fp.write_text('initial-fingerprint')
        self.cfg=dict(root=str(self.r),target=str(self.t),worktree=str(self.w),app=self.app)
        transition=self.r/'transition';transition.mkdir()
        for name in ['frozen-source.json','new-source.json','transition.json']:(transition/name).write_text('{}')
        self.patches=[patch.object(source_proof,'OLD_APP',self.old),patch.object(source_proof,'TRANSITION',transition),patch.object(source_proof,'verify_archive'),patch.object(c,'CANONICAL_SHA',digest(canonical)),patch.object(c,'MANIFEST_SHA',digest(self.a/'manifest.json')),patch.object(c,'OLD_ELF_SHA',digest(self.binary)),patch.object(c.subprocess,'run',return_value=types.SimpleNamespace(returncode=1,stdout='',stderr=''))]
        for p in self.patches:p.start()
    def tearDown(self):
        for p in reversed(self.patches):p.stop()
        self.directory.cleanup()
    def capture(self):return c.capture(self.cfg,digest,lambda root,*args:self.app if args[0]=='rev-parse' else '')
    def test_real_capture_accepts_terminated_same_owner_and_archived_elf(self):
        row=self.capture();self.assertNotIn('error',row);c.verify(row,row)
    def test_fingerprint_change_during_poste_wait_refused(self):
        baseline=self.capture();self.fp.write_text('changed');fresh=self.capture()
        with self.assertRaises(RuntimeError):c.verify(fresh,baseline)
    def test_prior_authority_reactivation_refused(self):
        (self.scope/'autorite.json').write_text(json.dumps(dict(etat='actif',echeance=1186772.020708913)))
        row=self.capture();self.assertIn('error',row)
    def test_prior_elf_change_refused(self):
        self.binary.write_bytes(b'changed')
        self.assertIn('error',self.capture())
    def test_prior_evidence_change_refused(self):
        (self.p/'composed-result.json').write_text('{"later":true}')
        self.assertIn('error',self.capture())
    def test_foreign_buildrs_readonly_probe_refused(self):
        with patch.object(c.subprocess,'run',return_value=types.SimpleNamespace(returncode=0,stdout='foreign-source',stderr='')):self.assertIn('error',self.capture())
if __name__=='__main__':unittest.main(failfast=True)
