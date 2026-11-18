#!/usr/bin/env python3
"""Witness of the search_code parity comparator (`parite_search_code.py`).

Two tiny relevés written by hand, in the form `tests/sonde_parite_search_code.rs` writes, judged in each
regime: identical (green), a swapped rank (red), a changed score (red under `reponses`), two different
query sets (refused), and one result replaced out of ten (green under `migration`, the set stays at 0.9).
`PARITE_SEARCH_CODE` points at another comparator: the witness is seen red against a broken copy first.
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

COMPARATOR = os.environ.get("PARITE_SEARCH_CODE", os.path.join(os.path.dirname(os.path.abspath(__file__)), "parite_search_code.py"))


def result(i, score=None, name=None):
    return {"rang": i + 1, "_uuid": f"u{i}", "file_path": f"src/f{i}.rs", "name": name or f"n{i}", "start_line": i, "content_hash": f"h{i}", "score": score if score is not None else 10.0 - i}


def releve(results, jeu=None):
    jeu = jeu or [{"requete": "q", "formes": ["sans filtre"]}]
    return {"base": "b", "forme_de_la_base": "Generations", "k": 10, "jeu": jeu, "releve": [{"requete": "q", "forme": "sans filtre", "resultats": results}]}


class ParityComparator(unittest.TestCase):
    def judge(self, before, after, regime):
        with tempfile.TemporaryDirectory() as d:
            a, b = os.path.join(d, "a.json"), os.path.join(d, "b.json")
            json.dump(before, open(a, "w")); json.dump(after, open(b, "w"))
            run = subprocess.run([sys.executable, "-I", COMPARATOR, a, b, "--regime", regime], capture_output=True, text=True)
            return run.returncode, run.stdout.strip().splitlines()[-1] if run.stdout.strip() else ""

    def test_identical_relevés_are_green_in_every_regime(self):
        r = [result(i) for i in range(10)]
        for regime in ("optimisation", "reponses", "migration"):
            self.assertEqual(self.judge(releve(r), releve(r), regime), (0, "VERT"), regime)

    def test_a_swapped_rank_is_red(self):
        r = [result(i) for i in range(10)]
        swapped = r[:]; swapped[0], swapped[1] = swapped[1], swapped[0]
        for regime in ("optimisation", "reponses"):
            self.assertEqual(self.judge(releve(r), releve(swapped), regime), (1, "ROUGE"), regime)

    def test_a_changed_score_is_red_under_reponses(self):
        r = [result(i) for i in range(10)]
        changed = [dict(x) for x in r]; changed[3]["score"] += 1e-6
        self.assertEqual(self.judge(releve(r), releve(changed), "reponses"), (1, "ROUGE"))

    def test_two_query_sets_are_refused(self):
        r = [result(i) for i in range(10)]
        other = releve(r, jeu=[{"requete": "autre", "formes": ["sans filtre"]}])
        code, _ = self.judge(releve(r), other, "optimisation")
        self.assertEqual(code, 1)

    def test_one_replaced_result_out_of_ten_stays_green_under_migration(self):
        r = [result(i) for i in range(10)]
        replaced = r[:9] + [dict(result(42), rang=10)]
        self.assertEqual(self.judge(releve(r), releve(replaced), "migration"), (0, "VERT"))


class ScoreTolerance(unittest.TestCase):
    """`--score-tolerance`: two scores are equal when they differ by less; the order is still exact."""

    def judge(self, before, after, *options):
        with tempfile.TemporaryDirectory() as d:
            a, b = os.path.join(d, "a.json"), os.path.join(d, "b.json")
            json.dump(before, open(a, "w")); json.dump(after, open(b, "w"))
            run = subprocess.run([sys.executable, "-I", COMPARATOR, a, b, *options], capture_output=True, text=True)
            return run.returncode, run.stdout.strip().splitlines()[-1] if run.stdout.strip() else run.stderr.strip()[-200:]

    def test_a_score_drift_under_the_tolerance_is_green(self):
        r = [result(i) for i in range(10)]
        drift = [dict(x) for x in r]; drift[3]["score"] += 1e-6
        self.assertEqual(self.judge(releve(r), releve(drift), "--regime", "reponses", "--score-tolerance", "1e-5"), (0, "VERT"))

    def test_without_the_option_the_drift_stays_red(self):
        r = [result(i) for i in range(10)]
        drift = [dict(x) for x in r]; drift[3]["score"] += 1e-6
        self.assertEqual(self.judge(releve(r), releve(drift), "--regime", "reponses"), (1, "ROUGE"))

    def test_a_drift_over_the_tolerance_is_red(self):
        r = [result(i) for i in range(10)]
        drift = [dict(x) for x in r]; drift[3]["score"] += 1e-3
        self.assertEqual(self.judge(releve(r), releve(drift), "--regime", "reponses", "--score-tolerance", "1e-5"), (1, "ROUGE"))

    def test_the_tolerance_never_forgives_a_swapped_rank(self):
        r = [result(i) for i in range(10)]
        swapped = r[:]; swapped[0], swapped[1] = swapped[1], swapped[0]
        self.assertEqual(self.judge(releve(r), releve(swapped), "--regime", "optimisation", "--score-tolerance", "1"), (1, "ROUGE"))


if __name__ == "__main__":
    unittest.main()
