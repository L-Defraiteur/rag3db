# Relecture spécialisée de la paire historique Rust

Référence préparée `1e5cb858b24776ff252785b1a89d010b67ab046965778f5dca924ca9a9b35cb5`, profil `81969f85c1cc6384ef67283fb6486cff22bbbcc8c5a795026339b7dbb13c4192`. Référence exécutable `fec35198b64b7e9da0496de6b01a7232fa7a8390`/parserbd133850 ; candidat `2b3d66f3d6dc6f567f6d0cf9b10cc8760134e30a`/parser34c74e41. Même [ticket du chemin standard homonyme](../../codeparsers/tickets/2026-11-16-un-chemin-de-la-bibliotheque-standard-se-relie-a-un-homonyme.md), plus ticket des mots vides Rust. Revue Git/fichiers seulement, zéro Cargo/test/sonde/scan proc/service/base.

**Avis total groupé : couple et méthode de comparaison cohérents ; trois réserves de protocole avant GO.** Les deux phases1800 restent proposées, pas allouées.

## Sources et corpus vérifiés

Diffs Git : référence depuis productiond395 et candidat depuis118 n’ajoutent hors docs que le même fichier sonde. Celui-ci est identique octet pour octet, SHA256 `f5de4f03dec0dc2290f103ca98d65f675e93ada801b79d38847f8ba0244a879e`. Pilote `ffbc477b01bf3159ed487b34eaa410e1112000fcaaef41294b944c263dd74a3b`, comparateur `46cad787a8700733b63109464478833c2f63b8164ab765bb517e51319ce03a69`, syntaxe AST seulement. Le pilote recontrôle HEAD/propreté, sous-module exact, absence de dérive produit, scripts/SDK et sonde égale aux pins de chaque phase.

Lecture indépendante sur PC des seuls fichiers de préparation/corpus : prepared/profil/manifeste ont les SHA annoncés. Les1840 fichiers physiques (290rw/1550engine) ont chacun taille et SHA conformes au manifeste `0a6c9af21e3666ff95427ecb44ea253d10907cd5619f96b0a96e91e6c1297185`, aucun fichier sélectionné n’est un lien. Le contrôleur refuse aussi les chemins supplémentaires, liens traversés et UTF8 invalide avant/après phase. Aucun corpus ou parseur n’a été exécuté par cette revue.

Cohortes recalculées :18 couples historiques sur14 sites ;117 couples var dont107 annotés par présence textuelle de std::env::var sur90 sites et10 autres. La présence textuelle sur une ligne n’est pas un oracle AST ni une justification pour supprimer un appel nu voisin.

## Entrée et comparaison

Même sonde via Cargo et SDK59 réel, pool1G, HashEmbedder64 CPU, default_scope_chunking et projet distinct par racine. Le profil strict exige découverte/admission/analyse/ingestion/File stockés égales à l’effectif attendu, chemins admis exacts, aucun skipped/failure et aucun scope orphelin. Diagnostics admission/analyse sont écrits avant assertions. CALL BUILD_ID() est réellement dans la sonde puis comparé au SDK attendu ; cette lecture statique ne prouve pas encore sa valeur à l’exécution.

Les JSONL portent key et UUID de chaque extrémité, chemins/types/noms/positions lignes et octets, ligne de relation et marque. Le contrôleur vérifie bornes/identifiants non vides et recoupe les compteurs de résolution. Les données ne permettent pas de conclure à zéro AST invalide : cette API ne l’expose pas. relations_dropped est consigné, sans que son existence soit assimilée à une couverture relationnelle complète.

Le comparateur travaille par racine, en Counter d’occurrences détaillées plutôt que six colonnes TSV ; UUID natifs comparés séparément et dérives conservées. Présences historiques à marque identique ou autre marque ne sont pas déclarées vraies automatiquement. Les20 retraits proposés sont sélectionnés par graine publiée, jusqu’à10/racine puis complément, anciens cas exclus ; sous-effectif conservé si nécessaire. Le jugement manuel reste pending, global_pass=false et REGLES7.

## Réserves groupées avant modification

1. **Identité du nouvel ELF avant le test.** La commande actuelle est directement cargo_garde test avec --ignored/--nocapture, compilation et exécution dans le même appel. Aucun no-run/artifactJSON/inventaire/ledger/copie du nouvel ELF avant son invocation n’existe dans pilot.py. Les trois ELF118 sont des témoins historiques archivés, pas le binaire de cette nouvelle paire. Le protocole demandé par C7 nécessite de séparer compilation et appel de l’ELF qualifié, préserver son identité/ledger avant le test puis le rehash final ; mêmes sonde/features/SDK et aucune phase supplémentaire implicite.
2. **Borne composée et fin.** Common1800 est neuf via le vrai confineur qui prend lui-même poste si nécessaire. Mais après son retour, main refait check_outputs/check_corpus et écrit classification.json hors de cette portée, sans watchdog extérieur ni contrôle END/post-open. La comparaison ultérieure est elle aussi distincte. La promesse1800/parphase incluant tous les contrôles et maximum3600 ne couvre donc pas ce code final actuel. Il faut borner explicitement la composition, finale/hashes/quiet/publication inclus, sans réinitialiser l’autorité ni prétendre que le budget de comparaison est déjà donné.
3. **Fresh attribution et fin du réemploi target.** Le helper cache_quiet est appelé avant la sonde ; aucun scan frais final ni vérification actuelle des identités owned/bus après Common n’est effectué avant classification. completed_scope vérifie le verdict enregistré et espace privé absent, pas un nouveau constat owned. De plus le scanner hérité ne lit pas argv/cgroup comme indices d’attribution et exempte un FD de poste sur le seul suffixe+ancêtre/UID/hash, sans le rôle/argv/start figé d’un contrôleur exact. Préserver les UNKNOWN, compléter la frontière d’attribution et l’attestation finale demandées ; les scopes historiques118 et leur conservation ne remplacent pas ces nouveaux contrôles.

L’adoption du seul target est bien prévue par cargo_garde avec REPRENDRE1, propriétaire fix→baseline→candidate, et le candidat exige la baseline VERT/nettoyée. La proposition n’autorise ni purge, target neuf, modèle ou PostgreSQL. Aucune de ces réserves ne constitue un résultat d’exécution ou un défaut produit établi par la sonde.

Pas d’autre réserve bloquante repérée dans cette revue bornée. Corriger/publiciser ensemble les réserves puis relire le nouveau couple/pins ; aucun GO déduit. Les unités118 restent acquises et distinctes ; passe complète/master entrant/PG/modèles et éventuel REGLES8 ont leur propre décision après jugement de la paire.
