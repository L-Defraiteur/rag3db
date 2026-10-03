"""Mesures complémentaires pour JevK5 (4B ou 2B), sur CPU :
  1. le critère écrit en français, sur les paires françaises ;
  2. le montage à deux étages : une référence classe tous les sujets existants, JevK5 ne juge
     que les trois premiers (plus « aucun »).

usage : python banc2.py <jevk5|jevk5-2b>      (llama-server déjà lancé sur le port 8091)
"""
import json, os, statistics, sys, time

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "")
ICI = os.path.dirname(os.path.abspath(__file__))
JEU = json.load(open(os.path.join(ICI, "jeu.json")))
tag = sys.argv[1]
sys.path.insert(0, os.path.join(os.path.dirname(ICI), "jevk5-src"))
from jevk5.gguf import JevK5GGUF

client = JevK5GGUF(url=os.environ.get("JEVK5_URL", "http://127.0.0.1:8091"), temperature=1.42 if "2b" in tag else 1.367)


def decider(etat, instructions, options):
    probs, _ = client.probabilities(etat, {"type": "choice", "instructions": instructions, "criteria": options})
    return {k: float(v) for k, v in probs.items()}


def choix(s):
    return max(s, key=s.get)


sortie = {"modele": tag}

# ------------------------------------------------------------------ 1. critère en français
FR = {
    "sujets": {
        "instructions": "Ces deux noms de sujet désignent-ils le même sujet ?",
        "options": {
            "same": "les deux expressions nomment le même sujet, dit autrement ou dans une autre langue",
            "variant": "sujets liés : l'un est une partie, un mécanisme ou un cas particulier de l'autre",
            "unrelated": "deux sujets différents",
        },
        "etat": lambda p: f"Sujet A : {p['a']}\nSujet B : {p['b']}",
    },
    "memoires": {
        "instructions": "Quel est le rapport entre la nouvelle mémoire et la mémoire existante ?",
        "options": {
            "same": "la nouvelle mémoire énonce le même fait que l'existante",
            "completes": "la nouvelle mémoire est d'accord avec l'existante et ajoute une information",
            "contradicts": "la nouvelle mémoire énonce quelque chose d'incompatible avec l'existante",
            "new": "la nouvelle mémoire parle d'autre chose",
        },
        "etat": lambda p: f"Mémoire existante : {p['existante']}\nNouvelle mémoire : {p['nouvelle']}",
    },
}
sortie["critere_francais"] = {}
for nom, cfg in FR.items():
    lignes = []
    for p in JEU[nom]["paires"]:
        if p["langue"] != "fr":
            continue
        r = decider(cfg["etat"](p), cfg["instructions"], cfg["options"])
        lignes.append({"id": p["id"], "etiquette": p["etiquette"], "sortie": r})
    ok = sum(choix(l["sortie"]) == l["etiquette"] for l in lignes)
    sortie["critere_francais"][nom] = {"lignes": lignes, "justes": ok, "total": len(lignes)}
    print(f"critère en français, {nom} : {ok} sur {len(lignes)}", flush=True)

# ------------------------------------------------------------------ 2. deux étages
P = JEU["sujets"]["paires"]
existants = sorted({p["b"] for p in P})
bons = {}
for p in P:
    if p["etiquette"] == "same":
        bons.setdefault(p["a"], set()).add(p["b"])
SANS = ["couleur du thème de l'éditeur", "raccourcis clavier du terminal", "keyboard layout of the laptop",
        "pricing page of the website", "traduction de l'interface en espagnol", "battery life of the tablet"]
requetes = [(a, b) for a, b in bons.items()] + [(q, set()) for q in SANS]

import torch
torch.set_num_threads(8)
from sentence_transformers import SentenceTransformer, CrossEncoder
st = SentenceTransformer("ibm-granite/granite-embedding-278m-multilingual", device="cpu")
ce = CrossEncoder("BAAI/bge-reranker-v2-m3", device="cpu")
E = st.encode(existants, normalize_embeddings=True)  # les sujets existants sont embarqués une fois, à l'avance


def classer_cosinus(q):
    e = st.encode([q], normalize_embeddings=True)[0]
    s = E @ e
    return [existants[i] for i in s.argsort()[::-1]]


def classer_reranker(q):
    s = ce.predict([(q, x) for x in existants])
    return [existants[i] for i in s.argsort()[::-1]]


INSTR = JEU["dix"]["instructions"]
sortie["deux_etages"] = {"existants": len(existants), "requetes": len(requetes), "avec_un_double": len(bons)}
for nom, classer in (("cosinus", classer_cosinus), ("reranker", classer_reranker)):
    lignes, t1s, t2s = [], [], []
    for q, attendu in requetes:
        t = time.perf_counter(); rang = classer(q); t1 = time.perf_counter() - t
        top = rang[:3]
        options = {f"t{i}": x for i, x in enumerate(top)}
        options["none"] = JEU["dix"]["aucun"]
        t = time.perf_counter(); r = decider(f"New topic: {q}", INSTR, options); t2 = time.perf_counter() - t
        c = choix(r)
        rendu = None if c == "none" else top[int(c[1:])]
        lignes.append({"requete": q, "attendu": sorted(attendu), "trois": top, "rendu": rendu, "probabilite": r[c],
                       "dans_les_trois": bool(attendu & set(top)) if attendu else None,
                       "juste": (rendu in attendu) if attendu else (rendu is None)})
        t1s.append(t1); t2s.append(t2)
    avec = [l for l in lignes if l["attendu"]]; sans = [l for l in lignes if not l["attendu"]]
    res = {
        "lignes": lignes,
        "bon_dans_les_trois": sum(l["dans_les_trois"] for l in avec), "avec_un_double": len(avec),
        "verdict_juste_avec_double": sum(l["juste"] for l in avec),
        "fusion_a_tort_avec_double": sum((l["rendu"] is not None) and not l["juste"] for l in avec),
        "sans_double": len(sans), "aucun_bien_rendu": sum(l["juste"] for l in sans),
        "latence_etage_1_ms": 1000 * statistics.median(t1s), "latence_etage_2_ms": 1000 * statistics.median(t2s),
        "latence_totale_mediane_ms": 1000 * statistics.median(a + b for a, b in zip(t1s, t2s)),
    }
    sortie["deux_etages"][nom] = res
    print(f"deux étages ({nom}) : bon dans les trois {res['bon_dans_les_trois']}/{len(avec)} ; verdict juste {res['verdict_juste_avec_double']}/{len(avec)} ; "
          f"fusions à tort {res['fusion_a_tort_avec_double']} ; « aucun » bien rendu {res['aucun_bien_rendu']}/{len(sans)} ; "
          f"étage 1 {res['latence_etage_1_ms']:.0f} ms + étage 2 {res['latence_etage_2_ms']:.0f} ms", flush=True)

json.dump(sortie, open(os.path.join(ICI, "resultats", f"complement-{tag}.json"), "w"), ensure_ascii=False, indent=1)
print("écrit resultats/complement-%s.json" % tag)
