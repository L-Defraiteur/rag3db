import tempfile
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

from parse_receipt import (DROP_MESSAGE, DROP_QUERY, RECOVERY_FIXTURE, admit_drop_failure,
                           excluded_scope, inspect_outcomes, inspect_raw_identities, verify_attestation)


class ReceiptTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.marker = 'rag3db-build-'+'8'*40+'-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'

    def file(self, name, value):
        path = self.root/name
        path.write_text(value)
        return path

    def test_attestation_is_exact_native_output(self):
        text = 'BUILD_ID='+self.marker+'\n'
        raw, root = self.file('raw', text), self.file('root', text)
        self.assertEqual(verify_attestation(raw, root, self.marker)['bytes'], len(text))
        root.write_text(text+'edited\n')
        with self.assertRaises(ValueError):
            verify_attestation(raw, root, self.marker)

    def test_missing_or_mixed_native_identity_is_rejected(self):
        for value in ['no identity\n', 'BUILD_ID='+self.marker+'\nrag3db-build-'+'0'*40+'-Release\n']:
            raw, root = self.file('raw', value), self.file('root', value)
            with self.assertRaises(ValueError):
                verify_attestation(raw, root, self.marker)

    def test_fixture_foreign_id_stays_visible_without_rewrite(self):
        foreign = self.marker.replace('8'*8, 'f'*8, 1)
        value = ('[ RUN      ] '+RECOVERY_FIXTURE+'\n'
                 'extension vector could not be loaded at open: was built from '+foreign+
                 ', this engine from '+self.marker+'\n[       OK ] '+RECOVERY_FIXTURE+'\n')
        path = self.file('fixture', value)
        before = path.read_bytes()
        result = inspect_raw_identities(path, self.marker)
        self.assertEqual(result['intentional_refusal_fixture'][0]['identity'], foreign)
        self.assertEqual(path.read_bytes(), before)

    def test_foreign_identity_outside_fixture_is_rejected(self):
        foreign = self.marker.replace('8'*8, 'f'*8, 1)
        for value in [foreign, '[ RUN      ] '+RECOVERY_FIXTURE+'\n'+foreign]:
            with self.assertRaises(ValueError):
                inspect_raw_identities(self.file('outside', value), self.marker)

    def test_scope_union_counts_overlap_once(self):
        long = self.file('long', '# header\na\nb\n')
        probabilistic = self.file('probabilistic', 'b\nc\n')
        result = excluded_scope(long, probabilistic)
        self.assertEqual(result['union_not_run'], ['a', 'b', 'c'])
        self.assertEqual(result['intersection'], ['b'])

    def test_new_skip_is_rejected_and_only_nominal_helpers_are_admitted(self):
        root = ET.Element('testsuites', errors='0')
        case = ET.SubElement(root, 'testcase', classname='Other', name='Unexpected', status='run')
        ET.SubElement(case, 'skipped')
        path = self.root/'outcomes.xml'
        ET.ElementTree(root).write(path)
        with self.assertRaises(ValueError):
            inspect_outcomes(path, 'transaction')
        case.set('classname', 'WalReader')
        case.set('name', 'PrintAFile')
        ET.ElementTree(root).write(path)
        self.assertEqual(inspect_outcomes(path, 'transaction')['nominal_helper_skips'], ['WalReader.PrintAFile'])

    def test_f1_failure_and_empty_suite_never_become_green(self):
        path = self.root/'outcomes.xml'
        root = ET.Element('testsuites', errors='0')
        ET.ElementTree(root).write(path)
        with self.assertRaises(ValueError):
            inspect_outcomes(path, 'transaction')
        case = ET.SubElement(root, 'testcase', classname='PkLookupUnderCheckpoint', name='LookupsDuringInsertsAndCheckpointsDoNotCrash', status='run')
        ET.SubElement(case, 'failure').text = '[check: pk-lookup-reads-right] 3'
        ET.ElementTree(root).write(path)
        with self.assertRaises(ValueError):
            inspect_outcomes(path, 'transaction')

    def drop(self, message):
        case = ET.Element('testcase', classname='transaction~ddl~ddl_empty', name='DropNodeTableReclaim')
        ET.SubElement(case, 'failure').text = message
        return case

    def test_only_historical_drop_assertion_is_admitted(self):
        self.assertEqual(admit_drop_failure(self.drop(DROP_MESSAGE), DROP_QUERY)['case'],
                         'transaction~ddl~ddl_empty.DropNodeTableReclaim')
        for message in ['pool full', DROP_MESSAGE+' additional failure', DROP_MESSAGE.replace('False', 'NULL')]:
            with self.assertRaises(ValueError):
                admit_drop_failure(self.drop(message), DROP_QUERY)
        with self.assertRaises(ValueError):
            admit_drop_failure(self.drop(DROP_MESSAGE), 'other query')

    def test_drop_name_alone_never_admits_another_failure(self):
        case = self.drop(DROP_MESSAGE)
        case.set('name', 'SomethingElse')
        with self.assertRaises(ValueError):
            admit_drop_failure(case, DROP_QUERY)
        case = self.drop(DROP_MESSAGE)
        ET.SubElement(case, 'skipped')
        with self.assertRaises(ValueError):
            admit_drop_failure(case, DROP_QUERY)


if __name__ == '__main__':
    unittest.main(verbosity=2)
