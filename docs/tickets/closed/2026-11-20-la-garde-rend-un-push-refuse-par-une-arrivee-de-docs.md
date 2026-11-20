# La garde du pousseur rend un lot quand des docs arrivent sur master entre son fetch et son push

- **État** : ouvert (20 novembre 2026, écrit par G à la demande de c7)
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : le pousseur et sa garde)
- **Gravité** : moyenne. Une passe Rust perdue (treize minutes le 20 novembre), et le créneau 2 du pousseur jeté avec
  elle.
- **Atteignable en service** : non, le pousseur seulement
- **Touche rag3weaver** : non (`tools/poste/push_garde.sh`)

## Ce que c'est

Le 20 novembre à 19 h 24, le lot `arbre-open-view` avait fini sa passe : il était vert, et la garde allait le pousser.
À ce moment, c7 a poussé directement sur master un commit de chronologie (35647eca8b, `docs(orchestration)` seules).
La garde a dit « ARRÊT : push refusé », a remis le worktree sur la tête d'avant le rebase, et le pousseur a reposé le
lot au pas suivant. La passe a été rejouée, treize minutes perdues. Le créneau 2 (`cx-relecture-retenue`) a été jeté.

La garde sait déjà qu'une arrivée de docs seules ne demande pas de rejeu : à son `fetch` (`push_garde.sh`, vers la
ligne 280), elle rebase la pile sur le nouveau master et pousse sans rejouer, tant que `code_apporte` est vide. Mais
entre ce `fetch` et le `git push` (vers la ligne 348), il y a l'ouverture de la pull request (`PRE_PUSH`), qui dure
quelques secondes. Une arrivée dans cet intervalle fait refuser le push, et la garde ne recommence pas.

## Le correctif proposé

Sur « push refusé », la garde relit master. Si ce qui est arrivé depuis son `fetch` n'est que des docs (rien hors
`.md` ni `docs/`, ou plus largement `code_apporte` vide), elle rebase et repousse sans rejeu, jusqu'à trois fois.
Si du code est arrivé, c'est une repose, comme aujourd'hui.

**Témoin rouge d'abord** (`test_push_garde.sh` ou `test_pusher.sh`) : un commit de docs posé sur master pendant le
crochet `PRE_PUSH` d'une fausse passe. Aujourd'hui, « push refusé » et le lot reposé. Attendu : le lot poussé dans le
même pas, et la ligne « docs arrivées pendant le push : rebasé, repoussé » au journal. Un commit de code posé au même
moment reste une repose.

## D'ici le correctif

c7 ne pousse plus ses docs que quand le pousseur est au repos : la dernière ligne du journal dit « poussé », « jeté »
ou « rouge », et rien n'est « pris » après. Les autres sessions passent par le pousseur.
