# step_done avant report : l'hôte relance un pas entier

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : coût : un pas et ses jetons perdus, 3 cas sur 4 dans D6.
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui
- **Signalé par** : c7, campagne D6 du 16 novembre 2026 (S4)

## Ce que c'est

Le modèle appelle `step_done` puis seulement `report`, ou `step_done` seul ; la garde B relance un
pas entier pour obtenir la ligne.

## Ce qu'il faut faire

Si le même tour porte, après `step_done`, un texte ou un `report`, le compter pour la demande (la
garde ne relance pas). Et l'outil `step_done` refuse d'abord, une fois, quand la demande attend une
ligne et qu'aucune n'est encore écrite : « write your answer first (as text), then call step_done »,
pour que le modèle corrige dans le même pas, sans relance.

## Comment le prouver

Rouge : faux modèle, `step_done` puis `report` dans le même tour ; la boîte n'a pas de relance, et
le pas compte un seul appel au modèle de plus. Passe comme le plafond.

## Taille et blocage

3 h. Après la garde B pour une consigne.

