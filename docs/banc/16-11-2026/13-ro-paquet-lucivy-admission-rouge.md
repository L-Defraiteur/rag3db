# Relecture du paquet du témoin consommateur Lucivy

Référence documentaire exacte `a039e4bbce3f25bf9d81baa4a6dddc72a51392b0`, APP Lucivy `81b0e74f0254c8216864f90a460f8969842432ed`. Pilote SHA256 `99a8afcacfabd76928e596f956345dbe786641cbd96baf8c86dae32c69e30db1`, admission `7649a9b9d34ec3cdfc1333f33bc216f260fbda9a6d52f3c70fe979d7aabff5e2`, préflight `3557847be05f314650dcb2afb9f5ced24c57de84c36ce959cfd161f3e642b76e`. Même [ticket indexed-false](../../lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md). Revue limitée recette/ledger/deadline/provenance ; l’avis sémantique sur extraction et raccord appartient à Recherche.

**Avis : ordre préparatoire cohérent, deux réserves de fin regroupées avant GO.** Aucun Cargo, ELF lancé, scan proc, base ou service pendant cette relecture.

## Empreintes et provenance

Les cinq entrées SHA256SUMS du dossier derived-admission-source-preparation ont été recalculées depuis les blobs Git : toutes conformes. Les huit copies vérifiables dans Git correspondent aux pins du préflight : deux nouveaux pilotes, helpers core/rouge, Common/poste, source-ledger et transcription. L’ancien ELF106fa n’a pas été relu physiquement ici ; sa copie RO et son rehash sont attestés par le porteur, pas transformés en observation indépendante.

Les six fichiers du source-ledger correspondent aux blobs du dépôt Lucivy au commit81b0 exact. AST des pilotes seulement, aucun import ni test exécuté. Le précontrôle publié conserve onze UNKNOWN non attribués et aucun bloquant ; ce n’est pas un scan refait ici. Les métadonnées Qdev55/inode168475170/nonce2e5cac17050a7d5dcf4ea8a932309625, Tdev55/inode164536077/nonce3552 et Common historique3528029 sont des observations archivées.

Le contrôleur vérifie composants sans symlink, UID, identités Q/T, provenance créatrice0c795, marqueurT5a1671, source HEAD81b0 propre, copies hachées et ancien ELF original/copied106fa avant compilation. Ancien Common doit être TERMINE101/erreur nulle/restes[]/nettoyé. Classifier /proc partagé : owned/attribué bloque, opaque non attribué reste UNKNOWN. Parent cwdQ est imposé, enveloppe distante bash explicite et noclobber du journal.

## Échéance et ordre

Admission mesure start après acquisition poste, fixe start+600, consomme les précontrôles dans ce même budget, puis confineur reçoit floor(solde)-2. Driver exige échéance active Common<=borne admission, nonce identique et solde<=600 ; chaque phase est sous le même Common, pas un reset. L’extérieur est TERM610/KILL615 après poste, pas une promesse de615s communes. Memory32G/HIGHmax/swap0/quota<=4, jobs issus de cpu_jobs, Rustflags/wrappers interdits sont contrôlés. La commande compile trois cibles release offline no-run ; pas d’exécution des corps des tests natifs test_derived_in_ram/test_sfx_v3_ground_truth.

Après artifactJSON, le ledger APP+trois chemins/SHA est ouvert exclusivement, flush/fsync avant tout inventaire. Trois listes et trois listes ignored sont publiées avant le ledger complet d’inventaire, flush/fsync avant l’unique témoin mock. Neuf noms dans sa cible, témoin présent et aucun ignoré y sont requis. Les huit autres cas ne sont pas joués. Le seul exact attendu demande101/running1/0pass1fail0ignored, message discriminant et left1/right0. Autre code STOP avec first-unexpected-red ; assertion différente STOP par exception, pas de retry. Le résultat attendu reste full_core_qualified=false et verdict Common explicitement pending.

## Deux réserves regroupées

