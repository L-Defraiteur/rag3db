# Tickets

Un fichier par défaut connu, non corrigé ou en cours. Les défauts que nous découvrons
viennent ici, avec leur témoin. Les « bugs connus » du journal (§6) y migreront.

## Convention

- **Nom** : `<année>-<mois>-<jour>-<titre-court>.md`, le jour de l'ouverture.
- **En tête** :
  - l'état : ouvert, en cours, corrigé (avec son commit) ;
  - la gravité : perte, plantage, blocage, réponse fausse ;
  - atteignable en service ou non ;
  - touche rag3weaver ou non.
- **Sections** :
  - ce que c'est, en une phrase ;
  - la recette minimale (Cypher brut), ou pourquoi il n'y en a pas ;
  - le témoin (fichier, nom du test), ou son absence assumée et ce qu'il faudrait pour l'écrire ;
  - la cause, si elle est connue ;
  - le correctif de l'amont s'il existe (à lire, ne pas copier) ;
  - ce qu'il faut pour le fermer.
- **Fermeture** : le commit qui corrige passe le ticket à « corrigé `<commit>` », retire le
  témoin de `test/transaction/concurrence/known_red.txt`, **déplace le fichier dans `closed/`**
  (`git mv`) et met à jour la ligne de l'index (table des fermés), dans le même commit. Les
  anciens liens vers un ticket fermé se retrouvent par `git log --follow`.

## Index

### Ouverts (41)

