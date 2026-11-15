# La garde compare des hashes courts de longueurs différentes

- **État** : fait (15 novembre 2026, master d68dbe3cb6) : la tête et le commit testé se comparent entiers (`rev-parse ^{commit}`) ; test_push_garde 42 vert, 2 ROUGE sur la garde d avant ; la garde recopiée au cache des deux postes (sha256 ad0be1f36ff12ea5). Vu par G à son push de garde-cpp-transferable.
- **Famille** : produit
- **Gravité** : un message faux au journal, sans effet sur le verdict ; une base qui le serait, elle, passerait.
- **Atteignable en service** : non, c'est l'outillage du push.
- **Touche rag3weaver** : non ; `tools/poste/push_garde.sh`.
- **Pour** : optimiseur (G).
- **Ouvert le** : 15 novembre 2026

## Ce qui se passe

Hors pile, la garde compare `git rev-parse --short HEAD` au commit testé de `resume.log`. La première forme rend ici
10 caractères (4f48b2d94e), parce que le dépôt en exige 10 pour rester sans ambiguïté. Le résumé en écrit 9 (4f48b2d94).
Les deux chaînes diffèrent, et la garde prend la voie « HEAD descend du testé par des commits de doc ». Elle dit alors
« HEAD ajoute de la doc seule au commit testé », alors que HEAD *est* le testé. Le verdict reste juste, puisque le diff
est vide.

## La correction

Comparer les commits entiers : `git rev-parse HEAD` contre `git rev-parse "$teste^{commit}"`. Un témoin : un résumé
écrit en 7, 9 ou 12 caractères, et une tête égale au testé, ne doivent jamais faire dire « de la doc seule ».

## Ce qui l'a révélé, à côté

Au même push, la base écrite pour la garde (ee3b5c901) était plus récente que la vraie base testée (42a916f08). G l'avait
lue sur `origin/master` après le rebase, et cette référence est commune à tous les worktrees du dépôt. Aucun code entre
les deux, donc sans effet. Depuis 5b8f8994f, `passe_epinglee.sh` écrit la base par `merge-base HEAD origin/master`,
au moment de la passe.
