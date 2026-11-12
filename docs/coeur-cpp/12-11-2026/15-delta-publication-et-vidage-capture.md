# Delta de revue : publication acquire et vidage hors rollback

12 novembre 2026. Même [ticket annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md),
porteur cœur c++. Source `6bfba5b4c` sur `cx-coeur-diagnostic-saturation`.
[Delta depuis 01185695f](preuves/diagnostic-saturation-prive/delta-publication-vidage.patch),
[diff complet depuis 0511](preuves/diagnostic-saturation-prive/capture.patch),
[recette actualisée](preuves/diagnostic-saturation-prive/recette-revue.json).
**Toujours aucune configuration, compilation, exécution moteur ou réservation.**

## Publication et happens-before

`arm` initialise origin, first, samples et captureOutput, puis publie enabled=true
avec release. `armed()` lit maintenant avec acquire. Un fil moteur préexistant
qui observe true acquiert ainsi les données initialisées, sans dépendre de sa
création après armement. Celui qui voit false quitte le callback sans lire origin.
Les champs des compteurs restent atomiques relaxed ; leur lecture est un relevé
de fenêtre non transactionnel. La première capture conserve son protocole propre
firstState release/acquire, y compris pour le sampler lisant avant les joins.

Un seul armement dans un processus neuf reste requis. Le protocole ne permet pas
un reset concurrent et ne prétend pas résoudre une course de réarmement.

## Sortie privée avant acteurs, un seul écrivain à la fois

Le test exige `CX_SATURATION_OUTPUT` absolu. Avant lancement des acteurs, fopen
avec mode `wx` crée un fichier neuf ; pas de fallback stderr ni d'écrasement.
Le wrapper futur doit valider son emplacement dans l'unique montage output privé
et refuser parents symboliques/chemin hors sortie autorisée. Le schéma est écrit
et fflush avant enabled=true ; échec d'ouverture/écriture initiale = arrêt avant
acteurs, capture non valide, sans contournement ni répétition.

Le sampler existant appelle sample à chaque passage d'environ 100 ms. Il commence
par lire firstState avec acquire : si 2, il écrit first_meta, les 16 champs de first,
puis `first_saved` et fflush. Aucun I/O, allocation de chaîne ou attente ajoutés
dans refuse/rollback PK. Il conserve et vide ensuite au plus un sample par seconde,
maximum 1280. Le fichier contient donc des relevés même si les acteurs ne finissent
pas ; le marqueur first_saved délimite la première capture complètement émise.
La cadence est une cible, pas une garantie de scheduling ou de latence d'I/O.

Après joins, le thread du test prend l'unique rôle d'écrivain : il vide une première
capture encore en attente et émet status avec complete, first_state, sample_count,
overflow, puis fflush et fclose contrôlé. Toute erreur I/O devient sticky : aucun
retry dans le sampler, aucune poursuite de sortie après échec, complete non déclaré
par le code de résultat. Le test signale une erreur finale nommée, sans remplacer
l'exception d'origine dans les callbacks moteur.

La mémoire de capture reste fixe ; pas de file d'attente de chaînes. Le volume CSV
est borné structurellement par 1280 lignes sample, un schéma, une première capture
et un status. Le plafond de sortie de la recette reste 16 Mio pour l'ensemble des
logs. Un fichier sur stockage plein peut néanmoins bloquer/échouer ; le délai
extérieur et la preuve d'arrêt restent nécessaires. Aucun fsync ou écriture
synchrone dans le rollback. fflush garantit la remise au noyau, pas une durabilité
face à panne machine.

## Timeout : preuve partielle, jamais faux succès

Le fichier est sur le montage output privé extérieur au processus : ne pas le
placer dans un tmpfs dont la destruction de l'unité supprimerait les preuves.
Sur timeout, conserver les octets déjà remis au noyau, code de sortie/limites et
journaux. Une dernière ligne coupée, absence de status final, firstState=1, erreur
I/O, overflow ou absence de marqueur first_saved pour firstState=2 empêchent de
classer la capture globale complète. Un first_saved valide peut soutenir une
observation partielle de saturation ; ce n'est ni une réussite de test ni une
preuve de récupération. Sans première capture publiée avant arrêt, ne pas conclure
qu'aucun refus n'est survenu. Aucune relance automatique.

## Limite du cumul PK, y compris en présence d'un relevé complet

`pk_completed_ns` n'ajoute une durée qu'à la destruction du Scope PK : la durée des
appels encore actifs est **intégralement absente**, même s'ils durent depuis longtemps.
`pk_active > 0` interdit donc d'écarter le coût PK sur le seul petit cumul des appels
terminés. Les lectures calls/completed/active ne sont pas un snapshot atomique ;
même active=0 sur un relevé isolé ne certifie pas l'absence d'appels pendant toute
la fenêtre. Comparer les relevés voisins, appels commencés et compteurs de travail,
et conserver « indéterminé » si des appels chevauchants manquent au cumul. Aucune
estimation inventée de leurs durées ni métrique CPU déduite de la somme murale.

## Recette modifiée pour revue

Le commit de source et les empreintes du diff complet/delta sont actualisés dans
recette-revue.json. Avant build, relire les argv CMake et les définitions effectives,
les dépendances prêtes sans fetch, les montages privés d'extensions et les limites
de stockage/cgroup. Le build reste séparément non autorisé à ce stade ; aucun
manifeste de binaire ne peut encore être fourni.

Runtime proposé seulement après revue du futur manifeste : même filtre unique,
charge/pool inchangés, `CX_SATURATION_OUTPUT={private_output}/capture.csv` neuf,
output monté persistant pour inspection après arrêt. Valider statut CSV ET statut
GTest séparément. Capture complète n'implique pas test vert ; test fini n'implique
pas capture complète. Les limites de provenance et métriques absentes du document
14 restent applicables. Vérification présente : lecture de source et diff --check,
pas un résultat de compilation ou de moteur.
