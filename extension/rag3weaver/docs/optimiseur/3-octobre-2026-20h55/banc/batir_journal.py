"""Bâtit un jeu de rangement qui n'est pas de notre main : les entrées du journal des chantiers
(`docs/journal-des-chantiers.md`, lu sur origin/master), rangées sous leurs titres de section.

Rien n'est étiqueté à la main : le sujet d'une entrée est le titre de la section où elle se trouve,
et le « sujet neuf proposé » pour une entrée est son en-tête en gras (ou sa première cellule).
usage : python batir_journal.py > journal.json
"""
import json, re, subprocess, sys

md = subprocess.run(["git", "-C", "/home/lucied/git_workspaces/rag3db", "show", "origin/master:docs/journal-des-chantiers.md"],
                    capture_output=True, text=True, check=True).stdout
commit = subprocess.run(["git", "-C", "/home/lucied/git_workspaces/rag3db", "rev-parse", "--short", "origin/master"],
                        capture_output=True, text=True).stdout.strip()


def propre(s):
    s = re.sub(r"`([^`]*)`", r"\1", s)
    s = re.sub(r"\*\*([^*]*)\*\*", r"\1", s)
    s = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", s)
    return re.sub(r"\s+", " ", s).strip()


sections, courant = [], None
lignes = md.split("\n")
i = 0
while i < len(lignes):
    l = lignes[i]
    m = re.match(r"^## (\d+)\. (.+)$", l)
    if m:
        courant = {"nom": propre(m.group(2)), "entrees": []}
        sections.append(courant)
    elif courant is not None:
        if l.startswith("- "):
            bloc = [l[2:]]
            while i + 1 < len(lignes) and lignes[i + 1].startswith("  ") and lignes[i + 1].strip():
                i += 1; bloc.append(lignes[i].strip())
            brut = " ".join(bloc)
            g = re.match(r"^\*\*(.+?)\*\*", brut)
            if g and len(propre(brut)) > 60:
                courant["entrees"].append({"en_tete": propre(g.group(1)).rstrip(" :.—-"), "texte": propre(brut)[:320]})
        elif l.startswith("| ") and not re.match(r"^\|\s*-", l) and not l.startswith("| non poussés"):
            cells = [c.strip() for c in l.strip().strip("|").split("|")]
            if len(cells) >= 2 and len(propre(l)) > 60 and not all(len(c) < 40 for c in cells):
                courant["entrees"].append({"en_tete": propre(cells[0])[:80].rstrip(" :.—-"), "texte": propre(" — ".join(cells))[:320]})
    i += 1

sections = [s for s in sections if len(s["entrees"]) >= 4]
json.dump({"source": "docs/journal-des-chantiers.md", "commit": commit,
           "note": "Jeu bâti par script : sujet = titre de section, sujet neuf proposé = en-tête de l'entrée. Aucune étiquette posée à la main.",
           "sections": sections}, sys.stdout, ensure_ascii=False, indent=1)
print("\n".join(f"{len(s['entrees']):3d} entrées · {s['nom']}" for s in sections), file=sys.stderr)
