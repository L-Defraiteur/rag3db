#!/usr/bin/env python3
"""**Un serveur de langage, oracle de mesure de nos arêtes** — jamais dans le produit (décision de
Lucie, 25 octobre 2026 : un compilateur à chaque coup est trop lourd pour nous).

Un serveur par langage, déclaré dans `SERVEURS` : clangd, rust-analyzer,
typescript-language-server, pyright, gopls, csharp-ls. Pour un échantillon tiré au hasard
(graine fixe) de nos arêtes CONSUMES d'un langage, le serveur dit où va le nom à sa ligne
d'usage (définition, puis déclaration), et le verdict tombe :
- juste : une des positions rendues désigne la cible (voir `meme_entite`) ;
- faux : le serveur désigne autre chose ;
- décl : il désigne la mention même — c'est une déclaration (un en-tête qui déclare ce que la
  cible définit), pas un usage ;
- muet : il ne rend rien (macro, code non compilé, nom absent de la ligne).
Les verdicts se ventilent par marque `resolution` : `nom`, `dossier` et `ambigu` sont nos arêtes
douteuses.

Le juge, `meme_entite`, ne compare pas des positions mais des entités : une position rendue vaut
la cible si elle y tombe, ou dans un de nos scopes de même nom et de même parent, ou dans la
classe parente de la cible (la déclaration d'un en-tête), ou sur la déclaration d'une fonction
libre de même nom. clangd sans index ne voit que l'unité de compilation ouverte : il rend la
déclaration d'en-tête d'une méthode définie ailleurs.

Rien ne s'écrit dans l'arbre lu : clangd tourne sans index d'arrière-plan. Les serveurs qui
indexent un projet (rust-analyzer, pyright, gopls, tsserver) lisent une copie de l'arbre faite
par reflink, et la vérification de rust-analyzer ne s'y lance pas.

Entrées, extraites par la sonde (tests/sonde_aretes_par_relation.rs) :
- les arêtes : SONDE_REQUETE="MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE <extensions> RETURN
  a.file_path, r.line, b.name, b.file_path, b.start_line, b.end_line, r.resolution, a.name,
  b.parent_name" ;
- les scopes : « MATCH (s:Scope) WHERE <extensions> RETURN s.name, s.parent_name, s.file_path,
  s.start_line, s.end_line, s.scope_type ».

Usage : oracle_lsp.py <langage> <aretes.txt> <scopes.txt> <racine de l'arbre> <racine du projet>
        <taille de l'échantillon> <sortie.tsv> [dossier de compile_commands.json, pour cpp]
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

# langage → (commande, identifiant LSP par extension, options d'initialisation, indexe le projet)
SERVEURS = {
    "cpp": (["clangd", "--compile-commands-dir={cc}", "--background-index=false", "--log=error", "-j=6",
             "--pch-storage=memory", "--header-insertion=never"], {"": "cpp"}, {}, False),
    "rust": (["rust-analyzer"], {"": "rust"},
             # Toutes les features : un module sous `cfg(feature = "daemon")` est sinon hors du graphe
             # des crates, et chaque question y rend un muet (144 sur 300 le 25 octobre 2026).
             {"checkOnSave": False, "check": {"command": "check"}, "cachePriming": {"enable": False},
              "cargo": {"features": "all", "buildScripts": {"enable": False}}, "procMacro": {"enable": False}}, True),
    "typescript": (["typescript-language-server", "--stdio"],
                   {".tsx": "typescriptreact", ".jsx": "javascriptreact", ".js": "javascript", ".mjs": "javascript", ".cjs": "javascript", "": "typescript"},
                   # Un TypeScript 5 : la 7 (le portage natif) n'a plus de tsserver.js, et le serveur refuse alors
                   # de s'initialiser. ORACLE_TSSERVER désigne son lib/ ; sinon celui du projet.
                   {"tsserver": {"path": os.environ["ORACLE_TSSERVER"]}} if os.environ.get("ORACLE_TSSERVER") else {}, True),
    "python": (["pyright-langserver", "--stdio"], {"": "python"}, {}, True),
    "go": (["gopls"], {"": "go"}, {}, True),
    "csharp": (["csharp-ls"], {"": "csharp"}, {}, True),
}


def lire_lignes_sonde(chemin):
    for l in open(chemin, encoding="utf-8", errors="replace"):
        if not l.startswith("ligne\t"):
            continue
        brut = l[len("ligne\t"):].strip()
        # [String("a"), Int(3), Null, ...] → liste Python
        brut = re.sub(r'String\((".*?")\)', r"\1", brut)
        brut = re.sub(r"Int\((-?\d+)\)", r"\1", brut).replace("Null", "None")
        try:
            yield ast.literal_eval(brut)
        except Exception:
            continue


def lire_aretes(chemin):
    aretes = []
    for v in lire_lignes_sonde(chemin):
        if len(v) < 9 or v[1] is None or v[4] is None:
            continue
        aretes.append(dict(de=v[0], ligne=int(v[1]), nom=v[2], vers=v[3], debut=int(v[4]),
                           fin=int(v[5] if v[5] is not None else v[4]), marque=v[6] or "", appelant=v[7], parent=v[8] or ""))
    return aretes


def lire_scopes(chemin):
    """Nos scopes, par fichier : (début, fin, nom, parent)."""
    par_fichier = defaultdict(list)
    for v in lire_lignes_sonde(chemin):
        if len(v) < 5 or v[3] is None:
            continue
        par_fichier[v[2]].append((int(v[3]), int(v[4] if v[4] is not None else v[3]), v[0] or "", v[1] or ""))
    return par_fichier


class Serveur:
    def __init__(self, commande, racine_projet, options):
        self.p = subprocess.Popen(commande, cwd=racine_projet, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.n = 0
        self.reponses = {}
        self.options = options
        # Les travaux annoncés par `$/progress` (l'indexation du projet) : en cours, et le dernier signe.
        self.en_cours = set()
        self.progres_vu = False
        self.dernier_signe = time.time()
        self.cv = threading.Condition()
        threading.Thread(target=self._lire, daemon=True).start()
        uri = f"file://{racine_projet}"
        init = self.appel("initialize", {"processId": os.getpid(), "rootUri": uri, "workspaceFolders": [{"uri": uri, "name": "oracle"}],
                                  "capabilities": {"textDocument": {"definition": {"linkSupport": True}, "declaration": {"linkSupport": True}}},
                                  "initializationOptions": options}, delai=300)
        # Un serveur qui refuse de s'initialiser rendrait des muets partout : on s'arrête en le disant.
        if init is None or "error" in init:
            raise SystemExit(f"{commande[0]} ne s'initialise pas : {init and init['error'].get('message')}")
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
            elif "id" in corps and "method" in corps:
                # Une requête du serveur (workspace/configuration, window/workDoneProgress/create) :
                # une réponse vide, qu'il n'attende pas.
                # workspace/configuration reçoit nos options : une réponse nulle ramènerait rust-analyzer
                # à ses défauts (sans les features).
                self._envoyer({"jsonrpc": "2.0", "id": corps["id"], "result": [self.options] * len(corps.get("params", {}).get("items", [])) if corps["method"] == "workspace/configuration" else None})
            elif corps.get("method") == "$/progress":
                v = corps.get("params", {}).get("value", {})
                with self.cv:
                    self.progres_vu = True
                    self.dernier_signe = time.time()
                    if v.get("kind") == "begin":
                        self.en_cours.add(str(corps["params"].get("token")))
                    elif v.get("kind") == "end":
                        self.en_cours.discard(str(corps["params"].get("token")))
                    self.cv.notify_all()

    def attendre_le_repos(self, calme=30, plafond=1800):
        """Attendre que le serveur ait fini d'indexer : un travail annoncé par `$/progress`, puis aucun en
        cours depuis `calme` secondes. Un serveur qui n'annonce rien en `calme` secondes est tenu prêt."""
        t0 = time.time()
        with self.cv:
            while time.time() < t0 + plafond:
                repos = not self.en_cours and time.time() - self.dernier_signe >= calme
                if repos and (self.progres_vu or time.time() - t0 >= calme):
                    return time.time() - t0
                self.cv.wait(2)
        return None

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

    def ouvrir(self, chemin, langue):
        if chemin in self.ouverts:
            return
        texte = open(chemin, encoding="utf-8", errors="replace").read()
        self.notifier("textDocument/didOpen", {"textDocument": {"uri": f"file://{chemin}", "languageId": langue, "version": 1, "text": texte}})
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


