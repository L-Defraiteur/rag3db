# Un témoin de codex_threads rougit sur « Text file busy » sous la charge

- **État** : ouvert, le 15 novembre 2026.
- **Famille** : produit. Le défaut est dans un témoin de la lib du crate ; le code de
  `codex_threads` n'est pas en cause.
- **Gravité** : un rouge intermittent de la lib entière, qui fait rejouer une passe. Il est vert
  seul.
- **Atteignable en service** : non.
- **Touche rag3weaver** : oui, `src/codex_threads.rs`, le test
  `un_codex_qui_ne_rend_pas_la_main_est_coupe_au_delai`.
- **Pour** : la session qui tient le pont vers Codex (pont entre agents de code), ou qui le veut.
- **Signalé par** : la session mémoire (96), dans la passe du lot `segment` du 15 novembre 2026 au
  portable.

## Ce qu'on voit

```
codex_threads::tests::un_codex_qui_ne_rend_pas_la_main_est_coupe_au_delai ... FAILED
/tmp/.tmpbxAo6B/lent queue : ne démarre pas : Text file busy (os error 26)
```

Il est rouge dans la lib entière (2472 passés, 1 rouge, 20 h 03), et vert joué seul juste après
(1/1).

## Pourquoi, très probablement

Le test écrit un script (`std::fs::write`), le rend exécutable, puis le lance aussitôt. Sous Linux,
`execve` refuse avec `ETXTBSY` un fichier encore ouvert en écriture par un processus. Les autres
tests de la lib forkent en parallèle : un enfant forké pendant l'écriture hérite du descripteur
ouvert, le garde jusqu'à son propre `exec`, et le lancement du script tombe dans cette fenêtre.
C'est la course classique d'un exécutable écrit puis lancé dans un programme multi-fil. Seule la
charge de la lib entière l'ouvre.

## Ce qui serait juste

- Écrire le script dans un fichier temporaire, le fermer, puis le **renommer** à son nom final avant
  de le lancer : un `rename` ne porte pas le descripteur hérité.
- Ou réessayer le lancement sur `ETXTBSY`, quelques fois avec un court délai, dans le test ou dans
  `livrer`. Un vrai binaire de codex n'est jamais écrit par nous, donc le réessai ne servirait
  qu'aux tests.
- Ou lancer par `/bin/sh <script>` plutôt que le script lui-même : le fichier n'est alors qu'ouvert
  en lecture.
