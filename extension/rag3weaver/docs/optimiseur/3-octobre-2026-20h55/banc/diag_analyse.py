"""Lit resultats/diag-*.json et rend les tableaux du diagnostic."""
import json, os, statistics, sys

ICI = os.path.dirname(os.path.abspath(__file__))
tag = sys.argv[1] if len(sys.argv) > 1 else "4b-q8-carte"
D = json.load(open(os.path.join(ICI, "resultats", f"diag-{tag}.json")))
C = D["configs"]
JEU = json.load(open(os.path.join(ICI, "jeu.json")))
TXT = {p["id"]: f"{p['a']} // {p['b']}" for p in JEU["sujets"]["paires"]}
TROIS = {"same": "same", "variant": "variant", "subtopic": "variant", "b_in_a": "variant", "a_in_b": "variant",
         "unrelated": "unrelated", "different": "different", "true": "true", "false": "false"}


def choix(s):
    return max(s, key=s.get)


def auc(pos, neg):
    return sum((p > n) + 0.5 * (p == n) for p in pos for n in neg) / (len(pos) * len(neg))


def p_meme(l):
    s = l["sortie"]
    return s.get("same", s.get("true"))


print("# 1. Ce que le modèle voit\n")
for v in D["vu"]:
    print(f"## paire {v['id']} (attendu : {v['etiquette']}) — {v['jetons']} jetons, masse de probabilité sur les trois lettres : {v['masse_sur_les_lettres']:.4f}\n")
    print("```\n" + v["prompt"] + "<ici le modèle écrit un jeton>\n```\n")
    print("jetons les plus probables : " + ", ".join(f"{json.dumps(t, ensure_ascii=False)} {lp:.2f}" for t, lp in v["top"][:8]) + "\n")

print("# 2. La formulation : ce qui bouge\n")
base = {l["id"]: l for l in C["base"]["lignes"]}
print("| configuration | exactitude à trois choix | même trouvés (15) | variantes trouvées (15) | sans rapport trouvés (12) | AUC « même » contre le reste | verdicts changés par rapport à la base | écart moyen sur P(même) | ms |")
print("|---|---|---|---|---|---|---|---|---|")
for nom, c in C.items():
    L = c["lignes"]
    cles = [k for k, _ in c["options"]]
    trois = set(TROIS[k] for k in cles) == {"same", "variant", "unrelated"}
    def juste(l):
        return TROIS[choix(l["sortie"])] == l["etiquette"]
    par = {e: sum(juste(l) for l in L if l["etiquette"] == e) for e in ("same", "variant", "unrelated")}
    a = auc([p_meme(l) for l in L if l["etiquette"] == "same"], [p_meme(l) for l in L if l["etiquette"] != "same"])
    chg = sum(TROIS[choix(l["sortie"])] != TROIS[choix(base[l["id"]]["sortie"])] for l in L) if trois else None
    ec = statistics.mean(abs(p_meme(l) - p_meme(base[l["id"]])) for l in L)
    ex = f"{sum(par.values())} sur 42" if trois else "—"
    pv = f"{par['variant']}" if trois else "—"
    pu = f"{par['unrelated']}" if trois else "—"
    print(f"| {nom} | {ex} | {par['same']} | {pv} | {pu} | {a:.2f} | {'—' if chg is None else chg} | {ec:.2f} | {c['ms_par_decision']:.0f} |")
print()

print("## Le biais de position (les six ordres des trois options)\n")
print("| ordre des options | choisit la 1re | la 2e | la 3e | exactitude |")
print("|---|---|---|---|---|")
tot = [0, 0, 0]
for nom, c in C.items():
    if nom != "base" and not nom.startswith("ordre_"):
        continue
    cles = [k for k, _ in c["options"]]
    pos = [0, 0, 0]
    for l in c["lignes"]:
        pos[cles.index(choix(l["sortie"]))] += 1
    tot = [a + b for a, b in zip(tot, pos)]
    print(f"| {' / '.join(cles)} | {pos[0]} | {pos[1]} | {pos[2]} | {sum(choix(l['sortie']) == l['etiquette'] for l in c['lignes'])} sur 42 |")
print(f"| **total** | {tot[0]} | {tot[1]} | {tot[2]} | (un tiers chacun = {sum(tot) // 3} si l'ordre ne comptait pas et les réponses étaient équilibrées) |\n")
ordres = [c for n, c in C.items() if n == "base" or n.startswith("ordre_")]
stables = sum(len({choix(c["lignes"][i]["sortie"]) for c in ordres}) == 1 for i in range(42))
print(f"Paires dont le verdict est le même sous les six ordres : {stables} sur 42.\n")
moy = []
for i in range(42):
    s = {}
    for c in ordres:
        for k, v in c["lignes"][i]["sortie"].items():
            s[k] = s.get(k, 0) + v / len(ordres)
    moy.append((ordres[0]["lignes"][i], s))
