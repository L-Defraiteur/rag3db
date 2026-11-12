# Paquet public74ad : suite complète verte, poste rendu

Étape corrective distincte autorisée par C7 sur recette0aa3f0272, GO publié0173a63b1 avant entrée. Source74ad9c25cd2a63e01abd54f25314992cda6fea60 ; mêmes 834 artefacts privés que le bâti02, tous vérifiés avant et après test contre leur manifeste. Aucun rebuild, installation ou code diagnostique privé0511 exécuté.

Suite unique sans filtre : **357 tests passés, deux outils sautés attendus, un cas désactivé**,359 sélectionnés/360 entrées XML, zéro failure/error. Durée1069,769s. Les deux sauts sont VectorIndexCrashReopenProbe.OpenInAFreshProcess et WalReader.PrintAFile. Le cas auparavant interrompu VectorCopyUnderASmallPoolTest.TheIndexReadsItsVectorsWhileTheHeadIsExchanged apparaît une seule fois, statusrun/resultcompleted, sans failure/error/skipped, en567,327s. Aucun verdict de deadlock ne découle du timeout précédent ; son ancien délai540s était insuffisant.

Unité03 : succès/exit0 en1071,981s ; CPU1252,743s, pic mémoire2G indiqué par systemd, swap0. Anciennes unités893,800s conservées : cumul1965,781s inférieur au maximum inter-unités annoncé2383,800s. L’étape corrective ne revendique pas le respect d’une ancienne deadline absolue2400s échue. Bornes inchangées :1480+10stop/1470contrôleur/1440suite, mémoire32G/scratch32G/swap0/net sans route.

Les montages effectifs ont été observés dans le processus test : parentancien02 privé tmpfsRO, source03 et alias02/sourceRO ; seules sorties test et preuvesRW. Aucune écriture dans l’ancien arbre source absent. Le fixture public garde512Mio/4fils ; environnement généralMAX_NUM_THREADS2 inchangé. Aucun service/GPU/modèle/PG touché. Le journal horodaté correspond à la réception des sorties, sans prétendre mesurer les instants internes exacts.

Après terminaison : unité inactive/dead, Resultsuccess, MainPID0, ControlGroupvide ; aucun processus de ce contrôleur ni de ce binaire test restant. Poste --qui ne montre que les autres lots, notre place rendue. Aucun nettoyage du paquet ou des sources privés conservés.

[Preuves complètes03](preuves/ci-format18/reprise-03/) : journal brut, réception horodatée, XML, exit/verdict, limites, argv/env, montages observés, manifeste avant test, scripts/recette exécutés, quiescence et inventaire SHA256. Les15 fichiers de cet inventaire ont été revérifiés après rapatriement sans écart. Les horodatages des preuves sont ceux de l’hôte ; l’enveloppe est évaluée sur durées monotones et durée systemd.

Le bâti4cibles02 et cette suite valident le paquet public74ad sous la recette décrite. Le formatage pur253fichiers et ses exclusions restent séparément documentés22 ; EROFS01 et timeout02 restent conservés et ne sont pas réécrits en verts. Ticket CI toujours ouvert : aucune CI future verte constatée ici, aucun GO ni publication master inclus. Résultat/source/recette/diff et preuves transmis à C7 pour revue et éventuel tour explicite.

## Revue finale et intégration

C7 donne revue finale favorable et « à toi immédiatement » surf8529a795. Exemption hygiène précise : git diff --check signale seulement une ligne vide en fin du journal brut preuves/ci-format18/reprise-03/transaction.log. Ce fichier est conservé fidèlement avec son SHA256 ; tous les autres chemins doivent rester verts. Rebase sur master65b57052f sous branche neuve cx-coeur-ci-format18-integration-2 ; aucun changement C++/flags entrant, donc aucun rejeu natif17min demandé par C7. Ticket CI demeure ouvert. Publication passe par poste push, dernier fetch, verdict VERT conditionnel et trois contrôles ; aucun GO de bibliothèque posée.
