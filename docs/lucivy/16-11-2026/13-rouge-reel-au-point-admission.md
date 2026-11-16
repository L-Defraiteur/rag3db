# Rouge discriminant réellement observé au point admission

Ticket : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Allocation exacte **C7-LUCIVY-ADMISSION-7833-16nov615** ; recette12,
APP81b0e74f0254c8216864f90a460f8969842432ed/pilote7833/admit7649/préflight0c90301e,
Qv2nonce d4f2d8cfd6d6705b2c17423ba66d91a9. Une seule passe, aucun retry.

## Ce que le témoin établit

Compilation offline release réussie **32,20s**, trois ELF attendus attestés sous
le target privé. Le pilote a publié/fsync le ledgerAPP/ELF avant les appels
d’inventaire, puis le ledger complet des trois inventaires avant l’unique
invocation de test. Les deux autres ELF ne sont invoqués que pour leur liste,
jamais pour leur test réel ni un builder d’index.

[Log natif](preuves/derived-admission-red-81b0/logs/consumer-witness.log) :
`missing_panel_is_rejected_before_any_builder`, **0passé/1échec/0ignoré/8filtered**,
rc101. Assertion d’admission précise, **left1/right0** : le builder mock a été
appelé une fois, alors qu’il devait être refusé avant toute construction.
Ce n’est pas un rouge de compilation, de fonction manquante ou de corpus réel.
Le témoin atteint l’entrée également utilisée par le consommateur derived-in-RAM,
avec50 documents en mémoire, assez nombreux mais sans couverture de son panel.
La baseline conserve donc bien le défaut à corriger. Aucun correctif de
couverture ou livraison n’est déduit de ce résultat rouge.

L’ELF effectivement testé est
**7c6f65e2e8427b53828a58cecd313d4a6c02cd7a3074f358d14ce5f2114ea9e8**.
Inventaires complets :

| Target | Tests listés | Ignorés listés | Corps de tests joués |
| --- | ---: | ---: | --- |
| test_derived_fixture_admission | 9 | 0 | Unique témoin rouge ; huit autres nonjoués |
| test_derived_in_ram | 1 | 0 | Aucun |
| test_sfx_v3_ground_truth | 12 | 1 | Aucun |

Les trois ELF ont été physiquement rehashés après le test par le pilote et
comparés au ledger initial, puis rehashés à nouveau lors du contrôle externe :

- Témoin mémoire :7c6f65e2e8427b53828a58cecd313d4a6c02cd7a3074f358d14ce5f2114ea9e8.
- Derived natif compilé :64758437584801255698598fea0db343a6528c22cfdfb5df851e79b2f5700ede.
- Ground-truth compilé :777c911f0216529d01fb8957cd3bcfe66d80b86b7378f10e0e33f7d977e5036d.

HEAD81b0/propreté/copies figées/préflight réattestés ; SHA final issu des
lectures physiques, pas un report aveugle du ledger initial. La limite de
ledger tardif du précédent core rouge reste historique, elle n’est pas effacée.
Les gardes Python avaient été vues rouge puis6/0 avant ce GO ; elles sont une
preuve distincte de ce premier rouge Rust.

## Fin, budgets et conservation

[Bruts et empreintes](preuves/derived-admission-red-81b0/SHA256SUMS),
[contrôle externe final](preuves/derived-admission-red-81b0/final-evidence.json).
CommonROUGE101/erreurnull/restes[]/nettoyétrue, scopevalidation-e1p6s0dc,
autoritéTERMINE **PID588652/start119729720**. L’appel SSH rend1, le test et
Common portent101. Common détenu absent et aucun owned/attribué actif au
scan final ; **11UNKNOWN nonattribués** conservés, après12 à la préparationv2.
Pas d’absence universelle/proc ou de permission contournée.

START après acquisition du poste1197297.224958866 ; borne de validation
START+600=1197897.224958866, deadlineCommon pour le reste moins2 arrondi
1197894.323357855. Contrôle final externe **1197585.443130607**, sous les deux
bornes ; aucun reset. Enveloppe dure distincte615 (TERM610+kill5) ; aucune
affirmation de borne600 pour tout le parcours/attente/nettoyage. Ressources
réellement lues :32G/HIGHmax/swap0, cpu.max400000/100000, jobs4, cwdQv2.
Parent bash/noclobber, aucun diagnostic fish cette fois.

Scratch vide, marqueurcréateurT3552/5a1671 inchangé, aucun nouveau target ou
purge. L’ancien ELFcore106fa peut être remplacé sousT par la compilation ; ses
copies RO Qv1/Qv2 restent physiquement106fa. L’ELF testé7c6f est copié RO sous
Qv2/tested-witness-elf. Les deux sont également archivés sur le portable sous
`~/.cache/rag3weaver-build/cx-lucivy-admission-proof-archive-81b0-16nov/`, avec
empreintes vérifiées et manifeste. Ce dossier est une archive de preuves, pas
un targetCargo ; aucun ELF archivé n’y est exécuté. Bruts conservés sur les deux
postes, anciens rapports/commits et corpus originaux intacts.

La capacité privée du superviseur n’est pas publiée : seuls les champs publics
pid/start/état/deadline/scriptSHA de son autorité sont archivés dans le dépôt.

## Suite bornée

RésultatROUGE attendu, **pasdelivery**. Les huit autres témoins Rust restent
nonjoués, de même que les corps natifs, les corpus réels et les anciens trois
ciblés warnings dans cette passe. NeufHOLD/ignorés/full_core=false, Linux7.2/ULP,
PR42/main/tag gardent leurs limites. Pas de sélection posthoc ou d’assertion
assouplie. Ticket ouvert avant tout correctif futur ; C7 reçoit ce résultat et
Banc est sollicité pour une RO des preuves uniquement, sans run/proc/base/service.
