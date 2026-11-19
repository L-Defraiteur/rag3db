# Composer les invites pour le cache des fournisseurs

19 novembre 2026, session ED. Question de Lucie, par c7 : « les optimisations de cache sont-elles prévues pour les
LLM du framework ? ». Réponse honnête : **non, seulement subies**. Sur la F300 de l'archiviste (banc, 15 novembre),
28 % des 27,0 M jetons d'invite sont venus du cache implicite de Vertex, sans que rien ne l'ait cherché.

Cette page repart de l'[audit du banc du 17 novembre](../../banc/17-11-2026/04-audit-cache-vertex-et-plan-de-mesure.md)
et de son ticket. Elle ne le refait pas.

## 1. La mesure : ce qui existe, ce que le lot du 19 ajoute

- **Existait déjà** : `Usage::cached_prompt_tokens`, lu par le client Anthropic (`cache_read_input_tokens`) et par le
  client compatible OpenAI (`prompt_tokens_details.cached_tokens`, ou `cached_content_token_count` quand Gemini
  transparaît). `LlmCall` le porte, l'agent le cumule dans le run, et le compteur le facture à part
  (`CachedInputTokens`).
- **Le lot `llm-cache-counted`**, déposé au pousseur le 19 :
  - `Usage::cache_reported`, porté par chaque `LlmCall`. **Un compte absent n'est plus un zéro** : un taux de cache
    se calcule sur les seuls appels qui l'ont dit.
  - Le `timings.cache_n` de llama-server est lu.
  - Témoin sur un flux simulé : les deux orthographes, le zéro dit, l'absence, `cache_n`.

**Le taux à publier** : ΣC / ΣP sur les appels où `cache_reported` est vrai, avec, à côté, le nombre de ces appels
sur le total (la couverture). C'est le R_tokens du banc.

## 2. La règle de composition

Un cache de fournisseur, implicite ou explicite, sert **un préfixe identique, octet pour octet**. D'où la règle :

> **Le stable d'abord, le variable à la fin.** Les instructions, puis les outils (dans un ordre fixe), puis le
> contexte long et stable, puis l'historique, et le changeant (l'état du moment, la question, le bout courant) en
> dernier. Tout ce qui change avant un bloc stable fait perdre ce bloc au cache.

Le code la suit déjà par endroits. `Agent::refresh_waiting_block` met le bloc d'attente **en dernier**, pour
cette raison même, dit son commentaire. Et le registre des outils est rangé par nom (un `BTreeMap`).

## 3. Où nos invites la violent aujourd'hui

| Où | Ce qui change trop tôt | Effet | Remède proposé |
|---|---|---|---|
| **Le décideur** (`dataflow/decide_nodes.rs::by_llm`) | les étiquettes candidates sont dans le **système**, avant le texte rendu (relevé du banc) | dès que l'ensemble des candidats change, tout le contexte qui suit est perdu pour le cache | déplacer les candidats **après** le contexte stable, dans le message utilisateur, juste avant la question |
| **L'archiviste** (`templates/backends/memory/graphs/archive_decide.mmd`, `archive_candidates.rhai`) | le bout de fenêtre en cours, puis les candidats proposés, qui évoluent d'un bout à l'autre (banc, 15 nov., § 5) | dans une même fenêtre, le préfixe instruction et contexte tient ; d'une fenêtre à la suivante, il tombe | garder l'ordre instruction, contexte de la fenêtre, puis les candidats et le bout courant en dernier ; le banc mesure avant tout changement |
| **Le chat** (`daemon_host/chat.rs`) | le système est **réécrit à chaque tour** : règles, `chat_instructions`, consigne du pas, nom, persona | aucun, **tant que les octets sont les mêmes** ; un changement de mode ou d'outil invalide tout l'historique d'un coup | rien à changer. Le témoin à poser : deux tours sans changement rendent un système identique à l'octet |
| **L'agent et l'équipier** (`agent.rs::run_inner`) | les outils sont relus à chaque tour ; un catalogue qui change (un outil créé, une description reformulée) change le préfixe | rare, mais tout l'historique est perdu ce tour-là | garder l'ordre fixe ; ne pas réécrire une description d'outil sans raison |
| **La compaction** (`TurnCompacted`) | réécrit des tours **anciens** | tout ce qui suit la réécriture repart sans cache, une fois par compaction | rien, c'est son prix ; à le compter séparément dans la mesure |

Rien de ce tableau n'est mesuré ici. Ce sont des lectures du code, croisées avec celles du banc. L'ordre des remèdes
suivra la mesure : le décideur d'abord, parce que c'est le seul endroit où le changeant passe **avant** un long
contexte stable.

## 4. Le cache explicite de Vertex, chiffré

Ce que disent les pages de Google, relues le 19 novembre :

- [vue d'ensemble du cache de contexte](https://docs.cloud.google.com/vertex-ai/generative-ai/docs/context-cache/context-cache-overview) ;
- [tarifs](https://cloud.google.com/vertex-ai/generative-ai/pricing) ;
- [cache de l'API Gemini](https://ai.google.dev/gemini-api/docs/caching).

Elles ne s'accordent pas toutes : à revérifier au moment de décider.

- **Implicite** : actif par défaut, **90 % de remise** sur les jetons servis, aucun frais de stockage. Seuil : 4 096
  jetons de préfixe pour la famille Gemini 3 (2 048 pour Gemini 2). Le cache n'est jamais garanti : une entrée vit
  au plus 24 h, et le plus souvent bien moins.
- **Explicite** : la même remise de 90 % (Gemini 2.5 et suivants). La création est facturée au tarif d'entrée,
  plus un **stockage à l'heure** par million de jetons gardés : environ 1 $/M/h pour un Flash, 4,50 $/M/h pour un
  Pro, au prorata de la minute. TTL d'une minute au moins, sans maximum. Annoncé pour le 12 janvier 2027 :
  l'écriture du cache facturée 1,25 fois le tarif d'entrée.

**Le calcul**, pour un préfixe stable de P jetons relu N fois par heure, au tarif d'entrée e ($/M) :

- sans cache, l'heure coûte N·P·e ;
- avec un cache explicite, elle coûte P·e (la création) + P·s (le stockage) + N·P·0,1·e.

Pour un Flash (s ≈ 1 $/M/h, e de l'ordre de 0,3 $/M), le gain commence vers **N ≈ 5 relectures par heure**. Et il
ne vaut **que si l'implicite manque déjà ces relectures** : quand l'implicite touche, il donne la même remise sans
stockage.

**Où ça pourrait valoir** : l'archiviste, qui relit la même instruction et le même contexte de fenêtre des dizaines
de fois par heure pendant une F300. Mais 28 % de cache implicite y sont déjà servis, et la part manquante vient
surtout du préfixe qui change entre les fenêtres (§ 3), ce qu'un cache explicite ne répare pas. **Conclusion** : pas
de cache explicite avant que le § 3 soit appliqué et remesuré. Le seuil de 4 096 jetons et un taux implicite
encore bas après correction de l'ordre seraient les deux signes qu'il faut y venir.

## 5. Ce que cette page ne fait pas

Elle ne code rien du § 3 ni du § 4 : chaque remède du § 3 est un lot, avec un témoin d'ordre (le préfixe identique
d'un appel au suivant) et une mesure avant et après par le banc.
