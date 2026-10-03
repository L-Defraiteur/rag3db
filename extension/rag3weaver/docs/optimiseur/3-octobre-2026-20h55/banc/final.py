"""Dernier tour : trois pistes, sur trois jeux à la fois, sans rien régler sur l'un d'eux.

  (a) ne pas montrer le nom du sujet neuf : l'option est « none of these topics » ;
  (b) pas d'option « créer » : on range parmi les existants, la création se lit dans la faiblesse
      du meilleur score (on rend la courbe, pas un seuil) ;
  (c) les textes complets plutôt que les titres.

Jeux : le nôtre (rangement.json), le journal des chantiers (journal.json, bâti par script),
un jeu public thématique (public.json, MASSIVE). Modèles : JevK5 (parmi les quatre premiers du
cosinus, sans forcer la présence du bon), cosinus granite-278m, reranker bge.

usage : JEVK5_URL=http://127.0.0.1:7982 python final.py <étiquette>
"""
import json, math, os, sys

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "")
ICI = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(ICI), "jevk5-src"))
from jevk5.gguf import JevK5GGUF
from jevk5.prompt import LETTERS, prompt_text
import torch
torch.set_num_threads(8)
from sentence_transformers import SentenceTransformer, CrossEncoder

URL = os.environ.get("JEVK5_URL", "http://127.0.0.1:8091")
TEMP = 1.367
client = JevK5GGUF(url=URL, temperature=TEMP)
st = SentenceTransformer("ibm-granite/granite-embedding-278m-multilingual", device="cpu")
ce = CrossEncoder("BAAI/bge-reranker-v2-m3", device="cpu")


def jevk5(etat, instructions, options):
    textes = [f"{k}: {d}" for k, d in options]
    prompt = prompt_text(etat, instructions, textes)
    jetons = client._post("/tokenize", {"content": prompt, "add_special": False, "parse_special": True})["tokens"]
    out = client._post("/completion", {"prompt": jetons, "n_predict": 1, "n_probs": 40, "temperature": 0, "cache_prompt": False})
    vus = {e["token"]: e["logprob"] for e in out["completion_probabilities"][0]["top_logprobs"]}
    plancher = min(vus.values()) - 2.0
    lp = [vus.get(LETTERS[i], plancher) for i in range(len(textes))]
    m = max(lp); w = [math.exp((z - m) / TEMP) for z in lp]; s = sum(w)
    return {k: x / s for (k, _), x in zip(options, w)}


court = lambda x, n=160: x if len(x) <= n else x[:n].rsplit(" ", 1)[0] + "…"
EX = " For example, among many other texts: "

# ------------------------------------------------------------------ les trois jeux, dans une même forme
# un jeu = liste de cas {texte, attendu (nom ou None), sujets: [{nom, complet}]}
JEUX = {}
R = json.load(open(os.path.join(ICI, "rangement.json")))
suj = [{"nom": s["nom"], "complet": f"{s['nom']} — {s['description']}." + EX + " ; ".join(f"«{e}»" for e in s["exemples"][:2])} for s in R["sujets"]]
JEUX["le nôtre"] = [{"texte": t["texte"], "attendu": t["sujet"], "sujets": suj} for t in R["textes"]]

J = json.load(open(os.path.join(ICI, "journal.json")))["sections"]
cas = []
for s in J:
    for e in s["entrees"][:8]:
        cas.append((e, s["nom"], []))
for s in J:
    for e in s["entrees"][-3:]:
        cas.append((e, None, [s["nom"]]))
JEUX["le journal"] = [{"texte": court(e["texte"], 320), "attendu": att,
                       "sujets": [{"nom": s["nom"], "complet": s["nom"] + "." + EX + " ; ".join(f"«{court(x['texte'])}»" for x in [y for y in s["entrees"] if y is not e][:2])}
                                  for s in J if s["nom"] not in absents]} for e, att, absents in cas]

Pb = json.load(open(os.path.join(ICI, "public.json")))
suj = [{"nom": s["nom"], "complet": s["nom"] + "." + EX + " ; ".join(f"«{e}»" for e in s["exemples"])} for s in Pb["sujets"]]
JEUX["le public"] = [{"texte": t["texte"], "attendu": t["sujet"], "sujets": suj} for t in Pb["textes"]]


