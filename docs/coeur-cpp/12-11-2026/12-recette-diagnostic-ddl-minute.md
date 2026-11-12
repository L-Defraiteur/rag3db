# Recette proposée : départager le rouge DdlEveryMinute

12 novembre 2026 — **proposition à relire, aucune exécution autorisée ici**.
Même [ticket annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md),
porteur cœur c++. Aucun regroupement A ni optimisation COPY dans cette recette.
Le diagnostic de la couverture de quarantaine est distinct ; son ticket propre
est préparé par la revue croisée. Ne pas confondre ce risque statique avec le
rouge observé ci-dessous.

## Preuves disponibles et première séparation causale

Deux journaux existants à luciepc, sous
`~/.cache/rag3db-moteur-notes/annulation/` :

| Observation | base-TwentyWriters.DdlEveryMinute.log | isole-TwentyWriters.DdlEveryMinute.log |
|---|---|---|
| Pic de tampon avant DDL 2 | 1152 Mio (:32) | 1145 Mio (:1153) |
| Pic de tampon avant DDL 3 à 180 s | 1176 Mio (:33) | 2051 Mio (:1154) |
| Pic avant DDL 4 à 240 s | 1177 Mio (:34) | 2052 Mio (:1155) |
| CREATE_VECTOR_INDEX | 752409 ms (:23–24), réussi | 172 ms (:427–429), refusé |
| ADD b | réussi (:28–29) | pool plein, refusé (:1167) |
| Échec final | contrôles de latence | SET b : Binder, b absente (:1201) |

L'atteinte du plafond précède CREATE_VECTOR_INDEX et même le troisième DDL.
Le coût de construction de cet index n'explique donc pas, à lui seul, cette
pression antérieure. Cela ne départage pas coût de rollback PK, débit réalisé,
transactions retenues, allocations résidentes ou autre mécanisme entre DDL 2 et 3.
Les pics agrégés ne donnent ni l'instant exact du premier échec ni le propriétaire
des pages. Le Binder final est une conséquence compatible avec ADD b refusé,
pas la preuve d'une faute d'annulation de colonne.

Les fichiers portent un BuildID historique 29a7 (dirty pour le correctif), pas
l'identité littérale des sources comparées. Toute nouvelle preuve doit joindre
commit, diff effectif et empreintes binaires ; ne pas attribuer une exécution
sur le seul BuildID. Les volumes réalisés ne sont pas égaux dans ce scénario
piloté par le temps. Aucun effet causal global du patch n'est établi.

## Trois hypothèses et preuves discriminantes

| Hypothèse | Preuve nécessaire | Ce qui ne suffit pas |
|---|---|---|
| Surcoût du retrait PK | Temps cumulé et nombre des rollbackPKIndexInsert avant 180 s ; tailles réellement parcourues N, plages U ; part dans CPU/latence/transactions retenues ; signatures par transaction | Ratio du microbanc ou durée du DDL 4 |
| Pression déjà constituée avant DDL | Chronologie d'occupation et allocations refusées entre 120 et 180 s ; lignes validées/locales, transactions actives et ancienneté ; rapprochement avec débit réalisé | Pic RSS ou pic de pool seul |
| Autre faute d'annulation | Première exception d'origine et première exception de rollback séparées, phase exacte, undoStarted/halfDone/rollbackFailed, transaction concernée ; preuve de refus/récupération | Binder après ADD b refusé, ou risque statique des checkpoints internes |

## Phase documentaire immédiatement possible, sans run

1. Conserver les journaux intégraux base/isole, leurs XML, scripts de lancement et
   empreintes. Extraire pour chaque fichier séparément : DDL début/fin, première
   erreur textuelle, refus de BEGIN, messages d'annulation inachevée et erreur
   finale. Ne pas dater une erreur d'après sa position textuelle si les impressions
   des fils peuvent s'entrelacer ; les agrégats de fenêtres ne sont pas des événements.
