# Le créneau 2 sort sans dire pourquoi : « rouge d'environnement » sans passe ni raison

- **État** : ouvert (24 novembre 2026, écrit par G)
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : le pousseur)
- **Gravité** : moyenne. Un créneau qui ne fait rien atterrir et dont la panne ne se diagnostique pas ; la nuit du
  23 au 24, quatre trains B, zéro lot atterri par lui.
- **Atteignable en service** : non, le pousseur seulement
- **Touche rag3weaver** : non (`tools/poste/pusher.sh`)

## Ce que c'est

Le 24 novembre 2026 à 01 h 19 min 27 s (`step-conditions+1`) et à 01 h 36 min 58 s (`ed-tty-theme-served+1`), le
journal du pousseur dit « créneau 2 jeté — rouge d'environnement, rien de jugé sur ses lots ». Aucune des deux n'a de
ligne « reposé (créneau 2) » ni de passe : `spec_run` est sorti avant de reposer, avec le code 2. À 01 h 37 min 10 s,
le créneau 2 suivant s'est reposé et a joué normalement : c'est intermittent.

Dans `spec_run`, deux chemins écrivent ce code 2 sans une ligne au journal : le `gfetch` des branches des lots du
train B, et le push de la branche reposée de B (`g push origin HEAD:refs/heads/$RB`). Les autres sorties disent leur
raison (« worktree absent », « … a bougé », « la repose sur la tête de A bute », « master rouge à check_tickets »).
Lequel des deux a joué ne peut se savoir après coup.

## Ce qu'il faut

1. Chaque sortie de `spec_run` écrit sa raison au journal, avec la première ligne d'erreur de git (comme « PR
   absente : gh pr create a échoué » le fait déjà).
2. Un témoin, rouge d'abord : un faux remote qui refuse le push de la branche de B ; la ligne « créneau 2 jeté » doit
   nommer le push refusé et le message de git.
3. Le créneau 2 fermé (`PUSHER_SLOTS=1`) tant que la cause n'est pas lue, si l'orchestrateur le décide.
