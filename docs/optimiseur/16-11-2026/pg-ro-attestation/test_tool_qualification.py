"""Faux préparés avant runner : AUCUNE exécution autorisée par ce fichier."""
import copy
import unittest
import tool_qualification as tool


class FakeBackend:
    def __init__(self, bad=None):
        self.calls=[];self.bad=bad
    def run(self, request):
        self.calls.append(copy.deepcopy(request))
        result=dict(name=request['name'],returncode=request['expected_code'],timed_out=False,
                    quiet=True,raw_published=True,within_whole=True,stdin_started_inside_whole=True,
                    native_invoked=True,stdout=b'{"request_valid":true,"pg_contact":false}\n',stderr=request['expected_stderr'].encode())
        if request['name']==self.bad:
            result['returncode']=127
        return result


class QualificationFakes(unittest.TestCase):
    def setUp(self):
        self.cases=[dict(name='case-'+str(i),argv=['--validate-request'],stdin='{}',returncode=1,stderr_contains='refused') for i in range(8)]
        self.config=dict(compile_argv=['PINNED_RUSTC'],client_path='/fixture/new-output/client',
                         compile_cap=60,worker_cap=5,sql_authorized=False,docker_authorized=False,
                         cases=self.cases)
    def test_single_compile_then_eight_cases(self):
        backend=FakeBackend();results=tool.orchestrate(self.config,backend)
        self.assertEqual(len(results),9)
        self.assertEqual([x['name'] for x in backend.calls].count('compile'),1)
        self.assertEqual(len(backend.calls),9)
    def test_link_failure_never_retries_or_runs_cases(self):
        backend=FakeBackend('compile')
        with self.assertRaises(ValueError):tool.orchestrate(self.config,backend)
        self.assertEqual([x['name'] for x in backend.calls],['compile'])
    def test_first_case_failure_stops_following_cases(self):
        backend=FakeBackend('case-0')
        with self.assertRaises(ValueError):tool.orchestrate(self.config,backend)
        self.assertEqual(len(backend.calls),2)
    def test_whole_deadline_covers_raw_publication_and_stdin(self):
        for key in ['quiet','raw_published','within_whole','stdin_started_inside_whole']:
            backend=FakeBackend();original=backend.run
            def run(request,key=key):
                result=original(request);result[key]=False;return result
            backend.run=run
            with self.assertRaises(ValueError):tool.orchestrate(self.config,backend)
    def test_capture_mode_or_contacts_are_refused(self):
        for key in ['sql_authorized','docker_authorized']:
            config=copy.deepcopy(self.config);config[key]=True
            with self.assertRaises(ValueError):tool.orchestrate(config,FakeBackend())
        config=copy.deepcopy(self.config);config['cases'][0]['argv']=['--capture-readonly']
        with self.assertRaises(ValueError):tool.orchestrate(config,FakeBackend())
    def test_worker5_is_not_inner4_and_cannot_be_extended(self):
        config=copy.deepcopy(self.config);config['worker_cap']=6
        with self.assertRaises(ValueError):tool.orchestrate(config,FakeBackend())
    def test_readiness_unknown_cannot_execute(self):
        with self.assertRaises(ValueError):tool.validate_go(dict(run_authorized=False),{},'0'*64,now=100.)
    def test_timeout_or_unknown_quiescence_is_not_success(self):
        for key in ['timed_out','quiet']:
            backend=FakeBackend();original=backend.run
            def run(request,key=key):
                result=original(request);result[key]=(key=='timed_out');return result
            backend.run=run
            with self.assertRaises(ValueError):tool.orchestrate(self.config,backend)


if __name__=='__main__':unittest.main(verbosity=2)
