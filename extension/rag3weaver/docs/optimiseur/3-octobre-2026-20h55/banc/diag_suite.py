"""Suite de l'exploration, JevK5 :
  A. le jeu du journal des chantiers (bâti par script, aucune étiquette de notre main), formulation telle quelle ;
  B. pourquoi les textes déjà rangés font baisser le résultat : un cas regardé de près, puis des variantes contrôlées.

usage : JEVK5_URL=http://127.0.0.1:7982 python diag_suite.py <étiquette>
"""
import json, math, os, sys, time

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "")
ICI = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(ICI), "jevk5-src"))
from jevk5.gguf import JevK5GGUF
from jevk5.prompt import LETTERS, prompt_text

URL = os.environ.get("JEVK5_URL", "http://127.0.0.1:8091")
TEMP = float(os.environ.get("JEVK5_TEMP", "1.367"))
client = JevK5GGUF(url=URL, temperature=TEMP)


def decider(etat, instructions, options, voir=False):
    textes = [f"{k}: {d}" for k, d in options]
    prompt = prompt_text(etat, instructions, textes)
    jetons = client._post("/tokenize", {"content": prompt, "add_special": False, "parse_special": True})["tokens"]
    out = client._post("/completion", {"prompt": jetons, "n_predict": 1, "n_probs": 40, "temperature": 0, "cache_prompt": False})
    top = [(e["token"], e["logprob"]) for e in out["completion_probabilities"][0]["top_logprobs"]]
    vus = dict(top)
    plancher = min(vus.values()) - 2.0
    lp = [vus.get(LETTERS[i], plancher) for i in range(len(textes))]
    m = max(lp); w = [math.exp((z - m) / TEMP) for z in lp]; s = sum(w)
    probs = {k: x / s for (k, _), x in zip(options, w)}
    if voir:
        return probs, {"prompt": prompt, "jetons": len(jetons), "top": top[:8]}
    return probs, len(jetons)


import torch
torch.set_num_threads(8)
from sentence_transformers import SentenceTransformer
st = SentenceTransformer("ibm-granite/granite-embedding-278m-multilingual", device="cpu")
sortie = {"url": URL}

# =================================================================== A. le journal des chantiers
J = json.load(open(os.path.join(ICI, "journal.json")))
S = J["sections"]
court = lambda x, n=160: x if len(x) <= n else x[:n].rsplit(" ", 1)[0] + "…"


def exemples(section, sauf):
    return [court(e["texte"]) for e in section["entrees"] if e is not sauf][:2]


cas = []
for s in S:  # rejoindre : l'entrée appartient à une section présente
    for e in s["entrees"][:8]:
        cas.append({"texte": court(e["texte"], 320), "attendu": s["nom"], "propose": e["en_tete"], "absents": [], "entree": e})
for s in S:  # créer : la section de l'entrée est retirée des options, et son titre est le sujet neuf proposé
    for e in s["entrees"][-3:]:
        cas.append({"texte": court(e["texte"], 320), "attendu": None, "propose": s["nom"], "absents": [s["nom"]], "entree": e})


def options_existantes(c, contexte):
    secs = [s for s in S if s["nom"] not in c["absents"]]
    # au plus quatre sujets existants, par proximité du texte ; le bon est gardé s'il existe
    E = st.encode([s["nom"] + " : " + " ".join(exemples(s, c["entree"])) for s in secs], normalize_embeddings=True)
    q = st.encode([c["texte"]], normalize_embeddings=True)[0]
    ordre = [secs[i] for i in (E @ q).argsort()[::-1]][:4]
    if c["attendu"] and c["attendu"] not in [s["nom"] for s in ordre]:
        ordre = ordre[:-1] + [next(s for s in secs if s["nom"] == c["attendu"])]
    def d(s):
        if contexte == "nom":
            return s["nom"]
        return s["nom"] + ". Texts already filed under it: " + " ; ".join(f"«{x}»" for x in exemples(s, c["entree"]))
    return [s["nom"] for s in ordre], [d(s) for s in ordre]


def jouer(forme, contexte, creer_en):
    lignes = []
    for c in cas:
        noms, descs = options_existantes(c, contexte)
        if forme == "alternative":
            instr = f"A new topic «{c['propose']}» is proposed for this text. Should the text go under that new topic, or can it join one of the existing topics?"
            opts = [(f"t{i}", "join the existing topic " + d) for i, d in enumerate(descs)]
            creer = ("new", f"create the new topic «{c['propose']}»")
        else:
            instr = "Which topic fits this text best?"
            opts = [(f"t{i}", d) for i, d in enumerate(descs)]
            creer = ("new", f"new topic «{c['propose']}» (no text has been filed under it yet)")
        opts = [creer] + opts if creer_en == "premier" else opts + [creer]
        probs, _ = decider(f"Text: {c['texte']}", instr, opts)
        k = max(probs, key=probs.get)
        tri = sorted(probs.values(), reverse=True)
        lignes.append({"texte": c["texte"][:90], "attendu": c["attendu"], "propose": c["propose"],
                       "rendu": None if k == "new" else noms[int(k[1:])], "p": tri[0], "p_second": tri[1], "p_neuf": probs["new"]})
    A = [l for l in lignes if l["attendu"]]; N = [l for l in lignes if not l["attendu"]]
    print(f"journal · {forme}, {contexte}, créer en {creer_en} : rejoint le bon {sum(l['rendu'] == l['attendu'] for l in A)}/{len(A)}, "
          f"crée à tort {sum(l['rendu'] is None for l in A)}, range ailleurs {sum(l['rendu'] not in (None, l['attendu']) for l in A)} ; "
          f"neufs : crée {sum(l['rendu'] is None for l in N)}/{len(N)}, fusionne à tort {sum(l['rendu'] is not None for l in N)}", flush=True)
    return lignes


