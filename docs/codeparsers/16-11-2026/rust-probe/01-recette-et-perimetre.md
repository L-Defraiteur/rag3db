# Sonde historique : deux productions figées, une entrée et un SDK communs

État : préparation seulement. Aucun Cargo, test natif, replay, allocation ou
ancienne sonde.sh lancé pour cette phase. Les unités/usages verts118 sont acquis,
pas rejoués ici. Ticket existant : chemin standard homonyme, plus mots vides Rust.
REGLES_DE_L_ANALYSE reste7 ; aucune passe complète ni permission master.

## Couple contrôlé

| Phase | Production | Parseur | Arbre privé |
|---|---|---|---|
| Référence | d395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6 | bd133850f89d6b12a3ca2b2f6ce238785ed37e3a | cx-codeparsers-rust-probe-baseline |
| Candidat | 118cc71eea57ebdfa4c79ae01d5d5a0caf181d4b, donc code7601 | 34c74e41b2e46570cdbe7d3e6d4218852a1abe1d | cx-codeparsers-rust-fix |

Référence exécutable préparée fec35198b64b7e9da0496de6b01a7232fa7a8390 : seuls
le test sonde et ses documents diffèrent de d395. Le candidat reçoit le **même
fichier sonde**, hash égal vérifié par le pilote. Sa version exécutable sera le
HEAD du commit de publication de cette préparation ; aucune modification de
production depuis118. Les versions exactes sont des arguments/constantes relus,
pas origin/master mobile ni source qu'on copie par-dessus un arbre pendant le jeu.

Entre les deux productions, les seuls chemins de rag3weaver modifiés sont
codeparsers,src/code.rs (64lignes ajoutées) et les témoins e2e_usages non utilisés
par cette sonde. Cargo.toml/lock,build.rs,anciens scripts et sonde initiale sont
identiques. La différence parser bd→34 est la liste Rust, son chargeur/générateur
et son test, pas un changement des tables de méthodes standard. Le pilote refuse
une autre différence de production par rapport à chaque point de départ.

## Corpus réellement admis et accès latéraux

Même copie PHYSIQUE aux deux postes, hors des worktrees compilés :
`~/.cache/codeparsers-notes/rust-followup-frozen-corpus/roots/{rw,engine}`.
1840sources,290rw et1550engine, manifesteSHA
0a6c9af21e3666ff95427ecb44ea253d10907cd5619f96b0a96e91e6c1297185.
Aucun lien, UTF8strict, taille<=1000000octets, liste exacte sans fichier ajouté ;
chaque SHA est vérifié avant et après chaque phase. L'ordre est rw puis engine,
un catalogue neuf en mémoire par racine, jamais un projet combiné.

Le lecteur de la sonde trie les fichiers ; son profil strict exige découverts=
admis=analysés=ingérés=fichiers stockés=effectif attendu, aucun skipped ou failed,
aucun scope orphelin. Les diagnostics d'analyse sont écrits avant les assertions.
Les erreurs ne deviennent donc pas une réussite du fait que le test s'est terminé.
Les ASTinvalides ne sont pas exposés par CodeAnalysis dans ce test : ne pas les
inférer de ces assertions ni qualifier un AST absent.

Chemin effectif relu : analyze→analyze_in_project_avec reçoit content_map complet,
resolve_cross_file=false,manifestes=None. Les #include sont résolus depuis la
**table File du catalogue** (Inclusions::charger), pas en relisant un arbre voisin.
Les résolveurs historiques capables de lire Cargo.toml/compile_commands ne sont
pas utilisés pour cet appel. L'absence de manifestes est commune et explicite,
comme dans la sonde originale ; cela ne qualifie pas la provenance d'un projet
complet. Le CoordinateRegistry reçoit des chemins physiques hors Git : cela peut
différer de l'ancien alias vers rag3db-garde343 ; ne pas certifier rétroactivement
les UUID de l'ancienne sonde, comparer les faits identifiés de la paire fraîche.

## SDK, configuration et profil proposé

