"""Banc des modèles de décision sur notre jeu de paires. CPU seulement, huit fils.

usage : python banc.py <laya|gliner|jevk5|granite|reranker>
Écrit resultats/<modele>.json : pour chaque paire, les probabilités par option (ou le score brut
pour les références), deux passes pour le déterminisme, et les temps mesurés.
"""
import json, math, os, statistics, sys, time

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "")
ICI = os.path.dirname(os.path.abspath(__file__))
JEU = json.load(open(os.path.join(ICI, "jeu.json")))
modele = sys.argv[1]
N_FILS = 8

ABSTENTION = ("unsure", "the names alone do not allow to tell")


def etat_sujets(p):
    return f"Topic A: {p['a']}\nTopic B: {p['b']}"


def etat_memoires(p):
    return f"Existing memory: {p['existante']}\nNew memory: {p['nouvelle']}"


# --------------------------------------------------------------------------- les modèles
if modele == "laya":
    import torch
    torch.set_num_threads(N_FILS)
    from laya import Router
    routeur = Router()

    def decider(etat, instructions, options):
        r = routeur.predict(etat, {"q": {"type": "choice", "instructions": instructions, "criteria": options}}, model="multilingual")
        return dict(r["answers"]["q"]["probabilities"])

elif modele == "gliner":
    import torch
    torch.set_num_threads(N_FILS)
    from gliner2 import AutoExtractor
    extracteur = AutoExtractor.from_pretrained("fastino/GLiNER2.5-multi-Decide")

    def decider(etat, instructions, options):
        tache = {instructions: {"labels": list(options), "label_descriptions": options,
                                "multi_label": True, "class_act": "softmax", "cls_threshold": 0.0}}
        r = extracteur.classify_text(etat, tache, include_confidence=True, format_results=False)
        return {k: float(v) for k, v in r[instructions]}

elif modele == "jevk5":
    sys.path.insert(0, os.path.join(os.path.dirname(ICI), "jevk5-src"))
    from jevk5.gguf import JevK5GGUF
    client = JevK5GGUF(url="http://127.0.0.1:8091", temperature=1.367)  # température de la carte pour la v0.3 4B

    def decider(etat, instructions, options):
        probs, _ = client.probabilities(etat, {"type": "choice", "instructions": instructions, "criteria": options})
        return {k: float(v) for k, v in probs.items()}

elif modele == "granite":
    import torch
    torch.set_num_threads(N_FILS)
    from sentence_transformers import SentenceTransformer
    st = SentenceTransformer("ibm-granite/granite-embedding-278m-multilingual", device="cpu")

    def score(a, b):
        e = st.encode([a, b], normalize_embeddings=True)
        return float((e[0] * e[1]).sum())

elif modele == "reranker":
    import torch
    torch.set_num_threads(N_FILS)
    from sentence_transformers import CrossEncoder
    ce = CrossEncoder("BAAI/bge-reranker-v2-m3", device="cpu")

    def score(a, b):
        # le cross-encodeur n'est pas symétrique : moyenne des deux sens, après sigmoïde
        s = ce.predict([(a, b), (b, a)], activation_fn=torch.nn.Sigmoid())
        return float((s[0] + s[1]) / 2)
else:
    raise SystemExit("modèle inconnu")

EST_REFERENCE = modele in ("granite", "reranker")
sortie = {"modele": modele, "fils": N_FILS}


def chrono(f, *a):
    t = time.perf_counter()
    r = f(*a)
    return r, time.perf_counter() - t


# --------------------------------------------------------------------------- sujets et mémoires
for nom, fabrique in (("sujets", etat_sujets), ("memoires", etat_memoires)):
    bloc = JEU[nom]
    lignes, temps = [], []
    for passe in (1, 2):
        for i, p in enumerate(bloc["paires"]):
            if EST_REFERENCE:
                a, b = (p["a"], p["b"]) if nom == "sujets" else (p["existante"], p["nouvelle"])
                r, dt = chrono(score, a, b)
                r = {"score": r}
            else:
                r, dt = chrono(decider, fabrique(p), bloc["instructions"], bloc["options"])
            if passe == 1:
                lignes.append({"id": p["id"], "langue": p["langue"], "etiquette": p["etiquette"], "sortie": r})
                temps.append(dt)
            else:
                lignes[i]["ecart_passe_2"] = max(abs(r[k] - lignes[i]["sortie"][k]) for k in r)
    sortie[nom] = {"lignes": lignes, "latence_mediane_s": statistics.median(temps[1:]),
                   "latence_max_s": max(temps[1:]), "premier_appel_s": temps[0]}
    print(nom, "fait :", len(lignes), "paires, médiane", round(statistics.median(temps[1:]) * 1000), "ms", flush=True)

# --------------------------------------------------------------------------- sujets avec une option d'abstention
if not EST_REFERENCE:
    bloc = JEU["sujets"]
    options = dict(bloc["options"]); options[ABSTENTION[0]] = ABSTENTION[1]
    lignes = []
    for p in bloc["paires"]:
        r = decider(etat_sujets(p), bloc["instructions"], options)
        lignes.append({"id": p["id"], "langue": p["langue"], "etiquette": p["etiquette"], "sortie": r})
    sortie["sujets_avec_abstention"] = {"lignes": lignes}
    print("abstention fait", flush=True)

# --------------------------------------------------------------------------- une décision à dix candidats
dix = JEU["dix"]
lignes, temps = [], []
for c in dix["cas"]:
    if EST_REFERENCE:
        t = time.perf_counter()
        scores = [score(c["nouveau"], e) for e in dix["existants"]]
        dt = time.perf_counter() - t
        r = {str(i): s for i, s in enumerate(scores)}
    else:
        options = {f"t{i}": e for i, e in enumerate(dix["existants"])}
        options["none"] = dix["aucun"]
        r, dt = chrono(decider, f"New topic: {c['nouveau']}", dix["instructions"], options)
    lignes.append({"id": c["id"], "attendu": c["attendu"], "sortie": r})
    temps.append(dt)
sortie["dix"] = {"lignes": lignes, "latence_mediane_s": statistics.median(temps), "latence_max_s": max(temps)}
print("dix fait, médiane", round(statistics.median(temps) * 1000), "ms", flush=True)

os.makedirs(os.path.join(ICI, "resultats"), exist_ok=True)
json.dump(sortie, open(os.path.join(ICI, "resultats", modele + ".json"), "w"), ensure_ascii=False, indent=1)
print("écrit resultats/%s.json" % modele)
