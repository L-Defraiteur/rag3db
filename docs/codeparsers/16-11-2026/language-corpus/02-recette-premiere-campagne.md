# Première campagne : source publiée, exécution à réserver

La CLI `examples/language_corpus_probe.rs` ajoute seulement un appelant des workers
`parallel::{parser_worker,generic_worker}` et des routeurs de production. Aucune
modification de ces API ni des grammaires. Base parser bd13385 explicite ; sa
branche `cx-language-corpus-probe` ajoute l'exemple. Aucun build encore joué.
Le contrat d'acquisition est dans `01-inventaire-et-protocole.md` et validé avec Banc.

## Fonctionnement réel de la source

Entrée directe `--manifest PATH --roots PATH --output PATH`, ou protocole worker
`--stream --roots PATH` : une entrée JSONL sur stdin, une réponse flushée sur stdout.
Le mode direct sert au diagnostic ; la campagne utilise `control_probe.py` pour
borner les fichiers. Aucune résolution inter-fichiers ni modèle/DB/SDK/native.
La bibliothèque se compile seule avec son propre Cargo.lock, `--locked`.
Le binaire utilise `sha256sum` système sur les octets lus, jamais un exécutable du corpus.
Il conserve BOM/CRLF, vérifie taille/hash et refuse les chemins symlinks ; limite
publiée de 16MiB par fichier, rejets nommés sans prétendre avoir analysé leur contenu.

