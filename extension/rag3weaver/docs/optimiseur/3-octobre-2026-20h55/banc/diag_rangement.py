"""La forme proposée par Lucie : ne plus comparer deux noms de sujets, demander quel sujet va le mieux
à un texte. Les options sont les sujets existants (avec plus ou moins de contexte) et le sujet neuf proposé.

usage : JEVK5_URL=http://127.0.0.1:7982 python diag_rangement.py <étiquette de sortie>
Écrit resultats/rangement-<étiquette>.json.
"""
import json, math, os, sys, time

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "")
ICI = os.path.dirname(os.path.abspath(__file__))
R = json.load(open(os.path.join(ICI, "rangement.json")))
JEU = json.load(open(os.path.join(ICI, "jeu.json")))
sys.path.insert(0, os.path.join(os.path.dirname(ICI), "jevk5-src"))
from jevk5.gguf import JevK5GGUF
from jevk5.prompt import LETTERS, prompt_text

URL = os.environ.get("JEVK5_URL", "http://127.0.0.1:8091")
TEMP = float(os.environ.get("JEVK5_TEMP", "1.367"))
client = JevK5GGUF(url=URL, temperature=TEMP)
SUJETS = R["sujets"]
PAR_NOM = {s["nom"]: s for s in SUJETS}


def decider(etat, instructions, options):
    textes = [f"{k}: {d}" for k, d in options]
    prompt = prompt_text(etat, instructions, textes)
    jetons = client._post("/tokenize", {"content": prompt, "add_special": False, "parse_special": True})["tokens"]
    out = client._post("/completion", {"prompt": jetons, "n_predict": 1, "n_probs": 40, "temperature": 0, "cache_prompt": False})
    vus = {e["token"]: e["logprob"] for e in out["completion_probabilities"][0]["top_logprobs"]}
    plancher = min(vus.values()) - 2.0
    lp = [vus.get(LETTERS[i], plancher) for i in range(len(textes))]
    m = max(lp); w = [math.exp((z - m) / TEMP) for z in lp]; s = sum(w)
    return {k: x / s for (k, _), x in zip(options, w)}, len(jetons)


# ------------------------------------------------------------------ l'étage de recherche : cosinus granite
import torch
torch.set_num_threads(8)
from sentence_transformers import SentenceTransformer
st = SentenceTransformer("ibm-granite/granite-embedding-278m-multilingual", device="cpu")
E = st.encode([f"{s['nom']} : {s['description']}" for s in SUJETS], normalize_embeddings=True)
RANG = {}
for t in R["textes"]:
    e = st.encode([t["texte"]], normalize_embeddings=True)[0]
    RANG[t["id"]] = [SUJETS[i]["nom"] for i in (E @ e).argsort()[::-1]]
manques = {k: sum(1 for t in R["textes"] if t["sujet"] and t["sujet"] not in RANG[t["id"]][:k]) for k in (2, 4, 9)}
print("recherche (cosinus) : bon sujet absent des 2 premiers :", manques[2], "; des 4 premiers :", manques[4], "sur 16", flush=True)


def existants(t, n):
    """Les n sujets existants proposés : les premiers de la recherche ; le bon est ajouté s'il manque,
    pour mesurer le verdict et non la recherche (les manques sont comptés à part)."""
    noms = RANG[t["id"]][:n]
    if t["sujet"] and t["sujet"] not in noms:
        noms = noms[:-1] + [t["sujet"]]
    return noms


def decrire(nom, contexte):
    s = PAR_NOM[nom]
    if contexte == "nom":
        return nom
    d = f"{nom} — {s['description']}"
    n = {"nom_description": 0, "un_exemple": 1, "deux_exemples": 2, "trois_exemples": 3}[contexte]
    if n:
        d += " Texts already filed under it: " + " ; ".join(f"«{x}»" for x in s["exemples"][:n])
    return d


def neuf(t, presentation):
    n = t["neuf_propose"]
    return {
        "sans_texte": f"new topic «{n}» (no text has been filed under it yet)",
        "texte_seul": f"new topic «{n}». Texts already filed under it: «{t['texte']}»",
        "description": f"new topic «{n}» — {t['neuf_description']} (no text has been filed under it yet)",
        "aucun": "none of these topics: the text needs a new topic",
    }[presentation]


INSTR = "Which topic fits this text best?"
CONFIGS = {}
for c in ("nom", "nom_description", "un_exemple", "deux_exemples", "trois_exemples"):
    CONFIGS["contexte_" + c] = dict(contexte=c, neuf="sans_texte", n=4, neuf_en="dernier")
