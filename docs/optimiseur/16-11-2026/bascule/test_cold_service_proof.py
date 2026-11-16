#!/usr/bin/env python3
"""Filets CPU du parsing de la preuve, aucun accès système/service réel."""
import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('proof',Path(__file__).with_name('provide_cold_service_proof.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)


class ParsingTests(unittest.TestCase):
    def test_service_exact_loaded_inactive_pid0(self):
        self.assertEqual(m.parse_units('LoadState=loaded\nActiveState=inactive\nSubState=dead\nMainPID=0\n')['MainPID'],'0')

    def test_socket_does_not_require_a_fictitious_pid_field(self):
        self.assertEqual(len(m.parse_units('LoadState=loaded\nActiveState=inactive\nSubState=dead\n',True)),3)

    def test_empty_stdout_cannot_attest_cold_units(self):
        with self.assertRaises(m.Refusal):m.parse_units('')

    def test_duplicate_key_cannot_hide_an_active_value(self):
        with self.assertRaises(m.Refusal):m.parse_units('LoadState=loaded\nActiveState=active\nActiveState=inactive\nSubState=dead\nMainPID=0\n')

    def test_all_non_cold_values_are_refused(self):
        original='LoadState=loaded\nActiveState=inactive\nSubState=dead\nMainPID=0\n'
        for old,new in [('loaded','not-found'),('inactive','active'),('dead','running'),('MainPID=0','MainPID=9')]:
            with self.subTest(new=new),self.assertRaises(m.Refusal):m.parse_units(original.replace(old,new))

    def test_cgroup_raw_and_normalized_match_but_ambiguous_input_refuses(self):
        self.assertEqual(m.normalize_cgroup('0::/a/b'),'/a/b')
        self.assertEqual(m.normalize_cgroup('/a/b'),'/a/b')
        for value in ['0::/a\n0::/b','/a/../b','2:cpu:/a']:
            with self.subTest(value=value),self.assertRaises(m.Refusal):m.normalize_cgroup(value)


if __name__=='__main__':unittest.main(verbosity=2)
