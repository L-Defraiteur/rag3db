# La référence du banc de l'étage, avant et après les suppressions

**2 octobre 2026, 1 h.** Référence demandée par l'orchestration avant toute
mesure des poids de fusion (décision 2 de Lucie, qui attend le pas C). Rien
n'est réglé ici : on fixe le point de départ.

## Le montage

| | |
|---|---|
| Banc | `tests/e2e_banc_etage.rs` (`banc_etage_qui_perd`), vecteur seul |
| Modèle | granite-278m sur burn, `RAG3WEAVER_BANC_MODELE=granite-278m` |
| Poids | **régénérés** le 2 octobre (`88495d5ba`) : `model.bpk` sha256 `69aed115…e52723f`, `tokenizer.json` sha256 `2a0d7366…865a8e9`. Mêmes valeurs que l'ONNX d'IBM à 3e-7 près en f32, mais **pas le même fichier** que celui du 18 septembre (`a54628b5…`). |
| Matériel | ROG Flow Z13, iGPU Radeon 8060S (gfx1151), burn `Device<Vulkan(DefaultDevice)>`, précision Flex32 (défaut), autotune et fusion actifs |
| Moteur | `build/lecteurs-csv` avec les correctifs du journal (D et E) |
| Durée | 204 s (après), 230 s (avant, compilation comprise) ; trois autres sessions compilaient à côté — les durées ne sont pas des mesures |

**Avant** = `master` d'aujourd'hui avec `01791e347` (les trois suppressions)
annulé dans un worktree, et non `a66bb0b9d` lui-même : `a66bb0b9d` épingle
les anciens hash des forks burn / cubecl / cubek, recréés depuis
(`04b5052ec`), et les deux passes n'auraient pas différé que par les
suppressions. Ici, la seule différence entre avant et après est `01791e347`
(code retiré de `src/`, deux questions et deux aiguilles en moins).
**Après** = `master` à `cfc4d612d` (branche `banc-reference`).

Le banc ne mesurait plus rien depuis le 18 septembre à 21 h 31 : deux de ses
aiguilles (`search_bm25`, `resolve_chunk_results`) avaient été retirées par
`67ca53e54`, une heure après la mesure de référence, et l'extraction
paniquait. Elles sont retirées des deux passes (`cfc4d612d`) ; aucune question
ne les visait, et `e2e_banc_qualite` les avait déjà ôtées.

## Les deux passes

| ligne | avant (45 questions) MRR | R@1 | R@5 | après (43 questions) MRR | R@1 | R@5 |
|---|---:|---:|---:|---:|---:|---:|
| tel quel — `Catalog::search` sur `src/` | 0,330 | 9 | 26 | 0,333 | 9 | 24 |
| M1 — un chunk par fonction, texte du cosinus nu | 0,844 | 32 | 45 | 0,837 | 30 | 43 |
| M2 — cosinus exact au lieu du HNSW | 0,339 | 9 | 26 | 0,342 | 9 | 24 |
| M3 — les 20 chunks bruts, avant résolution | 0,132 | 0 | 11 | 0,135 | 0 | 11 |
| M1b — texte de M1 sur tous les scopes | 0,387 | 9 | 28 | 0,390 | 9 | 28 |
| **G — tel quel, `scope_type` ∈ function, method** | **0,414** | **13** | **28** | **0,412** | **13** | **27** |

Corpus : 5 263 scopes avant, 5 247 après (16 de moins : le code retiré) ;
M1 sur 65 aiguilles avant, 63 après. M3 : le bon parent est dans les 20
chunks bruts pour 31/45 questions avant, 29/43 après.

## Contre le 18 septembre

| ligne | 18 sept. (carte TV, 45 q., ~4 820 scopes) | 2 oct. avant | 2 oct. après |
|---|---:|---:|---:|
| tel quel | 0,328 | 0,330 | 0,333 |
| M1b | 0,385 | 0,387 | 0,390 |
| G | 0,385 | 0,414 | 0,412 |

