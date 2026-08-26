# Gemini avec les fiches bornées : la mesure

26 août 2026, ~1h30. Suite du [10](10-parametres-de-config-entrees-du-graphe.md) : on
voulait savoir si `search_expand`, maintenant borné (`target`, `relation`,
`direction` en `enum` tirés du catalogue), devenait prenable pour un modèle
cloud. `tests/e2e_cloud_code_agent.rs`, `gemini-3.5-flash` sur Vertex.

## 1. Résultats

| | Fin | Appels | Jetons | Outils | Juste ? |
|---|---|---|---|---|---|
| Q1 `take_results` | Eos, 11 s | 4 | 13 535 | grep, read ×3 | oui |
| Q2 `FuseResultsNode.signals` | Eos, 13 s | 2 | 53 114 | search(Scope), read | oui |
| Q3 `register_builtins` | **MaxTokens**, 197 s | 7 | **369 666** | search, read, grep ×2, read, list ×2 | oui (29 + 6 types, `node_factories.rs` 1162–1208) |
| M1 `ServiceRegistry::len` | Eos, 13 s | 6 | 17 765 | grep ×2, read ×2, edit, read | oui, index à jour |
| M2 renommer `take_results` | Eos, 64 s | 10 | 36 695 | list, grep, read ×2, edit, read, edit, read ×2, grep | oui, 0 reste |

Q3 a demandé trois essais : les deux premiers sont morts sur un **HTTP 429
`RESOURCE_EXHAUSTED`** après les quatre réessais de la politique (60 s,
120 s, 120 s — cinq minutes). Les deux tests tournaient en parallèle et
partagent le quota ; Q3 arrive après les 53 k jetons de Q2. Le troisième
essai, seul, est passé.

## 2. Ce que ça dit

**L'`enum` ferme la porte à l'invention ; il ne rend pas l'outil attirant.**
Aucune relation inventée, aucune cible fausse — mais aucun `search_expand`
non plus. Pour « qu'est-ce que `register_builtins` enregistre ? », le
modèle lit le fichier, et il a raison : c'est une question locale à un
fichier, pas une question de graphe. La bonne mesure de `search_expand`
est une mission qui **exige** le graphe — « qui appelle `X`, dans tous les
fichiers ? » (`CONSUMED_BY`), « quelles méthodes a `Y` ? » (`PARENT_OF`) —
et un système qui le dit. C'est le prochain essai, pas celui-ci.

**Le coût est dans les résultats bruts.** 370 k jetons pour Q3, c'est le
contexte qui grossit à chaque itération : le résultat de `search` est le
JSON complet (`uuid`, `score`, `_content_hash`, le `content` entier de
chaque scope), et chaque `read` de 100 lignes s'y ajoute. `grep` et `read`
rendent du Markdown compact ; `search` devrait faire pareil — nom, fichier,
lignes, un extrait — avec le JSON en option (`format`), comme les outils de
code.

**`list(path_prefix="src/")` → 0 fichiers.** Les chemins sont relatifs à la
`FileSource` (piège 9 du [09](09-knowledge-dump.md)) ; le modèle a deviné
`src/` puis s'est rattrapé avec `list({})`. Un résultat vide devrait le dire
lui-même : « les chemins sont relatifs à la racine du projet ; `list({})`
pour voir le premier niveau » — le « vouliez-vous dire » de `read`, pour
`list`.

## 3. Le harnais

- Une question qui meurt sur un quota est **non mesurée**, pas un échec du
  test : les autres comptent. (`agent.run` en `Err` → ligne dans le bilan.)
- `RAG3WEAVER_CLOUD_QUESTIONS=3` (ou `1,3`) ne pose que celles-là — pour
  remesurer sans repayer.
- Ne pas lancer les deux tests cloud en parallèle sur un petit quota.