def meme_entite(f, l, a, racine, scopes):
    """**La position rendue désigne-t-elle notre cible ?** Oui si elle tombe dans la cible ; ou dans
    un scope de même nom et de même parent (la déclaration d'une définition faite ailleurs) ; ou,
    pour une méthode déclarée dans le corps de sa classe, dans la classe parente de la cible ; ou
    sur la déclaration d'une fonction libre de même nom, à la portée d'un fichier."""
    rel = os.path.relpath(f, racine)
    if rel == a["vers"] and a["debut"] <= l <= a["fin"]:
        return True
    contenants = sorted((s for s in scopes.get(rel, []) if s[0] <= l <= s[1]), key=lambda s: s[1] - s[0])
    debut, fin, nom, parent = contenants[0] if contenants else (0, 0, "", "")
    if nom == a["nom"] and parent == a["parent"]:
        return True
    if a["parent"]:
        return nom == a["parent"]
    if not contenants or nom.startswith("file_scope") or nom == "":
        try:
            texte = open(f, encoding="utf-8", errors="replace").read().split("\n")[l - 1]
        except Exception:
            return False
        return re.search(rf"\b{re.escape(a['nom'])}\s*\(", texte) is not None
    return False


def langue_de(table, chemin):
    for ext, langue in table.items():
        if ext and chemin.endswith(ext):
            return langue
    return table[""]


