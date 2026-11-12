# Le juge Q8_0 sur card2 : cinq et dix fiches, 12 novembre 2026

Le GPU de travail fonctionne : **Qwen3-4B Q8_0 classe cinq fiches en 540 ms
et dix fiches en 787 ms par cas médian**, sur card2 de luciepc. Les poids,
le cache et les buffers de calcul sont attribués à PCI 0000:04:00.0.
Le processus a conservé sur seat1 un petit contexte stable de 40 KiB de VRAM
et 2 080 KiB de GTT pendant les 128 captures, compatible avec la précision
de Lucie : garder la carte disponible pour jouer, sans exiger zéro FD.

Mesure et preuves : [ticket fermé](../../tickets/closed/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md).
Le premier passage arrêté est conservé ; la passe réussie est un nouveau passage
unique coordonné, pas une réécriture de son résultat rouge.

## Résultats

88 cas construits, avec un positif chacun. K5 conserve les quatre distracteurs
historiques ; k10 ajoute cinq fiches distinctes d’autres sessions, graine 20261108.
Les contextes sont ceux du jeu « début 2 k », borne estimée en caractères,
pas une limite exacte du nombre de jetons Qwen. Le préremplissage médian de la
première fiche traite 2 214 jetons, gabarit et fiche inclus.

| candidats | paires | utile première | utile dans les deux premières | AUC dans le cas | médiane par cas | p95 par cas |
|---|---:|---:|---:|---:|---:|---:|
| 5 | 440 | 67/88 = 76,1 % | 84/88 = 95,5 % | 0,918 | 539,9 ms | 635,8 ms |
| 10 | 880 | 58/88 = 65,9 % | 78/88 = 88,6 % | 0,932 | 786,6 ms | 883,4 ms |

Sur les 34 cas de test réservés par session : première utile 26/34 (76,5 %)
pour k5, 24/34 (70,6 %) pour k10 ; utile dans les deux premières 32/34 (94,1 %)
et 30/34 (88,2 %). Aucun ex æquo avec le positif. Les **440 scores communs
sont identiques**, écart maximum nul : l’ajout de distracteurs change les rangs,
pas les scores des paires conservées.

Les cinq distracteurs ajoutés sont aléatoires hors session, tandis que le groupe
historique comportait aussi deux négatifs proches. Une AUC plus haute en k10
ne prouve donc pas une amélioration du juge. Ce jeu construit ne mesure ni le
rappel réel ni le gain de couverture de passer un vrai rappel de cinq à dix.
Les latences n’incluent pas l’appel de rappel ni le temps de démarrage serveur.

## Exécution et état des caches

Même llama.cpp b9590/d2462f8, sans rebâtir ni modifier le clone de Lucie.
Poids convertis Q8_0 : SHA256
`ed119d433ab8076b2c05d9116b2b80b6ddc4115292583cea231024c5628a50bb`.
Binaire : `0f4196a018f8aeeb87dc8162f315b8466aefc5815197183429d324fd15ec83c7`.
Six fichiers de tokeniseur/configuration figés et revérifiés avant chargement.

Paramètres effectifs :

```text
ROCR_VISIBLE_DEVICES=0
--device ROCm0 --split-mode none --main-gpu 0
-ngl 99 -c 16384 -np 1 --fit off --no-warmup
--cache-ram 0 -lv 4 --host 127.0.0.1 --port 8099
--slot-save-path <dossier de preuves>/slots/
```

Un seul serveur, k5 puis k10, un jeton prédit par requête et `cache_prompt=true`.
Cache RAM global désactivé ; cache logique du slot effacé via l’action `erase`
avant chaque série, réponses conservées. La première paire de chaque série
rapporte `cache_n=0`. Les suivantes réutilisent le préfixe : cache positif sur
352/352 puis 792/792 paires suivantes des cas ; médiane 2 059,5 jetons réutilisés,
contre environ 151/152 jetons nouvellement évalués.

