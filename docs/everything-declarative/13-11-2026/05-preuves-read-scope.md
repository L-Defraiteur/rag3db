# Qualification isolée read_scope

Source exacte 461e8b868f40907bb0b2f731ff2fcaebc78923d2, branche
cx-ed-read-scope-qualification-r2 ; GO R4 exact C7. Produit préparé674f,
rebasé a919 sur masterb0 sans changement, puis une ligne TEST models.embed
ajoutée après rouge de fixture. Propagation des deux Result inchangée.
[Ticket ouvert](../tickets/2026-11-13-read-scope-masque-les-erreurs-de-lecture-des-worktrees.md).
[Recette et historique](04-recette-read-scope.md).

## Résultat

- Quatre nouveaux ciblés et nominal historique : cinq invocations, chacune1/0/0.
- Module daemon_host::atelier_fichiers:: complet :20passés/0échec/3ignorésNAV.
  Inventaire23, trois ignorés nommés avant runtime ; aucun skip ajouté.
- Script canonique check_features.sh : CHECK_FEATURES VERT8/8, dont vrai
  minimal, daemon, natif+daemon, natif+code et batterie avec/sansLLM.
- Superviseur et finaliseur VERT, rc0/restes[], bus privé retiré.

Les cinq ciblés sont inclus dans les20 du module :25invocations brutes,
20fonctions distinctes passées, dont quatre nouvelles. Les trois ignorés ne
sont pas des réussites ; aucun NAV, globale physique ou nouveau service
externe qualifié. Pas d’attribution de l’ancien FAILED physique.

Temps pilote : compilation148.33s, ciblés0.13/0.19/1.04/1.89/1.05s,
module2.45s, frontières223.08s. Ce sont des temps de recette en parallèle
avec le lot2a, pas des mesures de performance.

## Budget, identité et preuve native

Départ R3 conservé902183.114381791, échéance absolue908183.114381791.
Reçu neuf R4 après vrai poste :5205secondes restantes, échéance effective
908179.44508556 ; aucune remise à zéro6000. Finaliseur :1170.821897s
DEPUIS départR3, sous borne absolue. Sources PC et sous-module restés propres,
gitlink a730a9ba89ef233423bb7063f85181de26d98ce0.

ELF lib :033c31d00bb7d63b1034218b5b3bbfec257ad9f9f7bed94621dd77114800375a,
features rag3db-native,code,daemon. Copie hors target avant les juges,
source461e8b868 trouvée dans les octets, inventaires et ledger publiés hors
source avant runtime ; original et copie toujours présents/réhashés.
Ledger SHA313e177e8dde32ce70d5568ef5ea1e61486f220529037cf7843115f297246db8.

Six phases observées réellement (cible1..5 et module), chacune avec
PID/start revérifié, exe/SHA/cgroup et unique chemin SDK29 dans les maps.
Aucune map d’inventaire utilisée comme preuve de juge. SDK29 SHA
b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929.

## Ressources et quiescence bornée

Un slot lourd privé, MemoryMax34359738368/HIGHmax, j8/nice15/ionice3,
pool1GiB, quatre rôles BURN CPU. Aucun target/WT/DB partagé avec Arbre.
Événements high/max/oom/oom_kill0 avant/après ; pic mémoire non collecté.
Target final3192646980octets ; mesures périodiques10s et fins de phase
sous32GiB, pas un quota ni preuve d’absence de pic entre observations.

Scope validation-3jusrgd9 VERT0/restes[]/nettoyé, bus/tmp/r3v-nx67cr_3
absent. Scan final : live[], UNKNOWN_blocking[]. Huit UNKNOWN externes
consignés avec PID/start/cgroup hors scope : aucune absence d’exe/cwd
inférée pour eux, aucune absence générale d’accèsGPU revendiquée.
Aucun signal envoyé à ces processus externes, aucun retrait demandé.

## Archives et rouges conservés

PC : ~/.cache/rag3weaver-build/cx-preuves-ed-read-scope-r4 ; copie portable
cx-preuves-ed-read-scope-r4-local.306fichiers SHA identiques, intégralement
réhashés localement (dont ELF), manifeste
0d33d59f9caf97446700557b3d7f546230abd28c32f4bea68a60cddb7d4b835b.

R2source7a : refus avant reçu/driver/Cargo/target, pont systemd utilisateur
absent de la whitelist. Quatre fichiers conservés deux postes ; manifeste
b00796284c9140c5c3ef5f85bff68074adcdb0ead4f87ddd7cb578e9cfdd1111.
R3source47181 : compilation verte, premier juge rouge parce que le manifeste
helper omettait models.embed ; panic/assertion807 capturée, aucun autre
juge/F8.23fichiers SHA identiques deux postes, manifeste
fcbb0596b9fef45ca756f2e409a125a8e70d46b464483c9ff6c51eb02026c9d5.
ELF rouge6f5f5638a606c42fdcf0a7ce93753ac46d3ea8047fbd239b46a317a9bf418f7a
conservé ; postmortem séparé900identitésPID/start attribuées absentes,
bus retiré. Ces deux rouges ne sont jamais réécrits verts.

Prochain pas : revue des preuves et candidat d’intégration exact par C7.
Ticket ouvert avant livraison. Aucun GO master ni retrait de ressources
n’est déduit de ce bilan ; WT/targets/ELFs/proofs conservés.