Lib commune réelle59ac9c86bc, jamais373 transférée :
rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF.
SOb7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101,
vector1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561.
Le pilote compose le resolver SDK déjà qualifié UNEfois puis cargo_garde test
--locked --features rag3db-native,code --test sonde_marque_nom -j8 -- --ignored
--nocapture. La sonde lit CALL BUILD_ID() du vrai moteur, l'enregistre et exige
l'égalité au SDK annoncé. Les libs, manifests, scripts, probe et outils sont hashés.

HashEmbedder64CPU, pool1GiB, RUST_TEST_THREADS=1, CARGO_INCREMENTAL=0. Profil natif
**debug de test identique** pour les deux, comme la qualification118 ; l'ancien
release646/664s n'est pas pris comme mesure actuelle. Pas de modèle, PostgreSQL,
base de Lucie, bus utilisateur de Lucie ou mécanisme de C++/SDK reconstitué.

Deux propositions d'allocation distinctes, chacune1800s : référence puis candidat,
32GiB/high=max/quota5CPU/j<=8. Le coût sur lib59 reste inconnu ; la borne peut
arrêter une phase, preuve partielle et diagnostic, sans retry ni prolongation.
Les1800s incluent poste acquis→préparation privée→compilation→deuxracines→contrôles.
Aucun de ces budgets n'est armé par ce document ou par prepare.

Target unique PC ~/.cache/rag3weaver-build/target-import-pointe-pile. Départ :
propriétaire fix/extension/rag3weaver après END118 vérifié et ses trois ELF archivés.
Le candidat refuse de partir si la référence n'a pas terminé VERT/nettoyée.
Après chaque vrai poste : autorité vérifiée, nouveau scan OS/PQS sans exemption,
propriétaire attendu exact, adoption uniquement par garde_target via cargo_garde.
L'appelant et le pilote sont hors W/T ; TMP/XDG/state frères privés sur disque,
HOME conservé et magasin réel explicite. Le portable31GiB reste intact.

## Sorties et jugement

JSONL autoritaire : noms,types,chemins,lignes/octets,key et _uuid de chaque scope
mentionneur/cible,ligne de relation et marque. TSVsixcolonnes historique en plus,
pas comme représentation unique d'identité. Les compteurs reprennent les marques
lues du graphe et sont recoupés avec toutes les lignes ; bornes et IDs non vides
sont contrôlés. JSONL/multiensembles conservent les occurrences homonymes, sans
dédupliquer une identité parce que les six colonnes historiques se ressemblent.
Les dérives d'UUID natif pour une occurrence logique égale restent une donnée.

La comparaison exige les deux reçus canoniques complets puis produit des deltas
pour rw/engine séparés. Les14sites/18couples anciens sont listés avec présence
exacte et présence vers la même cible sous une autre marque, sans déclarer les
anciens liens nom tous justes. Le fichier historical-var-pairs.json fige aussi
les117anciens couples var :107lignes portent std::env::var,10autres restent à
juger. Une chaîne présente sur une ligne n'est pas un oracle d'occurrence AST ;
aucun appel nu n'est supprimé de l'oracle parce qu'un std est écrit à côté.

Après mesure : lire les14sites, les107couples annotés, les10autres var, et15–25
couples retirés. Expliquer les arêtes fausses retirées, les bonnes conservées,
les ajouts et les limites. L'outil n'automatise pas leur vérité par le seul nom.
Pas de REGLES8, fermeture du ticket ou global vert avant ce jugement.

L'échantillon des autres retraits est préfixé avant résultats : graine
rust-path-probe-20261116, triSHA de l'occurrence, jusqu'à10parracine puis complément
pour20au total, en excluant les cas historiques déjà listés. S'il n'y a pas assez
de retraits, tous sont rendus avec sous-effectif ; aucun couple n'est inventé.
Les refus d'admission ont leur fichier/reason écrits avant l'assertion, pas un
simple compteur global. Cette instrumentation a été formatée et relue mais
**n'a pas été compilée ni exécutée** ; pas de tests Python rejoués non plus depuis
la demande de préparation seule. La première allocation devra conserver tout
rouge de compilation/protocole, sans relance aveugle.
