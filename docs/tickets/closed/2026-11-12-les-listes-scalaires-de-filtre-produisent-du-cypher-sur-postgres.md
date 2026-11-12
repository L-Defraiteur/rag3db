# Les listes scalaires de filtre produisent du Cypher sur PostgreSQL

- **État** : corrigé sur 831d08a35 ; critères éprouvés par la passe finale complète, voir clôture.
- **Famille** : produit
- **Gravité** : recherche dense refusée par erreur de syntaxe dans le masque d'admission.
- **Atteignable en service** : oui, Catalog::rechercher avec liste d'UUID sur PostgreSQL.
- **Touche rag3weaver** : oui, FilterParser et représentation dialectale de l'appartenance scalaire.
- **Pour** : arbre-principal, coordination recherche/orchestration pour le contrat commun.
- **Signalé par** : témoin privé SQL des versions et occurrences du lot 1a.

## Preuve

Source **60961bb24**, `cx-pg-graphe-gdb-vert.log` (nom de relance, pas verdict global) :
le blocage disparaît, les deux ingestions passent, les assertions deux versions/deux chunks
sont franchies. La première lecture publique dense échoue en 0,37 s : SQL42601 sur
`p."_uuid" IN $1` dans la recherche ContenuPg_Chunk. Test final **0/1**, aucun vert SQL complet.
La connexion est intacte ; l'erreur vient de la compilation du filtre, pas d'un format
modifié de fixture. Le masque canonique emploie FilterValue::List comme les filtres publics.

FilterParser, inchangé par 1a, construit IN directement pour List/In/NotIn. Les primitives
IR du dialecte SQL savent déjà émettre = ANY(param), mais cette règle n'est pas partagée
avec FilterParser. La syntaxe est distincte de la sérialisation UUID[] corrigée plus tôt.
Aucune garantie supplémentaire de typage des tableaux n'est déduite de cette preuve UUID.

## Suite bornée

Poser un rouge de compilation de filtre scalaire texte sur PostgresDialect ; factoriser
la représentation d'appartenance dans le dialecte, puis la consommer dans FilterParser
et les prédicats IR existants. Les trois chemins List/In/NotIn doivent rester équivalents,
le natif conserver IN, les paramètres et le masque avant top-k rester présents.
Rejouer le vrai parcours public SQL et la suite PG entière (transactions comprises).
Lien : [versions et occurrences](2026-11-11-deux-occurrences-identiques-reembarquent-le-meme-contenu.md).

## Rouge et verts ciblés

Rouge réel sur **5edd09b6f**, `cx-filter-membership-rouge.log`, 0/1 : IN émis au lieu de ANY.
Correctif code isolé **d84c03432** : primitive SchemaDialect::filter_in, reprise par les
chemins List/In/NotIn et les prédicats IR existants, ni paramètres ni sérialiseur modifiés.
`cx-filter-membership-vert.log` : **49 tests filtres + 59 tests dialectes verts**, dont
liste vide/non vide, négation, natif IN et SQL ANY. Même source figée pour les parcours
suivants : `cx-contenu-pg-v5.log` **1/0** (0,71s) et `cx-pg-complet-v5.log` **36/0** (8,56s),
rc0 sans résidu. Le masque reste en amont du classement ; la projection n'est pas désactivée
pour obtenir ces résultats. Ces preuves portent les listes texte/UUID demandées ; aucune
nouvelle garantie de tableaux numériques ou hétérogènes n'est ajoutée.
Attente de la passe combinée 1a avant verdict d'intégration.

## Clôture sur le candidat final 831d08a35 — 12 novembre

Critères satisfaits sur source propre **831d08a35564982af1d0a3cb64707847e0614f3b** :
gardes vertes, features5/5, lib2139/0/64 ignorés, navigateur39/0/0,
**14/14 suites e2e, 225 tests réussis, aucun échec ni ignoré**, rc0.
Une nouvelle passe entière suit les correctifs de revue ; aucun vert6fb recyclé.

Preuves luciepc : `~/.cache/rag3weaver-build/cx-passe-combinee-1a-v6/`
(passe.log, verdict-detaille.json, VERDICT, preuves.sha256, binaire-final.sha256).
Binaire e2e SHA256 **0ffa3e43258b628890ea09aac590827bbd12687af443685f153a0f686bdf76ed**.
Résidu connu tx arrêt : trois CSV, ticket recherche conservé ; pas de promesse
sans résidu. Les premiers rouges et montages antérieurs gardent leurs sources.

Revues finales sur les mêmes tickets : ED pour le manifeste/SchemaFacts, c7
pour Scope/PG/dialecte, archi pour projection/enrichissement/provenance ; aucun
bloqueur restant. Le graphe BM25 compare un label réel non vide ; signals et
best_signal peuvent être vides, ce n'est pas une preuve nouvelle de fusion.
Migration1b, activation des gabarits code et mesures M3–M5 restent distinctes.
