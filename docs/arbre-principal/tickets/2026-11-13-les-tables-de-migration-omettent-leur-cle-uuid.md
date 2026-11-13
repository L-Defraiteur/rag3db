# Les tables de migration omettent leur clé _uuid

- **État** : ouvert ; ciblé privé P14 qualifié sur a43c475f9 (preuve liée), qualification globale de l’union et livraison attendues.
- **Famille** : produit.
- **Gravité** : création du journal de migration impossible sur base neuve native.
- **Atteignable en service** : primitive commune MigrationRunner/Catalog ; reproduit dans le profil physique synthétique privé, aucun index réel touché.
- **Touche rag3weaver** : oui, Catalog::migration_initialize.
- **Pour** : arbre-principal ; revue mémoire/orchestration.
- **Signalé par** : qualification P5 source39f798f99dfc6c5c100f5a73de9c00fc29ffc992.

Parent : [raccord des ancres](2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md).

## Preuve et cause

Premier juge natif exact frozen_program_codec_binds_sources_records_and_probe_inputs,
create_new : binder refuse PRIMARY KEY(_uuid), propriété absente. Arrêt P5 immédiat,
aucun autre juge natif ni enfant exécuté. Compilation et neuf témoins CPU verts.
Preuves PC : cx-raccord-physique-cible-v5/physical-00.log ; superviseur
cx-raccord-physique-confinement-v5/validation-51nb2q_j.

Catalog::migration_initialize déclare les champs métier de _DataflowMigration et
_DataflowMigrationLock sans _uuid, puis appelle SchemaDialect::create_table.
Rag3dbDialect et PostgresDialect posent PRIMARY KEY(_uuid) sans inventer la colonne.
Les écritures migration_record, migration_acquire_lock et le journal qualifié
fournissent déjà cette identité ; ce sont les deux déclarations qui manquent.
Le constat PostgreSQL est statique, aucune instance ni test PG lancé ici.

## Correctif et qualification attendue

7947de3c8 ajoute ColumnDef _uuid Text dans les deux listes communes. Ni adaptation
DDL locale au profil physique, ni modification du dialecte ou de l'identité.
Les tables préexistantes ne sont pas migrées : CREATE IF NOT EXISTS reste inchangé.
Relire le diff puis nouvelle qualification synthétique sous budget distinct
coordonné. Le premier juge natif doit dépasser les créations réelles des deux
tables ; les juges de journal/reprise doivent ensuite exercer leurs écritures.
Aucun VERT runtime attribué au correctif avant cette qualification.

Qualification ciblée P14 sura43c475f9 : VERTE (voir bilan parent lié).
Créations/écritures/reprise ou lecteur RO réellement éprouvés dans le profil
synthétique BM25. Qualification générale et livraison non faites ; ticket ouvert.


## Reprise corrective distincte — décision C7 après expiration

L’échéance initiale 896572.526453239 est expirée : aucune reprise ni prolongation
de ce reçu. Le rouge global initial reste conservé. Source application EXACTE
4e5ae8a324f50313b74f70772eb2e92ee41fad6f et quatre ELF du ledger inchangés.
Pilote préparé hors WT source, étape corrective DISTINCTE de 3600 s après poste
réel, runtime complet avec cwd crate, sans Cargo/rebuild/F8/min déjà verts.
PG initiale cx_phys_2191_20261113_u1 conservée, identité/OID et dump requis avant
tout retrait ; nouvelle PG privée cx_phys_2191_20261113_u1c, absence préalable
et création unique contrôlées. Aucun DROP/reset. Revue et GO exact requis avant
lancement. Aucun résultat runtime global vert ; P14 ciblé seul demeure qualifié.
Les champs protégés UNKNOWN ne prouvent pas une absence d’accès GPU.
Voir [recette corrective](../13-11-2026/04-recette-corrective-runtime-physique.md).


## Étape corrective distincte : arrêt au premier nouveau rouge

C7 a donné GO exact au pilote e8721a5683166e8ff688e55977bc79cdcf50de11,
application figée 4e5ae8a324f50313b74f70772eb2e92ee41fad6f et quatre ELF hérités.
Portée DISTINCTE de 3600 s : départ monotone 898110.830219227, échéance
901710.830219227 ; aucune prolongation/réinitialisation de l’ancienne portée.
Aucun Cargo/F8/min/rebuild exécuté. Ledger neuf publié et quatre ELF copiés
avec SHA vérifiés AVANT runtime :
7fe4942919dd2ff3fede289c010be11509b5b1fb026b61c7c1e704bf5b9d209a.

Lib depuis le crate : ancien juge CWD passé ; nouveau premier FAILED :
daemon_host::atelier_fichiers::tests::the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree.
Arrêt immédiat, durée portée 85.917 s, runtime environ 8 s. Aucun résumé
lib complet, aucun corps de panique publié avant arrêt : cause précise NON
attribuée. Ni rejeu inchangé ni Cargo pour récupérer une panique. NAV/CLI/
lectures/BGE/PG-suites/TX non lancés. Le rouge global reste ROUGE.

Superviseur validation-ej1zgwoe : ROUGE/code1/restes[]/nettoyé ; bus
/tmp/r3v-py3oi02g absent. Scan RO exe/cwd target+deux WT : live[].
Les UNKNOWN restent non concluants sur absence GPU. PG u1 conservée et
PG u1c créée une seule fois, OID1989865 ; aucune suite PG jouée, aucun DROP.
Fixtures restantes conservées, sans ouverture native ni nettoyage.

Preuves PC : cx-passe-raccord-physique-union-complement-v1 ; copie locale
cx-preuves-raccord-physique-union-complement-v1-local/client : 2843 fichiers
identiques SHA au manifeste après arrêt. Superviseur aussi copié localement.
SHA lib.log : 8295e630e40de1f29d5b0a7df6eae5643cdb865e69722a66ce004acbbf3b71a5.
C7/ED poursuivent attribution RO ; aucun défaut production encore établi.
P14 ciblé reste vert dans sa portée ; aucune qualification globale obtenue.


## Qualification corrective v2 obtenue — livraison en attente

GO C7 exact pilotea18/application4e, lib2299/0/66,NAV40,CLI5,
lectures27jugesdistincts/28bruts,CREATEexact,hôte13,PG36 et4TX passés ;
superviseur/finaliseurVERT sansrésidu, échéancecorrective conservée.
1064 fichiersSHAidentiques deuxpostes. AucunCargo/rebuild ; deuxrouges
antérieurs conservés. PasGOmaster, tickets ouverts avant livraison/revue.
FD /dev/dri observés : aucune absence d’ouverture GPU revendiquée.
Détails, compteshelpers, bornes et dump dans
[preuves07](../13-11-2026/07-preuves-globales-correctives-physiques.md).
