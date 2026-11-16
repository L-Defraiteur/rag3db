# Suite Rust7601 : préparation sans exécution

Tickets existants : chemin standard homonyme et mots vides Rust. La référence
publique FULL02b est livrée et close ; elle n'a pas validé ce correctif de
rag3weaver. REGLES_DE_L_ANALYSE reste7. Aucun ancien GO900 ou1800 n'est réactivé.

## Premier créneau proposé : unités et usages,900s neufs

Source produit : contenu de7601ccf059ed1ab66c4d507f64c200c4ae45761f, avec seulement
ce dossier de préparation ajouté. Parseur34c74e41b2e46570cdbe7d3e6d4218852a1abe1d,
lib59ac9c86bc/SOb7e9fc75...2101/vector1669d639...7561, HashEmbedder64,
features rag3db-native,code pour rag3weaver ; pas de PostgreSQL ni modèle/service.

Le nouveau pilote compose les helpers déjà qualifiés du pilote rouge/vert,
le vrai resolver SDK, la vraie garde et le superviseur canonique. Il vérifie
la fin du corpus, l'ELF bff1cafb conservé et l'ancien propriétaire du target :
`/home/lucied/git_workspaces/cx-codeparsers-language-corpus/extension/rag3weaver/codeparsers`.
Target PC unique `~/.cache/rag3weaver-build/target-import-pointe-pile`,32GiB/
high=max/quota5CPU/j8 ; aucun accès au target portable31GiB. PQS neuf après poste.

Ordre dans une même portée de900s proposée à C7 :

1. cargo_garde test --locked sur codeparsers --test rust_stop_words : 1test attendu,
   vérifie la liste/chargement anglais et les références Rust nommées comme mots JS.
2. Resolver réel puis cargo_garde test --locked rag3weaver --features rag3db-native,code
   --lib code::reference_path_tests : 1test attendu, bytesUTF8/frontières/occurrence.
3. run_e2e.sh --test e2e_usages sans filtre : vérifie toute la suite disponible,
   dont std seul, mot-clé JS en Rust et les deux ordres sur une ligne déjà verts.
   Zéro ignoré requis, noms des quatre témoins lus dans stdout ; pas d'absence masquée.

Commandes/codes/stdout/stderr sont gardés avant assertions, premier rouge
inattendu arrête. La compilation peut consommer tout le budget : diagnostic,
pas de prolongation implicite. Ce créneau ne comporte ni sonde1840fichiers ni
passe complète. La source exacte et le prepared.json seront publiés avant GO.

Le shell appelant doit être hors W/T, comme le pilote ; la commande run se lance
par chemin absolu depuis son répertoire de preuves. Le défaut cwd du corpus
n'est pas importé ici. `prepare` ne fait ni Cargo, ni service, ni autorité.

## Deuxième phase : les quatorze sites historiques et la sonde

Les vrais artefacts ont été retrouvés sans toucher aux WIP :
`~/.cache/rag3weaver-build/stopwords/{roots,sonde-1,sonde-2}`.
Le manifeste frozen-corpus.json fige1840fichiers :290rw et1550engine, chaque
chemin/taille/SHA. Aucun script ancien n'est lancé : sonde.sh changeait les sources
et les dates, utilisait la lib373 et une référence origin/master dynamique.
Ses journaux ne sont pas une nouvelle preuve sur la lib59.

Parmi les lignes retirées sonde-2avant/après, collect/F combinés à un chemin
::collections/::Future/::Formatter sur leur ligne source identifient **14sites**
(fichier/ligne/cible) et **18couples de scopes** : des parents/méthodes portent la
même occurrence. Les deux granularités sont préservées, jamais renommées14couples.
Le candidat doit conserver les bons liens de ces sites ; l'historique seul ne
qualifie pas la justesse de chaque ancien lien « nom » ni les nouveaux liens.
Un collect retiré à watch.rs312 sans ce motif est laissé hors cette sous-liste,
mais reste dans les TSV complets à relire.

Pour la vraie sonde : préparer des arbres baseline/candidat séparés, sans modifier
les sources gelées, fixer les deux couples source/parser et leurs libs identiques59,
rejouer rw et engine séparément sous budgets frais. Comparer les couples mention /
cible et marques, pas seulement totaux ; lire14sites et15–25autres couples retirés,
contrôler les107std::env::var et les appels de projet conservés. N'utiliser une
référence historique comme oracle actuel qu'après avoir établi ses sources exactes.
Le coût historique était646/664s par passe release, hors compilation, sur une
ancienne lib/enveloppe : ce n'est pas un budget validé sur PC actuel.

Les roots sont déjà figés localement ; leur miroir PC et le profil sonde restent
à préparer/valider avec C7. Aucun calcul lourd ni cible/empreinte modifiée par
cette préparation. REGLES8 et passe complète attendent cette qualification.

## Provenance physique du corpus retrouvé

Les chemins historiques roots/{rw,engine}/src étaient des **symlinks**, pas des
copies : ils désignent rag3db-garde, HEAD343ead79962ebcd7ed752a802de6cc87860a2baa.
Les seuls chemins src et extension/rag3weaver/src y sont propres ; le sous-module
codeparsers y est modifié, hors de ces entrées et jamais touché par la préparation.
Une copie régulière, sans lien interne, a donc été créée dans nos notes :
`~/.cache/codeparsers-notes/rust-followup-frozen-corpus/roots/{rw,engine}/src`.
Chaque SHA copié égale celui de la source lue ;290+1550fichiers retenus suivant
les extensions de la sonde historique, ni .env ni .vault lus/copiés.
Manifeste SHA0a6c9af21e3666ff95427ecb44ea253d10907cd5619f96b0a96e91e6c1297185.

Les anciens symlinks et rag3db-garde restent intacts ; cette copie ne certifie
pas rétroactivement que tous les octets étaient identiques au premier passage.
Le contrat frais baseline/candidat portera sur cette copie physique, avec son
manifeste complet, pas sur une référence origin/master ou un lien mobile.
