# La file d'écriture ne suit pas la cellule de lecture

- **État** : ouvert, correctif isolé en vérification sur la branche 1a.
- **Famille** : produit
- **Gravité** : une recherche stricte d'une cellule peut drainer les écritures d'une autre.
- **Atteignable en service** : oui, recherche avec scope explicite différent du catalogue.
- **Touche rag3weaver** : oui, catalogue, bascule de cellule et consigne de recherche.
- **Pour** : arbre-principal ; orchestration pour la composition et la passe.
- **Signalé par** : témoin de lecture du lot 1a, demande de séparation orchestration.

## Preuve et portée

Sur le code 06b3ee4cb, test
`projection_lecture_alpha_rattrape_alpha_et_preserve_la_file_beta`, journal luciepc
`~/.cache/rag3weaver-build/cx-projection-lecture-cellule-rouge-v2.log` : une lecture
Strict d'alpha insère `beta-en-file.rs` (1 au lieu de 0). Les racines durables alpha
ont perdu leurs références pour éprouver le rattrapage ; le catalogue reste en bêta.
Le premier essai Immediate n'exigeait aucun rattrapage et n'est pas une preuve du
même défaut : journal sans suffixe v2, zéro résultat alpha, sans drain bêta.

La cause structurelle est commune : `pending` appartient au catalogue courant,
`dans_la_cellule` gare les index mais pas les écritures ; la source de recherche
applique sa consigne dans la cellule du catalogue, pas celle de l'appel.
Le défaut paraît antérieur au partage de contenu, mais la comparaison sur master
avec des entités ordinaires reste à jouer : aucune régression antérieure n'est affirmée.
Ce ticket couvre cette frontière commune ; le ticket 1a garde les versions et projections.

## Primitive et invariants du correctif candidat

Le catalogue possède la file active et `parked_pending: HashMap<Scope, PendingWork>`.
Chaque file possède les cinq familles existantes : créations, liens, mises à jour,
suppressions, dérivations. La bascule gare la file complète puis reprend celle de la
cellule demandée ; aucune mutation des enregistrements ni réestampillage implicite.
Le coût ajouté est une entrée par cellule ayant une file inactive non vide, sans copie.

- Les bascules temporaires reviennent à la cellule et à la file initiales après Ok ou Err.
- Une transaction ouverte interdit une bascule de cellule avant toute mutation.
- L3 extrait/restaure seulement le lot de la cellule active ; les files garées restent intactes.
- La dette publiée agrège les files, donc finir alpha n'efface pas la marque de bêta.
- Drop et must_reopen comptent aussi les files garées : aucune perte dissimulée.
- Les files restent en RAM, comme auparavant ; aucune nouvelle promesse de durabilité après arrêt.
- La recherche exécute la consigne et ouvre le FTS dans le Scope complet de l'appel.

## Restant et lien 1a

Vert du témoin, comparaison master avec entités ordinaires, rollback/retour d'erreur,
changement explicite de cellule avec travail accepté, puis régressions et passe commune.
Le ticket reste ouvert jusqu'aux preuves et au déplacement réglementaire vers closed.
Lien : [versions et occurrences 1a](2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md).

## Comparaison et premiers verts

La préexistence hors 1a est maintenant reproduite sur master **c04c62021d0d90a29ad0b1d23de22934f49e7526**,
worktree luciepc `cx-wt-file-cellule-baseline`, aucun changement runtime, seul le test ordinaire
`les_files_acceptees_restent_attachees_a_leur_cellule` ajouté à e2e_scope. Entité Product,
aucune dérivée ni projection : le drain bêta écrit A et B au lieu de B seul.
Journal `cx-file-cellule-baseline-rouge.log`, 0 réussi, 1 échec ; garde du lanceur 2 réussis.
Le test identique passe avec le candidat 1a : `cx-projection-cellules-regressions.log`,
e2e_scope 13 réussis et e2e_entites_derivees 19 réussis, code retour 0.

Le témoin de lecture stricte alpha passe également (`cx-projection-lecture-cellule-vert-v2.log`,
5 réussis, code retour 0) : deux résultats alpha reconstruits, fichier bêta encore absent,
cellule bêta restaurée, puis son drain écrit le fichier bêta. Le v1 candidat gardait la file
mais omettait l'index associé dans la fermeture de lecture : zéro résultat alpha malgré
partial=false ; ce second défaut 1a est corrigé par la dépendance de lecture search_index.
Les 18 combinaisons de défense cellule/signal/mode passent dans la suite des 19 dérivées.
Les preuves restent ciblées : tests de rollback et de bascule transactionnelle et passe globale
restent à compléter. Aucun nettoyage de worktree/target inachevé.