2. Lire la définition du compteur de tampon et de ses fenêtres dans le banc,
   avant d'interpréter « avant » comme une mesure instantanée. Garder les pics
   comme pics. Les champs absents (temps PK, N/U, transaction, propriétaire mémoire)
   doivent rester « non enregistrés ».
3. Séparer trace.log, qui appartient au témoin COPY, de la preuve TwentyWriters.
   Rapprocher les exceptions des appels source sans prétendre obtenir une pile
   du scénario à partir de la pile du témoin réduit.
4. Livrer la chronologie attestée et la liste des inconnues avant décision d'un
   nouveau run. Si les journaux ne contiennent pas le discriminant, arrêter la
   conclusion à cette limite ; aucun relancement implicite.

## Capture minimale envisageable après autorisation distincte

Le scénario temporel inchangé sert à reproduire l'incident, pas à comparer un
coût à volume égal. Un seul passage instrumenté de 0511, sans répétition ni
extension automatique, pourrait trancher d'abord **où** naît la pression ; une
comparaison base/correctif ne serait proposée qu'après cette localisation.

L'instrumentation privée proposée, à soumettre sous forme de diff avant tout
build, n'altère ni le pool ni la charge ni les erreurs :

- Compteurs scalaires par transaction/phases : entrées et nanosecondes cumulées
  du retrait PK, nombre de plages et taille de l'index local, début/fin de rollback.
  Accumulation sans allocation ni chaîne pendant rollback ; formatage après.
  Définir si N compte slots visités ou entrées vivantes et capturer les deux
  si disponibles, sans nouveau parcours uniquement pour compter.
- Échantillonnage borné d'une seconde : occupation **actuelle** du pool, pic,
  lignes validées, transactions actives et âge de la plus ancienne ; préciser
  explicitement les métriques indisponibles au lieu d'ajouter une API métier.
- Enregistrement de la première erreur par phase dans un espace préalloué :
  timestamp monotone, transaction, phase, type d'undo, bornes de plage,
  undoStarted/halfDone, exception d'origine versus échec d'annulation.
  Pas de backtrace allouante sous pool plein ni journal par clé.
- Sortie limitée à 16 Mio, capture de première occurrence puis compte des
  suivantes ; dépassement signalé et arrêt contrôlé, jamais perte silencieuse.

Bornes **proposées**, non réservées : poste mesure exclusif après libération des
postes lourds concurrents ; 1200 s murales après acquisition, arrêt propre puis
terminaison du seul groupe privé avant 1210 s ; mémoire cgroup 8 Gio, swap nul,
stockage privé borné 2 Gio ; aucun GPU, service, PG ou magasin utilisateur. Un
processus enfant, aucune relance. Le budget de compilation est séparé et doit
être relu avec le diff ; aucun binaire existant ne prouve ces nouvelles captures.
Si ces limites ne permettent pas le scénario, on rapporte une troncature et on
ne relève pas les plafonds automatiquement.

## Lecture attendue du résultat éventuel

- Pression monte avec peu/pas de retraits PK : exclure ce retrait comme cause
  directe dominante de cette montée sur ce passage ; examiner allocations et
  volume avant de proposer une optimisation.
- Beaucoup de temps PK avec transactions retenues : contribution plausible,
  pas causalité établie par corrélation. Proposer ensuite une charge à volumes
  fixés et les mêmes bornes, sans fusion A inutile pour une seule plage Doc.
- Échec d'annulation après premiers effets : isoler son chemin et le rattacher
  au ticket approprié, avec récupération à démontrer. Aucun test destructif de
  checkpoint interne dans cette recette ; le diagnostic quarantaine est distinct.
- ADD b refusé puis Binder sans échec d'annulation enregistré : garder le refus
  de DDL comme première faute attestée, ne pas baptiser le Binder corruption.

Livrable attendu : sources/diff/empreintes, argv, limites effectives, journal brut,
chronologie, compteurs avec unités et champs absents, verdict par hypothèse,
code de sortie et preuve d'arrêt. Aucun verdict global vert à partir d'un seul
passage, aucune optimisation choisie avant la preuve discriminante.
