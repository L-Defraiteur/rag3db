# Les tables de migration omettent leur clé _uuid

- **État** : ouvert, correctif privé publié, qualification non jouée.
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
