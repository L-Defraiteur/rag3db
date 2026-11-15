# L'usage du modèle dans la trace de l'archiviste

15 novembre 2026, session mémoire (96). C'est le lot `usage` : la branche `cx-memoire-usage`,
posée sur `compose_restart` (c733b81dd). La demande vient du banc, par c7 : avant toute dépense
chez Vertex, il faut lire ce que le cache du fournisseur sert sur une fenêtre, et Lucie posera
peut-être une borne de dépense sur ce chiffre.

## Ce qui manquait

Le nœud `Decide` appelait `llm.generate` et jetait l'`Usage` rendu. Seule la boucle d'agent
émet `LlmCall` (`src/agent.rs`), si bien que ni `rag3weaver-backend` ni le journal des runs d'un
hôte ne gardaient les jetons d'un appel de l'archiviste.

## La forme

- **Sur l'élément** : `Decide` pose `usage` sur chaque élément qu'il étiquette par le modèle,
  et `LlmNode` posera la même forme quand il arrivera. Ses champs :
  - `ms`, toujours : la durée rendue par le client, ou mesurée autour de l'appel ;
  - `prompt_tokens`, `cached_prompt_tokens` et `completion_tokens`, **seulement si le fournisseur
    a rendu un bloc usage**.
- **Par section** : sous `trace: true`, chaque entrée de `results` du rapport d'`archive` porte
  cet `usage`.
- **Au total de la passe** : `usage_total`, en fin de rapport (`scripts/archive_rapport.rhai`),
  sous `trace: true` aussi. Le banc lit un total, pas une ligne par section. Ses champs :
  - `calls` ;
  - `without_usage` : les sections sans compteurs, y compris les échecs ;
  - `prompt_tokens`, `cached_prompt_tokens`, `completion_tokens` et `ms`, sommés sur les seules
    sections qui ont des compteurs.

## Le signe d'absence : `prompt_tokens == 0`

Un modèle qui ne rend pas de bloc usage (un modèle local, un faux serveur sans bloc) laisse les
trois compteurs **absents**, jamais à zéro : un zéro se lirait comme un appel gratuit.

Le critère est `prompt_tokens == 0`. Un appel envoie toujours une invite, donc un fournisseur qui
a compté rend toujours `prompt_tokens > 0`. `completion_tokens` ne sert pas de signe : sans bloc
usage, le client compatible OpenAI compte quand même les fragments reçus. Le premier essai du lot
l'a montré : `completion_tokens: 1` et `prompt_tokens: 0` sur la réponse sans bloc.

`cached_prompt_tokens: 0` dans une section qui a des compteurs est un vrai zéro : le fournisseur
a compté, et rien n'est venu du cache.

## Les témoins

- `e2e_archiviste::l_usage_du_modele_se_lit_par_section_et_au_total_de_la_passe` : trois réponses
  du faux serveur.
  - Avec usage, 120 jetons d'invite dont 100 en cache, et 7 de sortie.
  - Sans bloc : les champs sont absents.
  - Avec usage, 80 jetons d'invite dont 0 en cache, et 3 de sortie.
  - Le total attendu est `calls` 3, `without_usage` 1, 200 jetons d'invite dont 100 en cache,
    et 10 de sortie.
  - Le rouge a été vu à 14 h 29 : `usage` était null.
- Les témoins de l'étiquette dans `decide_nodes/tests.rs` lisent l'élément sans son `usage`, et
  vérifient l'usage à part.