print(f"En moyennant les probabilités sur les six ordres : {sum(choix(s) == l['etiquette'] for l, s in moy)} sur 42 ; "
      f"AUC « même » {auc([s['same'] for l, s in moy if l['etiquette'] == 'same'], [s['same'] for l, s in moy if l['etiquette'] != 'same']):.2f}.\n")

print("# 3. D'autres découpes de la même décision\n")
on, op = {l["id"]: l for l in C["oui_non_meme"]["lignes"]}, {l["id"]: l for l in C["oui_non_partie"]["lignes"]}
ok = {"same": 0, "variant": 0, "unrelated": 0}
for pid, l in on.items():
    v = "same" if l["sortie"]["true"] > 0.5 else ("variant" if op[pid]["sortie"]["true"] > 0.5 else "unrelated")
    ok[l["etiquette"]] += v == l["etiquette"]
print(f"- **deux oui/non enchaînés** (« même sujet ? » puis « l'un est-il une partie de l'autre ? ») : {sum(ok.values())} sur 42 "
      f"(même {ok['same']}/15, variante {ok['variant']}/15, sans rapport {ok['unrelated']}/12).")
L = C["deux_options"]["lignes"]
b = sum((choix(l["sortie"]) == "same") == (l["etiquette"] == "same") for l in L)
fm = sum(choix(l["sortie"]) == "same" and l["etiquette"] != "same" for l in L)
print(f"- **deux options** (même / différent) : {b} sur 42 justes ; {fm} fusion(s) à tort ; "
      f"{sum(choix(l['sortie']) == 'same' and l['etiquette'] == 'same' for l in L)} vraies « même » trouvées sur 15.")
L = C["sous_sujet_dirige"]["lignes"]
print(f"- **sous-sujet dirigé** : pour les 15 « variantes », il répond : " + ", ".join(
    f"{k} ×{sum(choix(l['sortie']) == k for l in L if l['etiquette'] == 'variant')}" for k in ("same", "b_in_a", "a_in_b", "unrelated")) + ".\n")

print("## Les « variantes », paire par paire, sous toutes les formulations à trois choix\n")
conf3 = [n for n, c in C.items() if set(TROIS[k] for k, _ in c["options"]) == {"same", "variant", "unrelated"}]
print(f"({len(conf3)} configurations)\n")
print("| paire | les deux sujets | jugée variante | jugée même | jugée sans rapport | lecture |")
print("|---|---|---|---|---|---|")
for i, l in enumerate(C["base"]["lignes"]):
    if l["etiquette"] != "variant":
        continue
    v = [TROIS[choix(C[n]["lignes"][i]["sortie"])] for n in conf3]
    nv, ns, nu = v.count("variant"), v.count("same"), v.count("unrelated")
    lecture = "étiquette douteuse : « sans rapport » avec constance" if nu >= 0.8 * len(v) else (
        "étiquette douteuse : « même » avec constance" if ns >= 0.8 * len(v) else ("tenue" if nv >= 0.6 * len(v) else "instable"))
    print(f"| {l['id']} | {TXT[l['id']]} | {nv} | {ns} | {nu} | {lecture} |")
print()

print("# 4. Les fautes une par une\n")
meilleure = sys.argv[2] if len(sys.argv) > 2 else "base"
print(f"Configuration : `{meilleure}`.\n")
print("| paire | langue | les deux sujets | attendu | répondu | score |")
print("|---|---|---|---|---|---|")
for l in C[meilleure]["lignes"]:
    c = choix(l["sortie"])
    att = l["etiquette"] if set(l["sortie"]) != {"same", "different"} else ("same" if l["etiquette"] == "same" else "different")
    if TROIS.get(c, c) != att:
        print(f"| {l['id']} | {l['langue']} | {TXT[l['id']]} | {att} | {c} | {l['sortie'][c]:.2f} (P(même) {p_meme(l):.2f}) |")
print()

print("# Mémoires : l'ordre des quatre options\n")
for nom, c in D["memoires"].items():
    L = c["lignes"]
    cles = [k for k, _ in c["options"]]
    pos = [sum(cles.index(choix(l["sortie"])) == i for l in L) for i in range(4)]
    print(f"- {nom} ({' / '.join(cles)}) : {sum(choix(l['sortie']) == l['etiquette'] for l in L)} sur 20 ; position choisie : {pos}")