def main():
    langage, aretes_txt, scopes_txt = sys.argv[1], sys.argv[2], sys.argv[3]
    racine, projet = os.path.realpath(sys.argv[4]), os.path.realpath(sys.argv[5])
    taille, sortie = int(sys.argv[6]), sys.argv[7]
    cc = sys.argv[8] if len(sys.argv) > 8 else ""
    commande, langues, options, indexe = SERVEURS[langage]
    commande = [c.replace("{cc}", cc) for c in commande]
    scopes = lire_scopes(scopes_txt)
    aretes = lire_aretes(aretes_txt)
    random.seed(25)
    echantillon = random.sample(aretes, min(taille, len(aretes)))
    par_fichier = defaultdict(list)
    for a in echantillon:
        par_fichier[a["de"]].append(a)
    s = Serveur(commande, projet, options)
    if indexe:
        attente = s.attendre_le_repos()
        print(f"{langage} : indexation " + (f"finie en {attente:.0f} s" if attente is not None else "pas finie au plafond (30 min)"), flush=True)
    verdicts = Counter()
    par_marque = defaultdict(Counter)
    t0 = time.time()
    pret = not indexe
    with open(sortie, "w") as out:
        out.write("verdict\tmarque\tappelant\tfichier:ligne\tnom\tnotre cible\tserveur\n")
        for fichier, liste in par_fichier.items():
            chemin = os.path.join(racine, fichier)
            if not os.path.exists(chemin):
                continue
            lignes = open(chemin, encoding="utf-8", errors="replace").read().split("\n")
            s.ouvrir(chemin, langue_de(langues, chemin))
            for a in liste:
                verdict, rendu = "muet", []
                if 1 <= a["ligne"] <= len(lignes):
                    m = re.search(rf"\b{re.escape(a['nom'])}\b", lignes[a["ligne"] - 1])
                    if m:
                        # Un serveur qui indexe le projet ne répond qu'une fois prêt : la première
                        # question se repose jusqu'à une réponse, dix minutes au plus pour toute la
                        # passe ; ensuite, une réponse vide se repose deux fois, à dix secondes,
                        # puisqu'un serveur qui a répondu une fois peut indexer encore.
                        essais = 0
                        while True:
                            rendu = []
                            for methode in ("textDocument/definition", "textDocument/declaration"):
                                rendu += s.positions(methode, chemin, a["ligne"] - 1, m.start())
                            if rendu or not indexe:
                                break
                            if not pret and time.time() < t0 + 600:
                                time.sleep(5)
                                continue
                            essais += 1
                            if essais > 2:
                                break
                            time.sleep(10)
                        pret = True
                        if rendu:
                            if any(meme_entite(f, l, a, racine, scopes) for f, l in rendu):
                                verdict = "juste"
                            elif all(f == chemin and l == a["ligne"] for f, l in rendu):
                                verdict = "décl"
                            else:
                                verdict = "faux"
                verdicts[verdict] += 1
                par_marque[a["marque"]][verdict] += 1
                rendus = ";".join(f"{os.path.relpath(f, racine)}:{l}" for f, l in rendu[:2])
                out.write(f"{verdict}\t{a['marque']}\t{a['appelant']}\t{a['de']}:{a['ligne']}\t{a['nom']}\t{a['vers']}:{a['debut']}-{a['fin']}\t{rendus}\n")
            s.fermer(chemin)
            out.flush()
    print(f"{langage} : {sum(verdicts.values())} arêtes en {time.time() - t0:.0f} s : {dict(verdicts)}")
    for marque, v in sorted(par_marque.items()):
        print(f"  marque « {marque or '(aucune)'} » : {dict(v)}")
    s.p.terminate()


if __name__ == "__main__":
    main()
