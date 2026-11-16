# Les passes épinglent la lib 373b5477b : à changer au moment de la bascule

- **État** : ouvert (16 novembre 2026) — bloqué par la bascule de lib (mot de Lucie sur l'heure)
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
