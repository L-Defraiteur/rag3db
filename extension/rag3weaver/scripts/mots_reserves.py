#!/usr/bin/env python3
"""Les mots réservés du dialecte rag3db, tirés de la grammaire du moteur.

Un mot réservé est un mot-clé de `src/antlr4/keywords.txt` que la règle
`kU_NonReservedKeywords` de `src/antlr4/Cypher.g4` n'accepte pas comme nom.
Ce script écrit `src/mots_reserves_rag3db.rs`, commité ; le test
`dialect::tests::les_mots_reserves_suivent_la_grammaire` rougit s'il diverge
de la grammaire. À relancer après un changement de grammaire :

    python3 extension/rag3weaver/scripts/mots_reserves.py
"""
import re
from pathlib import Path

CRATE = Path(__file__).resolve().parent.parent
ANTLR = CRATE.parent.parent / "src" / "antlr4"


def mots_reserves():
    mots = [l.strip().upper() for l in (ANTLR / "keywords.txt").read_text().splitlines() if l.strip()]
    g4 = (ANTLR / "Cypher.g4").read_text()
    regle = re.search(r"^kU_NonReservedKeywords\s*:(.*?);", g4, re.S | re.M).group(1)
    libres = {m.strip().upper() for m in re.split(r"\|", regle) if m.strip()}
    return sorted(set(mots) - libres)


def main():
    mots = mots_reserves()
    lignes = ",\n".join(f'    "{m}"' for m in mots)
    (CRATE / "src" / "mots_reserves_rag3db.rs").write_text(
        "// Fichier généré par scripts/mots_reserves.py depuis la grammaire du moteur\n"
        "// (src/antlr4/keywords.txt moins kU_NonReservedKeywords de Cypher.g4).\n"
        "// Ne pas éditer à la main : relancer le script.\n\n"
        "/// Les mots que la grammaire de rag3db refuse comme nom de table, de\n"
        "/// relation ou de champ, en majuscules.\n"
        f"pub const RAG3DB_RESERVED_WORDS: &[&str] = &[\n{lignes},\n];\n"
    )
    print(f"{len(mots)} mots réservés écrits")


if __name__ == "__main__":
    main()