for p in ("texte_seul", "description", "aucun"):
    CONFIGS["neuf_" + p] = dict(contexte="deux_exemples", neuf=p, n=4, neuf_en="dernier")
CONFIGS["neuf_en_premier"] = dict(contexte="deux_exemples", neuf="sans_texte", n=4, neuf_en="premier")
CONFIGS["neuf_description_en_premier"] = dict(contexte="deux_exemples", neuf="description", n=4, neuf_en="premier")
CONFIGS["deux_existants"] = dict(contexte="deux_exemples", neuf="sans_texte", n=2, neuf_en="dernier")
CONFIGS["dix_existants"] = dict(contexte="deux_exemples", neuf="sans_texte", n=10, neuf_en="dernier")
CONFIGS["dix_existants_nom_seul"] = dict(contexte="nom", neuf="sans_texte", n=10, neuf_en="dernier")
CONFIGS["ordre_inverse"] = dict(contexte="deux_exemples", neuf="sans_texte", n=4, neuf_en="dernier", inverse=True)

sortie = {"url": URL, "temperature": TEMP, "manques_recherche": manques, "configs": {}}
for nom, c in CONFIGS.items():
    lignes, t0, jet = [], time.time(), []
    for t in R["textes"]:
        noms = existants(t, c["n"])
        if c.get("inverse"):
            noms = noms[::-1]
        opts = [(f"t{i}", decrire(x, c["contexte"])) for i, x in enumerate(noms)]
        o_neuf = ("new", neuf(t, c["neuf"]))
        opts = [o_neuf] + opts if c["neuf_en"] == "premier" else opts + [o_neuf]
        probs, nj = decider(f"Text: {t['texte']}", INSTR, opts)
        jet.append(nj)
        k = max(probs, key=probs.get)
        rendu = None if k == "new" else noms[int(k[1:])]
        tri = sorted(probs.values(), reverse=True)
        lignes.append({"id": t["id"], "langue": t["langue"], "attendu": t["sujet"], "rendu": rendu, "p": tri[0], "p_second": tri[1],
                       "p_neuf": probs["new"], "p_bon": probs.get(f"t{noms.index(t['sujet'])}") if t["sujet"] else probs["new"],
                       "position_choisie": [o for o, _ in opts].index(k)})
    sortie["configs"][nom] = {**c, "lignes": lignes, "ms": 1000 * (time.time() - t0) / len(lignes), "jetons_moyens": sum(jet) / len(jet)}
    A = [l for l in lignes if l["attendu"]]; N = [l for l in lignes if not l["attendu"]]
    print(f"{nom} : rejoint le bon {sum(l['rendu'] == l['attendu'] for l in A)}/16, crée à tort {sum(l['rendu'] is None for l in A)}, "
          f"range ailleurs {sum(l['rendu'] not in (None, l['attendu']) for l in A)} ; neufs : crée {sum(l['rendu'] is None for l in N)}/8, "
          f"fusionne à tort {sum(l['rendu'] is not None for l in N)} ; {sortie['configs'][nom]['jetons_moyens']:.0f} jetons, {sortie['configs'][nom]['ms']:.0f} ms", flush=True)

# ------------------------------------------------------------------ seconde formulation : l'alternative posée explicitement
# (a) un seul choix : « créer le nouveau sujet » ou l'un des existants ; le neuf est présenté dans la question.
# (b) deux temps : oui/non « un sujet existant convient-il ? », puis « lequel ? » seulement si oui.
def bilan(nom, lignes):
    A = [l for l in lignes if l["attendu"]]; N = [l for l in lignes if not l["attendu"]]
    print(f"{nom} : rejoint le bon {sum(l['rendu'] == l['attendu'] for l in A)}/16, crée à tort {sum(l['rendu'] is None for l in A)}, "
          f"range ailleurs {sum(l['rendu'] not in (None, l['attendu']) for l in A)} ; neufs : crée {sum(l['rendu'] is None for l in N)}/8, "
          f"fusionne à tort {sum(l['rendu'] is not None for l in N)}", flush=True)


for ordre in ("creer_en_premier", "creer_en_dernier"):
    lignes = []
    for t in R["textes"]:
        noms = existants(t, 4)
        instr = (f"A new topic «{t['neuf_propose']}» ({t['neuf_description']}) is proposed for this text. "
                 "Should the text go under that new topic, or can it join one of the existing topics?")
        opts = [(f"t{i}", "join the existing topic " + decrire(x, "deux_exemples")) for i, x in enumerate(noms)]
        creer = ("new", f"create the new topic «{t['neuf_propose']}»")
        opts = [creer] + opts if ordre == "creer_en_premier" else opts + [creer]
        probs, _ = decider(f"Text: {t['texte']}", instr, opts)
        k = max(probs, key=probs.get)
        tri = sorted(probs.values(), reverse=True)
        lignes.append({"id": t["id"], "attendu": t["sujet"], "rendu": None if k == "new" else noms[int(k[1:])],
                       "p": tri[0], "p_second": tri[1], "p_neuf": probs["new"]})
    sortie["configs"]["alternative_un_choix_" + ordre] = {"lignes": lignes}
    bilan("alternative, un choix, " + ordre, lignes)

