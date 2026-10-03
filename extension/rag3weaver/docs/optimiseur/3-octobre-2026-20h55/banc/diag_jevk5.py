"""Diagnostic exploratoire de JevK5 : ce qu'il voit, ce que la formulation change, d'autres découpes.

usage : JEVK5_URL=http://127.0.0.1:7982 python diag_jevk5.py <étiquette de sortie>
Écrit resultats/diag-<étiquette>.json. Aucun tableau de vainqueur : des faits par configuration.
"""
import itertools, json, math, os, sys, time

ICI = os.path.dirname(os.path.abspath(__file__))
JEU = json.load(open(os.path.join(ICI, "jeu.json")))
sys.path.insert(0, os.path.join(os.path.dirname(ICI), "jevk5-src"))
from jevk5.gguf import JevK5GGUF
from jevk5.prompt import LETTERS, prompt_text

URL = os.environ.get("JEVK5_URL", "http://127.0.0.1:8091")
TEMP = float(os.environ.get("JEVK5_TEMP", "1.367"))
client = JevK5GGUF(url=URL, temperature=TEMP)
P = JEU["sujets"]["paires"]
BASE_I = JEU["sujets"]["instructions"]
BASE_O = JEU["sujets"]["options"]


def brut(etat, instructions, textes):
    """Un appel : le prompt exact, les 40 premiers jetons rendus et leurs log-probabilités."""
    prompt = prompt_text(etat, instructions, textes)
    jetons = client._post("/tokenize", {"content": prompt, "add_special": False, "parse_special": True})["tokens"]
    out = client._post("/completion", {"prompt": jetons, "n_predict": 1, "n_probs": 40, "temperature": 0, "cache_prompt": False})
    top = out["completion_probabilities"][0]["top_logprobs"]
    return prompt, len(jetons), [(e["token"], e["logprob"]) for e in top]


def decider(etat, instructions, options):
    """options : liste ordonnée de (clé, description). Rend {clé: probabilité}, comme le client de l'auteur."""
    textes = [f"{k}: {d}" if d is not None else k for k, d in options]
    _, _, top = brut(etat, instructions, textes)
    vus = dict(top)
    plancher = min(vus.values()) - 2.0
    lp = [vus.get(LETTERS[i], plancher) for i in range(len(textes))]
    m = max(lp)
    w = [math.exp((z - m) / TEMP) for z in lp]
    s = sum(w)
    masse = sum(math.exp(vus[LETTERS[i]]) for i in range(len(textes)) if LETTERS[i] in vus)
    return {k: x / s for (k, _), x in zip(options, w)}, masse


def etat_en(p):
    return f"Topic A: {p['a']}\nTopic B: {p['b']}"


sortie = {"url": URL, "temperature": TEMP, "configs": {}}

# ------------------------------------------------------------------ 1. ce que le modèle voit
vues = []
for pid in ("s02", "s04", "s06"):
    p = next(x for x in P if x["id"] == pid)
    textes = [f"{k}: {d}" for k, d in BASE_O.items()]
    prompt, n, top = brut(etat_en(p), BASE_I, textes)
    vues.append({"id": pid, "etiquette": p["etiquette"], "prompt": prompt, "jetons": n, "top": top[:12],
                 "masse_sur_les_lettres": sum(math.exp(lp) for t, lp in top if t in LETTERS[:3])})
sortie["vu"] = vues

# ------------------------------------------------------------------ 2 et 3. les configurations
O = list(BASE_O.items())
COURT = [("same", "same topic"), ("variant", "related but distinct topics"), ("unrelated", "different topics")]
LONG = [
    ("same", "both phrases name the same topic: a paraphrase, a synonym, or a translation into another language; notes about one would be filed under the other"),
    ("variant", "the topics are related but not the same: one is a part, a mechanism, a consequence or a special case of the other, and deserves its own entry"),
    ("unrelated", "two different topics that have nothing to do with each other: a note about one would never be looked up under the other"),
]
ABSENT = [(k, None) for k, _ in O]
EXEMPLES = ("\nExamples: \"unit tests\" and \"tests unitaires\" -> same. \"database\" and \"index of the database\" -> variant. "
            "\"database\" and \"colour of the buttons\" -> unrelated.")
