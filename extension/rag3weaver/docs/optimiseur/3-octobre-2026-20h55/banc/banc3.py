"""Contrôle indépendant : les paires que le scénario du banc de la mémoire longue
(`extension/rag3weaver/scripts/test_banc_memoire.py`) écrit lui-même, donc dont la réponse est
connue sans que nous les ayons étiquetées. On y rejoue JevK5 avec le critère et le seuil du
jeu d'origine, sans rien régler.

usage : python banc3.py <jevk5|jevk5-2b> <seuil>      (llama-server déjà lancé sur le port 8091)
"""
import json, os, sys

ICI = os.path.dirname(os.path.abspath(__file__))
JEU = json.load(open(os.path.join(ICI, "jeu.json")))
tag, seuil = sys.argv[1], float(sys.argv[2])
sys.path.insert(0, os.path.join(os.path.dirname(ICI), "jevk5-src"))
from jevk5.gguf import JevK5GGUF

client = JevK5GGUF(url=os.environ.get("JEVK5_URL", "http://127.0.0.1:8091"), temperature=1.42 if "2b" in tag else 1.367)
M = JEU["memoires"]

# Les trois faits de la session 1 et la redite de la session 3, mot pour mot du scénario.
FAITS = ["/tmp est de la mémoire vive", "la passe complète se lance sans demander", "laisser deux coeurs libres"]
REDITE = "le dossier temporaire vit en RAM"
PAIRES = [(FAITS[0], REDITE, "same")] + [(f, REDITE, "new") for f in FAITS[1:]]
PAIRES += [(FAITS[i], FAITS[j], "new") for i in range(3) for j in range(3) if i < j]

lignes = []
for existante, nouvelle, attendu in PAIRES:
    probs, _ = client.probabilities(f"Existing memory: {existante}\nNew memory: {nouvelle}",
                                    {"type": "choice", "instructions": M["instructions"], "criteria": M["options"]})
    c = max(probs, key=probs.get)
    lignes.append({"existante": existante, "nouvelle": nouvelle, "attendu": attendu, "choix": c,
                   "p_same": float(probs["same"]), "au_dessus_du_seuil": probs["same"] > seuil})
    print(f"{attendu:5} | choix {c:11} | P(same) {probs['same']:.3f} | {'fusion' if probs['same'] > seuil else 'pas de fusion'} | {existante}  //  {nouvelle}")
json.dump({"modele": tag, "seuil": seuil, "lignes": lignes},
          open(os.path.join(ICI, "resultats", f"controle-{tag}.json"), "w"), ensure_ascii=False, indent=1)
