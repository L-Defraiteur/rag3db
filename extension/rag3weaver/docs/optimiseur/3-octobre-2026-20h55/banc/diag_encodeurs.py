"""Laya et GLiNER, servis dans la forme que leurs auteurs montrent. Les avait-on mal employés ?

usage : python diag_encodeurs.py     (CPU, huit fils)
Écrit resultats/diag-encodeurs.json.
"""
import json, os, sys

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "")
import torch
torch.set_num_threads(8)
ICI = os.path.dirname(os.path.abspath(__file__))
JEU = json.load(open(os.path.join(ICI, "jeu.json")))
P = JEU["sujets"]["paires"]
O = JEU["sujets"]["options"]
sortie = {"laya": {}, "gliner": {}}


def noter(nom_modele, nom, lignes, cle_meme):
    sortie[nom_modele][nom] = lignes
    just = sum(max(l["sortie"], key=l["sortie"].get) == l["attendu"] for l in lignes)
    print(f"{nom_modele} / {nom} : {just} sur {len(lignes)}", flush=True)


# ------------------------------------------------------------------ Laya
import laya
agent = laya.load("convaiinnovations/laya-multilingual")


def laya_choix(etat, instructions, criteres):
    r = agent.predict(etat, {"q": {"type": "choice", "instructions": instructions, "criteria": criteres}})
    return dict(r["answers"]["q"]["probabilities"])


# (a) notre usage : état en texte, critère en question
L = [{"id": p["id"], "langue": p["langue"], "attendu": p["etiquette"],
      "sortie": laya_choix(f"Topic A: {p['a']}\nTopic B: {p['b']}", JEU["sujets"]["instructions"], O)} for p in P]
noter("laya", "notre_usage", L, "same")
# (b) la forme de sa carte : état en objet, champs cités entre accents graves, descriptions courtes
L = [{"id": p["id"], "langue": p["langue"], "attendu": p["etiquette"],
      "sortie": laya_choix({"topic_a": p["a"], "topic_b": p["b"]}, "How does `topic_b` relate to `topic_a`?",
                           {"same": "the same topic, reworded or translated", "variant": "a part or special case of it",
                            "unrelated": "a different topic"})} for p in P]
noter("laya", "forme_de_la_carte", L, "same")
# (c) deux options à clés neutres et descriptions oui/non : le conseil de sa carte pour les oui/non
binaire = lambda e: "same" if e == "same" else "different"
L = []
for p in P:
    r = laya_choix({"topic_a": p["a"], "topic_b": p["b"]}, "Are `topic_a` and `topic_b` the same topic?",
                   {"A": "yes, they are the same topic", "B": "no, they are different topics"})
    L.append({"id": p["id"], "langue": p["langue"], "attendu": binaire(p["etiquette"]), "sortie": {"same": r["A"], "different": r["B"]}})
noter("laya", "deux_options_neutres", L, "same")
# (d) la primitive oui/non elle-même
L = []
for p in P:
    r = agent.predict({"topic_a": p["a"], "topic_b": p["b"]},
                      {"q": {"type": "noul", "instructions": "Are `topic_a` and `topic_b` the same topic?"}})
    a = r["answers"]["q"]
    pv = a.get("noul", a.get("probability"))
    L.append({"id": p["id"], "langue": p["langue"], "attendu": binaire(p["etiquette"]), "sortie": {"same": float(pv), "different": 1 - float(pv)}, "brut": {k: v for k, v in a.items() if k != "action"}})
noter("laya", "oui_non", L, "same")

# ------------------------------------------------------------------ GLiNER
from gliner2 import AutoExtractor
ext = AutoExtractor.from_pretrained("fastino/GLiNER2.5-multi-Decide")


def gliner(texte, nom_tache, etiquettes, descriptions=None):
    cfg = {"labels": list(etiquettes), "multi_label": True, "class_act": "softmax", "cls_threshold": 0.0}
    if descriptions:
        cfg["label_descriptions"] = descriptions
    r = ext.classify_text(texte, {nom_tache: cfg}, include_confidence=True, format_results=False)
    return {k: float(v) for k, v in r[nom_tache]}


# (a) notre usage
L = [{"id": p["id"], "langue": p["langue"], "attendu": p["etiquette"],
      "sortie": gliner(f"Topic A: {p['a']}\nTopic B: {p['b']}", JEU["sujets"]["instructions"], O, O)} for p in P]
noter("gliner", "notre_usage", L, "same")
# (b) la forme de sa carte : un texte, un nom de tâche court, des étiquettes courtes sans description
NOMS = {"same_topic": "same", "related_topic": "variant", "different_topic": "unrelated"}
L = []
for p in P:
    r = gliner(f"Topic A: {p['a']}. Topic B: {p['b']}.", "relation", NOMS)
    L.append({"id": p["id"], "langue": p["langue"], "attendu": p["etiquette"], "sortie": {NOMS[k]: v for k, v in r.items()}})
noter("gliner", "forme_de_la_carte", L, "same")
# (c) une phrase naturelle, deux étiquettes
L = []
for p in P:
    r = gliner(f"Is \"{p['a']}\" the same subject as \"{p['b']}\"?", "answer", ["same_topic", "different_topic"])
    L.append({"id": p["id"], "langue": p["langue"], "attendu": binaire(p["etiquette"]), "sortie": {"same": r["same_topic"], "different": r["different_topic"]}})
noter("gliner", "phrase_et_deux_etiquettes", L, "same")
# (d) son vrai métier : ranger un texte parmi des sujets nommés (le nouveau sujet parmi les existants)
dix = JEU["dix"]
L = []
for c in dix["cas"]:
    etiq = list(dix["existants"]) + [dix["aucun"]]
    r = gliner(c["nouveau"], "topic", etiq)
    att = dix["aucun"] if c["attendu"] < 0 else dix["existants"][c["attendu"]]
    L.append({"id": c["id"], "attendu": att, "sortie": r})
noter("gliner", "ranger_parmi_dix_sujets", L, None)

json.dump(sortie, open(os.path.join(ICI, "resultats", "diag-encodeurs.json"), "w"), ensure_ascii=False, indent=1)
print("écrit")
