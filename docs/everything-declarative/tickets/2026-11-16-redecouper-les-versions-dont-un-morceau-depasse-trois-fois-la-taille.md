# Redécouper les versions dont un morceau dépasse trois fois la taille

- **État** : ouvert (16 novembre 2026)
- **Famille** : produit (confort)
- **Gravité** : confort. Les morceaux géants d'une base déjà indexée ont un vecteur : celui de leurs 512 premiers jetons,
  ce que le modèle ferait de toute façon. Mais la fin d'une ligne géante reste introuvable par le vecteur.
- **Atteignable en service** : oui, dans toute base indexée avant le correctif du découpeur (`line-chunker-char-bound`),
  pour les fichiers de données d'une ou deux lignes énormes.
- **Touche rag3weaver** : oui (le court-circuit de l'inchangé, `Catalog`, et le découpage des versions `ScopeContent`).
- **Pour** : ED, un jour calme (c7, 16 novembre 2026).
- **Signalé par** : ED, en livrant le correctif du découpeur ; décision de c7.

## Ce que c'est

Le correctif du découpeur coupe une ligne plus longue que `max_size` en cœurs d'au plus `max_size`. Un morceau fait donc
au plus trois fois la taille : un cœur et, de chaque côté, au plus `max_size` de contexte. Mais il ne vaut que pour un
contenu découpé de nouveau.

Une version déjà écrite garde ses morceaux. Le court-circuit de l'inchangé reconnaît son contenu et ses artefacts
dérivés, et rien ne la redécoupe. Les bases de Lucie gardent donc leurs morceaux géants (jusqu'à 53 294 caractères sur
le dépôt) jusqu'à la réécriture de leur contenu. c7 l'accepte pour la bascule.

## Ce qu'il faudrait

Un redécoupage forcé, borné, des seules versions dont un morceau dépasse `3 × max_size` :
- les repérer par la longueur de leurs morceaux ;
- les redécouper ;
- réembarquer leurs morceaux neufs ;
- le dire dans l'état d'index (combien, et lesquelles).

Ni montée de `REGLES_DE_L_ANALYSE`, ni repassage de toute la base : seules ces versions bougent.

## Comment le fermer

Par un témoin rouge d'abord : une base écrite avec un morceau de plus de `3 × max_size`, puis le redécoupage. Plus aucun
morceau ne dépasse la borne, et les autres versions ne sont ni redécoupées ni réembarquées.
