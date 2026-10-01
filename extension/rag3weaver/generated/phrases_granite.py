#!/usr/bin/env python3
"""Le jeu fixe de phrases de la vérification granite : français, anglais, code,
du mot seul au texte qui dépasse 512 jetons (pour éprouver la troncature)."""
import json
import sys

phrases = [
    "bonjour",
    "Le chat dort sur le canapé depuis ce matin.",
    "The function opens the file and reads its contents into memory.",
    "La fonction ouvre le fichier et lit son contenu en mémoire.",
    "comment découper une liste de textes en lots selon un budget de caractères ?",
    "split a list of texts into batches under a character budget",
    "pub fn budget_batches(lens: &[usize], max_items: usize, max_chars: usize) -> Vec<Range<usize>>",
    "fn cosine(a: &[f32], b: &[f32]) -> f32 { a.iter().zip(b).map(|(x, y)| x * y).sum() }",
    "SELECT name, count(*) FROM files WHERE language = 'rust' GROUP BY name ORDER BY 2 DESC LIMIT 10;",
    "MATCH (s:Scope)-[:DEFINES]->(y:Symbol) WHERE y.name = 'budget_batches' RETURN s.path",
    "détecter les interblocages entre des agents qui s'attendent mutuellement",
    "detect deadlocks between agents waiting on each other",
    "Une base de graphe embarquée, avec recherche plein texte, vectorielle et creuse.",
    "An embedded graph database with full-text, vector and sparse search.",
    "L'index HNSW se reconstruit en masse après une première ingestion ; une insertion ligne à ligne paie le placement du vecteur, voisin par voisin.",
    "def embed(texts):\n    return [model(tok(t)).last_hidden_state[:, 0] for t in texts]",
    "struct LocalVariablePool { values: BTreeMap<FuseType, BTreeMap<TensorId, usize>> }",
    "El modelo fue entrenado con texto y código en doce idiomas.",
    "Das Modell wurde mit Text und Code in zwölf Sprachen trainiert.",
    "日本語のテキストも埋め込めます。",
    "12 345,67 € — 3 × 10⁻⁵ — ±0,5 % — α β γ",
    " ".join(["Le journal d'écriture garde chaque transaction jusqu'au point de contrôle."] * 12),
    " ".join(["fn lire(chemin: &Path) -> io::Result<String> { fs::read_to_string(chemin) }"] * 40),
    " ".join(["rag3weaver indexe du code et des documents ; chaque scope devient un nœud, chaque chunk un vecteur."] * 60),
]
json.dump(phrases, open(sys.argv[1], "w"), ensure_ascii=False, indent=1)
print(len(phrases), "phrases")
