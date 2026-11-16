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

## Recette de retour V2 publiée avant CLI live

Source opérationnelle publique `bf0aba2cfc011f51f56e0419c63a3768cdb198ea`,
`docs/optimiseur/16-11-2026/bascule/restore_snapshot.py`, SHA256
`e1fd96b25c1cc61b70507c59f7f203acb21936e5b676f68c74b88d216f923854`.
Témoin public rouge92d8e807b6 sur ancien comportement ; cinq filets verts aux
deux postes : voisins tardifs+WAL archivés avec SHA/types, ancien ensemble
reformé, snapshot corrompu/extra refusés, API active refusée, preuve empiétant
sur snapshot refusée sans écriture. Ni moteur ni service réel dans les fixtures.

Attestation séparée principale PC `RET_v2-attestation.json` dans le dossier du
reçu frais, SHA24eced05414acb1e505b49ff018519820cc48cbbb6597824bf7af47befda452b ;
`RETOUR_v2.sh` du snapshot, SHA f421c67d8df9437db2870660a7fd83ace49e4e68e12e88226286c515e1b74b89.
Le reçu e215 et son retour initial restent immuables, données733 identiques.
Qualification limitée aux fixtures privées ; aucune restauration réelle ni
permission CLI déduite. Le générateur de snapshots demeure à corriger : ticket
ouvert, même si chaque snapshot de cette fenêtre reçoit un retour V2 qualifié.

## Reprise du générateur — lot G autorisé après garde PILE

Branche `cx-optimiseur-return-generator-16`, base df2a578988. Localisation RO :
source réellement utilisée `docs/procedures/scripts/instantanes.sh`, identique à
`~/.cache/rag3weaver-notes-A/instantanes.sh`, SHA
f4ab49a97dea7983af91ddb52523a4a71e4cca4eadd0283313f065ce3d94464e.
Appelée par `docs/procedures/scripts/instantanes-bascule.sh` ; le défaut est bien
la liste développée à la génération de RETOUR, pas le helper V2 déjà qualifié.
Filets existants : cinq tests `bascule/test_restore_snapshot.py` sur le helper,
aucun témoin trouvé du vrai générateur. Aucun code Rust/Catalog concerné.

Périmètre avant diff : fixture de fichiers privés, faux systemctl exclusivement,
invocation du vrai générateur puis de son RETOUR après création d'un nouveau WAL.
Correctif prévu : émettre une recette autonome épinglant le helper V2 et un
inventaire du snapshot, en réutilisant sa conservation dynamique des voisins.
Les snapshots existants, recettes, attestations72h et caches posés restent
inchangés. Aucun moteur, writer CLI, fermeture ou opération API autorisé ici.

Le helper bf0aba et ses cinq témoins, encore sur la branche publique G, sont
repris bitidentiques dans ce lot pour rendre le générateur autonome dans Git.
