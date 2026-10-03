"""Tire un petit jeu thématique rangé par d'autres que nous : MASSIVE (Amazon), par sa reprise
`mteb/amazon_massive_scenario` sur Hugging Face (carte : apache-2.0, annotations humaines ; jeu
d'origine `AmazonScience/massive`, cc-by-4.0). Des demandes à un assistant vocal, rangées par des
annotateurs sous 18 scénarios, en 50 langues dont le français et l'anglais.

Tirage, graine fixe 2026 : les scénarios sont mélangés ; les dix premiers sont les sujets existants,
les trois suivants sont retirés pour fabriquer les cas « à créer ». Par sujet existant : trois textes
du jeu de test (français et anglais alternés) et deux exemples du jeu d'entraînement (un français,
un anglais). Par sujet retiré : quatre textes (deux français, deux anglais).

usage : python tirer_public.py > public.json
"""
import gzip, io, json, random, sys, urllib.request

BASE = "https://huggingface.co/datasets/mteb/amazon_massive_scenario/resolve/main/"


def lire(chemin):
    req = urllib.request.Request(BASE + chemin, headers={"User-Agent": "rag3db-verif/1.0"})
    brut = urllib.request.urlopen(req, timeout=60).read()
    txt = gzip.decompress(brut).decode()
    try:
        d = json.loads(txt)
        return d if isinstance(d, list) else [json.loads(l) for l in txt.splitlines() if l.strip()]
    except json.JSONDecodeError:
        return [json.loads(l) for l in txt.splitlines() if l.strip()]


D = {(part, lg): lire(f"{part}/{lg}.json.gz") for part in ("test", "train") for lg in ("fr", "en")}
ex0 = D[("test", "fr")][0]
print("champs :", list(ex0), file=sys.stderr)
cle = "label_text" if "label_text" in ex0 else ("label" if "label" in ex0 else "scenario")
rng = random.Random(2026)
scenarios = sorted({x[cle] for x in D[("test", "en")]})
rng.shuffle(scenarios)
existants, retires = scenarios[:10], scenarios[10:13]


def tirer(part, lg, sc, n):
    L = [x["text"] for x in D[(part, lg)] if x[cle] == sc and 25 <= len(x["text"]) <= 160]
    rng.shuffle(L)
    return L[:n]


sujets, textes = [], []
for i, sc in enumerate(existants):
    sujets.append({"nom": str(sc), "exemples": tirer("train", "fr", sc, 1) + tirer("train", "en", sc, 1)})
    lgs = ["fr", "en", "fr"] if i % 2 == 0 else ["en", "fr", "en"]
    for lg in lgs:
        for t in tirer("test", lg, sc, 1):
            textes.append({"texte": t, "langue": lg, "sujet": str(sc)})
for sc in retires:
    for lg in ("fr", "en"):
        for t in tirer("test", lg, sc, 2):
            textes.append({"texte": t, "langue": lg, "sujet": None, "sujet_retire": str(sc)})
json.dump({"source": "mteb/amazon_massive_scenario (reprise de AmazonScience/massive)", "licence": "apache-2.0 sur la carte de la reprise ; cc-by-4.0 pour le jeu d'origine",
           "graine": 2026, "existants": [str(s) for s in existants], "retires": [str(s) for s in retires], "sujets": sujets, "textes": textes},
          sys.stdout, ensure_ascii=False, indent=1)
print(f"{len(sujets)} sujets, {sum(1 for t in textes if t['sujet'])} textes à ranger, {sum(1 for t in textes if not t['sujet'])} à créer ; existants {existants} ; retirés {retires}", file=sys.stderr)
