"""Synthetic receipt cases only: no engine, compiler, database or systemd."""
import tempfile
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

from receipt import BINDER, CASE, ENGINE, qualify_red

MARKER = 'rag3db-build-'+ENGINE+'-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
MESSAGE = ('Value of: result->isSuccess()\n  Actual: false\nExpected: true\n'
           '[check: rows-outside-graph-is-readable] '+BINDER)


class RedReceipt(unittest.TestCase):
    def check(self, message=MESSAGE, code=1, name=CASE, extra=None, stdout=None):
        with tempfile.TemporaryDirectory(prefix='rows-red-receipt-') as tmp:
            root = Path(tmp)
            xml = ET.Element('testsuites', errors='0')
            suite = ET.SubElement(xml, 'testsuite')
            cls, case_name = name.rsplit('.', 1)
            case = ET.SubElement(suite, 'testcase', classname=cls, name=case_name, status='run')
            ET.SubElement(case, 'failure').text = message
            if extra is not None:
                ET.SubElement(case, extra).text = 'unexpected extra outcome'
            ET.ElementTree(xml).write(root/'result.xml')
            (root/'stdout.log').write_text(stdout if stdout is not None else
                                           'BUILD_ID='+MARKER+'\n'+BINDER+'\n')
            return qualify_red(root/'result.xml', root/'stdout.log', code, MARKER)

    def test_expected_binder_red_is_not_product_green(self):
        result = self.check()
        self.assertEqual(result['status'], 'EXPECTED_BINDER_RED_OBSERVED')
        self.assertIs(result['product_green'], False)
        self.assertIs(result['retry'], False)

    def test_missing_column_is_required_not_a_setup_error(self):
        for message in ['[check: setup] extension absent', 'pool full\n'+MESSAGE,
                        MESSAGE+'\nsecond error', MESSAGE.replace('rows_outside_graph', 'other')]:
            with self.subTest(message=message), self.assertRaises(ValueError):
                self.check(message=message)

    def test_native_success_crash_or_wrong_case_cannot_qualify(self):
        for code in [0, -11, 124]:
            with self.subTest(code=code), self.assertRaises(ValueError):
                self.check(code=code)
        with self.assertRaises(ValueError):
            self.check(name='OtherTest.OtherCase')

    def test_extra_failure_skip_or_error_is_refused(self):
        for extra in ['failure', 'skipped', 'error']:
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                self.check(extra=extra)

    def test_native_identity_must_be_exact_and_once(self):
        for stdout in [BINDER, 'BUILD_ID='+MARKER+'\nBUILD_ID='+MARKER+'\n'+BINDER,
                       'BUILD_ID='+MARKER+'\n'+BINDER+'\n'+MARKER.replace(ENGINE, 'f'*40)]:
            with self.subTest(stdout=stdout), self.assertRaises(ValueError):
                self.check(stdout=stdout)

    def test_source_location_is_accepted_but_arbitrary_prefix_is_not(self):
        result = self.check(message='/private/source/test/transaction/vector_index_diagnostics_test.cpp:72\n'+MESSAGE)
        self.assertEqual(result['case'], CASE)
        with self.assertRaises(ValueError):
            self.check(message='pool full\n/private/source/test/transaction/vector_index_diagnostics_test.cpp:72\n'+MESSAGE)


if __name__ == '__main__':
    unittest.main(failfast=True)
