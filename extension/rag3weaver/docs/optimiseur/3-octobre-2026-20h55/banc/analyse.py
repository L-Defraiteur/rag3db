"""Lit resultats/*.json et rend les tableaux : exactitude, séparation par un seuil, calibration,
abstention, déterminisme, latence."""
import json, os, statistics, sys

ICI = os.path.dirname(os.path.abspath(__file__))
ORDRE = ["laya", "gliner", "jevk5", "granite", "reranker"]
R = {}
for m in ORDRE:
    f = os.path.join(ICI, "resultats", m + ".json")
    if os.path.exists(f):
        R[m] = json.load(open(f))
REF = ("granite", "reranker")
LANGUES = ["fr", "en", "mixte"]


def pct(x):
    return "—" if x is None else f"{100 * x:.0f} %"


def auc(pos, neg):
    if not pos or not neg:
        return None
    g = sum((p > n) + 0.5 * (p == n) for p in pos for n in neg)
    return g / (len(pos) * len(neg))


def meilleur_seuil(pos, neg):
    """Exactitude du meilleur seuil unique (optimiste : réglé sur les mêmes paires)."""
    best = (0, None)
    for s in sorted(set(pos + neg)):
        ok = sum(p >= s for p in pos) + sum(n < s for n in neg)
        if ok > best[0]:
            best = (ok, s)
    return best[0] / (len(pos) + len(neg)), best[1]


def rappel_sans_fausse_fusion(pos, neg):
    """Part des vraies « même chose » au-dessus du plus haut score d'une paire qui ne l'est pas."""
    plafond = max(neg)
    return sum(p > plafond for p in pos) / len(pos), plafond


def ece(conf_ok, bacs=5):
    n = len(conf_ok); e = 0.0
    for b in range(bacs):
        lo, hi = b / bacs, (b + 1) / bacs
        dans = [(c, ok) for c, ok in conf_ok if (lo < c <= hi) or (b == 0 and c == 0)]
        if dans:
            e += len(dans) / n * abs(sum(c for c, _ in dans) / len(dans) - sum(ok for _, ok in dans) / len(dans))
    return e


def choix(sortie):
    return max(sortie, key=sortie.get)


print("# Exactitude du verdict (choix le plus probable)\n")
for nom, titre in (("sujets", "sujets : même / variante / sans rapport (42 paires, hasard 33 %)"),
                   ("memoires", "mémoires : même / complète / contredit / neuve (20 paires, hasard 25 %)")):
    print(f"## {titre}\n")
    print("| modèle | toutes | français | anglais | mixtes | confusions les plus fréquentes |")
    print("|---|---|---|---|---|---|")
    for m, r in R.items():
        if m in REF:
            continue
        L = r[nom]["lignes"]
        def ex(ls):
            return sum(choix(l["sortie"]) == l["etiquette"] for l in ls) / len(ls) if ls else None
        conf = {}
        for l in L:
            c = choix(l["sortie"])
            if c != l["etiquette"]:
                conf[(l["etiquette"], c)] = conf.get((l["etiquette"], c), 0) + 1
        top = ", ".join(f"{a}→{b} ×{n}" for (a, b), n in sorted(conf.items(), key=lambda x: -x[1])[:3])
        print(f"| {m} | {pct(ex(L))} | " + " | ".join(pct(ex([l for l in L if l['langue'] == g])) for g in LANGUES) + f" | {top} |")
    print()

print("# Le score à critère fixe : un seuil unique sépare-t-il « même chose » du reste ?\n")
for nom, positif in (("sujets", "same"), ("memoires", "same")):
    print(f"## {nom} : score = P(« {positif} ») pour les modèles, score brut pour les références\n")
    print("| modèle | AUC | AUC fr | AUC en | AUC mixtes | vraies : min / médiane / max | autres : min / médiane / max | meilleur seuil unique (exactitude) | rappel sans aucune fausse fusion |")
    print("|---|---|---|---|---|---|---|---|---|")
    for m, r in R.items():
        L = r[nom]["lignes"]
        sc = lambda l: l["sortie"]["score"] if m in REF else l["sortie"][positif]
        pos = [sc(l) for l in L if l["etiquette"] == positif]
        neg = [sc(l) for l in L if l["etiquette"] != positif]
        d = lambda v: f"{min(v):.2f} / {statistics.median(v):.2f} / {max(v):.2f}"
        par = []
        for g in LANGUES:
            p = [sc(l) for l in L if l["etiquette"] == positif and l["langue"] == g]
            n = [sc(l) for l in L if l["etiquette"] != positif and l["langue"] == g]
            a = auc(p, n); par.append("—" if a is None else f"{a:.2f}")
        ex, s = meilleur_seuil(pos, neg)
        rap, plafond = rappel_sans_fausse_fusion(pos, neg)
        print(f"| {m} | {auc(pos, neg):.2f} | " + " | ".join(par) + f" | {d(pos)} | {d(neg)} | {s:.2f} ({pct(ex)}) | {pct(rap)} (seuil > {plafond:.2f}) |")
    print()

