#!/usr/bin/env python3
"""**clangd, oracle de mesure de nos arêtes C++** — jamais dans le produit (décision de Lucie,
25 octobre 2026 : un compilateur à chaque coup est trop lourd pour nous).

Entrée : les lignes `ligne\t[...]` d'une sonde à requête libre (SONDE_REQUETE), une arête
CONSUMES par ligne : fichier du mentionneur, ligne d'usage, nom de la cible, fichier de la
cible, lignes de début et de fin de la cible, marque `resolution`, nom du mentionneur,
parent de la cible. Pour un échantillon tiré au hasard (graine fixe), clangd dit où va le nom
à cette ligne (définition, puis déclaration) ; le verdict :
- juste : une des positions rendues désigne la cible (voir `meme_entite`) ;
- faux : clangd désigne autre chose ;
- décl : clangd désigne la mention même — elle est une déclaration (un en-tête qui déclare la
  fonction que la cible définit), pas un usage ;
- muet : clangd ne rend rien (macro, code non compilé, nom absent de la ligne).
Les verdicts se ventilent par marque : `nom`, `dossier` et `ambigu` sont nos arêtes douteuses.

clangd tourne sans index d'arrière-plan, et n'écrit rien dans l'arbre lu.

Le juge se lit dans `meme_entite` : une position rendue vaut la cible si elle y tombe, ou dans
un scope de même nom et de même parent, ou dans la classe parente (la déclaration d'en-tête).

L'échantillon s'extrait par la sonde (tests/sonde_aretes_par_relation.rs) :
  SONDE_REQUETE="MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE (a.file_path ENDS WITH '.cpp' OR
    a.file_path ENDS WITH '.h') AND (b.file_path ENDS WITH '.cpp' OR b.file_path ENDS WITH '.h')
    RETURN a.file_path, r.line, b.name, b.file_path, b.start_line, b.end_line, r.resolution,
    a.name, b.parent_name"
et les scopes par « MATCH (s:Scope) WHERE s.file_path ENDS WITH '.cpp' OR … RETURN s.name,
s.parent_name, s.file_path, s.start_line, s.end_line, s.scope_type ».

Usage : oracle_clangd.py <aretes.txt> <racine de l'arbre> <dossier de compile_commands.json>
        <taille de l'échantillon> <sortie.tsv> <scopes.txt>
"""
import ast
import json
import os
import random
import re
import subprocess
import sys
import threading
import time
from collections import Counter, defaultdict


def lire_aretes(chemin):
    aretes = []
    for l in open(chemin, encoding="utf-8", errors="replace"):
        if not l.startswith("ligne\t"):
            continue
        brut = l[len("ligne\t"):].strip()
        # [String("a"), Int(3), Null, ...] → liste Python
        brut = re.sub(r'String\((".*?")\)', r"\1", brut)
        brut = re.sub(r"Int\((-?\d+)\)", r"\1", brut)
        brut = brut.replace("Null", "None")
        try:
            v = ast.literal_eval(brut)
        except Exception:
            continue
        if len(v) < 9 or v[1] is None or v[4] is None:
            continue
        aretes.append(dict(de=v[0], ligne=int(v[1]), nom=v[2], vers=v[3], debut=int(v[4]),
                           fin=int(v[5] if v[5] is not None else v[4]), marque=v[6] or "", appelant=v[7], parent=v[8] or ""))
    return aretes