Les unités retour Err, reprise de file, refus de bascule en transaction et maintien de la
marque de dette passent dans `cx-projection-file-regressions-v2.log` (bibliothèque 2101/0/58).
Le test L3 multicellule passe seul après désarmement de la faute avant la reprise bêta
(`cx-projection-rollback-cellule-rouge-v3.log`, 1/0) ; le nom du journal ne vaut pas verdict.
Une assertion dédiée d'index physique reste à lire, puis relance des suites affectées.

## Garde FTS physique demandée en revue

Témoin exact `projection_rollback_alpha_preserve_la_file_beta` : la faute est désarmée
avant le drain bêta. Son assertion d'absence de warning d'index source échoue dans
`cx-projection-rollback-cellule-rouge-v4.log`, source 06b3ee4cb + candidat file/scan décrit
par les empreintes de `cx-projection-file-regressions-v2.log`. Message : Occurrence déclare
BM25 mais aucun index plein texte propre n'est ouvert. Ce rouge est distinct de la fixture
qui laissait armée la faute (`v3` du témoin seul était vert sans cette nouvelle assertion).

Correctif : rendre accessible à la crate le résolveur physique existant et l'employer
uniquement dans `open_fts_handles_for`. Le drain, la reprise et l'ouverture des index
entretiennent les stockages déclarés ; la résolution publique conserve l'association,
les masques et le refus de recherche directe des versions internes. Le lot code commun
reste dans la branche 1a, pas un nouveau lot parallèle. Verdict attendu dans la passe v3.
Une fermeture/réouverture multicellule sans réinscription du schéma est également préparée
pour vérifier la reprise de la dette alpha laissée persistée après le traitement bêta.

Provenance v3 figée : source exacte 965b835b8b58cb3937df26b99ed6a982e01d2f0d,
origin/cx-arbre-source-cellules-v3 (archive, pas nouveau lot). Les journaux portent HEAD10c
avec modifications ; ce snapshot les matérialise sans changement. Le checkpoint261ff54fc
ajoute les témoins de réouverture/ScopeNONE et le fix SQL, non joués par v3.
Binaire e2e SHA256 97e438e4dae5a42e6baaea3a2077ebb8fb10125231ea2212a9a4cc13bea8d120 ;
provenance complète et hash du journal v3 dans le ticket recherche de résidu lié ci-dessus.

Source288bc80ae : `cx-cellules-reouverture.log`, **21/0 dérivées et 3/0 Scope privé**, rc0,
sans e2e_search. Le témoin de fermeture/réouverture retrouve les deux racines alpha dues
après traitement bêta, sans réinscrire les schémas ; lectures alpha et bêta correctes,
cellule initiale restaurée. La garde FTS de reprise et le rollback multicellule passent.
Le témoin PG v3 franchit les UUID puis révèle l'UNWIND natif du tampon FTS post-commit ;
son correctif utilise la primitive dialectale select_node_ids déjà existante, suivi dans 1a.

## Question de revue finale : marque après rollback

Revue c7 du candidat6fb, PG/dialecte sans blocage à ce stade. Le témoin
projection_rollback_alpha_preserve_la_file_beta prouve les écritures conservées,
pas la marque publiée ni Strict d'un autre lecteur. rollback rétablit a priori
la marque DB antérieure, mais vide marque_posee/dette_publiee en mémoire. Une
nouvelle mise en file sur une autre entité pourrait alors reconstruire une dette
partielle depuis ce cache vide. Hypothèse bornée à vérifier, pas défaut affirmé :
lire la marque avant/après rollback et après création Note, Occurrence beta
restant en file ; contrôler le lecteur externe ensuite. Porteur arbre, même
ticket, aucune relance massive ni changement de runtime autorisé par hypothèse.

Frontière relue : has_pending, pending_work et drain_stats.pending décrivent la
file active que les appels consigne/undo/code_sync/backend batch peuvent drainer.
Les compteurs cumulés de drain et index_progress global ont une autre portée ;
agréger has_pending sans changer le drain ferait travailler la mauvaise cellule.
La préparation de clôture est archivée hors dépôt en attente des avis.
