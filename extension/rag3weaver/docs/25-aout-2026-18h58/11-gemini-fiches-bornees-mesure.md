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

> **Corrigé le 26 août au matin** (`09c4ef782`) : `search` et
> `search_expand` rendent du markdown compact (`RenderResultsNode`).
> Mesure déterministe sur notre propre code, même appel : **1 027
> caractères contre 4 721** en JSON, soit 4,6 fois moins, sans rien
> perdre de ce qui se lit (nom, entité, score, fichier, lignes, extrait,
> voisins). Q3 rejouée : **35 432 jetons en 30 s** contre 369 666 en
> 197 s, réponse toujours juste — avec une nuance d'honnêteté : cette
> fois le modèle a choisi `grep`/`read` plutôt que `search`, donc les
> deux chiffres ne comparent pas exactement la même trajectoire. Le
> rapport de 4,6 sur le rendu, lui, est mesuré à trajectoire identique.
> Reste le « vouliez-vous dire » de `list` : le modèle a encore tenté
> `read('src/dataflow/node_factories.rs')` (chemin du dépôt, pas de la
> source) et s'est rattrapé au coup suivant.

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

## 2 bis. Le même jeu, sur un modèle local (26 août, 8h)

Aucun adaptateur : `OpenAiLlm::new(base_url, model)` sans authentification
**est** le client `llama-server`. `RAG3WEAVER_LOCAL_LLM` suffit, et les
cinq épreuves sont exactement les mêmes.

**Qwen3-Coder-30B-A3B abliterated, Q6_K (24 Go)**, llama.cpp Vulkan, une
seule AMD Radeon AI PRO R9700 (`--device Vulkan1`, 28,3 Go occupés ; la
carte de l'écran n'a pas bougé) :

| | Fin | Appels | Jetons | Juste ? |
|---|---|---|---|---|
| Q1 `take_results` | Eos, **2,4 s** | **0** | 2 135 | **non** — voir plus bas |
| Q2 `FuseResultsNode.signals` | Eos, 21,6 s | 7 (1 erreur) | 40 928 | oui |
| Q3 `register_builtins` | Eos, 29,4 s | 7 (1 erreur) | 35 472 | oui |
| M1 `ServiceRegistry::len` | Eos, 15,3 s | 5 | 22 197 | **oui**, index à jour |
| M2 renommer `take_results` | Eos, 47,5 s | 11 | 178 281 | **oui**, 0 reste |

**Les deux missions d'édition passent**, sur la machine, sans quota et sans
un centime — c'est le résultat qui compte. Quatre-vingt-deux secondes pour
les cinq épreuves, là où le nuage en demandait trois cents avec ses 429.

**Q1 est un défaut de protocole, pas d'intelligence.** Le modèle a bien
décidé de chercher, mais il a écrit son appel *dans le texte*, au format
XML de Qwen (`<function=search><parameter=target>Scope</parameter>…`),
au lieu du champ `tool_calls` de l'API. llama.cpp ne l'a pas converti, notre
boucle a vu « aucun outil demandé » et a conclu le tour. Deux façons de le
fermer, et elles ne s'excluent pas :

- **côté llama.cpp** : la conversion des appels d'outils dépend du gabarit
  et de la version ; c'est le premier essai à faire ;
- **côté harnais** : détecter un appel d'outil resté dans le texte
  (`<function=…>`, `<tool_call>`) et le renvoyer au modèle en résultat
  d'erreur lisible — exactement ce qu'on a déjà fait pour Vertex avec
  `repair_arguments_json` et `stray_error`. Général, bon marché, utile à
  tout modèle local.

Les deux erreurs d'outil de Q2 et Q3 sont **la même que chez Gemini** :
`read('extension/rag3weaver/src/…')`, le chemin du dépôt au lieu de celui
de la source. Le « vouliez-vous dire » de `list` et `grep` est en place
depuis ce matin ; celui de `read` existait déjà, et les deux modèles s'en
rattrapent au coup suivant.

## 3. Le harnais

- Une question qui meurt sur un quota est **non mesurée**, pas un échec du
  test : les autres comptent. (`agent.run` en `Err` → ligne dans le bilan.)
- `RAG3WEAVER_CLOUD_QUESTIONS=3` (ou `1,3`) ne pose que celles-là — pour
  remesurer sans repayer.
- Ne pas lancer les deux tests cloud en parallèle sur un petit quota.
