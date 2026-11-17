"""ROUGE causal proposé : ancien constructeur B, canari fictif, zéro natif.

Une FAIL assertion attendue, pas une ERROR ; ne pas inclure dans la suite verte.
"""
from contextlib import ExitStack
import unittest
import test_tool_qualification as legacy_fixtures


class LegacySecretBoundaryRed(unittest.TestCase):
    def setUp(self):legacy_fixtures.NativeBoundaryFakes.setUp(self)
    def test_legacy_b_artifacts_must_not_contain_stdin_canary(self):
        canary='G_V2_CAUSAL_CANARY_NOT_A_CREDENTIAL'
        self.request['stdin']=canary
        with ExitStack() as stack:
            legacy_fixtures.NativeBoundaryFakes.install(self,stack)
            self.backend._run(self.request)
        archived=(self.out/'compile/request.json').read_text()
        self.assertNotIn(canary,archived,'legacy B archives stdin; real credentials require a different harness')


if __name__=='__main__':unittest.main(verbosity=2)
