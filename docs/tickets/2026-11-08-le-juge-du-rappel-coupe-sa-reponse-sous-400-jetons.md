# Le juge du rappel des hooks coupe sa réponse sous un plafond de 400 jetons : la réflexion de Gemini les prend

- **État** : ouvert (8 novembre 2026), pour archi (les hooks de Claude Code).
- **Gravité** : coût, et réponse perdue. Le juge paie l'appel et ne rend rien : sa réponse coupée
  est illisible, donc aucune fiche n'est rendue.
- **Atteignable en service** : oui, dès que le juge différé est allumé avec Gemini 3.8 Flash (le
  modèle Vertex par défaut). Il n'est pas allumé chez Lucie aujourd'hui (aucun verdict dans
  hooks.log).
- **Touche rag3weaver** : oui (`src/claude_code_hook.rs`, `juger`).
- **Famille** : produit

## Ce que c'est

`juger` appelle le modèle avec `with_max_tokens(400)`. Gemini 3.8 Flash compte sa réflexion dans ce
plafond : sur une partie des appels, elle prend 350 à 384 jetons. La réponse s'arrête alors sur
`finish_reason: "length"` après 2 à 33 jetons, par exemple `{\n  "pert`. `juger` rend « réponse
illisible », et rien n'est rendu.

## Recette

La consigne et le schéma de `juger`, mot pour mot, par l'endpoint OpenAI de Vertex (global,
`google/gemini-3.8-flash`, température 0, `response_format` à schéma strict, `max_tokens` 400). Le
message est un prompt réel d'une session et ses 8 fiches candidates. Mesure du banc, 8 novembre
(`~/.cache/rag3db-banc-notes/decision/`) :

| cas | max_tokens | finish_reason | sortie | dont réflexion |
|---|---|---|---|---|
| 25 | 400 | length | 13 | 383 |
| 25 | 4 000 | stop | 62 | 258 |
| 30 | 400 | length | 2 | 384 |
| 30 | 4 000 | stop | 50 | 362 |
| 35 | 400 | length | 5 | 378 |
| 35 | 4 000 | stop | 50 | 512 |

La coupe tombe sur 8 des 27 premiers appels dont la raison de fin a été gardée, soit 30 %. Le
décompte final sera dans la page du banc `extension/rag3weaver/docs/08-novembre-2026-banc/`. Relancés à 4 000, tous ces cas finissent sur
`stop`, avec une réflexion de 205 à 512 jetons.

## Le précédent

Le nœud `Decide` a déjà rencontré la même chose (`src/dataflow/decide_nodes.rs`). Son plafond est
de 2 048 jetons par défaut. Une réponse tronquée y est nommée : « réponse tronquée au plafond de
{} jetons (max_tokens) … montez max_tokens, ou bornez la réflexion du modèle ». `juger` n'a ni l'un
ni l'autre.

## Témoin

À écrire avec le correctif : sur un faux modèle (`tests/common/fake_sse.rs`) qui rend
`finish_reason: "length"` et un JSON coupé, `juger` doit dire la troncature, et pas seulement
« réponse illisible ».

## Pour le fermer

Un plafond qui laisse la place à la réflexion (celui de `Decide`, ou une réflexion bornée par la
requête), la troncature nommée dans le journal du fond, et le témoin.