1. **Identité finale des nouveaux artefacts et du source.** Les trois SHA sont calculés après compile puis simplement réutilisés dans le résultat. Aucun rehash après inventaires/témoin n’est effectué ; aucune nouvelle attestation HEAD/propreté/copies n’est faite avant expected-red.json. Le scan final ne remplace pas ces comparaisons de contenu. Pour qualifier la continuité de provenance, réattester les trois ELF contre le ledger et le source/copies sous la même échéance avant publication finale. L’ancien ELF sous T peut légitimement être recompilé : conserver/vérifier sa copie historique, ne pas exiger qu’il reste physiquement ancien sous T.
2. **Échéance après ouverture du résultat final.** budget() est appelé avant publish(expected-red.json), mais publish ouvre/écrit/fsync sans contrôle après open. Une ouverture retardée au-delà de END peut donc laisser un fichier expected-red qui porte encore le statut attendu ; l’autorité Common reste un contrôle distinct et le champ pending interdit d’en déduire seul une fin qualifiée. Ajouter le contrôle après ouverture et avant dump, avec refus conservé si tardif ; aucun nouveau budget. Même réserve pour tout ledger présenté comme publié avant une phase : l’ordre séquentiel est bon, mais le résultat final doit expliciter sa ponctualité.

La fin complète reste à établir séparément : Common terminé/nettoyé sans restes, owned absents, conservation des scans UNKNOWN et des hashes, sous autorité valide. Le contrôleur rend101 et ne contient pas de composé externe qui qualifierait lui-même cette fin ; la recette l’exige après retour. Ce manque d’observation actuelle n’est pas un résultat de test et ne devient pas vert par lecture.

Pas d’autre réserve bloquante dans ce périmètre statique. Le témoin est une baseline mock volontairement non corrigée, pas une couverture réelle du corpus, pas le fix d’admission ni la qualification du core. Les anciens rouges, neuf HOLD, corpus7.2/ULP/PR42 restent distincts. Nouveau SHA/pins à relire si modification ; aucune allocation déduite de cette revue.

## Delta final44a7849a9c : les deux réserves sont levées

Référence exacte `44a7849a9c9a6d345c936b74522d647f5115c701`, source81b0 inchangée. Pilote final SHA256 `7833a6f4bc42c137c1766aba889f529b006a27ee72402d6227c22b9defdad0fc`, admission7649 inchangée. Revue du seul delta depuis a039 et des pins/preuves nouveaux, sans refaire la revue sémantique du source ni exécuter de témoin.

Après le mock, les trois nouveaux ELF physiques sont chacun réhachés et comparés au ledger initial ; le résultat reprend ces SHA finaux. HEAD81b0 et propreté sont recontrôlés, copies figées réhachées, préflight comparé au SHA pris à l’entrée. La boucle vérifie le budget ; le dernier publish le vérifie encore avant ouverture, après ouverture avant dump et après flush/fsync. Un dépassement interdit le retour101 attendu. La copie historique106fa reste dansQ et est réattestée sans imposer l’ancien contenu au chemin recompilable sousT. Les deux réserves7f6 sont donc traitées sans nouveau budget.

Huit entrées SHA256SUMS des dossiers final-guards et source-preparation-v2 recalculées depuis Git, sans écart. AST des deux pilotes seulement. Nouveau préflight SHA `0c90301ed5324f8a07d80641283e581ed3f427bc6355dcf1304afa14763f7325`, pilote7833 pin cohérent, Qderived-red-v2, nonced4f2d8cfd6d6705b2c17423ba66d91a9. Le relevé de douze UNKNOWN non attribués est archivé ; pas de scan nouveau ni relecture physique d’ELF par cette revue.

Les logs Python conservés montrent un seul rouge changed-copy (AssertionError not raised), failfast/cinq autres non joués avant correctif, puis six réussites/zéro échec après publication272c. Ce sont des tranches AST avec objets simulés, pas une observation du mock Rust, des fichiers PC réels ou d’un builder natif ; aucun rejeu ici. Le log vert dit0.001s, sans en déduire une durée native.

**Avis favorable à ce delta/pins v2 exacts, réserves levées ; pas GO600.** Le budget unique, ledger avant inventaires puis ledger avant témoin et résultat pendingCommon restent ceux déjà relus. Fin complète Common/owned et rouge Rust réel sont encore NON OBSERVÉS. Aucun Cargo, ELF exécuté, scan proc, base ou service lancé.
