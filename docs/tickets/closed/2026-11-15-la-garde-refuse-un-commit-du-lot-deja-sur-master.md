# La garde refuse un commit du lot déjà sur master

- **État** : fait (15 novembre 2026, master bdeac0b86) : un « < » du range-diff dont le patch est déjà sur master (git cherry) est toléré ; fermé à la passation de G le 16 novembre
- **Famille** : produit
- **Gravité** : faux refus ; un lot juste ne passe pas, et la session repose sa tête à la main.
- **Atteignable en service** : non, c'est l'outillage du push.
- **Touche rag3weaver** : non ; `tools/poste/push_garde.sh` (`verifier_pile`).
- **Pour** : optimiseur (G).
- **Ouvert le** : 15 novembre 2026

## Ce qui se passe

Un lot reprend un commit de docs, parce que sa base de pile n'avait pas le document. Ce même commit est déjà sur master,
poussé par un autre lot. Au rebase, git laisse tomber le patch comme déjà appliqué. Le range-diff du lot montre alors ce
commit en « < » (perdu), et `verifier_pile` refuse : « le lot n'est plus celui qui a été testé ».

## La correction attendue

Un « < » dont le patch est déjà sur master (`git cherry <master> <commit> <commit>^` le marque « - ») est toléré, et dit
au journal : « déjà sur master : <hash> ». Un « < » dont le patch manque à master reste un refus. Rouge d'abord dans
`test_push_garde.sh`, sur le dépôt jouet : un commit du lot déjà sur master par un autre push.
