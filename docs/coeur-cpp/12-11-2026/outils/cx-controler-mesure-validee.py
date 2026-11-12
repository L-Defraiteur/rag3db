#!/usr/bin/env python3
"""Seule mesure autorisée ; garde ELF exacte, ancien contrôleur inchangé, pas de retry."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
root = Path(__file__).resolve().parent
p = argparse.ArgumentParser()
p.add_argument("--unit", required=True)
args = p.parse_args()
try:
    original = root / "cx-controler-pilote.py"
    assert hashlib.sha256(original.read_bytes()).hexdigest() == "119db4c82313e1c3e908f90f3d9f7db4513c466523611183eac4dcbfec9a7260"
    assert not (root / "mesure-tentative.json").exists()
    assert not (root / "premier-echec-mesure.json").exists()
    assert not list(root.glob("cellule-*.stdout"))
    receipt = json.loads((root / "compile-manifest.json").read_text())
    assert receipt["requalification_metadata_only"] is True and receipt["compiler_rerun"] is False
    assert receipt["binary_sha256"] == "0c868053afea43df3cb2eb5fe8e77f597701b8ab4eb2a0c403ca854ef64cf704"
    assert len(receipt["needed_system_libraries"]) == 5
    assert set(receipt["needed_system_libraries"]) == {"libstdc++.so.6", "libm.so.6", "libgcc_s.so.1", "libc.so.6", "ld-linux-x86-64.so.2"}
    initial = hashlib.sha256((root / "premier-echec.json").read_bytes()).hexdigest()
    assert initial == receipt["initial_refusal_preserved_sha256"]
    (root / "mesure-tentative.json").write_text(json.dumps({"unit": args.unit, "binary_sha256": receipt["binary_sha256"]}) + "\n")
    spec = importlib.util.spec_from_file_location("cx_original_controller", original)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)  # Ne lance pas le bloc __main__ de préparation.
    sys.argv = [str(original), "measure", "--unit", args.unit]  # Jamais compile.
    module.main()
    assert hashlib.sha256((root / "premier-echec.json").read_bytes()).hexdigest() == initial
except Exception as error:
    (root / "premier-echec-mesure.json").write_text(json.dumps({"error": type(error).__name__, "detail": str(error)}, ensure_ascii=False) + "\n")
    print("STOP_PREMIERE_MESURE_SANS_RETRY", type(error).__name__, str(error), flush=True)
    raise SystemExit(2)