class Clangd:
    def __init__(self, racine, cc):
        self.p = subprocess.Popen(
            ["clangd", f"--compile-commands-dir={cc}", "--background-index=false", "--log=error", "-j=6",
             "--pch-storage=memory", "--header-insertion=never"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.n = 0
        self.reponses = {}
        self.cv = threading.Condition()
        threading.Thread(target=self._lire, daemon=True).start()
        self.appel("initialize", {"processId": os.getpid(), "rootUri": f"file://{racine}", "capabilities": {}})
        self.notifier("initialized", {})
        self.ouverts = set()

    def _envoyer(self, msg):
        b = json.dumps(msg).encode()
        self.p.stdin.write(f"Content-Length: {len(b)}\r\n\r\n".encode() + b)
        self.p.stdin.flush()

    def _lire(self):
        f = self.p.stdout
        while True:
            entete = {}
            while True:
                l = f.readline()
                if not l:
                    return
                l = l.decode().strip()
                if not l:
                    break
                k, v = l.split(":", 1)
                entete[k.lower()] = v.strip()
            corps = json.loads(f.read(int(entete["content-length"])))
            if "id" in corps and "method" not in corps:
                with self.cv:
                    self.reponses[corps["id"]] = corps
                    self.cv.notify_all()

    def appel(self, methode, params, delai=120):
        self.n += 1
        i = self.n
        self._envoyer({"jsonrpc": "2.0", "id": i, "method": methode, "params": params})
        fin = time.time() + delai
        with self.cv:
            while i not in self.reponses:
                reste = fin - time.time()
                if reste <= 0:
                    return None
                self.cv.wait(reste)
            return self.reponses.pop(i)

    def notifier(self, methode, params):
        self._envoyer({"jsonrpc": "2.0", "method": methode, "params": params})

    def ouvrir(self, chemin):
        if chemin in self.ouverts:
            return
        texte = open(chemin, encoding="utf-8", errors="replace").read()
        self.notifier("textDocument/didOpen", {"textDocument": {"uri": f"file://{chemin}", "languageId": "cpp", "version": 1, "text": texte}})
        self.ouverts.add(chemin)

    def fermer(self, chemin):
        if chemin in self.ouverts:
            self.notifier("textDocument/didClose", {"textDocument": {"uri": f"file://{chemin}"}})
            self.ouverts.discard(chemin)

    def positions(self, methode, chemin, ligne0, col):
        r = self.appel(methode, {"textDocument": {"uri": f"file://{chemin}"}, "position": {"line": ligne0, "character": col}})
        if not r or "result" not in r or not r["result"]:
            return []
        res = r["result"]
        if isinstance(res, dict):
            res = [res]
        out = []
        for x in res:
            uri = x.get("uri") or x.get("targetUri")
            rng = x.get("range") or x.get("targetSelectionRange") or x.get("targetRange")
            if uri and rng:
                out.append((uri.replace("file://", ""), rng["start"]["line"] + 1))
        return out


def lire_scopes(chemin):
    """Nos scopes C++, par fichier : (début, fin, nom, parent)."""
    par_fichier = defaultdict(list)
    if not chemin:
        return par_fichier
    for l in open(chemin, encoding="utf-8", errors="replace"):
        if not l.startswith("ligne\t"):
            continue
        brut = l[len("ligne\t"):].strip()
        brut = re.sub(r'String\((".*?")\)', r"\1", brut)
        brut = re.sub(r"Int\((-?\d+)\)", r"\1", brut).replace("Null", "None")
        try:
            v = ast.literal_eval(brut)
        except Exception:
            continue
        if v[3] is None:
            continue
        par_fichier[v[2]].append((int(v[3]), int(v[4] if v[4] is not None else v[3]), v[0] or "", v[1] or ""))
    return par_fichier


def meme_entite(f, l, a, racine, scopes):
    """**La position que rend clangd désigne-t-elle notre cible ?** Oui si elle tombe dans la
    cible ; ou dans un scope de même nom et de même parent (la déclaration d'une définition faite
    ailleurs) ; ou, pour une méthode déclarée dans le corps de sa classe, dans la classe parente
    de la cible. Sans index, clangd ne voit que l'unité de compilation ouverte : il rend la
    déclaration de l'en-tête, pas la définition d'un autre .cpp."""
    rel = os.path.relpath(f, racine)
    if rel == a["vers"] and a["debut"] <= l <= a["fin"]:
        return True
    contenants = sorted((s for s in scopes.get(rel, []) if s[0] <= l <= s[1]), key=lambda s: s[1] - s[0])
    debut, fin, nom, parent = contenants[0] if contenants else (0, 0, "", "")
    if nom == a["nom"] and parent == a["parent"]:
        return True
    if a["parent"]:
        return nom == a["parent"]
    # Une fonction libre déclarée à la portée du fichier (un en-tête) : la ligne nomme la cible.
    if not contenants or nom.startswith("file_scope") or nom == "":
        try:
            texte = open(f, encoding="utf-8", errors="replace").read().split("\n")[l - 1]
        except Exception:
            return False
        return re.search(rf"\b{re.escape(a['nom'])}\s*\(", texte) is not None
    return False


def main():
    aretes_txt, racine, cc, taille, sortie = sys.argv[1], os.path.realpath(sys.argv[2]), sys.argv[3], int(sys.argv[4]), sys.argv[5]
    scopes = lire_scopes(sys.argv[6] if len(sys.argv) > 6 else "")
    aretes = lire_aretes(aretes_txt)
    random.seed(25)
    echantillon = random.sample(aretes, min(taille, len(aretes)))
    # Par fichier du mentionneur : un fichier ouvert une fois, puis fermé.
    par_fichier = defaultdict(list)
    for a in echantillon:
        par_fichier[a["de"]].append(a)
    c = Clangd(racine, cc)
    verdicts = Counter()
    par_marque = defaultdict(Counter)
    t0 = time.time()
    with open(sortie, "w") as out:
        out.write("verdict\tmarque\tappelant\tfichier:ligne\tnom\tnotre cible\tclangd\n")
        for fichier, liste in par_fichier.items():
            chemin = os.path.join(racine, fichier)
            if not os.path.exists(chemin):
                continue
            lignes = open(chemin, encoding="utf-8", errors="replace").read().split("\n")
            c.ouvrir(chemin)
            for a in liste:
                verdict, rendu = "muet", []
                if 1 <= a["ligne"] <= len(lignes):
                    texte = lignes[a["ligne"] - 1]
                    m = re.search(rf"\b{re.escape(a['nom'])}\b", texte)
                    if m:
                        for methode in ("textDocument/definition", "textDocument/declaration"):
                            rendu += c.positions(methode, chemin, a["ligne"] - 1, m.start())
                        if rendu:
                            if any(meme_entite(f, l, a, racine, scopes) for f, l in rendu):
                                verdict = "juste"
                            elif all(f == chemin and l == a["ligne"] for f, l in rendu):
                                # clangd désigne la mention même : c'est une déclaration (un en-tête
                                # qui déclare la fonction que la cible définit), pas un usage.
                                verdict = "décl"
                            else:
                                verdict = "faux"
                verdicts[verdict] += 1
                par_marque[a["marque"]][verdict] += 1
                cl = ";".join(f"{os.path.relpath(f, racine)}:{l}" for f, l in rendu[:2])
                out.write(f"{verdict}\t{a['marque']}\t{a['appelant']}\t{a['de']}:{a['ligne']}\t{a['nom']}\t{a['vers']}:{a['debut']}-{a['fin']}\t{cl}\n")
            c.fermer(chemin)
            out.flush()
    print(f"{sum(verdicts.values())} arêtes en {time.time() - t0:.0f} s : {dict(verdicts)}")
    for marque, v in sorted(par_marque.items()):
        print(f"  marque « {marque or '(aucune)'} » : {dict(v)}")
    c.p.terminate()


if __name__ == "__main__":
    main()
