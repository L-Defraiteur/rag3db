"""Contrôle indépendant, version à 48 paires : les paires de contrôle du banc de la mémoire longue
(`extension/rag3weaver/scripts/test_banc_memoire.py`, master 5d1a3189b). Vérité par construction.

Le banc écrit ces paires en artefact quand il tourne ; ici elles sont reconstruites avec les mêmes
listes, recopiées mot pour mot du script, pour ne pas lancer le backend. Critère, options et seuil
du document 02, sans rien régler.

usage : JEVK5_URL=http://127.0.0.1:7982 python banc4.py <étiquette> <seuil>
"""
import json, os, sys

ICI = os.path.dirname(os.path.abspath(__file__))
JEU = json.load(open(os.path.join(ICI, "jeu.json")))
tag, seuil = sys.argv[1], float(sys.argv[2])
sys.path.insert(0, os.path.join(os.path.dirname(ICI), "jevk5-src"))
from jevk5.gguf import JevK5GGUF

client = JevK5GGUF(url=os.environ.get("JEVK5_URL", "http://127.0.0.1:8091"), temperature=1.42 if "2b" in tag else 1.367)
M = JEU["memoires"]

claims = ['/tmp est de la mémoire vive', 'la passe complète se lance sans demander', 'laisser deux coeurs libres',
          'le journal survit à la passe', 'annoncer cargo avant de le prendre', 'ne pas fusionner sans preuve']
redites = [
    ('le dossier temporaire vit en RAM', '/tmp est de la mémoire vive'),
    ('rien de lourd dans le repertoire temporaire', '/tmp est de la mémoire vive'),
    ('la batterie se joue sans rien demander', 'la passe complète se lance sans demander'),
    ('garder deux processeurs pour la machine', 'laisser deux coeurs libres'),
    ('le compte rendu de la passe reste sur le disque', 'le journal survit à la passe'),
    ('dire qu on prend le compilateur', 'annoncer cargo avant de le prendre'),
]
contradictions = [
    ('on peut mettre un target cargo dans /tmp', '/tmp est de la mémoire vive'),
    ('compiler sur tous les coeurs ne gêne personne', 'laisser deux coeurs libres'),
]
paires = []
for nouveau, cible in redites:
    for claim in claims:
        paires.append({'nouveau': nouveau, 'existant': claim, 'verite': 'meme' if claim == cible else 'different', 'genre': 'redite'})
for nouveau, cible in contradictions:
    for claim in claims:
        paires.append({'nouveau': nouveau, 'existant': claim, 'verite': 'contredit' if claim == cible else 'different', 'genre': 'contradiction'})
assert len(paires) == 48

for p in paires:
    probs, _ = client.probabilities(f"Existing memory: {p['existant']}\nNew memory: {p['nouveau']}",
                                    {"type": "choice", "instructions": M["instructions"], "criteria": M["options"]})
    p["choix"] = max(probs, key=probs.get); p["p_same"] = float(probs["same"]); p["probs"] = {k: float(v) for k, v in probs.items()}

meme = [p for p in paires if p["verite"] == "meme"]; contr = [p for p in paires if p["verite"] == "contredit"]; diff = [p for p in paires if p["verite"] == "different"]
auc = sum((a["p_same"] > b["p_same"]) + 0.5 * (a["p_same"] == b["p_same"]) for a in meme for b in contr + diff) / (len(meme) * (len(contr) + len(diff)))
print(f"48 paires : {len(meme)} même, {len(contr)} contredit, {len(diff)} différent")
print(f"AUC de P(même) : {auc:.3f}")
print(f"seuil {seuil} : vraies « même » au-dessus {sum(p['p_same'] > seuil for p in meme)}/{len(meme)} ; "
      f"fusions à tort : {sum(p['p_same'] > seuil for p in diff)}/{len(diff)} différentes, {sum(p['p_same'] > seuil for p in contr)}/{len(contr)} contradictions")
print(f"P(même) : vraies min {min(p['p_same'] for p in meme):.3f} max {max(p['p_same'] for p in meme):.3f} ; "
      f"différentes max {max(p['p_same'] for p in diff):.3f} ; contradictions {', '.join('%.3f' % p['p_same'] for p in contr)}")
print("verdict à quatre choix : même →", [p["choix"] for p in meme], "; contredit →", [p["choix"] for p in contr],
      "; différent → new", sum(p["choix"] == "new" for p in diff), "sur", len(diff))
for p in meme + contr + [d for d in diff if d["p_same"] > seuil]:
    print(f"   {p['verite']:9} P(même) {p['p_same']:.3f} choix {p['choix']:11} | {p['existant']}  //  {p['nouveau']}")
json.dump({"modele": tag, "seuil": seuil, "source": "test_banc_memoire.py, master 5d1a3189b, listes recopiées", "paires": paires},
          open(os.path.join(ICI, "resultats", f"controle48-{tag}.json"), "w"), ensure_ascii=False, indent=1)