Les deux séries ont un prompt initial froid, mais le même serveur reste utilisé :
la première paire k5 paie l’échauffement des kernels, k10 hérite d’un serveur déjà
échauffé. Le premier cas coûte respectivement 594,5 et 834,9 ms. Les captures
chaque seconde et les différences de flags/caches avec le 11 novembre font
partie du protocole. **Ne pas présenter cette latence comme un speedup à
configuration identique** de l’ancien banc sur card0.

## Disponibilité de seat1 et retour des services

Le code figé, la sélection explicite, le PCI dans le journal, les buffers
modèle/KV/calcul et les allocations du processus prouvent le placement de la
charge du modèle sur card2. Le filtre de visibilité ROCm n’empêche pas
l’ouverture de petits états internes sur les GPU masqués ; les sources 7.2.4
et les limites de cette explication sont dans le ticket.

Les compteurs d’exécution du processus HIP ne sont pas exposés dans ces
captures : ils restent **inconnus**, pas zéro. La chaîne de placement et le
contexte seat1 resté minuscule ne mesurent pas chaque microactivité du pilote
et ne constituent pas une garantie de FPS. La charge globale du bureau/jeu
n’est pas attribuée au banc par déduction. Pas de profilage GPU supplémentaire.

Fenêtre exclusive `poste mesure` acquise à 18:32:47 Paris, startup à acquisition
+120 s, budget global 1 200 s. 7882 remis après arrêt ciblé et contrôles frais,
avec 11,685 Gio libres sur card2. Placement validé avant paires à 18:33:53.
K5 finit à 18:34:43 ; k10 à 18:35:53. Aucun échec du banc ni retry.

Serveur et ses groupes arrêtés à 18:35:54, port 8099 libéré, mémoire card2
revenue exactement à baseline 21 661 892 608 octets ; pic 28 807 573 504 octets,
soit une hausse de 6,655 Gio. L’optimiseur restaure 7882 à 18:36:29.156,
PID 3431955, même binaire/configuration, santé réelle granite-278m/768/Flex32
et GPU card2 ; les trois autres services restent inchangés. Banc puis orchestration
revérifient le retour ; verrou rendu à 18:37:45, fin sans échec, 298,274 s.

Le wrapper d’arrêt avait donné un faux rouge sur un relevé du port trop précoce.
Ce journal reste conservé ; l’absence effective du PID/port, la mémoire rendue
et les autres services ont été vérifiés indépendamment avant la remise.
Son correctif est suivi par [optimiseur](../../optimiseur/tickets/2026-11-12-attendre-la-disparition-du-port-apres-l-arret-du-prive-7882.md).

## Preuves conservées et portée

Aux deux postes : `~/.cache/rag3db-banc-notes/k10-20261112-passe2/`, avec scores,
captures brutes, journaux, effacements de cache, déchargement, retour et fin.
Analyse CPU locale : `analyser.py`, `analyse.json`, `k5-cas.json`, `k10-cas.json`.
Les résultats principaux ont été recalculés indépendamment par l’orchestration.
Paquet figé : `k10-controleur-v3/`, manifeste SHA256
`66d9a6b59dcadea022a2b9521d39776bc5c898d86f067cf9ba29e2b16dcbf3f3`.

Scores k5 : `250a487e4aee034ebdd9abab68ca9c6b97af2d1ef9979abc4a40080e1a438d8c`.
Scores k10 : `0676d9963be8f436891e9a5084444f8be907daa6ac54358044834d609ec18f06`.

Ces résultats sont transmis à la session mémoire pour lecture et choix du
rappel ; aucune activation de juge dans la mémoire réelle ni nouveau service
permanent n’a été effectuée. Card2 reste une ressource de travail normale :
coordination selon mémoire, charge et consommateurs, pas quota arbitraire de
processus ni nouvel accord GPU à chaque usage courant.
