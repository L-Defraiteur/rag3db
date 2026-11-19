# L'IDE en plusieurs fenêtres

- **État** : ouvert. Pris par la session tty le 19 nov. 2026, au mot de c7 ; ED relit la conception et le diff.
- **Famille** : produit
- **Gravité** : manque. Un souhait de Lucie resté en graine depuis le 13 novembre.
- **Atteignable en service** : oui, dans l'IDE.
- **Touche rag3weaver** : oui (`ui/ide`, le SharedWorker du flux `lib/flux-worker.ts`, `etatPartage` de `lib/host.ts`, les vues).

## Ce que Lucie a dit

Le 13 novembre 2026, au soir (relevé dans la graine `docs/orchestration/visions/README.md` et dans la mémoire des retours sur l'IDE) : elle a **plusieurs onglets de l'IDE ouverts en même temps**, et elle veut pouvoir **séparer l'IDE en fenêtres** : le chat dans une fenêtre, le graphe ou le dépôt dans une autre, **sur deux écrans**, avec un petit état partagé (dépôt courant, sélection, conversation ouverte, cartes d'accord en attente) porté par le même worker que le flux.

## Ce qu'il faut

La conception est dans [`docs/tty/19-11-2026/10-l-ide-en-plusieurs-fenetres-conception.md`](../19-11-2026/10-l-ide-en-plusieurs-fenetres-conception.md), en cinq tranches. Chacune a son témoin navigateur, rouge d'abord, et la règle « l'IDE ne demande rien à l'hôte au repos » tient.