Le contrôleur vérifie le HEAD lorsqu’un checkout est présent ; pour une archive,
il vérifie freeze.json (identité, commit, racine, absence de code exécuté), puis
les SHA de source.tar.gz et inventory.jsonl avant les workers. Le manifeste,
les racines, le contrôleur et le binaire sont hashés. Les workers vérifient les
hashes des fichiers avant parsing ; les checkouts restent gelés pendant la mesure.
Chaque appel utilise les options par défaut du worker (son champ options est
ignoré par l'implémentation actuelle), aucune option de façade inventée.
Markdown n'analyse pas les blocs embarqués. HTML et Svelte sont refusés explicitement.

Le délai de 10s/fichier inclut transport, lecture, vérification de hash,
initialisation éventuelle et réponse ; `elapsed_ns` dans la mesure couvre seulement
l'appel au worker de production, initialisation paresseuse incluse. Cache série
réutilisé ; restart après panique, timeout ou sortie. Aucun fichier rejoué.
Limite supplémentaire RLIMIT_AS 2GiB/worker : espace d'adressage, pas RSS.
La portée canonique reste la borne globale 32GiB/CPU et temps de campagne.
Le contrôleur ne tue que son propre groupe de processus créé avec le worker.
Erreurs AST/absence de capture/décodage sont des données ; provenance/protocole
incohérents arrêtent la campagne. Les diagnostics stderr sont conservés.

Compteurs de scopes récursifs et types IR, références/membres comme faits ; pas de
conversion automatique en symboles. Pour les façades spécialisées, les tailles
d'arrays de leur résultat sérialisé sont publiées avec leur chemin, pas comme
un IR commun inventé. Le pic RSS est celui du processus réutilisé, pas du fichier.
L'absence de champ AST sur une façade est `not_exposed`, pas « valide ».
Toutes les lignes portent `coverage_status=unjudged`. La phase suivante exigera
résolution par projet et oracle annoté sur échantillon, sans promesse de 100%.

## Recette bornée proposée à C7

Poste : luciepc. Budget global neuf demandé : 1800s, 32GiB, memory.high=max,
quota5 CPU, jobs8 au plus. Compilation puis contrôleur dans une unique portée
`confiner_validation.py lancer --delai 1800`, acquérant le vrai `poste lourd`.
Aucune commande lourde n'est autorisée par cette page seule.

Réutiliser le target de session existant à luciepc,
`~/.cache/rag3weaver-build/target-import-pointe-pile`, après PQS OS et adoption
par `cargo_garde.py` ; propriétaire précédent : worktree Rust fix. Aucun target
portable ni deuxième cache. L'adoption peut invalider des empreintes de crate :
coût inconnu, à consigner, sans copie de cache ni retouche de fingerprints.
L'usage du même target interdit un chevauchement avec la qualification Rust.
Les deux lots doivent donc être séquencés par C7.

Dans la portée réelle, environnement privé TMP/XDG/state, HOME conservé,
services/modèles/PG absents, target exact, CPU, autorité vérifiée :

```sh
python3 extension/rag3weaver/scripts/cargo_garde.py build --locked \
  --manifest-path extension/rag3weaver/codeparsers/Cargo.toml \
  --example language_corpus_probe -j8
python3 -B docs/codeparsers/16-11-2026/language-corpus/control_probe.py \
  --binary "$CARGO_TARGET_DIR/debug/examples/language_corpus_probe" \
  --manifest /absolute/frozen/selected.jsonl --roots /absolute/frozen/roots.json \
  --proofs /absolute/private/new-campaign --file-seconds 10
```

Le pilote de lancement doit encore figer source parent/sous-module et hashes,
chemins manifestes réels de Banc, allocation fraîche, PQS et autorité canonique.
Cette recette ne remplace pas ce pilote ni le GO. La compilation peut consommer
tout le budget : si elle expire, preuve et diagnostic, pas de relance implicite.

## Vérifications déjà jouées

`rustfmt --edition 2021` accepte la syntaxe de l'exemple ; ce n'est pas une
compilation. Sept tests légers du contrôleur Python : réponse JSON fragmentée,
worker silencieux borné et récolté, sortie17 distinguée du timeout, protocole
invalide refusé, chemin canonique réel, archive sans .git acceptée, archive
altérée refusée. Le témoin archive sans .git a été vu rouge (git rc128), puis vert. Aucun Cargo, aucun corpus ni code de projet public exécuté.
La compilation, les témoins de parsing et la campagne restent NON JOUÉS.

## Complément exécutable et oracle compact

Exemple désormais `7b09657` (même branche/base) : inventaire compact des scopes
avec nom/type/parent/positions lignes et octets, membres, paramètres, variables,
héritage/génériques, décorateurs et bindings d'import, sans contenu ni signature.
Budget d'inventaire 2MiB/fichier ; omissions comptées explicitement. Les définitions
peuvent ainsi être jugées sur les fichiers déjà parsés, sans relancer le corpus.
Les compteurs complets restent séparés. Les façades spécialisées ont encore
seulement leurs compteurs de sorties : leur oracle nécessitera une phase ciblée.

`pilot.py` prépare sans Cargo un profil figé sur luciepc puis compose compile
standalone et contrôleur sous la portée canonique. PQS OS reprise du pilote Rust
qualifié, après acquisition du vrai poste ; adoption exclusivement par la vraie
garde, aucun fingerprint touché à la main. Le profil refuse un propriétaire
différent de celui du target Rust7601 attendu. La compilation et la campagne
ont leurs commandes, stdout/stderr et codes séparés, même si le résultat est rouge.
Le verdict global termine uniquement la campagne exploratoire ; coverage reste
non jugée et aucune passe complète rag3weaver n'est revendiquée.

```sh
python3 -B docs/codeparsers/16-11-2026/language-corpus/pilot.py prepare \
  --worktree /home/lucied/git_workspaces/cx-codeparsers-language-corpus \
  --source FULL_PARENT_COMMIT \
  --manifest /home/lucied/.cache/rag3db-banc-notes/public-corpus-20261116/selection-01/selected.jsonl \
  --roots /home/lucied/.cache/rag3db-banc-notes/public-corpus-20261116/selection-01/roots.json \
  --proofs /home/lucied/.cache/codeparsers-notes/language-corpus-01
python3 -B docs/codeparsers/16-11-2026/language-corpus/pilot.py run \
  --proofs /home/lucied/.cache/codeparsers-notes/language-corpus-01 \
  --allocation FRESH_C7_REFERENCE
```

Les deux placeholders sont remplacés par les valeurs réellement relues avant GO.
Le manifeste gel01 attendu est `6ab2b309e0365974d8ee59d6d1c442e7b2d78963659bfd20f225d83a764053b2`.
Le miroir PC est pris en charge par Banc à mêmes chemins pour préserver les gels.

## Delta avant allocation : erreurs de façade et gel complet

La source exemple `1b35cfb66ae160954981d7dd3a90f5ef88e95b01` corrige la
classification lue en revue : un Err du worker spécialisé ne remonte plus comme
validation_error globale. Il devient parser_error avec diagnostic et durée.
Un test de la conversion réellement appelée est ajouté ; NON JOUÉ avant Cargo.
Le pilote le joue explicitement (1 passé attendu, zéro ignoré), puis construit
l'ELF normal et lance la campagne dans la même borne globale de 1800s.

La première campagne vise désormais directement `selection-02` complète :
32382 fichiers, manifeste
`96c2e771fefc4cd0e204975feb8d1c73b1a66815cb1d3a1edc1b9002b94b644c`.
Remplacer les chemins selection-01 par selection-02 et le répertoire de preuves
language-corpus-01 par language-corpus-02 dans la commande initiale. Aucun fichier
du premier gel n'a été parsé ; il n'y a donc pas de campagne à rejouer.
Le coût complet reste à mesurer ; une échéance globale donne une preuve partielle,
pas une réussite des 32382 fichiers. Le miroir FULL est en cours chez Banc.

## Correction du lancement après rouge PQS

L'allocation C7-CORPUS-384-16nov a été consommée et arrêtée avant tout Cargo :
deux cwd du lanceur étaient encore sous source. La preuve rouge et sa cause
sont conservées au ticket parent, sans réactivation de cette allocation.
`test_pilot.py` appelle le vrai constructeur de `main(run)` en arrêtant juste
avant l'appel du superviseur : rouge sur cwd worktree, puis vert après déplacement
du pilote vers les preuves. Aucun faux reçu ni lancement d'autorité dans le test.
Le contrôle PQS lui-même reste strict et inchangé.

**Le shell appelant doit lui aussi être hors source.** La commande SSH suivante
est la forme de reprise ; ne pas faire `cd worktree` avant de lancer le pilote :

```sh
cd /home/lucied
python3 -B /home/lucied/git_workspaces/cx-codeparsers-language-corpus/docs/codeparsers/16-11-2026/language-corpus/pilot.py run \
  --proofs /home/lucied/.cache/codeparsers-notes/language-corpus-02-r2 \
  --allocation FRESH_C7_REFERENCE
```

Le profil r2 sera préparé sur la nouvelle source parent avec **le même parseur
1b35cfb**, corpus/roots inchangés, aucune nouvelle fonctionnalité de CLI. Le commit
877cb4a (modificateurs compacts) est poussé et conservé hors campagne selon C7 ;
les modificateurs seront donc « non conservés par cet outil », jamais « absents IR ».
Le coût/contrat des 32382 fichiers reste identique, budget proposé1800s neuf.