for ordre in ("oui_en_premier", "non_en_premier"):
    lignes = []
    for t in R["textes"]:
        noms = existants(t, 4)
        liste = "\n".join(f"{i + 1}. {decrire(x, 'deux_exemples')}" for i, x in enumerate(noms))
        etat = f"Text: {t['texte']}\nProposed new topic: «{t['neuf_propose']}» ({t['neuf_description']})\nExisting topics:\n{liste}"
        o1 = [("existing", "yes: one of the existing topics already fits this text"), ("new", "no: the text needs the proposed new topic")]
        if ordre == "non_en_premier":
            o1 = o1[::-1]
        p1, _ = decider(etat, "Does one of the existing topics already fit this text?", o1)
        rendu, p2, p2s = None, None, None
        if p1["existing"] > 0.5:
            probs, _ = decider(f"Text: {t['texte']}", INSTR, [(f"t{i}", decrire(x, "deux_exemples")) for i, x in enumerate(noms)])
            k = max(probs, key=probs.get); rendu = noms[int(k[1:])]
            tri = sorted(probs.values(), reverse=True); p2, p2s = tri[0], tri[1]
        lignes.append({"id": t["id"], "attendu": t["sujet"], "rendu": rendu, "p_existant": p1["existing"], "p": p2, "p_second": p2s, "p_neuf": p1["new"]})
    sortie["configs"]["alternative_deux_temps_" + ordre] = {"lignes": lignes}
    bilan("alternative, deux temps, " + ordre, lignes)

# ------------------------------------------------------------------ la forme d'origine, sur les mêmes cas
# « le nom proposé et le nom existant sont-ils le même sujet ? » : même / différent
O2 = [("same", JEU["sujets"]["options"]["same"]), ("different", "two different topics, even if they are related")]
lignes = []
for t in R["textes"]:
    existant = t["sujet"] or RANG[t["id"]][0]
    probs, _ = decider(f"Topic A: {t['neuf_propose']}\nTopic B: {existant}", JEU["sujets"]["instructions"], O2)
    lignes.append({"id": t["id"], "attendu": "same" if t["sujet"] else "different", "contre": existant, "p_same": probs["same"]})
sortie["forme_origine_deux_noms"] = lignes
A = [l for l in lignes if l["attendu"] == "same"]; N = [l for l in lignes if l["attendu"] == "different"]
print(f"forme d'origine (deux noms, même/différent) : rejoint {sum(l['p_same'] > 0.5 for l in A)}/16 ; neufs : fusionne à tort {sum(l['p_same'] > 0.5 for l in N)}/8", flush=True)

# ------------------------------------------------------------------ la même forme pour les mémoires
M = JEU["memoires"]["paires"]
toutes = []
for p in M:
    if p["existante"] not in toutes:
        toutes.append(p["existante"])
lignes = []
for i, p in enumerate(M):
    autres = [x for x in toutes if x != p["existante"]]
    distr = [autres[(i * 3) % len(autres)], autres[(i * 3 + 1) % len(autres)]]
    cand = distr[:]; cand.insert(i % 3, p["existante"])
    opts = [(f"m{j}", x) for j, x in enumerate(cand)] + [("none", "none of these: the new memory is about something else")]
    probs, _ = decider(f"New memory: {p['nouvelle']}", "Which existing memory is this new memory about?", opts)
    k = max(probs, key=probs.get)
    rendu = None if k == "none" else cand[int(k[1:])]
    attendu = None if p["etiquette"] == "new" else p["existante"]
    lignes.append({"id": p["id"], "etiquette": p["etiquette"], "juste": rendu == attendu, "p": probs[k],
                   "p_bon": probs["none"] if attendu is None else probs[f"m{cand.index(p['existante'])}"]})
sortie["memoires_meme_forme"] = lignes
for e in ("same", "completes", "contradicts", "new"):
    L = [l for l in lignes if l["etiquette"] == e]
    print(f"mémoires, {e} : {sum(l['juste'] for l in L)}/{len(L)}", flush=True)

json.dump(sortie, open(os.path.join(ICI, "resultats", f"rangement-{sys.argv[1]}.json"), "w"), ensure_ascii=False, indent=1)
print("écrit")
