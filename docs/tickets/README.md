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
  témoin de `test/transaction/concurrence/known_red.txt` et met à jour la ligne de l'index,
  dans le même commit.

## Index

| ticket | état | gravité | atteignable en service | touche rag3weaver |
|---|---|---|---|---|
| [Un point de reprise efface les relations des régions non touchées](2026-10-04-relations-effacees-au-point-de-reprise.md) | corrigé | perte | oui | oui (il supprime des relations et laisse passer des points de reprise) |
| [Le point de reprise plante, à jamais, après ALTER TABLE … DROP](2026-10-04-point-de-reprise-impossible-apres-drop-de-colonne.md) | ouvert | plantage | oui | non (rag3weaver ne supprime jamais de colonne) |
| [La lecture plante après le point de reprise d'une relation créée puis supprimée](2026-10-04-lecture-apres-reprise-d-une-relation-creee-puis-supprimee.md) | corrigé | plantage | oui | oui (il crée et supprime des relations) |
| [MERGE avec deux ON MATCH SET et une clé répétée plante](2026-10-04-merge-deux-on-match-set-et-cle-repetee.md) | corrigé | plantage | oui | non (sonde du 4 octobre) |
| [UNION plante quand une branche projette deux fois la même propriété](2026-10-04-union-propriete-projetee-deux-fois.md) | corrigé | plantage | oui | non vérifié |
| [Un chemin utilisé seulement dans un lambda fait planter](2026-10-04-chemin-utilise-seulement-dans-un-lambda.md) | corrigé | plantage | oui | non vérifié |
| [Comparer une relation à une variable de lambda fait planter](2026-10-04-relation-comparee-a-une-variable-de-lambda.md) | corrigé | plantage | oui | non vérifié |
| [WITH sans colonne utile filtré par un paramètre fait planter](2026-10-04-with-filtre-par-un-parametre.md) | corrigé | plantage | oui | non (vérifié par l'arbre principal) |
| [L'UUID nul empêche tout point de reprise](2026-10-04-uuid-nul-empeche-le-point-de-reprise.md) | corrigé | blocage | oui | non vérifié |
| [Un CHECKPOINT retient la validation d'un lecteur jusqu'à son délai](2026-10-04-checkpoint-retient-un-lecteur.md) | ouvert | blocage | oui | probable (lecteurs et points de reprise automatiques) |
| [Le point de reprise échoue pendant une indexation quand le tampon du moteur est petit](2026-10-04-point-de-reprise-echoue-quand-le-tampon-est-petit.md) | ouvert | blocage | oui | oui (première indexation, poussée des blobs du plein texte) |
| [RETURN DISTINCT … SKIP sans LIMIT rend zéro ligne](2026-10-04-distinct-skip-sans-limit.md) | corrigé | réponse fausse | oui | non (vérifié par l'arbre principal) |
| [Un terme de WHERE qui ne cite que des paramètres est ignoré](2026-10-04-where-sur-parametres-seuls-ignore.md) | corrigé | réponse fausse | oui | non (vérifié par l'arbre principal) |
| [L'étiquette d'une variable liée plus tôt sans étiquette est ignorée](2026-10-04-etiquette-d-une-variable-deja-liee-ignoree.md) | corrigé | réponse fausse | oui | non (vérifié par l'arbre principal) |
| [OPTIONAL MATCH qui répète un nœud lié double les lignes](2026-10-04-optional-match-qui-repete-un-noeud-lie.md) | corrigé | réponse fausse | oui | non (vérifié par l'arbre principal) |
| [MERGE d'une clé répétée rend une autre valeur que celle stockée](2026-10-04-merge-d-une-cle-repetee-rend-un-autre-noeud.md) | corrigé | réponse fausse | oui | oui, par `batch_link` |
| [Dans une transaction, un balayage de plusieurs tables de relations relit les relations d'une autre table](2026-10-04-balayage-de-plusieurs-tables-de-relations-en-transaction.md) | ouvert | réponse fausse | oui | non vérifié |
| [COPY : un champ CSV vide entre guillemets devient NULL](2026-10-04-csv-champ-vide-entre-guillemets-devient-null.md) | corrigé | réponse fausse | oui | non vérifié |
| [COPY refuse un CSV qui finit par un champ vide sans saut de ligne](2026-10-04-csv-champ-vide-final-sans-saut-de-ligne.md) | corrigé | réponse fausse | oui | non vérifié |
| [Un accent grave doublé dans un nom n'est pas réduit](2026-10-04-accent-grave-double-dans-un-nom.md) | ouvert | réponse fausse | oui | non vérifié |
| [QueryResult::toString() consomme le résultat](2026-10-04-tostring-consomme-le-resultat.md) | corrigé | réponse fausse | oui (par l'API C++) | non vérifié |
| [Deux défauts de chaînes annoncés par Ladybug, non reproduits chez nous](2026-10-04-recettes-de-l-amont-non-reproduites.md) | ouvert — non reproduit | perte (annoncée) / réponse fausse (annoncée) | incertain | non vérifié |
| [Durabilité sur faute d'entrée-sortie, coupure ou mort au mauvais instant](2026-10-04-durabilite-sur-faute-d-entree-sortie-ou-coupure.md) | ouvert | perte (pour la plupart) | non au banc sans crochet dans src/ | oui en cas d'incident (disque plein, coupure) |
| [Un conteneur garde les références de ses membres](2026-10-04-un-conteneur-garde-les-references-de-ses-membres.md) | ouvert | réponse fausse | oui | oui (usages, impact, liens) |
| [Un appel par chemin vers un type externe prend rendez-vous avec un homonyme du projet](2026-10-04-appel-par-chemin-vers-un-type-externe.md) | ouvert | réponse fausse | oui | oui (rendez-vous) |
| [Le type d'un champ déclaré dans un autre fichier ne se lit pas](2026-10-04-type-d-un-champ-declare-dans-un-autre-fichier.md) | ouvert | réponse fausse | oui | oui (rendez-vous) |
| [Les prototypes C au niveau du fichier sont du bruit de références](2026-10-04-prototypes-c-au-niveau-du-fichier.md) | ouvert | réponse fausse | oui | oui |
| [738 définitions C++ restent anonymes](2026-10-04-fonctions-anonymes-cpp-restantes.md) | ouvert | réponse fausse | oui | oui |
| [474 relations dépendent encore de la taille du paquet](2026-10-04-relations-qui-dependent-encore-du-paquet.md) | ouvert — cause localisée | réponse fausse | oui | oui |
| [Les journaux d'annulation du dataflow restent à vie dans /tmp, qui est de la mémoire vive](2026-10-04-journaux-d-annulation-laisses-dans-tmp.md) | ouvert | perte | oui | oui (son code) |
| [Les services de modèles n'ont aucune authentification](2026-10-04-services-de-modeles-sans-authentification.md) | ouvert (accepté aujourd'hui) | à faire avant toute exposition hors d'un tunnel | depuis l'un des deux postes | oui (démon d'embarquement et ses clients) |
| [La réouverture d'une base échoue par intermittence, sous charge seulement](2026-10-04-reouverture-intermittente-sous-charge.md) | ouvert — trois hypothèses écartées | plantage | non établi (vu une seule fois, sous batterie) | oui (test rag3weaver ; la lecture qui échoue est celle du moteur) |
| [La portée `person` d'une mémoire est écrite, et jamais rappelée](2026-10-04-portee-person-ecrite-jamais-rappelee.md) | ouvert — dette nommée, condition de sortie écrite | réponse fausse | oui, dès que le gabarit `memory` sert | oui (crochet de rappel, protocole d'outil) |
