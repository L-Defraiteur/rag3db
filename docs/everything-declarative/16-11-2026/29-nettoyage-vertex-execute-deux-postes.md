# Nettoyage Vertex exécuté le 17 novembre

GO `C7-ED-CLEANUP-6309-17nov`, conditions lues dans orchestration
`c1098c0c68`, rapport17, section ED. Périmètre exact du
[contrat27](27-contrat-nettoyage-vertex-preparation-seule.md), JSON
`6f6f4f84559024c202f5e6828954f957bab042b2e5695ef9b6a52aa116d98742`.
Pas de nouvelle qualification produit ni de push master.

## Résultat borné

| Hôte | Worktrees et admins retirés | Quarantaines conservées | Target retiré | Final publié avant 600 s |
| --- | ---: | ---: | --- | ---: |
| lucie-tablette | 14 | 1 | absent avant opération | 395.322830274 s |
| LuciePC | 7 | 3 | seule racine capturée du contrat | 404.943129533 s |

Chaque retrait a une nouvelle garde privée6f3 VERT, avec son rapport conservé.
Les quatre ignorés exacts ont seulement été renommés vers des quarantaines
privées exclusives, toujours présentes. Toutes les branches sont conservées.
Les références du WT actif landing-docs avaient normalement avancé de la
photo0fc vers le contrat autorisé6309 ; ce WT portable reste présent.

Sur PC, le target a été retiré AVANT son WT propriétaire union. Vérifications
fraîches : UID/dev/inode/marker, 19763 fichiers, 4302 sous-dossiers, six liens
lus sans suivre leur cible, dix dépôts de fixtures inspectés avec les primitives
6f3 et utilisateurs exclusivement internes. Copie de fixtures fidèle à
l'archive. Garde du WT propriétaire VERT pendant qu'il existait, puis renommage
de la racine capturée et retrait de cette seule racine.

Git normal a refusé trois chemins PC pour la seule présence de sous-modules :
token-fix-r1, token-refresh-r3, token-union-r1. Refus rc128 conservés. Ensuite,
nouvelles copies source/admin fidèles et archive PAX réempreintée et extraite
sur le portable, inventaires mode/UID/GID/mtime_ns/SHA exacts. Archive
`force-snapshots-pax.tar.zst` SHA
`750701936596c04629b01b8552034ca03fd66dc9ebaa00424169f335e52e217e`.
Nouvelle identité/quiescence, garde6f3 VERT et aucun pair du module privé,
puis UNE tentative force par chemin exact, chacune rc0. Aucune garde rouge
forcée, aucun prune, aucune branche supprimée.

## Refus et erreurs opératoires conservés

Le premier contrôle du target comptait sa racine en plus des sous-dossiers :
refus du harnais avant renommage/retrait, puis correction du compteur sans
changement des données. Le premier nouvel archivage pour force ne préservait
pas les nanosecondes à l'extraction : fidélité refusée ; archive originale
gardée et remplacée pour admission par une NOUVELLE archive PAX fidèle.
Une entrée force sans accusé deux-hôtes disponible a refusé avant tout retrait.
Ces arrêts sont dans `actions.jsonl` ; ils ne deviennent pas des contrôles verts.

Les contrôles finaux ont aussi corrigé deux hypothèses opératoires : photo de
référence docs antérieure au contrat6309, puis présence du WT actif portable
supposée à tort sur PC. Aucun nouvel effacement ni changement de branche pour
les faire passer. Les sorties initiales sont dans la conversation opératoire ;
les paquets ne prétendent pas contenir leurs consoles brutes manquantes.
La même borne600/hôte a été conservée, sans nouvelle allocation ni remise à zéro.

## Preuves conservées et copiées

Sous `~/.cache/rag3weaver-build/`, les DEUX paquets suivants existent sur les
DEUX hôtes ; tailles/SHA de chaque fichier réempreintés après copie, zéro écart :

| Paquet | Fichiers hors manifeste | SHA manifeste |
| --- | ---: | --- |
| cx-ed-cleanup-6309-proof-local-v1 | 59 | ccef8912ea861752218c3d83270283713402cf3fc91d3d72aed7cb17422808cf |
| cx-ed-cleanup-6309-proof-pc-v1 | 54 | e37598461a6c32d8d717a2d66cc6a68d1752e1ecb06d0d2a48af0d6fe59cdffc |

Autorités600, rapports préflight/guards, actions, refus, scans et FINAL inclus.
Scans finaux `found=[]`, aucun UNKNOWN attribué ; les UNKNOWN globaux restent
neutres, aucune absence GPU n'est inférée. Les quatre TAR du contrat et les
preuves originales rouge39/vert59/union54 ont été intégralement réempreintées
avant le FINAL sur chaque hôte. Archives source/admin, quarantaines, P/Q/S,
RAW/ELF, corpus et OUT_T Gdeps restent conservés. Aucune économie physique
déduite en présence des copies COW.

Aucun signal, Cargo, rustc, test natif, service, SQL, modification de cache
commun ou SDK. Les tickets métier gardent leurs critères et états antérieurs.