print("## mémoires : ce que les scores des références valent par étiquette\n")
print("| modèle | même | complète | contredit | neuve |")
print("|---|---|---|---|---|")
for m in REF:
    if m in R:
        L = R[m]["memoires"]["lignes"]
        print(f"| {m} | " + " | ".join(f"{statistics.mean(l['sortie']['score'] for l in L if l['etiquette'] == e):.2f}" for e in ("same", "completes", "contradicts", "new")) + " |")
print()

print("# Calibration : l'assurance annoncée vaut-elle l'exactitude ?\n")
print("| modèle | jeu | assurance moyenne | exactitude | écart de calibration (ECE, 5 bacs) | quand l'assurance dépasse 0,9 : cas, justes |")
print("|---|---|---|---|---|---|")
for m, r in R.items():
    if m in REF:
        continue
    for nom in ("sujets", "memoires"):
        L = r[nom]["lignes"]
        co = [(max(l["sortie"].values()), choix(l["sortie"]) == l["etiquette"]) for l in L]
        haut = [ok for c, ok in co if c > 0.9]
        print(f"| {m} | {nom} | {statistics.mean(c for c, _ in co):.2f} | {pct(sum(ok for _, ok in co) / len(co))} | {ece(co):.2f} | {len(haut)}, {sum(haut)} |")
print()

print("# Abstention : avec une option « unsure » en plus (sujets)\n")
print("| modèle | fois où « unsure » est choisi | dont sur une paire que le modèle avait fausse sans elle | exactitude sur le reste |")
print("|---|---|---|---|")
for m, r in R.items():
    if m in REF:
        continue
    A = r["sujets_avec_abstention"]["lignes"]; B = {l["id"]: l for l in r["sujets"]["lignes"]}
    abst = [l for l in A if choix(l["sortie"]) == "unsure"]
    utiles = sum(choix(B[l["id"]]["sortie"]) != l["etiquette"] for l in abst)
    reste = [l for l in A if choix(l["sortie"]) != "unsure"]
    print(f"| {m} | {len(abst)} sur {len(A)} | {utiles} | {pct(sum(choix(l['sortie']) == l['etiquette'] for l in reste) / len(reste)) if reste else '—'} |")
print()

print("# Déterminisme et latence (CPU, huit fils)\n")
print("| modèle | écart max entre deux passes | une décision à 3 options : médiane (max) | à 4 options | une décision à dix candidats : médiane (max) | bon candidat sur dix (6 cas) | premier appel |")
print("|---|---|---|---|---|---|---|")
for m, r in R.items():
    e = max(l["ecart_passe_2"] for nom in ("sujets", "memoires") for l in r[nom]["lignes"])
    ms = lambda b, k: f"{1000 * r[b][k]:.0f} ms"
    ok = 0
    for l in r["dix"]["lignes"]:
        if m in REF:
            ok += None is None and 0  # les références n'ont pas de « aucun » : jugé à part ci-dessous
        else:
            c = choix(l["sortie"]); att = "none" if l["attendu"] < 0 else f"t{l['attendu']}"
            ok += c == att
    if m in REF:
        ok = sum(int(max(l["sortie"], key=l["sortie"].get)) == l["attendu"] for l in r["dix"]["lignes"] if l["attendu"] >= 0)
        bon = f"{ok} sur 5 (le cas « aucun » n'a pas de sens sans seuil)"
    else:
        bon = f"{ok} sur 6"
    print(f"| {m} | {e:.1e} | {ms('sujets', 'latence_mediane_s')} ({ms('sujets', 'latence_max_s')}) | {ms('memoires', 'latence_mediane_s')} | {ms('dix', 'latence_mediane_s')} ({ms('dix', 'latence_max_s')}) | {bon} | {r['sujets']['premier_appel_s']:.1f} s |")
print()

if "--details" in sys.argv:
    print("# Détail par paire (sujets)\n")
    ids = [l["id"] for l in next(iter(R.values()))["sujets"]["lignes"]]
    print("| paire | langue | attendu | " + " | ".join(R) + " |")
    print("|---|---|---|" + "---|" * len(R))
    for i, pid in enumerate(ids):
        l0 = next(iter(R.values()))["sujets"]["lignes"][i]
        cell = []
        for m, r in R.items():
            l = r["sujets"]["lignes"][i]
            cell.append(f"{l['sortie']['score']:.2f}" if m in REF else f"{choix(l['sortie'])} ({max(l['sortie'].values()):.2f})")
        print(f"| {pid} | {l0['langue']} | {l0['etiquette']} | " + " | ".join(cell) + " |")
