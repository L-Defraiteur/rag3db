#!/usr/bin/env python3
"""Politiques LIVE : fixtures CPU, aucun service, aucune CLI ou base réelle."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

HERE=Path(__file__).parent
spec=importlib.util.spec_from_file_location('live',HERE/'provide_live_cold_service_proof.py')
live=importlib.util.module_from_spec(spec);spec.loader.exec_module(live)
core=live.load_pinned(HERE/'provide_cold_service_proof.py','copy_core',live.CORE_SHA)


class LivePolicyTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='live-policy-',dir=Path.home()/'.cache')
        self.base='code-principal';self.snapshot=Path(self.temp.name)/'snapshot';self.snapshot.mkdir()
        self.guard=Path.home()/'.cache/rag3weaver-build/bascule-live-20261116/migration-offline-01'/self.base/'guard'
        canonical=str(Path.home()/'.local/share/rag3weaver'/(self.base+'.rag3db'))
        self.cold=dict(qualification='FRESH_DIAGNOSTIC_SNAPSHOT_READY',base=self.base,canonical_base=canonical,
                       snapshot_root=str(self.snapshot),source=core.SOURCE_SHA,host_sha256=core.HOST_SHA)
        self.request=dict(schema_version=1,stage='live-code-principal-01',base=self.base,target_kind='live',
                          phase='versions',nonce='a'*32,entered_monotonic=100.,requested_monotonic=101.,
                          canonical_base=canonical,target_base=canonical,source_snapshot=str(self.snapshot),
                          expected_host=dict(pid=3761669,exe_path=str(Path.home()/'.local/bin/rag3weaver-backend'),
                                             exe_sha256=core.HOST_SHA,source_sha=core.SOURCE_SHA),
                          reply_path=str(self.guard/'versions-response.json'),
                          reply_sha_path=str(self.guard/'versions-response.sha256'))
    def tearDown(self):self.temp.cleanup()
    def validate(self):live.validate_live(self.request,self.base,self.cold,self.guard,'versions',102.,core)

    def test_exact_live_policy_is_accepted(self):self.validate()
    def test_exact_code_lourd_policy_is_also_accepted(self):
        self.base='code-lourd'
        self.guard=Path.home()/'.cache/rag3weaver-build/bascule-live-20261116/migration-offline-01'/self.base/'guard'
        canonical=str(Path.home()/'.local/share/rag3weaver'/(self.base+'.rag3db'))
        self.cold.update(base=self.base,canonical_base=canonical)
        self.request.update(stage='live-code-lourd-01',base=self.base,canonical_base=canonical,target_base=canonical,
                            reply_path=str(self.guard/'versions-response.json'),
                            reply_sha_path=str(self.guard/'versions-response.sha256'))
        self.validate()
    def test_copy_policy_still_refuses_live(self):
        with self.assertRaises(core.Refusal):core.validate_request(self.request,'versions',self.guard,102.)
    def test_copy_target_cannot_be_called_live(self):
        self.request['target_base']=str(self.snapshot/'copy.rag3db')
        with self.assertRaises(core.Refusal):self.validate()
    def test_cross_base_stage_and_kind_refuse(self):
        for field,value in [('base','code-lourd'),('stage','live-code-lourd-01'),('target_kind','rehearsal')]:
            with self.subTest(field=field):
                old=copy.deepcopy(self.request);self.request[field]=value
                with self.assertRaises(core.Refusal):self.validate()
                self.request=old
    def test_wrong_fresh_snapshot_refuses(self):
        self.request['source_snapshot']=str(Path(self.temp.name))
        with self.assertRaises(core.Refusal):self.validate()
    def test_wrong_cold_receipt_and_source_refuse(self):
        for field,value in [('base','code-lourd'),('source','0'*40),('host_sha256','0'*64)]:
            with self.subTest(field=field):
                old=copy.deepcopy(self.cold);self.cold[field]=value
                with self.assertRaises(core.Refusal):self.validate()
                self.cold=old
    def test_stale_nonce_and_redirected_reply_refuse(self):
        for field,value in [('requested_monotonic',1.),('nonce','bad'),('reply_path',str(self.snapshot/'other.json'))]:
            with self.subTest(field=field):
                old=copy.deepcopy(self.request);self.request[field]=value
                with self.assertRaises(core.Refusal):self.validate()
                self.request=old


if __name__=='__main__':unittest.main(verbosity=2)
