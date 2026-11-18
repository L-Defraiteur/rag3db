#!/usr/bin/env python3
"""Witness of `vector_parity.py`: vectors of the same texts in two bases, judged by tolerance.

Two relevés in the form `structure-sonde` writes (`requetes[0].lignes`, one row per chunk:
`[String("<text hash>"), …, Float(…)…]`), written by hand: bit-identical (green), a drift of 1e-6 (green,
not bit-identical), a drift of 1e-2 (red), and no common chunk (red: nothing was compared).
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

SCRIPT = os.environ.get("VECTOR_PARITY", os.path.join(os.path.dirname(os.path.abspath(__file__)), "vector_parity.py"))


def relevé(rows):
    return {"requetes": [{"lignes": [f'[String("{h}"), String("t"), List([{", ".join(f"Float({x!r})" for x in v)}])]' for h, v in rows]}]}


class VectorParity(unittest.TestCase):
    def judge(self, a_rows, b_rows):
        with tempfile.TemporaryDirectory() as d:
            a, b = os.path.join(d, "a.json"), os.path.join(d, "b.json")
            json.dump(relevé(a_rows), open(a, "w")); json.dump(relevé(b_rows), open(b, "w"))
            run = subprocess.run([sys.executable, "-I", SCRIPT, a, b], capture_output=True, text=True)
            return run.returncode, run.stdout

    def test_bit_identical_vectors_are_green(self):
        rows = [("aa", [0.6, 0.8]), ("bb", [1.0, 0.0])]
        code, out = self.judge(rows, rows)
        self.assertEqual(code, 0); self.assertIn("bit-identical 2", out)

    def test_a_drift_under_the_tolerance_is_green_but_not_bit_identical(self):
        code, out = self.judge([("aa", [0.6, 0.8])], [("aa", [0.600001, 0.8])])
        self.assertEqual(code, 0); self.assertIn("bit-identical 0", out)

    def test_a_drift_over_the_tolerance_is_red(self):
        code, _ = self.judge([("aa", [0.6, 0.8])], [("aa", [0.61, 0.79])])
        self.assertEqual(code, 1)

    def test_no_common_chunk_is_red(self):
        code, _ = self.judge([("aa", [0.6, 0.8])], [("bb", [0.6, 0.8])])
        self.assertEqual(code, 1)


if __name__ == "__main__":
    unittest.main()
