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


## Correctif du vrai générateur publié et qualifié

Ticket/reprise fcc0b7d020 puis sources helper V2 reprises bitidentiquesf5b3e8d825 ;
témoin du générateur publiéc1cb61a414 AVANT fix. Rouge réel : génération et RET
rc0 mais WAL tardif encore présent, fixture privée
`~/.cache/generated-return-nsiznm4f/`, journal
`~/.cache/optimiseur-notes/return-generator-16/generated-red.log`.

Correctif e6ca9b4426, compléments des témoins62e6415159 : génération de RET
par `generate_snapshot_return.py`, helper e1fd copié dans chaque NOUVEAU
snapshot, inventaires épinglés et vérifiés tous avant le premier déplacement.
Aucune duplication du mécanisme de restauration : le RET appelle le helper V2.
Le fichier source voisin `restore_snapshot.py` est un lien Git vers le helper
public qualifié ; la recette générée embarque une copie régulière indépendante.

Six tests générateur/RET et cinq tests helper passent aux DEUX POSTES :
11/11, zéro échec, zéro saut. Le vrai instantanes.sh est exercé avec HOME privé,
reflinks réels, fichiers de fixture et faux systemctl ; aucun moteur ou service
réel lancé. Les autres filets couvrent deux bases/chemin avec apostrophe, pins
helper/inventaire altérés, second snapshot corrompu AVANT premier déplacement,
unités actives refusées, et invariants historiques du helper.
Les fixtures générateur et leurs journaux sont conservés.

SHA sources : instantanes.sh
`0cecea650a9db3dbe9dbe57159f63372339dd99cb35ef2b0d5909cb99eec9437` ;
generate_snapshot_return.py
`9e9885f52b20248c7502a7d96a64499c4c8d4b74c0a571ea55219f8b15723f33` ;
helper V2 toujours `e1fd96b25c1cc61b70507c59f7f203acb21936e5b676f68c74b88d216f923854`.
Reçus `~/.cache/optimiseur-notes/return-generator-16/local-62e6415159/verdict.json`
SHA595acf0b6e72c866647aebd32a418ab16792930e11fb6775235ef5a0a1155ef1 ;
PC `pc-62e6415159/verdict.json`
SHA73aa4ed3f98d1b62825bcffe9929f7c6a9cd422672e13637d28d07bf26262449.

Ticket encore ouvert : intégration et éventuelle pose du bundle décidées par C7.
Les caches historiques restent inchangés ; aucun ancien snapshot, RET, reçu ou
attestation72h modifié. Aucune CLI native, API, lib, Cargo ou base originale opéré.


## Audit de périmètre et préparation de pose, sans activation

Audit RO bb5 : huit chemins attendus, aucun hors scope ; arbres `src`, `test`,
`extension` et `tools/rust_api` byteidentiques à df2a. Helper et ses cinq tests
identiques à bf0aba. Lien source suivi Git vers le helper qualifié, bundle futur
composé de trois fichiers réguliers755 ; recette générée autonome ensuite.
Audit privé `~/.cache/optimiseur-notes/return-generator-16/audit-bb5/audit.json`,
SHA db81aa80932fe8db1dbdbed649ef229a64668c19e90cfa8998f34b8f75a94393.
Bundle privé préparé sous le même dossier, `return-generator-bb5d4695fd/`.

Plan de pose À EXÉCUTER UNIQUEMENT après ATOI exact C7 : vérifier source master
et trois SHA, copier un bundle versionné régulier dans
`~/.cache/rag3weaver-notes-A/return-generator-bb5d4695fd/`, préserver par `cp -p`
l'entrée historique sous `instantanes.sh.avant-return-generator-bb5-20261116`,
puis publier par renommage un lien atomique `instantanes.sh` vers le bundle.
`realpath` de l'entrée sélectionne les deux helpers voisins du bundle. L'ancien
inode n'est pas réécrit. Mode du script cible755, SHA0cecea65 ; helper e1fd et
outil9e9885f5 dans la même version. Rejouer les fixtures contre la copie posée
sur chaque poste et conserver les reçus. Aucun de ces chemins cache n'a été écrit.
Lecture fresh des deux entrées historiques : SHA f4ab49a97dea7983af91ddb52523a4a71e4cca4eadd0283313f065ce3d94464e inchangé.

Garantie documentée : refus de PRÉFLIGHT = aucun déplacement ; après premier
move une erreur peut être partielle, état après et preuves restent conservés.
Succès = ensemble ancien exact, pas fichier seul. Retour opérateur hors Common,
jamais de réinjection du bus propriétaire dans une validation privée. Aucune
qualification réelle de retour, restauration ou fermeture induite par les fixtures.