def auc(pos, neg):
    return sum((p > n) + 0.5 * (p == n) for p in pos for n in neg) / (len(pos) * len(neg)) if pos and neg else float("nan")


def courbe(lignes):
    """La création se lit dans la faiblesse du meilleur score. Pour quelques points : si l'on crée sous le
    seuil qui attrape telle part des textes neufs, combien de textes à ranger sont encore rangés, et au bon endroit."""
    A = [l for l in lignes if l["attendu"]]; N = [l for l in lignes if not l["attendu"]]
    neufs = sorted(l["meilleur"] for l in N)
    pts = []
    for part in (1.0, 0.75, 0.5):
        k = max(1, math.ceil(part * len(neufs)))
        seuil = neufs[k - 1]  # on crée si le meilleur score est <= seuil
        ranges = [l for l in A if l["meilleur"] > seuil]
        pts.append({"neufs_attrapes": k, "sur": len(neufs), "ranges_au_bon_endroit": sum(l["rendu"] == l["attendu"] for l in ranges),
                    "ranges_ailleurs": sum(l["rendu"] != l["attendu"] for l in ranges), "crees_a_tort": len(A) - len(ranges), "a_ranger": len(A)})
    return pts


sortie = {"url": URL, "jeux": {k: {"cas": len(v), "a_ranger": sum(1 for c in v if c["attendu"]), "a_creer": sum(1 for c in v if not c["attendu"])} for k, v in JEUX.items()}, "resultats": {}}
for nom_jeu, cas in JEUX.items():
    for contexte in ("nom", "complet"):
        res = {"cosinus": [], "reranker": [], "jevk5_b": [], "jevk5_a": []}
        manque = 0
        for c in cas:
            reprs = [s[contexte] for s in c["sujets"]]; noms = [s["nom"] for s in c["sujets"]]
            E = st.encode(reprs, normalize_embeddings=True); q = st.encode([c["texte"]], normalize_embeddings=True)[0]
            cs = (E @ q).tolist()
            i = max(range(len(cs)), key=cs.__getitem__)
            res["cosinus"].append({"attendu": c["attendu"], "rendu": noms[i], "meilleur": cs[i]})
            rs = ce.predict([(c["texte"], r) for r in reprs], activation_fn=torch.nn.Sigmoid()).tolist()
            i = max(range(len(rs)), key=rs.__getitem__)
            res["reranker"].append({"attendu": c["attendu"], "rendu": noms[i], "meilleur": rs[i]})
            ordre = sorted(range(len(cs)), key=cs.__getitem__, reverse=True)[:4]
            if c["attendu"] and c["attendu"] not in [noms[j] for j in ordre]:
                manque += 1
            opts = [(f"t{k}", reprs[j]) for k, j in enumerate(ordre)]
            p = jevk5(f"Text: {c['texte']}", "Which topic fits this text best?", opts)
            k = max(p, key=p.get)
            res["jevk5_b"].append({"attendu": c["attendu"], "rendu": noms[ordre[int(k[1:])]], "meilleur": p[k]})
            p = jevk5(f"Text: {c['texte']}", "Which topic fits this text best?", opts + [("none", "none of these topics")])
            k = max(p, key=p.get)
            res["jevk5_a"].append({"attendu": c["attendu"], "rendu": None if k == "none" else noms[ordre[int(k[1:])]], "p": p[k], "p_aucun": p["none"]})
        bilan = {"bon_absent_des_quatre_premiers": manque}
        for m in ("cosinus", "reranker", "jevk5_b"):
            L = res[m]; A = [l for l in L if l["attendu"]]; N = [l for l in L if not l["attendu"]]
            bilan[m] = {"premier_juste": sum(l["rendu"] == l["attendu"] for l in A), "a_ranger": len(A),
                        "auc_meilleur_score": auc([l["meilleur"] for l in A], [l["meilleur"] for l in N]), "courbe": courbe(L)}
        L = res["jevk5_a"]; A = [l for l in L if l["attendu"]]; N = [l for l in L if not l["attendu"]]
        bilan["jevk5_a"] = {"rejoint_le_bon": sum(l["rendu"] == l["attendu"] for l in A), "cree_a_tort": sum(l["rendu"] is None for l in A),
                            "range_ailleurs": sum(l["rendu"] not in (None, l["attendu"]) for l in A), "a_ranger": len(A),
                            "neufs_crees": sum(l["rendu"] is None for l in N), "fusions_a_tort": sum(l["rendu"] is not None for l in N), "a_creer": len(N)}
        sortie["resultats"][f"{nom_jeu}|{contexte}"] = {"bilan": bilan, "lignes": res}
        b = bilan
        print(f"{nom_jeu} · {contexte} · (a) JevK5 avec « aucun » : rejoint {b['jevk5_a']['rejoint_le_bon']}/{b['jevk5_a']['a_ranger']}, crée à tort {b['jevk5_a']['cree_a_tort']}, "
              f"ailleurs {b['jevk5_a']['range_ailleurs']} ; neufs créés {b['jevk5_a']['neufs_crees']}/{b['jevk5_a']['a_creer']}", flush=True)
        for m in ("cosinus", "reranker", "jevk5_b"):
            c0 = b[m]["courbe"]
            print(f"   (b) {m:9} : premier juste {b[m]['premier_juste']}/{b[m]['a_ranger']} ; AUC du meilleur score {b[m]['auc_meilleur_score']:.2f} ; "
                  + " | ".join(f"{p['neufs_attrapes']}/{p['sur']} neufs attrapés → {p['ranges_au_bon_endroit']} bien rangés, {p['ranges_ailleurs']} ailleurs, {p['crees_a_tort']} créés à tort" for p in c0), flush=True)

