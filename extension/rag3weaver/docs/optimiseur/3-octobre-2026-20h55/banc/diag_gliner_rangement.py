"""GLiNER2.5-multi-Decide dans la forme « rangement » (son métier : un texte, des étiquettes),
sur les mêmes 24 textes et dix sujets que JevK5, puis sur le jeu du journal. CPU, huit fils.

usage : python diag_gliner_rangement.py
"""
import json, os, statistics, sys, time

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "")
import torch
torch.set_num_threads(8)
ICI = os.path.dirname(os.path.abspath(__file__))
R = json.load(open(os.path.join(ICI, "rangement.json")))
from gliner2 import AutoExtractor
from sentence_transformers import SentenceTransformer
ext = AutoExtractor.from_pretrained("fastino/GLiNER2.5-multi-Decide")
st = SentenceTransformer("ibm-granite/granite-embedding-278m-multilingual", device="cpu")


def classer(texte, tache, etiquettes, descriptions=None):
    cfg = {"labels": list(etiquettes), "multi_label": True, "class_act": "softmax", "cls_threshold": 0.0}
    if descriptions:
        cfg["label_descriptions"] = descriptions
    r = ext.classify_text(texte, {tache: cfg}, include_confidence=True, format_results=False)
    return {k: float(v) for k, v in r[tache]}


SUJETS = R["sujets"]; PAR = {s["nom"]: s for s in SUJETS}
E = st.encode([f"{s['nom']} : {s['description']}" for s in SUJETS], normalize_embeddings=True)
RANG = {}
for t in R["textes"]:
    q = st.encode([t["texte"]], normalize_embeddings=True)[0]
    RANG[t["id"]] = [SUJETS[i]["nom"] for i in (E @ q).argsort()[::-1]]


def quatre(t, n=4):
    noms = RANG[t["id"]][:n]
    if t["sujet"] and t["sujet"] not in noms:
        noms = noms[:-1] + [t["sujet"]]
    return noms


CONFIGS = {
    # formulation 1 : quel sujet va à ce texte ? Les étiquettes sont les noms des sujets.
    "f1_nom_neuf_dernier": dict(tache="topic", contexte=None, neuf="nom", premier=False),
    "f1_nom_neuf_premier": dict(tache="topic", contexte=None, neuf="nom", premier=True),
    "f1_description": dict(tache="topic", contexte="description", neuf="nom", premier=False),
    "f1_deux_textes": dict(tache="topic", contexte="textes", neuf="nom", premier=False),
    "f1_aucun": dict(tache="topic", contexte=None, neuf="aucun", premier=False),
    "f1_dix_sujets": dict(tache="topic", contexte=None, neuf="nom", premier=False, n=10),
    # formulation 2 : l'alternative ; GLiNER n'a pas de critère, elle passe par le nom de la tâche et les étiquettes.
    "f2_creer_dernier": dict(tache="join an existing topic or create the new topic", contexte="description", neuf="creer", premier=False),
    "f2_creer_premier": dict(tache="join an existing topic or create the new topic", contexte="description", neuf="creer", premier=True),
}
sortie = {"rangement": {}}
for nom, c in CONFIGS.items():
    lignes, temps = [], []
    for t in R["textes"]:
        noms = quatre(t, c.get("n", 4))
        etiq = list(noms)
        if c["neuf"] == "aucun":
            o_neuf = "none of these topics"
        elif c["neuf"] == "creer":
            etiq = [f"join {x}" for x in noms]; o_neuf = f"create new topic {t['neuf_propose']}"
        else:
            o_neuf = t["neuf_propose"]
        tous = [o_neuf] + etiq if c["premier"] else etiq + [o_neuf]
        desc = None
        if c["contexte"]:
            desc = {}
            for lab, x in zip(etiq, noms):
                s = PAR[x]; d = s["description"]
                if c["contexte"] == "textes":
                    d += " Texts already filed under it: " + " ; ".join(s["exemples"][:2])
                desc[lab] = d
            desc[o_neuf] = "a new topic, with no text filed under it yet"
        t0 = time.perf_counter()
        probs = classer(t["texte"], c["tache"], tous, desc)
        temps.append(time.perf_counter() - t0)
        k = max(probs, key=probs.get)
        rendu = None if k == o_neuf else noms[etiq.index(k)]
        tri = sorted(probs.values(), reverse=True)
        lignes.append({"id": t["id"], "attendu": t["sujet"], "rendu": rendu, "p": tri[0], "p_second": tri[1], "p_neuf": probs[o_neuf]})
    A = [l for l in lignes if l["attendu"]]; N = [l for l in lignes if not l["attendu"]]
    sortie["rangement"][nom] = {"lignes": lignes, "ms": 1000 * statistics.median(temps)}
    print(f"gliner · {nom} : rejoint le bon {sum(l['rendu'] == l['attendu'] for l in A)}/16, crée à tort {sum(l['rendu'] is None for l in A)}, "
          f"range ailleurs {sum(l['rendu'] not in (None, l['attendu']) for l in A)} ; neufs : crée {sum(l['rendu'] is None for l in N)}/8, "
          f"fusionne à tort {sum(l['rendu'] is not None for l in N)} ; {1000 * statistics.median(temps):.0f} ms", flush=True)

# ------------------------------------------------------------------ le jeu du journal, mêmes formes
J = json.load(open(os.path.join(ICI, "journal.json"))); S = J["sections"]
court = lambda x, n=160: x if len(x) <= n else x[:n].rsplit(" ", 1)[0] + "…"
cas = []
for s in S:
    for e in s["entrees"][:8]:
        cas.append({"texte": court(e["texte"], 320), "attendu": s["nom"], "propose": e["en_tete"], "absents": []})
for s in S:
    for e in s["entrees"][-3:]:
        cas.append({"texte": court(e["texte"], 320), "attendu": None, "propose": s["nom"], "absents": [s["nom"]]})
sortie["journal"] = {}
for nom, premier in (("f1_nom_neuf_dernier", False), ("f1_nom_neuf_premier", True)):
    lignes = []
    for c in cas:
        noms = [s["nom"] for s in S if s["nom"] not in c["absents"]][:4] if not c["attendu"] else None
        if noms is None:
            noms = [s["nom"] for s in S][:5]
            noms = [x for x in noms if x != c["attendu"]][:3] + [c["attendu"]]
        tous = [c["propose"]] + noms if premier else noms + [c["propose"]]
        probs = classer(c["texte"], "topic", tous)
        k = max(probs, key=probs.get)
        lignes.append({"attendu": c["attendu"], "rendu": None if k == c["propose"] else k, "p": probs[k]})
    A = [l for l in lignes if l["attendu"]]; N = [l for l in lignes if not l["attendu"]]
    sortie["journal"][nom] = lignes
    print(f"gliner · journal · {nom} : rejoint le bon {sum(l['rendu'] == l['attendu'] for l in A)}/{len(A)}, crée à tort {sum(l['rendu'] is None for l in A)}, "
          f"range ailleurs {sum(l['rendu'] not in (None, l['attendu']) for l in A)} ; neufs : crée {sum(l['rendu'] is None for l in N)}/{len(N)}, "
          f"fusionne à tort {sum(l['rendu'] is not None for l in N)}", flush=True)

json.dump(sortie, open(os.path.join(ICI, "resultats", "gliner-rangement.json"), "w"), ensure_ascii=False, indent=1)
print("écrit")