sortie["journal"] = {"commit": J["commit"], "sections": [(s["nom"], len(s["entrees"])) for s in S], "cas": len(cas)}
for forme, contexte, creer_en in (("alternative", "deux_exemples", "dernier"), ("alternative", "deux_exemples", "premier"),
                                  ("alternative", "nom", "dernier"), ("quel_sujet", "nom", "dernier"), ("quel_sujet", "deux_exemples", "dernier")):
    sortie["journal"][f"{forme}|{contexte}|{creer_en}"] = jouer(forme, contexte, creer_en)

# =================================================================== B. pourquoi les textes déjà rangés nuisent
R = json.load(open(os.path.join(ICI, "rangement.json")))
SUJETS = R["sujets"]; PAR = {s["nom"]: s for s in SUJETS}
E = st.encode([f"{s['nom']} : {s['description']}" for s in SUJETS], normalize_embeddings=True)
RANG = {}
for t in R["textes"]:
    q = st.encode([t["texte"]], normalize_embeddings=True)[0]
    RANG[t["id"]] = [SUJETS[i]["nom"] for i in (E @ q).argsort()[::-1]]


def quatre(t):
    noms = RANG[t["id"]][:4]
    if t["sujet"] and t["sujet"] not in noms:
        noms = noms[:-1] + [t["sujet"]]
    return noms


NEUTRE = " This topic has been in use for a long time and several texts are already filed under it, on various occasions."
LONG_NEUF = " It would be created now for this text, and could later receive other texts on the same matter if some come up."


def variante(nom_variante, t):
    noms = quatre(t)
    def d(x):
        s = PAR[x]; base = f"{x} — {s['description']}"
        ex = " Texts already filed under it: " + " ; ".join(f"«{e}»" for e in s["exemples"][:2])
        if nom_variante in ("sans_textes", "sans_textes_neuf_long"):
            return base
        if nom_variante == "longueur_seule":      # même longueur, sans contenu : est-ce la longueur ?
            return base + NEUTRE
        if nom_variante == "textes_au_bon_seulement":
            return base + (ex if x == t["sujet"] else "")
        if nom_variante == "textes_aux_autres_seulement":
            return base + ("" if x == t["sujet"] else ex)
        if nom_variante == "textes_proches":      # les deux textes rangés les plus proches du texte à ranger
            q = st.encode([t["texte"]], normalize_embeddings=True)[0]
            ee = st.encode(s["exemples"], normalize_embeddings=True)
            tri = [s["exemples"][i] for i in (ee @ q).argsort()[::-1]][:2]
            return base + " Texts already filed under it: " + " ; ".join(f"«{e}»" for e in tri)
        if nom_variante == "textes_dits_exemples":  # dire que ce ne sont que des exemples
            return base + " For example, among many other texts: " + " ; ".join(f"«{e}»" for e in s["exemples"][:2])
        return base + ex
    neuf = f"new topic «{t['neuf_propose']}» (no text has been filed under it yet)"
    if nom_variante in ("neuf_long", "sans_textes_neuf_long"):
        neuf += LONG_NEUF
    opts = [(f"t{i}", d(x)) for i, x in enumerate(noms)]
    if nom_variante == "neuf_au_milieu":
        opts = opts[:2] + [("new", neuf)] + opts[2:]
    else:
        opts = opts + [("new", neuf)]
    return noms, opts


# un cas raté, de près : r01 avec deux textes par sujet
t = next(x for x in R["textes"] if x["id"] == "r01")
vues = {}
for v in ("sans_textes", "deux_textes"):
    noms, opts = variante(v, t)
    probs, vu = decider(f"Text: {t['texte']}", "Which topic fits this text best?", opts, voir=True)
    vues[v] = {"options": [k for k, _ in opts], "noms": noms, "probs": probs, **vu}
sortie["cas_de_pres"] = vues

sortie["variantes"] = {}
for v in ("sans_textes", "deux_textes", "longueur_seule", "textes_au_bon_seulement", "textes_aux_autres_seulement",
          "textes_proches", "textes_dits_exemples", "neuf_long", "sans_textes_neuf_long", "neuf_au_milieu"):
    lignes = []
    for t in R["textes"]:
        noms, opts = variante(v, t)
        probs, nj = decider(f"Text: {t['texte']}", "Which topic fits this text best?", opts)
        k = max(probs, key=probs.get)
        lignes.append({"id": t["id"], "attendu": t["sujet"], "rendu": None if k == "new" else noms[int(k[1:])], "p_neuf": probs["new"], "jetons": nj})
    sortie["variantes"][v] = lignes
    A = [l for l in lignes if l["attendu"]]; N = [l for l in lignes if not l["attendu"]]
    print(f"variante {v} : rejoint le bon {sum(l['rendu'] == l['attendu'] for l in A)}/16, crée à tort {sum(l['rendu'] is None for l in A)} ; "
          f"neufs créés {sum(l['rendu'] is None for l in N)}/8 ; P(neuf) moyen sur les textes à ranger {sum(l['p_neuf'] for l in A) / 16:.2f} ; "
          f"{sum(l['jetons'] for l in lignes) / len(lignes):.0f} jetons", flush=True)

json.dump(sortie, open(os.path.join(ICI, "resultats", f"suite-{sys.argv[1]}.json"), "w"), ensure_ascii=False, indent=1)
print("écrit")
