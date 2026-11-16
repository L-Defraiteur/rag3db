# Le retour d’un snapshot oublie les voisins créés après sa prise

- **État** : ouvert (16 novembre2026), remède borné préparé pour le snapshot diagnostique principal ; recette génératrice non corrigée
- **Famille** : produit
- **Gravité** : importante — un WAL postérieur peut rester à côté de l’ancienne base restaurée
- **Atteignable en service** : non (recette de retour, hôte arrêté)
- **Touche rag3weaver** : oui

`~/.cache/rag3weaver-notes-A/instantanes.sh` écrit RETOUR.sh en développant
fichiers_de lors de la prise, et non au moment du retour. Si le snapshot n’a
pas de WAL et qu’une opération ultérieure en crée un, le retour remet les
anciens fichiers mais laisse ce WAL dans le vivant. Aucun retour réel lancé.

Rouge vu le16novembre sur fixture privée avec la recette réellement générée
pour principal : rc0, ancien fichier restauré, nouveau WAL encore vivant.
Preuves portable `~/.cache/optimiseur-notes/bascule-16nov-927/return-fixtures/old/`.
Pour le snapshot principal frais, RETOUR.sh préparé préserve dynamiquement tous
les voisins actuels dans un dossier après-retour, vérifie les SHA du snapshot,
puis copie les anciens fichiers ; API exigée inactive. Même filet isolé vert :
WAL et état après préservés hors vivant, ancien état cohérent. Preuves `new/`
et `verdict.json` dans ce dossier. Les unités sont des faux pour ce filet,
aucune base ni service réel opéré. La recette initiale reste archivée séparément.

Suite optimiseur : porter ce comportement dans le générateur versionné/sa
source de procédure, avec témoin rouge avant correctif ; tous voisins
base/wal/extensions/fts/checkpoints doivent être préservés au moment du retour,
aucun écrasement d’état après. Ne pas fermer ce ticket sur le seul remède privé.
