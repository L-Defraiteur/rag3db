# Qualification core adaptée — proposition, aucun lancement

Même ticket : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Porteur : lucivy ; arbitrage du périmètre et GO exact : C7.

## Candidat et périmètre

Source `890c86af44989f3fea6c6fa62fd40d688b4084df`, déjà descendante de
`origin/main=a1c0dfddfb97666aee148480f8eb3e98d00885ce`. Aucun commit entrant
au dernier fetch, aucun rebase nécessaire. Le diff comprend API pure des
avertissements, appel depuis build_schema, documentation et trois témoins ;
il ne change pas l’indexation. Les trois ciblés sont déjà verts, pas de rejeu
séparé sans cause ; leur présence dans le profil entier serait une régression
plus large, pas une nouvelle preuve ciblée indépendante.

Proposition : profil **par défaut** complet de lucivy-core (lib, tests
integration dont les quatre bancs déclarés comme tests à harness=true), puis
doctests. Compilation offline release, inventaire par exécutable attesté sous
T, liste `--list --ignored` publiée pour chacun, exécution sans filtre et sans
`--include-ignored`. Les diagnostics manuels, bancs et tests PostgreSQL ignorés
restent non qualifiés : pas de zéro ignoré fabriqué. Les nombres libtest doivent
correspondre aux inventaires, avec zéro filtered-out. Les doctests ont un log
séparé ; leurs comptes ne sont pas fusionnés aux autres suites. Arrêt après
premier processus non nul, sans retry. Un délai atteint est un résultat incomplet.

## Corpus : décision préalable nécessaire

Les tests derived-in-RAM, dictionary-index et positions-off exigent le chemin
fixe `/tmp/lucivy-cmp` et au moins 50 sources réelles. Il est absent. Aucun
ABSENT_OK, aucun fallback synthétique admis pour ces contrôles. Le test dédié
`a_missing_corpus_is_red_unless_allowed` teste lui-même la variable dans son
propre processus : son cas synthétique attendu ne constitue pas un substitut
au corpus des autres suites.

Proposition distincte à C7 : admettre Linux **2.6.0** pour cette régression du
core uniquement. Archive déjà en cache à luciepc :
`~/.cache/lucivy-corpora/linux-linux-2.6.0.tar.gz`, SHA256
`4d0299b3f9a5149635173998fc104c88c533523a2b43098a95b4ceba72dc6f73`.
La provenance publique est déclarée dans `playground/corpora.json` ; aucune
extraction, installation ou création de lien effectuée. Préparer ensuite une
extraction privée sûre, UID/nonce/inventaire SHA par fichier, et un répertoire
réel au chemin fixe seulement après admission explicite et contrôle d’absence.
Pas de lien symbolique ni réemploi d’une racine d’autrui.

Pour les tests rag3db et V3 : proposition de lecture du clone connu
`~/.cache/lucivy-tests/rag3db-clone`, commit
`1b22409b31a79543a9fe63e516c5b311bc43f1da`, via RAG3DB_ROOT et V3_CORPUS explicites.
Propreté et identité à recontrôler ; aucune mise à jour ni mutation du clone.
Aucun filtre V3 hérité admis, pas de maxima réduits pour accélérer. Cargo offline
ne suffit pas à empêcher un clone déclenché par un test : les racines existantes
explicites évitent cette branche. Les retours conditionnels et les ignorés
restent à relire lors de l’inventaire final, sans verdict universel implicite.

**Linux 7.2 manque toujours pour le témoin ULP** : cette proposition ne le
remplace pas, ne lève pas son attente et ne qualifie pas parse-sum-order.

## Pilotes préparés, paquet non admis

`outils/qualify-indexed-false-core.py`, SHA256
`76592af50e30c18587934279990ea8d145ab08bab696e930b680455f170b63da`.
`outils/admit-indexed-false-core.py`, SHA256
`3cde2f1ebd9a038641c403e75d5a4eb425ba25fe34cf0d2b0c62361101d707a2`.
Ce sont des propositions à relire, syntaxe AST vérifiée seulement, aucune
exécution. Le manifeste de corpus, le paquet PC et son preflight ne sont pas
créés ; ces scripts ne constituent donc pas encore une commande admise.

Réemploi proposé du seul T `~/.cache/rag3weaver-build/cx-lucivy-target-16nov`,
nonce créateur3552903efab590b44f44cd8002f7b028, UID1000/dev55/inode164536077,
marker SHA5a1671d22dc784bec78f60cf25c8b3003acc5f21e28290f05d0dfc22e0c394df.
ELF précédent vert d10152c78102ee701ef2b808fc2627d7e8044866fcbc5cebe9e5a23550958adc
à copier et rehash ; Commonvert PID1575006/start118237703 TERMINE/verdict0,
restes[] et nettoyé. Nouveaux contrôles frais avant et après, incluant enfants
owned ; UNKNOWN nonattribués conservés, owned/attribués opaques bloquants.
Pas de purge, adoption tierce ou cible supplémentaire. Les bibliothèques sont
Rust Lucivy : aucun SDK/ABI rag3weaver lié à prétendre ici.

Budget proposé : **1800 secondes uniques après acquisition du vrai poste**,
32G/HIGHmax, jobs min(4, affinité moins deux, quota hiérarchique), libtest2threads.
Common SHA6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea,
poste SHA79e3a8bbf457e3bc9104aa1558e7d03072216c70b68e08ae606ace6aa4584c7b.
Nouvelle enveloppe timeout1810, scope/logs neufs et nonce neuf ; pas de reset de
l’échéance lors du passage au confineur. Manifeste des copies et trois références
source/pilote/exécutable (ELF final issu du JSON Cargo) publiés avant verdict.
Le pilote demeure provisoire jusqu’au verdict Common et au scan final owned.
La préparation du corpus doit aussi être bornée et comptée, sans budget caché.

## Relecture banc des preuves ciblées

RO favorable bornée, commit3068ad7c7fcff78e7e83a732a40afed92852dd0d,
branche origin/cx-banc-lucivy-indexed-ro-20261116,
`docs/banc/16-11-2026/07-relecture-des-preuves-indexed-warning.md`.
28 empreintes du manifeste conformes et hashes internes cohérents. Aucun run
Banc, aucun rehash des binaires par Banc. Nuance : 12 UNKNOWN entrée, **13 après
tests**, 12 final, tous nonattribués ; pas d’absence universelle /proc.
Aucun core entier, ULP, corpus7.2, fusion, tag ou publication qualifié.
PR42 reste HOLD suite au refus, sans contournement.

Préparation admise et matérialisée ensuite : voir08, ses empreintes finales,
corpus privés et neuf targets HOLD explicites. Les hashes proposés ci-dessus
sont ceux du brouillon28bdd49e72, pas ceux du paquet final à soumettre au GO.
