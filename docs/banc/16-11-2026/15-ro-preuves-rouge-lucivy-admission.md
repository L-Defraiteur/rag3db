# Relecture des preuves du rouge réel Lucivy admission

Référence exacte `9a64654e7f8df0f10afcb98623081881745c9198`, APP81b0, pilote7833/admit7649/préflight0c903 et allocation C7-LUCIVY-ADMISSION-7833-16nov615. Même [ticket indexed-false](../../lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md). **Avis favorable à la preuve du seul rouge discriminant mock, pas correctif ni core vert.**

Relecture des blobs publiés uniquement : les35 fichiers de SHA256SUMS ont été recalculés sans écart. Aucun ELF ouvert/exécuté, aucun Cargo/test, scan proc, base ou service lancé. Les rehashes physiques et copies sur deux postes sont des observations du porteur archivées ; cette revue vérifie leur cohérence, sans les refaire.

## Cas réellement joué et artefacts

Le log natif contient running1, le nom exact missing_panel_is_rejected_before_any_builder, une seule assertion d’admission au fichier Rust attendu, left1/right0 et sommaire **0passé/1échec/0ignoré/8filtered**. La commande d’identité sélectionne bien ce même ELF avec --exact/--test-threads=1/--nocapture. Expected-red porte101, le verdict Common101 et SSH1 : aucune inversion du rouge en vert. Cela établit que le mock builder a été appelé malgré le panel absent ; pas une absence d’API ni un échec de compilation.

Le compile.jsonl contient exactement trois artifacts exécutables aux chemins du ledger, build-finished.success=true et durée Cargo32.20s. Il contient aussi51 lignes humaines nonJSON : sa lecture doit rester ligne par ligne, pas prétendre que tout le fichier est un document JSON strict.

Le ledger ELF est horodaté1197329.725521448, le ledger d’inventaire1197329.734893729. Les six listes brutes correspondent exactement aux listes et ignorés publiés :9/0,1/0,12/1. L’unique corps joué est le premier mock ; les deux autres ELF ne sont invoqués que pour inventaire. L’ordre durable ledgerELF→inventaires→ledgercomplet→témoin est imposé par le pilote7833 préalablement relu, cohérent avec les preuves ; les ticks start PID seuls ont une résolution insuffisante pour fabriquer un ordre sous-centiseconde supplémentaire.

Les trois paires chemin/SHA du ledger initial égalent strictement final_artifacts du pilote ET physical_final_artifacts du contrôle externe. SHA du témoin : `7c6f65e2e8427b53828a58cecd313d4a6c02cd7a3074f358d14ce5f2114ea9e8` ; derived natif compilé64758437584801255698598fea0db343a6528c22cfdfb5df851e79b2f5700ede ; ground-truth compilé777c911f0216529d01fb8957cd3bcfe66d80b86b7378f10e0e33f7d977e5036d. Le résultat atteste HEAD81b0/propreté/copies/préflight réattestés et reprend le SHA final. L’ancienne copie106fa et nouvelle7c6f ont leur reçu d’archive portable distinct ; ce reçu ne constitue pas un nouveau rehash physique par Banc.

## Fin et bornes

Entrée7833/noncev2d4f et préflight0c903 concordent. Ressources archivées :32G/HIGHmax/swap0, cpu.max400000/100000, jobs4 et cwdQv2. STARTadmission1197297.224958866 donne bound1197897.224958866 ; deadlineCommon1197894.323357855 est bien plus courte. Observation finale1197585.443130607 antérieure aux deux, sans reset. Le budget600 et l’enveloppe TERM610/KILL615 restent distincts ; aucune durée complète de600s ni borne d’attente de file n’est déduite.

Autorité publique PID588652/start119729720 TERMINE, SHACommon6439 ; verdict ROUGE101/erreur nulle/restes[]/nettoyétrue. Contrôle externe own_common_alive=false, assessment.admitted=true/blocked[], onze UNKNOWN non owned/non attribués, universal_quiescence_claimed=false. Scratch vide et marqueurT5a1671 consignés. Le résultat pilote reste honnêtement pendingCommon, complété par cette preuve externe ; ne pas réécrire son statut historique.

Aucun écart bloquant trouvé dans ces preuves. Le rouge attendu est acquis pour ce mock et ce couple exacts, aucun fix de couverture ou livraison. Huit autres mocks, corps natifs, corpus réels, neufHOLD/ignorés/full_core, anciens warnings,7.2/ULP/PR42 restent hors qualification. Ticket ouvert ; toute correction/qualification future attend une portée et une décision distinctes.