Ce qui a changé depuis le 18 septembre : le corpus vivant (`src/` est passé
d'environ 4 820 à 5 263 scopes : fusion de `mtg-experiments`, correctifs), les
poids régénérés, le poste et sa carte (iGPU Vulkan contre Radeon R9700), et,
pour « après », les deux questions retirées.

- **tel quel et M1b** : +0,002 à +0,005, du même ordre que la variance déjà
  mesurée le 18 septembre (un HNSW reconstruit réordonne les quasi-ex-æquo,
  ±1 question sur 45 ; 0,346 contre 0,328 entre deux passes). **Non
  attribué.**
- **G : +0,03 (0,385 → 0,414)**, R@1 12 → 13. Plus que la variance connue,
  mais trois causes possibles se superposent (corpus, poids, poste) et rien
  ici ne les sépare. **Non attribué.**
- **Avant contre après** : les écarts (−0,007 à +0,003 ; M1 −2 à R@1 et R@5)
  suivent surtout le retrait de deux questions sur 45, dont les cibles étaient
  sans doute bien classées (M1 perd 2 à R@5 sur 2 questions retirées). Rien ne
  dit que les suppressions aient changé le classement des 43 autres ; le banc
  ne sort pas le détail par question pour le prouver. **Non attribué au code
  retiré.**

## Pour la suite

La mesure des poids de fusion (0,6 / 0,4 contre 0,3 / 0,7, et le sparse)
part de la colonne **après** : 43 questions, 5 247 scopes, poids
`69aed115…`. Si l'on veut comparer des passes futures question par question,
le banc devrait sortir le rang de chaque question : sans cela, un écart de
0,005 restera « non attribué ».

## Addendum du 3 octobre — les poids d'origine, et la stabilité des lignes

Les poids granite-278m du 6 septembre sont revenus (`a54628b5…`). Le banc a
été rejoué sur `master` (`7928974ba`, même corpus de 5 247 scopes, 43
questions), deux fois avec chaque fichier :

| ligne | régénérés, passe 1 | régénérés, passe 2 | origine, passe 1 | origine, passe 2 |
|---|---|---|---|---|
| tel quel | 0,333 / 9 / 24 | 0,333 / 9 / 24 | 0,333 / 9 / 24 | 0,333 / 9 / 24 |
| M1 | 0,837 / 30 / 43 | 0,764 / 29 / 37 | 0,775 / 29 / 38 | 0,771 / 29 / 38 |
| M2 | 0,342 / 9 / 24 | 0,342 / 9 / 24 | 0,342 / 9 / 24 | 0,342 / 9 / 24 |
| M3 | 0,182 / 3 / 14 | 0,230 / 4 / 18 | 0,142 / 2 / 8 | 0,090 / 0 / 4 |
| M1b | 0,390 / 9 / 28 | 0,390 / 9 / 28 | 0,390 / 9 / 28 | 0,390 / 9 / 28 |
| G | 0,412 / 13 / 27 | 0,414 / 13 / 27 | 0,412 / 13 / 27 | 0,412 / 13 / 27 |

- **Les deux fichiers de poids se valent** : toutes les lignes stables sont
  identiques entre eux.
- **M1 et M3 ne sont pas stables d'une passe à l'autre**, avec le même fichier
  de poids : M1 va de 0,764 à 0,837 (le 0,837 du 2 octobre était une passe
  haute), M3 de 0,090 à 0,230. Ce sont des index HNSW sur de petits ensembles
  (63 scopes pour M1, 20 chunks bruts pour M3), qui réordonnent les
  quasi-ex-æquo à chaque reconstruction. **Ni M1 ni M3 ne servent de
  référence à une passe.**
- **G bouge de 0,002 entre deux passes** (0,412 / 0,414), avec le même R@1 et
  le même R@5. C'est l'ordre de grandeur de la variance de la ligne qui sert
  de référence : un écart de cet ordre ne se lit pas.
- Les références à retenir : **tel quel 0,333, G 0,412 ± 0,002, M2 0,342,
  M1b 0,390.**
