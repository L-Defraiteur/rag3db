# Les passes épinglent la lib 373b5477b : à changer au moment de la bascule

- **État** : fait 68590270e21d00285b29800bd9cdecf0cb921461 — pin59ac intégré, copies et témoins verts aux deux postes le16novembre à11:52
- **Famille** : produit (outillage : `tools/poste/passe_epinglee.sh`, scripts de G)
- **Gravité** : après la bascule, toute passe tournerait sur l'ancienne lib, sous `RAG3WEAVER_MOTEUR_ANCIEN=1`.
- **Atteignable en service** : non
- **Touche rag3weaver** : non
- **Pour** : la session qui reprend l'outillage de G (Codex), juste après la bascule
- **Signalé par** : G, au plan de la bascule (`docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md`)

## Ce qu'on voit

`tools/poste/passe_epinglee.sh` nomme la lib en dur :
`export RAG3DB_LIB=rag3db-build-373b5477b4fd413f96605061b8d09393439a1a30-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
Le commentaire au-dessus dit l'historique des libs épinglées. `test_passe_epinglee.sh` attend « lib 373b5477b » en
première ligne du résumé. `~/.cache/rag3weaver-build/lib_seule.sh`, aux deux postes et dans aucun dépôt (sha256
8f60e82f60a4a7b5), épingle la même lib : il est remplacé par `run_e2e.sh --lib <filtre>` depuis ccfded41c4.

## Ce qu'il faudrait, le jour de la bascule (après « courant » posé aux deux postes)

1. Un lot court : la ligne `RAG3DB_LIB` de `passe_epinglee.sh` nomme la lib de la bascule
   (`rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`,
   ou celle qu'annonce G ou c7 si elle change avant). Le commentaire ajoute la date et l'ancienne lib.
   `test_passe_epinglee.sh` attend le nouveau nom, et est rouge sur la passe d'avant.
2. Rejeu court : `bash tools/poste/test_passe_epinglee.sh`. Push au « à toi » de c7, en tête de file.
3. Recopie au cache des deux postes (forme dans la page de passation de G), puis l'annonce à toutes les sessions :
   « RAG3WEAVER_MOTEUR_ANCIEN retiré, lib <id> épinglée ».
4. `lib_seule.sh` : le retirer du cache des deux postes, en gardant `lib_seule.sh.avant-retrait`, après l'annonce que
   `run_e2e.sh --lib` le remplace.

## Reprise après pose réelle du 16 novembre

Bascule FINI11:32 : plan existant complété avec sources, hashes, snapshots,
contrôles et réouvertures. Témoin du pin vu rouge sur la ligne373b (journal
`~/.cache/optimiseur-notes/bascule-16nov-927/pin-red.log` portable), test e271ee084c.
Correction pin59ac complète en préparation ; garder ouvert jusqu’à atterrissage,
recopie au cache des deux postes, témoins contre ces copies et annonce aux pairs.

## Clôture vérifiée — 16 novembre

Témoin rouge e271ee084c ; correctif68590270e2 intégré par garde et poste push,
status propre/aucun rebase/origin-master=HEAD lus aux deux postes. Vraie passe
candidat : témoin lectures1/1 et filets2/2, CHECK_FEATURES8/8, preuve de recompilation,
lib2594 réussis/0échec/72ignorés, navigateur45/45 sans ignoré. Les45 tests navigateur
font partie des72 ignorés de la lib ; les27 autres restent NON JUGÉS, inventaire
`~/.cache/optimiseur-notes/bascule-16nov-927/ignored-qualification.json` à luciepc.
Aucune hausse des tests ignorés introduite : aucun Rust produit modifié depuis927.
Scope `pin-validation/validation-vykn41uz` VERT/code0/restes[]/nettoye=true,
32Gio. Journaux réels archivés aux deux postes :
`~/.cache/rag3weaver-build/archives/pin-59ac-20261116-68590270/`.

`passe_epinglee.sh` installé atomiquement aux deux caches, mode755, source et copie
SHA256 `2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d`.
Témoins joués contre les copies réelles, verts aux deux postes. Anciens conservés
`.avant-59ac-20261116-68590270`. Annonce à c7, banc et arbre : lib59ac épinglée,
MOTEUR_ANCIEN inutile pour source927 ; `run_e2e.sh --lib` remplace l’ancien script.
Après annonce, `lib_seule.sh` déplacé aux deux caches vers
`lib_seule.sh.avant-retrait-59ac-20261116` ; aucun contenu supprimé.

La maintenance des bases et l’oracle sont distincts : maintenance portable arrêtée
sur422, pas de migration déclarée faite. Targets/deps release préservés à la demande
de c7 pour Banc ; aucun nettoyage forcé, aucune reprise d’ancien GO.