# ------------------------------------------------------------------ (c) les 48 paires du banc, mémoires complètes
POURQUOI = {
    '/tmp est de la mémoire vive': 'tmpfs de 61 Gio sur ce poste : aucun target cargo dans le scratchpad.',
    'la passe complète se lance sans demander': 'Depuis le 6 septembre, le debug de performance est fini.',
    'laisser deux coeurs libres': 'Jamais tous les coeurs : c est la compilation qui fige le poste.',
    'le journal survit à la passe': 'run_e2e ecrit target/e2e-last.log dans les deux branches, et nomme les tests en échec.',
    'annoncer cargo avant de le prendre': 'Le target est partagé : une ligne avant chaque passe, sinon deux sessions se marchent dessus.',
    'ne pas fusionner sans preuve': 'Un rapprochement se propose ; c est quelqu un qui tranche, jamais le seuil seul.',
}
C48 = json.load(open(os.path.join(ICI, "resultats", "controle48-jevk5-q8-carte.json")))["paires"]
M = json.load(open(os.path.join(ICI, "jeu.json")))["memoires"]
sortie["memoires_48"] = {}
for forme in ("titre", "complet"):
    lignes = []
    for p in C48:
        ex = p["existant"] if forme == "titre" else f"{p['existant']} — {POURQUOI[p['existant']]}"
        pr = jevk5(f"Existing memory: {ex}\nNew memory: {p['nouveau']}", M["instructions"], list(M["options"].items()))
        e = st.encode([ex, p["nouveau"]], normalize_embeddings=True)
        r = ce.predict([(p["nouveau"], ex)], activation_fn=torch.nn.Sigmoid()).tolist()[0]
        lignes.append({"verite": p["verite"], "jevk5": pr["same"], "choix": max(pr, key=pr.get), "cosinus": float((e[0] * e[1]).sum()), "reranker": r})
    sortie["memoires_48"][forme] = lignes
    meme = [l for l in lignes if l["verite"] == "meme"]; autres = [l for l in lignes if l["verite"] != "meme"]
    print(f"mémoires, 48 paires, {forme} : " + " ; ".join(
        f"{m} AUC {auc([l[m] for l in meme], [l[m] for l in autres]):.2f}, redites au-dessus de toute autre paire {sum(l[m] > max(x[m] for x in autres) for l in meme)}/6"
        for m in ("jevk5", "cosinus", "reranker")) + f" ; verdict « même » ou « complète » pour une redite : {sum(l['choix'] in ('same', 'completes') for l in meme)}/6", flush=True)

json.dump(sortie, open(os.path.join(ICI, "resultats", f"final-{sys.argv[1]}.json"), "w"), ensure_ascii=False, indent=1)
print("écrit")