| ticket | état | gravité | atteignable en service | touche rag3weaver |
|---|---|---|---|---|
| [L'index plein texte en fichiers n'est pas tenu par le point de reprise du moteur](2026-10-10-index-plein-texte-en-fichiers-hors-du-point-de-reprise.md) | ouvert — décision de Lucie : comme Neo4j pour l'instant, pas sur la stèle | réponse incomplète après un arrêt brutal, dite par l'état d'index | oui (défaut fichiers) | oui (état d'index, gardes, banc d'arrêt brutal) |
| [Le démon d'un test unitaire « n'a pas répondu » sous la charge du poste](2026-10-10-le-demon-de-test-n-a-pas-repondu-sous-charge.md) | ouvert | test instable | non | oui (tests du démon) |
| [Un groupe de copies bien plus grand que le degré perd des lignes dans l'index vectoriel](2026-10-10-un-groupe-de-copies-bien-plus-grand-que-le-degre.md) | ouvert, confort | réponse fausse (lignes injoignables, pas de plantage) | seulement avec des centaines de copies d'un vecteur | non (plus de vecteur nul écrit) |
| [Le journal d'une transaction reste sans borne pour les insertions ordinaires](2026-10-05-journal-d-une-transaction-sans-borne-hors-copy.md) | ouvert, suite de la borne du COPY | refus d'une très grosse transaction | oui | le mode à blobs, qui s'en garde |
| [Au journal, chaque cellule réécrit son type](2026-10-05-cout-par-cellule-au-journal.md) | ouvert, confort | volume et temps | toujours | en volume |
| [Un COPY refusé pour mémoire laisse la table faussée en mémoire, et le repli y écrivait](2026-10-05-copy-refuse-pour-memoire-table-faussee-en-memoire.md) | ouvert côté moteur ; filet rag3weaver `99e6f0ee0` | réponse fausse | oui | oui (repli REPLI_EN_MASSE) |
| [Des erreurs du moteur avalées par des replis, et l'écriture continue](2026-10-05-erreurs-du-moteur-avalees-par-les-replis.md) | ouvert (lecture) | réponse fausse possible | oui | oui |
| [Une erreur pendant la validation, après l'ajout des lignes aux groupes, laisse la transaction à moitié validée](2026-10-10-validation-a-moitie-appliquee-sur-echec-apres-l-ajout-des-lignes.md) | ouvert (relevé par le banc, 10 oct.) | réponse fausse, puis plantage | en principe oui ; le seul chemin vu est fermé par A3′ | pas de cas connu |
| [Sous les verrous, une seconde suppression ou mise à jour de la même relation, après attente, reçoit « Write-write conflict » et non l'erreur de sérialisation](2026-10-10-seconde-suppression-d-une-relation-apres-attente-sans-erreur-de-serialisation.md) | ouvert (A4′) | nom de l'erreur | non (mode multi-écrivains) | non |
| [Après un arrêt brutal du mélange C7, des relations à une extrémité manquante n'existent qu'après le rejeu](2026-10-10-c7-apres-arret-brutal-relations-pendantes-nees-au-rejeu.md) | ouvert (A4′ faite, chemin non trouvé) | réponse fausse après reprise | non (mode multi-écrivains) | non |
| [Le point de reprise échoue pendant une indexation quand le tampon du moteur est petit](2026-10-04-point-de-reprise-echoue-quand-le-tampon-est-petit.md) | ouvert | blocage | oui | oui (première indexation, poussée des blobs du plein texte) |
| [analyze() avec une racine relative rend zéro scope, en silence](2026-10-04-analyze-racine-relative-rend-zero-scope.md) | ouvert | réponse fausse | oui | oui (son API d'analyse) |
| [Un NULL en tête d'une liste de paramètres type sa colonne en STRING](2026-10-04-null-en-tete-d-une-liste-de-parametres-type-string.md) | ouvert (contourné dans rag3weaver) | perte | oui | oui, par `LinkRecordNode` |
| [Un accent grave doublé dans un nom n'est pas réduit](2026-10-04-accent-grave-double-dans-un-nom.md) | ouvert, confort (hors condition 1) | réponse fausse | oui | non |
| [Deux défauts de chaînes annoncés par Ladybug, non reproduits chez nous](2026-10-04-recettes-de-l-amont-non-reproduites.md) | ouvert — non reproduit | perte (annoncée) / réponse fausse (annoncée) | incertain | non vérifié |
| [Durabilité sur faute d'entrée-sortie, coupure ou mort au mauvais instant](2026-10-04-durabilite-sur-faute-d-entree-sortie-ou-coupure.md) | ouvert | perte (pour la plupart) | non au banc sans crochet dans src/ | oui en cas d'incident (disque plein, coupure) |
| [Un appel par chemin vers un type externe prend rendez-vous avec un homonyme du projet](2026-10-04-appel-par-chemin-vers-un-type-externe.md) | ouvert | réponse fausse | oui | oui (rendez-vous) |
| [Le type d'un champ déclaré dans un autre fichier ne se lit pas](2026-10-04-type-d-un-champ-declare-dans-un-autre-fichier.md) | ouvert | réponse fausse | oui | oui (rendez-vous) |
| [Les prototypes C au niveau du fichier sont du bruit de références](2026-10-04-prototypes-c-au-niveau-du-fichier.md) | ouvert | réponse fausse | oui | oui |
| [738 définitions C++ restent anonymes](2026-10-04-fonctions-anonymes-cpp-restantes.md) | ouvert | réponse fausse | oui | oui |
| [Une synchronisation reprise après un arrêt brutal exige takeover](2026-10-04-reprise-apres-arret-exige-takeover.md) | ouvert (accepté aujourd'hui) | blocage | oui | oui (sessions de `sync_source`) |
| [Les journaux d'annulation du dataflow restent à vie dans /tmp, qui est de la mémoire vive](2026-10-04-journaux-d-annulation-laisses-dans-tmp.md) | ouvert | perte | oui | oui (son code) |
| [Les services de modèles n'ont aucune authentification](2026-10-04-services-de-modeles-sans-authentification.md) | ouvert (accepté aujourd'hui) | à faire avant toute exposition hors d'un tunnel | depuis l'un des deux postes | oui (démon d'embarquement et ses clients) |
| [La portée `person` d'une mémoire est écrite, et jamais rappelée](2026-10-04-portee-person-ecrite-jamais-rappelee.md) | ouvert — dette nommée, condition de sortie écrite | réponse fausse | oui, dès que le gabarit `memory` sert | oui (crochet de rappel, protocole d'outil) |
| [Une arête ne dit pas comment elle a été résolue](2026-10-04-une-arete-ne-dit-pas-comment-elle-a-ete-resolue.md) | en cours — filtre dans Liens et impact (master) ; réexportations à faire | réponse fausse | oui | oui (rendez-vous, usages, impact, liens) |
| [Trois rendus d'arbre à fondre](2026-10-04-trois-rendus-d-arbre-a-fondre.md) | ouvert — deux sur trois fondus | dette | non | oui (rendu) |
| [Un COPY de relations annulé laisse gonflée l'estimation du nombre de relations](2026-10-05-copy-de-relations-annule-gonfle-l-estimation.md) | ouvert, confort | estimation fausse | oui | non |
| [Le point de reprise réécrit en entier une table de blobs dès qu'une de ses lignes change](2026-10-04-point-de-reprise-reecrit-toute-une-table-de-blobs.md) | ouvert (optimisation, hors stèle) | lenteur | oui | oui (`_index_blobs`, contourné) |
| [Après une mise à jour massive de vecteurs, des lignes restent injoignables dans l'index](2026-10-04-mise-a-jour-massive-de-vecteurs-lignes-injoignables.md) | ouvert | réponse fausse | oui | peu |
| [Les « nom seul » et les segments de module](2026-10-04-nom-seul-et-segments-de-module.md) | ouvert — pas pour maintenant | réponse fausse | oui | oui (rendez-vous, usages, Liens) |
| [Un chemin vers une réexportation reste « par le nom »](2026-10-04-un-chemin-vers-une-reexportation-reste-par-le-nom.md) | ouvert — décidé, en attente de code.rs | réponse fausse | oui | oui (rendez-vous, Liens, impact) |
| [Des receveurs restent sans type](2026-10-04-des-receveurs-restent-sans-type.md) | ouvert — pas pour maintenant | réponse fausse | oui | oui (usages, Liens, impact) |
| [Un COPY de relations vers des nœuds locaux garde leurs décalages provisoires](2026-10-05-copy-de-relations-vers-des-noeuds-locaux-sans-remappage.md) | ouvert (témoin attendu d'A3′) | réponse fausse | non (multi-écrivains seulement) | non aujourd'hui |
| [L'index RTree de l'extension geo reçoit des décalages provisoires](2026-10-05-rtree-recoit-des-decalages-provisoires.md) | ouvert (confort ; lecture seule, non exécuté) | réponse fausse (annoncée) | incertain | non |
| [Au journal, un vecteur coûte trois fois sa taille](2026-10-04-un-flottant-coute-douze-octets-au-journal.md) | ouvert (optimisation, hors stèle) | lenteur | oui | oui (l'embarquement en fond) |
| [Sur PostgreSQL, la transaction d'un paquet part sur deux sessions du pool](2026-10-10-postgresql-transaction-de-paquet-sur-deux-sessions.md) | ouvert — corrigé dans le texte, témoin vivant à jouer | perte | non aujourd'hui ; oui si la transaction par paquet devient le défaut | oui |
| [Sur PostgreSQL, le rebâti du plein texte en fond envoie du Cypher](2026-10-10-postgresql-rebati-du-plein-texte-en-cypher.md) | ouvert | blocage | avec un montage PostgreSQL et lucivy | oui |
| [Les échecs de chargement d'extension au rejeu, que personne ne lit](2026-10-10-echecs-de-chargement-au-rejeu-que-personne-ne-lit.md) | ouvert | coût silencieux (index entier à rebâtir), réponse fausse possible | oui, dès qu'un fichier d'extension noté a bougé | oui — il paie le rebâti et il pourrait le dire |
| [Sous IGNORE_ERRORS, laquelle de deux lignes de même clé un COPY garde dépend des fils](2026-10-10-copy-garde-un-doublon-selon-les-fils.md) | ouvert, confort | réponse non déterministe | oui | non |
| [CALL analyze n'est pas transactionnel](2026-10-10-analyze-non-transactionnel.md) | ouvert, accepté tel quel | estimation périmée | oui | non |
| [Pas d'analyze automatique : les distincts restent gonflés jusqu'à un CALL analyze](2026-10-10-analyze-au-seuil.md) | ouvert, à ouvrir sur un mauvais plan | estimation fausse | oui | au plus des plans moins bons |
| [Les comptes de valeurs distinctes se trompent d'environ 13 % : 64 registres](2026-10-10-hyperloglog-a-64-registres.md) | ouvert, confort | estimation imprécise | toujours | non |
| [Une base s'ouvre sans dire qu'une extension dont une table dépend n'a pas pu être chargée](2026-10-10-ouverture-sans-une-extension-dont-une-table-depend.md) | ouvert — proposition (refuser l'ouverture) ; voir aussi « les échecs de chargement d'extension au rejeu » | index à rebâtir, sans que l'ouverture le dise | oui | oui (index vectoriel) |
| [Sur PostgreSQL, défaire un lien ne trouve rien](2026-10-11-postgresql-defaire-un-lien-ne-trouve-rien.md) | ouvert — corrigé dans le texte, témoin vivant à jouer | réponse fausse | avec un montage PostgreSQL | oui (annulation des liens) |

### Fermés (52), dans `closed/`

| ticket | état | gravité | atteignable en service | touche rag3weaver |
|---|---|---|---|---|
| [« unknown entity: File » ne dit pas ce qui manque — un manifeste sans `workspace.index`](closed/2026-10-10-unknown-entity-file-ne-dit-pas-d-indexer.md) | corrigé `9dc3f083a` | interprétation : un modèle y lit un défaut d'outil au lieu d'une déclaration absente | seulement avec un manifeste qui ne déclare pas `workspace.index: "code"` | oui — toute la surface de code |
| [Après un COPY annulé ou refusé, le point de reprise suivant ne finit pas](closed/2026-10-05-point-de-reprise-sans-fin-apres-un-copy-annule.md) | corrigé | blocage, panne de mémoire | oui | oui (paquet défait, COPY refusé) |
| [Une transaction qui mêle des écritures et un COPY revient à moitié après une mort pendant le point de reprise de sa validation](closed/2026-10-05-transaction-a-moitie-apres-une-mort-pendant-son-point-de-reprise.md) | corrigé | perte (et base inouvrable sous IGNORE_ERRORS) | oui (chemin par défaut) | oui, par la transaction par paquet (éteinte par défaut) |
| [Retirer une colonne déclarée avant la clé primaire casse l'insertion](closed/2026-10-05-drop-d-une-colonne-declaree-avant-la-cle-primaire.md) | corrigé (fenêtres A et B) | réponse fausse, perte, plantage | oui | non (rag3weaver ne retire jamais de colonne) |
| [L'annulation d'un COPY dans une table indexée rend des lignes validées introuvables par leur vecteur](closed/2026-10-05-annulation-d-un-copy-abime-l-index-vectoriel.md) | corrigé avec la ligne lointaine (`2da041058`, `c841d507c`) ; ce n'était pas l'annulation | réponse fausse | oui | oui (paquet défait) |
| [Les pages d'un COPY journalisé sont perdues quand la base rouvre par le rejeu](closed/2026-10-05-pages-d-un-copy-journalise-perdues-au-rejeu.md) | corrigé (0aed3c4b5) | espace perdu, qui s'accumule | oui : une mort pendant un COPY d'un groupe plein | oui (chargement final des relations) |
| [Une clé sort de l'index de clé primaire après deux COPY de 50 000 lignes](closed/2026-10-05-cle-perdue-par-l-index-de-cle-primaire.md) | corrigé `eb2d78e46` ; témoin vert `e31575881` ; une base touchée se réindexe | perte | oui (chemin par défaut) | oui (toute pose de liens par clé) |
| [Une erreur du moteur dans la branche vecteur fait tomber toute la recherche hybride](closed/2026-10-05-erreur-de-la-branche-vecteur-fait-tomber-la-recherche-hybride.md) | corrigé `872d60e99` — décision de Lucie (repli par branche ?) | blocage | oui | oui (toutes les entrées de recherche) |
| [L'annulation d'un COPY efface les lignes qu'un autre écrivain valide ensuite](closed/2026-10-05-annulation-d-un-copy-efface-les-lignes-d-un-autre-ecrivain.md) | corrigé par A3′ (`verrous-a3`, le verrou d'index du COPY) | perte | non (mode multi-écrivains) | non |
| [Les étiquettes d'un carnet peuvent avoir été échangées, sans moyen sûr de le voir](closed/2026-10-05-etiquettes-du-carnet-peut-etre-echangees.md) | fermé (aucun carnet en usage réel ; 25b3b45dc) | perte | oui, avant 25b3b45dc | oui (gabarit carnet) |
| [Une relation porte deux valeurs d'une même propriété selon le sens où on la lit](closed/2026-10-04-proprietes-de-relation-differentes-selon-le-sens.md) | corrigé `25b3b45dc` (même défaut que « un sens ou l'autre ») | réponse fausse | oui | oui (CONSUMES, CONSUMED_BY) |
| [Un point de reprise efface les relations des régions non touchées](closed/2026-10-04-relations-effacees-au-point-de-reprise.md) | corrigé | perte | oui | oui (il supprime des relations et laisse passer des points de reprise) |
| [Le point de reprise plante, à jamais, après ALTER TABLE … DROP](closed/2026-10-04-point-de-reprise-impossible-apres-drop-de-colonne.md) | corrigé | plantage | oui | non (rag3weaver ne supprime jamais de colonne) |
| [La lecture plante après le point de reprise d'une relation créée puis supprimée](closed/2026-10-04-lecture-apres-reprise-d-une-relation-creee-puis-supprimee.md) | corrigé | plantage | oui | oui (il crée et supprime des relations) |
| [MERGE avec deux ON MATCH SET et une clé répétée plante](closed/2026-10-04-merge-deux-on-match-set-et-cle-repetee.md) | corrigé | plantage | oui | non (sonde du 4 octobre) |
| [UNION plante quand une branche projette deux fois la même propriété](closed/2026-10-04-union-propriete-projetee-deux-fois.md) | corrigé | plantage | oui | non vérifié |
| [Un chemin utilisé seulement dans un lambda fait planter](closed/2026-10-04-chemin-utilise-seulement-dans-un-lambda.md) | corrigé | plantage | oui | non vérifié |
| [Comparer une relation à une variable de lambda fait planter](closed/2026-10-04-relation-comparee-a-une-variable-de-lambda.md) | corrigé | plantage | oui | non vérifié |
| [WITH sans colonne utile filtré par un paramètre fait planter](closed/2026-10-04-with-filtre-par-un-parametre.md) | corrigé | plantage | oui | non (vérifié par l'arbre principal) |
| [L'UUID nul empêche tout point de reprise](closed/2026-10-04-uuid-nul-empeche-le-point-de-reprise.md) | corrigé | blocage | oui | non vérifié |
| [Un CHECKPOINT retient la validation d'un lecteur jusqu'à son délai](closed/2026-10-04-checkpoint-retient-un-lecteur.md) | corrigé | blocage | oui | probable (lecteurs et points de reprise automatiques) |
| [RETURN DISTINCT … SKIP sans LIMIT rend zéro ligne](closed/2026-10-04-distinct-skip-sans-limit.md) | corrigé | réponse fausse | oui | non (vérifié par l'arbre principal) |
| [Un refus d'ouverture en lecture sur quelques centaines n'est pas le refus nommé](closed/2026-10-05-un-refus-d-ouverture-sur-quelques-centaines-n-est-pas-le-nomme.md) | corrigé (doublon de « lecteur affamé ») | réponse fausse | oui | oui (Rag3dbConnection::read_only) |
| [Un terme de WHERE qui ne cite que des paramètres est ignoré](closed/2026-10-04-where-sur-parametres-seuls-ignore.md) | corrigé | réponse fausse | oui | non (vérifié par l'arbre principal) |
| [L'étiquette d'une variable liée plus tôt sans étiquette est ignorée](closed/2026-10-04-etiquette-d-une-variable-deja-liee-ignoree.md) | corrigé | réponse fausse | oui | non (vérifié par l'arbre principal) |
| [OPTIONAL MATCH qui répète un nœud lié double les lignes](closed/2026-10-04-optional-match-qui-repete-un-noeud-lie.md) | corrigé | réponse fausse | oui | non (vérifié par l'arbre principal) |
| [MERGE d'une clé répétée rend une autre valeur que celle stockée](closed/2026-10-04-merge-d-une-cle-repetee-rend-un-autre-noeud.md) | corrigé | réponse fausse | oui | oui, par `batch_link` |
| [Dans une transaction, un balayage de plusieurs tables de relations relit les relations d'une autre table](closed/2026-10-04-balayage-de-plusieurs-tables-de-relations-en-transaction.md) | corrigé | réponse fausse | oui | non vérifié |
| [Deux COPY annulés dans une transaction laissent les clés du premier dans l'index](closed/2026-10-04-cles-fantomes-apres-deux-copy-annules.md) | corrigé | blocage | non par défaut (oui avec la transaction par paquet) | oui (transaction par paquet) |
| [COPY : un champ CSV vide entre guillemets devient NULL](closed/2026-10-04-csv-champ-vide-entre-guillemets-devient-null.md) | corrigé | réponse fausse | oui | non vérifié |
| [COPY refuse un CSV qui finit par un champ vide sans saut de ligne](closed/2026-10-04-csv-champ-vide-final-sans-saut-de-ligne.md) | corrigé | réponse fausse | oui | non vérifié |
| [QueryResult::toString() consomme le résultat](closed/2026-10-04-tostring-consomme-le-resultat.md) | corrigé | réponse fausse | oui (par l'API C++) | non vérifié |
| [Un conteneur garde les références de ses membres](closed/2026-10-04-un-conteneur-garde-les-references-de-ses-membres.md) | corrigé dans codeparsers 766e4bd — en attente du pointeur | réponse fausse | oui | oui (usages, impact, liens) |
| [474 relations dépendent encore de la taille du paquet](closed/2026-10-04-relations-qui-dependent-encore-du-paquet.md) | corrigé (4fe5a3bfc) | réponse fausse | oui | oui |
| [La réouverture d'une base échoue par intermittence, sous charge seulement](closed/2026-10-04-reouverture-intermittente-sous-charge.md) | fermé sous surveillance (double ouverture dans un processus, 57c8389b4 et ad220d0eb) | plantage | non établi (vu une seule fois, sous batterie) | oui (test rag3weaver ; la lecture qui échoue est celle du moteur) |
| [Une corruption de mémoire tue `e2e_code`, une passe sur trente à soixante](closed/2026-10-04-memoire-corrompue-dans-e2e-code.md) | corrigé (moteur) | plantage | inconnu | oui (sa suite `e2e_code`) |
| [Un `COPY` rend une erreur alors qu'il est validé, quand une autre transaction est ouverte](closed/2026-10-04-copy-rend-une-erreur-alors-qu-il-est-valide.md) | corrigé | réponse fausse | oui | à vérifier |
| [L'ordre de synchronisation au point de reprise laisse peut-être des pages non durables](closed/2026-10-04-ordre-de-synchronisation-au-point-de-reprise.md) | corrigé (sans témoin) | perte | sur coupure seulement | oui (ses données passent par les points de reprise) |
| [Une variable locale Rust passe pour l'usage d'une fonction homonyme](closed/2026-10-04-variables-locales-rust-prises-pour-des-usages.md) | corrigé (codeparsers 75077f7 + 46cc492, pointeur 4fe5a3bfc) | réponse fausse | oui | oui (usages, impact, liens) |
| [Un COPY refusé laisse la cardinalité de la table gonflée](closed/2026-10-04-copy-refuse-gonfle-la-cardinalite.md) | corrigé | estimation fausse | oui | non |
| [Le point de reprise écrit hors de son bloc après un ajout annulé](closed/2026-10-04-point-de-reprise-ecrit-hors-bloc-apres-un-ajout-annule.md) | corrigé | plantage | oui | oui si un de ses `COPY` est refusé ou annulé |
| [Après un COPY refusé pour clé en double, la ligne d'origine disparaît de l'index](closed/2026-10-04-cle-d-origine-perdue-apres-un-copy-refuse.md) | corrigé | réponse fausse | oui | par `RAG3WEAVER_COPY_NAISSANCES=1` seulement |
| [Le qualificatif d'un chemin se perd dans une macro](closed/2026-10-04-le-qualificatif-d-un-chemin-se-perd-dans-une-macro.md) | corrigé dans codeparsers 5778d92 — en attente du pointeur | réponse fausse | oui | oui (marque de résolution) |
| [Une ligne très loin des autres est injoignable dans un index bâti d'un coup](closed/2026-10-04-ligne-lointaine-injoignable-index-bati-d-un-coup.md) | corrigé `2da041058`, `c841d507c` (aussi par des COPY successifs ; vrai corpus : 0 introuvable) | réponse fausse | oui | peu probable |
| [Dans une transaction, après des insertions puis un COPY dans la même table, la clé d'une ligne mène à une autre ligne](closed/2026-10-04-copy-apres-des-insertions-dans-la-meme-transaction.md) | corrigé | réponse fausse, écriture sur la mauvaise ligne | oui | contourné (une table écrite par MERGE reste en MERGE jusqu'à la validation) |
| [Après un COPY annulé sur une table à index vectoriel, la recherche échoue, puis le COPY suivant n'est pas indexé](closed/2026-10-05-copy-annule-sur-une-table-a-index-vectoriel.md) | corrigé (une base abîmée avant refuse par son nom et se rebâtit) | réponse fausse | oui | oui (chemin d'échec d'un paquet) |
| [Une propriété de chaîne d'une relation se lit fausse dans un sens ou l'autre après un point de reprise](closed/2026-10-04-propriete-de-chaine-faussee-dans-le-sens-direct.md) | corrigé (les bases déjà écrites se réindexent) | réponse fausse | oui | oui (`resolution`, `usage`, `kind` des arêtes) |
| [Après un COPY annulé, refusé ou à court de mémoire, le point de reprise de l'index de clé primaire ne finit plus](closed/2026-10-05-index-de-cle-primaire-apres-un-copy-a-court-de-memoire.md) | corrigé | blocage, danger pour l'hôte | oui | oui si un COPY manque de mémoire |
| [Sous IGNORE_ERRORS, un doublon de clé fait supprimer une ligne innocente](closed/2026-10-10-ignore-errors-supprime-une-ligne-innocente.md) | corrigé `de8fc8c0f` | perte, réponse fausse | oui (un COPY sous IGNORE_ERRORS, index de clé sur disque) | non (il n'écrit jamais IGNORE_ERRORS) |
| [Le tampon de 256 Mio est plein à la première écriture d'un backend](closed/2026-10-10-tampon-de-256-mio-plein-a-la-premiere-ecriture.md) | fermé — non reproduit sur master, sous 256 Mio et 32 fils, sur les deux postes | blocage (annoncé) | non reproduit | oui (backend) |
| [Une extension chargée au rejeu n'est pas vérifiée contre le bâti du moteur](closed/2026-10-05-extension-chargee-au-rejeu-sans-controle-de-bati.md) | corrigé `458ff7157` — identifiant de bâti, refus nommé à LOAD EXTENSION et au rejeu | blocage à l'ouverture au lieu d'un refus nommé | si moteur et extension ne sont pas du même bâti | à l'atelier |
| [Un lecteur en lecture seule est refusé tant que les points de reprise d'un autre processus se suivent](closed/2026-10-04-lecteur-affame-par-les-points-de-reprise.md) | corrigé `3498ece5d` — le verrou des lecteurs : l'ouverture attend le point de reprise (5 s, refus nommé au-delà), le point de reprise attend les ouvertures (1 s, puis il passe) | blocage | oui | oui (`read_only`, e2e_prise_atomique ; sa reprise devient un filet) |
