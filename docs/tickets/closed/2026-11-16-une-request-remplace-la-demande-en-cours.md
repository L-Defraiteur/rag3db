# Une nouvelle request (« continue ») remplace la demande en cours

- **État** : fermé le 19 novembre 2026 par `39203390d` (pile-archi-19, par le pousseur).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : réponse fausse : l'agent perd ce qu'on lui demandait, et va chercher ailleurs (D6).
- **Atteignable en service** : oui, dès qu'on relance une tâche par une request courte.
- **Touche rag3weaver** : oui
- **Signalé par** : c7, campagne D6 du 16 novembre 2026 (S3)

## Ce que c'est

La demande en cours est « la dernière request reçue » (`teammate.rs`, `let derniere = lines.iter().rposition(… "request" …)`).
Une request « continue » devient la demande ; l'agent a cherché le cahier des charges dans une page
de vision. Le champ `request` de `Step` et le nom de la fiche de session prennent la première, la
frontière la dernière : deux vérités.

## Ce qu'il faut faire

Une request qui suit une request encore ouverte (sans fin, `ended`) **s'ajoute** à la demande au lieu
de la remplacer : la frontière dit `Request (opened at …): <la première>` puis `Follow-up: <les
suivantes>`. Une request après `ended` ouvre une demande neuve. Ne pas toucher au format de la boîte
(le `kind` reste `request`).

## Comment le prouver

Rouge : faux modèle, une request « Écris un test de bonjour », un pas sans fin, puis une request
« continue » ; la frontière du pas suivant porte les deux textes, la première d'abord. Passe comme
le plafond.

## Taille et blocage

3 h. Aucun.

## Fermeture

La demande ouverte, ce sont les requests du superviseur depuis le dernier `ended`. La frontière dit `Request (opened at …): <la première>` puis `Follow-up: …`, et les pas faits se comptent depuis la première. La garde B et la boîte ne changent pas. Conception : `docs/rag3weaver-archi/18-11-2026/06-s3-la-demande-s-ajoute.md`. Rouge vu sur 6e9e06e4c ; passe complète verte sur 7def47a4a ; poussé avec la pile.
