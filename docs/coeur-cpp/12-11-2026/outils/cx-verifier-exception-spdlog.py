#!/usr/bin/env python3
"""Vérifie uniquement la règle de comparaison ; ne lance jamais le build ni un pilote."""
import importlib.util
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
source = Path(__file__).with_name("cx-build-archive-privee.py")
spec = importlib.util.spec_from_file_location("cx_private_build_review", source)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)  # main() reste inactif.
allow = module.REVIEWED_OMISSION
ref = ["/usr/bin/c++", "-D__64BIT__", "-I<SOURCE>/third_party/alp/include", allow,
       "-I<SOURCE>/third_party/utf8proc/include", "-O3"]
private = ["/usr/bin/c++", "-D__64BIT__", "-I<SOURCE>/third_party/alp/include",
           "-I<SOURCE>/third_party/utf8proc/include", "-O3"]
assert module.is_reviewed_omission(ref, private)
cases = [
    (ref, private[:-1] + ["-O2"]),
    (ref, ["/usr/bin/clang++"] + private[1:]),
    (ref, ["/usr/bin/c++", "-D__32BIT__"] + private[2:]),
    (ref, private + ["-DEXTRA=1"]),
    (ref, private[:2] + private[3:]),
    (ref, private + ["-I/autre/chemin"]),
    (ref, private[:2] + list(reversed(private[2:]))),
    (ref + [allow], private),
    ([x.replace("/spdlog", "/autre") for x in ref], private),
    (None, private),
    (ref, None),
    (ref, ref),  # Identité littérale acceptée ailleurs, pas via cette exception.
]
for number, (a, b) in enumerate(cases):
    assert not module.is_reviewed_omission(a, b), number
result = {"exact_omission_accepted": True, "other_changes_refused": len(cases),
          "no_launcher_or_compiler_executed": True}
if len(sys.argv) == 2:
    review = json.loads(Path(sys.argv[1]).read_text())
    differences = review["differences"]
    assert len(differences) == 739
    assert all(module.is_reviewed_omission(v["reference"], v["private"])
               for v in differences.values())
    mutated = dict(next(iter(differences.values())))
    mutated["private"] = mutated["private"] + ["-DUNREVIEWED=1"]
    assert not module.is_reviewed_omission(mutated["reference"], mutated["private"])
    result["real_commands_reviewed"] = len(differences)
    result["additional_macro_refused_on_real_command"] = True
print(json.dumps(result, ensure_ascii=False))