FR_I = "Ces deux noms de sujet désignent-ils le même sujet ?"
FR_O = [("same", "les deux expressions nomment le même sujet, dit autrement ou dans une autre langue"),
        ("variant", "sujets liés : l'un est une partie, un mécanisme ou un cas particulier de l'autre"),
        ("unrelated", "deux sujets différents")]

CONFIGS = {
    "base": (etat_en, BASE_I, O),
    "critere_1": (etat_en, "Are topic A and topic B the same topic?", O),
    "critere_2": (etat_en, "Decide how topic B relates to topic A.", O),
    "critere_3": (etat_en, "Notes are filed by topic. Should the notes of topic A and the notes of topic B be filed under one single topic?", O),
    "options_sans_description": (etat_en, BASE_I, ABSENT),
    "options_courtes": (etat_en, BASE_I, COURT),
    "options_longues": (etat_en, BASE_I, LONG),
    "critere_francais": (lambda p: f"Sujet A : {p['a']}\nSujet B : {p['b']}", FR_I, FR_O),
    "avec_exemples": (etat_en, BASE_I + EXEMPLES, O),
    # d'autres découpes de la même décision
    "deux_options": (etat_en, BASE_I, [("same", BASE_O["same"]), ("different", "two different topics, even if they are related")]),
    "sous_sujet": (etat_en, BASE_I, [("same", BASE_O["same"]), ("subtopic", "one topic is a sub-topic of the other"), ("unrelated", BASE_O["unrelated"])]),
    "sous_sujet_dirige": (etat_en, "How does topic B relate to topic A?",
                          [("same", "topic B is the same topic as topic A"), ("b_in_a", "topic B is a sub-topic of topic A"),
                           ("a_in_b", "topic A is a sub-topic of topic B"), ("unrelated", "two different topics")]),
    "oui_non_meme": (etat_en, "Topic A and topic B are the same topic.", [("true", "The proposition is true."), ("false", "The proposition is false.")]),
    "oui_non_partie": (etat_en, "One of the two topics is a part, a mechanism or a special case of the other.",
                       [("true", "The proposition is true."), ("false", "The proposition is false.")]),
}
for i, perm in enumerate(itertools.permutations(O)):
    if i:
        CONFIGS["ordre_" + "-".join(k for k, _ in perm)] = (etat_en, BASE_I, list(perm))

t0 = time.time()
for nom, (etat, instr, options) in CONFIGS.items():
    lignes, t = [], time.time()
    for p in P:
        probs, masse = decider(etat(p), instr, options)
        lignes.append({"id": p["id"], "langue": p["langue"], "etiquette": p["etiquette"], "sortie": probs, "masse_lettres": masse})
    sortie["configs"][nom] = {"instructions": instr, "options": options, "lignes": lignes, "ms_par_decision": 1000 * (time.time() - t) / len(P)}
    print(nom, "fait,", round(1000 * (time.time() - t) / len(P)), "ms par décision", flush=True)

# ------------------------------------------------------------------ mémoires : base et ordres, pour le biais de position
M = JEU["memoires"]
MO = list(M["options"].items())
sortie["memoires"] = {}
for nom, options in (("base", MO), ("ordre_inverse", MO[::-1]), ("ordre_tourne", MO[2:] + MO[:2])):
    lignes = []
    for p in M["paires"]:
        probs, masse = decider(f"Existing memory: {p['existante']}\nNew memory: {p['nouvelle']}", M["instructions"], options)
        lignes.append({"id": p["id"], "langue": p["langue"], "etiquette": p["etiquette"], "sortie": probs})
    sortie["memoires"][nom] = {"options": options, "lignes": lignes}
    print("mémoires", nom, "fait", flush=True)

json.dump(sortie, open(os.path.join(ICI, "resultats", f"diag-{sys.argv[1]}.json"), "w"), ensure_ascii=False, indent=1)
print("écrit, total", round(time.time() - t0), "s")
