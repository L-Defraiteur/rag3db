# La CI refuse la réouverture du témoin physique

- **État** : ouvert ; échec CI exact observé, diagnostic de cycle de vie à attribuer sans rejeu.
- **Famille** : produit.
- **Gravité** : CI de master rouge ; portée production et cause à établir.
- **Atteignable en service** : à départager ; échec actuel dans une fixture native privée.
- **Touche rag3weaver** : oui, dataflow/migrations/physical_index/tests.rs:414 et cycle de vie de la connexion native ; moteur statique CI à examiner.
- **Pour** : orchestration, ED pour le diagnostic Rust et coeur-cpp pour la provenance et le registre des writers ; coordination C7.
- **Signalé par** : contrôle périodique C7 du 13 novembre 2026.

## Preuve exacte, sans attribution prématurée

CommitCI d17a230b14f60fbac59aa1e4ac10510ba5f3c7e2 ;
[run43407787602](https://github.com/L-Defraiteur/rag3db/actions/runs/43407787602),
job131826283645, étape11 cargo test --lib
rag3db-native,code,daemon,openai-llm. F8 et compilation des tests ont réussi.
Résultat :2257 passés,1échec,66ignorés,2exclusions existantes du workflow.

Échec native_legacy_foreign_schema_and_values_refuse_owned_open_without_writes,
src/dataflow/migrations/physical_index/tests.rs:414:49 : connection(root,false)
refusée parce que owned/index.rag3db est déjà ouvert en écriture dans ce
processus. La ligne précédente crée puis droppe OwnedPhysicalIndex ; il faut
identifier quel writer ou quelle référence reste vivant, sans inventer la cause.
La fixture /tmp/.tmpPROaIV est signalée PHYSICAL_FAILED_FIXTURE par le log.
Aucune copie ni base de Lucie n'a été ouverte pour ce contrôle.

Les quatre nouveaux juges de layers passent dans ce même log. Aucune
attribution du rouge au résolveur pur2a n'est établie. Son ticket livré n'est
pas rouvert par simple coïncidence temporelle.

La CI utilise RAG3DB_LIB=statique ; les qualifications locales utilisent
SDK29 épinglé. Ne pas assimiler ces environnements ni réétiqueter l'un avec
les preuves de l'autre. Source native exacte, garde d'écriture et lib réellement
liée en CI restent à relever depuis workflow/logs/sources.

## Prochaine action et limites

ED : lire fixture, création/Drop/ouverture et détentions Rust réelles ; proposer
un diagnostic et, si établi, un correctif isolé fondé sur le contrat existant.
Coeur-cpp : identifier la garde du writer dans les sources de la CI et sa
libération ; comparer aux conditions du SDK local sans rebâtir/patcher la lib.

Lecture seule initialement ; aucun Cargo/rustc/C++/DB/service ni rerun GitHub.
Tout runtime futur exige un plan publié/source exacte et un créneau coordonné.
Ne pas supprimer le juge, ajouter un sleep arbitraire, affaiblir la protection
contre deux writers ou forcer une ouverture concurrente.

Fin : cause attribuée avec preuve, correction et qualification pertinentes,
CI future verte sur source corrigée. Aucun ancien run rouge repeint ; ticket
reste ouvert tant que cette conclusion n'est pas acquise.
