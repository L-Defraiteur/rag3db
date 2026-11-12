#!/usr/bin/env python3
"""Requalification de métadonnées du même ELF, sans compilateur ni exécution du pilote."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
BINARY_SHA = "0c868053afea43df3cb2eb5fe8e77f597701b8ab4eb2a0c403ca854ef64cf704"
SOURCE_SHA = "fdc3e90feb364178537c11f2a05743fbb965851e898987288d1f69cb6eca1d08"
LOADER_SHA = "2a950d46575efa0210d22440988f5a6b4507ebddc6c86d9a42e6e7ef6f01463e"
FIVE = {"libstdc++.so.6", "libm.so.6", "libgcc_s.so.1", "libc.so.6", "ld-linux-x86-64.so.2"}


def sha(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for data in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(data)
    return h.hexdigest()


def main():
    os.environ["LC_ALL"] = "C"
    binary = ROOT / "cx-mesure-retrait"
    assert sha(binary) == BINARY_SHA
    assert sha(ROOT / "cx-mesure-retrait.cpp") == SOURCE_SHA
    assert sha(ROOT / "cx-commandes-lien-revue.json") == "b4d424cae166f90ba5d3c67aa7a0533bc579bcd17a86c1b973e250e0e618b985"
    recipe = json.loads((ROOT / "cx-commandes-lien-revue.json").read_text())
    manifest = Path(recipe["accepted_manifest"]["path"])
    assert sha(manifest) == "0049111df769914dbeea66fd0d2d07c6add58a670eb80e797589604d48c80df7"
    assert not (ROOT / "compile-manifest.json").exists()
    initial = sha(ROOT / "premier-echec.json")
    results = {}
    for phase in ("compile", "link"):
        result = json.loads((ROOT / (phase + "-result.json")).read_text())
        assert result["code"] == 0 and result["argv"] == recipe[phase + "_argv"]
        results[phase] = result
    text = subprocess.check_output(["readelf", "-W", "-h", "-d", "-l", str(binary)], text=True)
    (ROOT / "elf-requalification.txt").write_text(text)
    assert re.search(r"Class:\s+ELF64", text)
    assert re.search(r"Machine:\s+Advanced Micro Devices X86-64", text)
    needed = re.findall(r"\(NEEDED\).*?\[(.*?)\]", text)
    assert len(needed) == 5 and set(needed) == FIVE
    assert "RPATH" not in text and "RUNPATH" not in text
    match = re.search(r"Requesting program interpreter: ([^\]]+)", text)
    assert match and match.group(1) == "/lib64/ld-linux-x86-64.so.2"
    loader = Path(match.group(1)).resolve()
    assert str(loader) == "/usr/lib/ld-linux-x86-64.so.2" and sha(loader) == LOADER_SHA
    assert sha(ROOT / "premier-echec.json") == initial
    receipt = {"status": "COMPILE_LINK_VERIFIES", "requalification_metadata_only": True,
               "initial_refusal_preserved_sha256": initial, "compiler_rerun": False,
               "binary_sha256": BINARY_SHA, "source_sha256": SOURCE_SHA,
               "object_sha256": sha(ROOT / "pilote.o"), "recipe_sha256": sha(ROOT / "cx-commandes-lien-revue.json"),
               "manifest_input_sha256": sha(manifest), "needed_system_libraries": needed,
               "loader_canonical": str(loader), "loader_sha256": LOADER_SHA,
               "compile_argv": results["compile"]["argv"], "link_argv": results["link"]["argv"],
               "pilot_executed": False, "engine_tus_compiled": 0}
    (ROOT / "elf-requalification.json").write_text(json.dumps(receipt, indent=2) + "\n")
    (ROOT / "compile-manifest.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"status": "ELF_REQUALIFIE_MEME_BINAIRE", "binary_sha256": BINARY_SHA,
                      "five_exact_system_components": True, "pilot_executed": False}))


if __name__ == "__main__":
    main()
