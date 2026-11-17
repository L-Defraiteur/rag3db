"""Witnesses for R1/R2 against the actual driver's AST; no imported driver or OS child.

Prepared only. Running even these hermetic witnesses needs its distinct allocation.
"""
import ast
import io
import json
import math
from pathlib import Path
import re
import time
from types import SimpleNamespace
import unittest

DRIVER = Path(__file__).with_name('qualification_driver.py')
FILETS = ['chaque_suite_est_montee', 'aucune_suite_ne_se_relance_par_son_nom_court']
TTY = ['e2e_tty::' + name for name in [
    'le_flux_coupe_au_milieu_d_un_run_de_l_equipier_se_reprend_sans_perte_ni_doublon',
    'describe_porte_la_politique_de_chaque_outil',
    'le_code_d_un_bloc_se_colore_par_les_spans_de_l_hote',
    'le_terminal_accorde_une_commande_que_l_equipier_attend']]


def actual(name, namespace):
    tree = ast.parse(DRIVER.read_text(), filename=str(DRIVER))
    node, = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == name]
    exec(compile(ast.Module(body=[node], type_ignores=[]), str(DRIVER), 'exec'), namespace)
    return namespace[name]


def block(header, names):
    return '\n'.join([header, f'running {len(names)} tests',
        *[f'test {name} ... ok' for name in names],
        f'test result: ok. {len(names)} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s', ''])


class MemoryPath:
    files = {}

    def __init__(self, name):
        self.name = name

    def __truediv__(self, name):
        return MemoryPath(self.name + '/' + name)

    def __str__(self):
        return self.name

    def write_text(self, value):
        self.files[self.name] = value

    def read_text(self):
        return self.files[self.name]

    def open(self, mode):
        owner = self

        class Writer(io.StringIO):
            def close(self):
                owner.write_text(self.getvalue())
                super().close()

        return Writer()


class ReviewWitnesses(unittest.TestCase):
    def parser(self):
        return actual('results', dict(re=re, INVENTORIES={'e2e_tty': [FILETS, TTY]}))

    def test_full_filets_then_tty_is_a_positive_control(self):
        raw = block('Running tests/e2e_filets.rs', FILETS) + block('Running tests/e2e_tty.rs', TTY)
        self.parser()(raw, passed=4)

    def test_positive_filets_cannot_mask_zero_test_suite(self):
        raw = block('Running tests/e2e_filets.rs', FILETS) + block('Running tests/e2e_lectures.rs', [])
        with self.assertRaises(AssertionError):
            self.parser()(raw)

    def test_four_other_names_cannot_qualify_tty(self):
        raw = block('Running tests/e2e_filets.rs', FILETS) + block('Running tests/e2e_tty.rs', ['foreign::a', 'foreign::b', 'foreign::c', 'foreign::d'])
        with self.assertRaises(AssertionError):
            self.parser()(raw, passed=4)

    def test_foreign_filets_cannot_precede_the_right_tty_names(self):
        raw = block('Running tests/e2e_filets.rs', ['foreign::first', 'foreign::second']) + block('Running tests/e2e_tty.rs', TTY)
        with self.assertRaises(AssertionError):
            self.parser()(raw, passed=4)

    def test_duplicate_name_cannot_replace_a_missing_test(self):
        raw = block('Running tests/e2e_filets.rs', FILETS) + block('Running tests/e2e_tty.rs', [TTY[0], TTY[0], *TTY[2:]])
        with self.assertRaises(AssertionError):
            self.parser()(raw, passed=4)

    def test_suite_cannot_run_before_its_filets(self):
        raw = block('Running tests/e2e_tty.rs', TTY) + block('Running tests/e2e_filets.rs', FILETS)
        with self.assertRaises(AssertionError):
            self.parser()(raw, passed=4)

    def test_should_panic_text_is_not_a_native_failure(self):
        stopped = []
        MemoryPath.files = {}
        raw = "thread 'expected' panicked at fixture.rs:1:1:\nexpected panic\ntest expected - should panic ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n"

        def backend_run(command, *, stdout, **kwargs):
            stdout.write(raw)
            return SimpleNamespace(returncode=0)

        stage = actual('stage', dict(p=MemoryPath('evidence'), a={'end': time.monotonic()+600},
            time=time, math=math, json=json, subprocess=SimpleNamespace(run=backend_run, STDOUT=-2),
            request_common_stop=lambda *args, **kwargs: stopped.append(True)))
        self.assertEqual(stage('tty-lib', ['fake-native'], MemoryPath('source'), {}, 240), raw)
        self.assertEqual(stopped, [])

    def test_native_failure_stops_before_backend_continuation(self):
        stopped, continuation = [], []
        MemoryPath.files = {}

        def stop(*args, **kwargs):
            stopped.append('own Common stop requested')

        def backend_run(command, *, stdout, **kwargs):
            stdout.write('running 1 test\ntest actual_test ... FAILED\n')
            stdout.flush()
            continuation.append(not bool(stopped))
            return SimpleNamespace(returncode=101)

        stage = actual('stage', dict(p=MemoryPath('evidence'), a={'end': time.monotonic()+600},
            time=time, math=math, json=json,
            subprocess=SimpleNamespace(run=backend_run, STDOUT=-2),
            request_common_stop=stop))
        with self.assertRaises(AssertionError):
            stage('tty-lib', ['fake-native'], MemoryPath('source'), {}, 240)
        self.assertTrue(stopped, 'no stop requested while the backend was still executing')
        self.assertEqual(continuation, [False], 'native code continued after its first observed FAILED')


if __name__ == '__main__':
    unittest.main(verbosity=2)
