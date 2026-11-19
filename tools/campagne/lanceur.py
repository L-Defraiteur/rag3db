#!/usr/bin/env python3
"""The launcher of the campaign of past fixes (rag3weaver-archi, 19 Nov 2026; design:
docs/rag3weaver-archi/19-11-2026/04-le-lanceur-de-la-campagne-des-correctifs-passes.md).

Lot 1: the case and the tree.
- read_case(path) -> (block, report): the « lanceur » JSON block and the section « Le signalement,
  tel qu'il était », the only thing Algareth is given;
- refusals(block, report, repo, case_path) -> [reason]: what forbids playing the case;
- build_tree(block, repo, depots, dest): the whole repository at the parent (or its `paths`),
  without any docs/ folder but docs/procedures, each submodule at its recorded commit, the files of
  `witness_files_from`, then a fresh git repository with one commit « base » and no history.
"""
import json
import os
import re
import subprocess
import sys

REPORT_TITLE = "## Le signalement, tel qu'il était"


def read_case(path):
    raise NotImplementedError


def refusals(block, report, repo, case_path):
    raise NotImplementedError


def build_tree(block, repo, depots, dest):
    raise NotImplementedError


if __name__ == "__main__":
    sys.exit("usage: lanceur.py <case.md> …")
