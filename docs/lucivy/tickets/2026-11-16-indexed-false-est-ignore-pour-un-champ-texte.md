# `indexed: false` est ignoré pour un champ texte

- **État** : ouvert (16 novembre 2026). Noté le 14 novembre au ticket du poids du plein texte, jamais ouvert à part.
- **Gravité** : faible, mais silencieuse. Un appelant qui déclare un champ texte `indexed: false`, pour le stocker
  sans le chercher, paie quand même son index entier (dictionnaire, postings, positions). Et rien ne le lui dit.
- **Atteignable en service** : seulement si un manifeste ou un appelant pose `indexed: false` sur un champ `text`.
  Aucun manifeste de rag3weaver ne le fait aujourd'hui.
- **Touche rag3weaver** : non.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

`lucivy_core/src/handle.rs`, `build_schema`, branche `"text" =>` : le champ reçoit toujours
`TextFieldIndexing` (tokenizer `RAW_TOKENIZER`, `WithFreqs` en v3). Seul `stored` est lu, jamais `indexed`. Les
types `u64`, `i64` et `f64` lisent bien `indexed`. La branche `"string"` l'ignore aussi : elle pose toujours
`STRING`, donc un champ indexé. L'avertissement et la doc valent pour les deux.

## Ce qu'il faut faire (ce lot-ci)

- Un **avertissement** à la création, une fois par champ concerné, quand un champ `text` ou `string` porte
  `indexed: Some(false)` : `eprintln!("Warning: …")`, la forme que `handle.rs` emploie déjà (l. 231) ; ne pas ajouter de dépendance. En
  anglais, par exemple `field "<name>": indexed=false is not supported for text and string fields; the field is indexed`.
- **La doc** : la doc du champ `indexed` (`lucivy_core/src/query.rs:93`) et la section des champs de `lucivy_core/README.md` disent que `indexed: false` n'a pas d'effet sur un
  champ texte.
- **Ne pas** honorer `indexed: false` dans ce lot. Un champ texte stocké sans index change ce que les requêtes
  `contains` et les spans peuvent faire sur lui. C'est un choix de Lucie, à lui présenter à part, avec ce que ça
  coûte et ce que ça casse.

## Comment le prouver

- Rouge d'abord, dans un test neuf `lucivy_core/tests/test_text_field_indexed_false.rs` : créer un index avec un
  champ `text` à `indexed: false`, et exiger que la création le signale. Le moyen le plus simple de l'observer : une
  fonction pure qui rend les avertissements du schéma (`schema_warnings(&SchemaConfig) -> Vec<String>`), appelée
  par `build_schema` et testée directement. Rouge tant qu'elle n'existe pas ou rend une liste vide.
- Vert ensuite, puis `cargo test --release -p lucivy-core` en entier.

## Préparation du témoin, 16 novembre

Porteur lucivy ; branche poussée `cx-lucivy-indexed-false-warning`, tête
`07ccd12e484ea20f4ae85d23639f9da7316acbfd`, base main a1c0dfd.
Worktree portable `/home/lucied/git_workspaces/cx-lucivy-indexed-false-warning`.
Deux fichiers : handle.rs (API scaffold renvoyant liste vide, pas de correctif),
`tests/test_text_field_indexed_false.rs` (text/string false doivent avertir,
true/omis et champs numériques ne doivent pas ajouter d’avertissement).
**NON compilé, NON joué, rouge pas encore observé.** Aucun appel de cette API
ajouté à build_schema ; aucune sémantique d’indexation modifiée.
Prochain pas après créneau CPU : voir ce témoin réellement rouge avant de
remplir schema_warnings, l’appeler et compléter query.rs/README. Ne pas confondre
préparation du rouge et preuve exécutée. Aucun lourd pendant fenêtre G/Arbre.

Conception brève avant correctif :
`docs/lucivy/16-11-2026/04-conception-avertissement-indexed-false.md`.
APIpurepublique dans handle, avertissementparchamp/parappelbuild_schema après
validate ; pas de déduplication globale, de nouveau flag, ni d’effet d’indexation
modifié. Noms échappés, texte anglais ; docquery/README à compléter aprèsrouge.
Préparation du témoin corrigée en littéraux bruts Rust ; toujours NON compilé,
NON joué, aucun correctif tant que rouge d’assertion pas vu. Fenêtre G/Arbre
respectée, aucun nouveau lourd lancé.

Recette bornée publiée pour RO puis GOslot :
`docs/lucivy/16-11-2026/05-recette-rouge-indexed-false.md` et pilote
`outils/qualify-indexed-false-red.py`, sourcefixe0c795f99, budgetproposé600s après
vraiposte/32G-HIGHmax/j4, compilationoffline+inventaire+exact1configpure.
Rougeassertion attendu distingué compilation/inventaire/confinement ; aucun
lancement effectué. Corpus7.2/PR42 restent deux blocages distincts noncontournés.

Suite RO C7 : chemins concrets/PQS et4hashes copiés préparés sur luciepc,
précontrôle `preuves/indexed-warning-preflight-v2.json`, targetABSENT et P/scope
ABSENTS, Qneuf UID1000 et nonce de provenance. Pilote refuse les composants
symlink sansresolve, créeT/provenance exclusivement et abaisse j4 au quotaCPU
hiérarchique réel. Admission600 aprèsvraiposte avec END conservé, commande
littérale sansplaceholder dans recette05. Supportd395 copies privées, aucun
cache déployé modifié. Aucun Cargo/build/test joué ; attend admissionexactefinale.

## Rouge exécuté le 16 novembre, source0c795f99

GO exact C7 APP0c795f99/pilote86989d7ab5/Qv2nonce3552903efab590b44f44cd8002f7b028.
Compilationoffline réussie (release1m02s), exécutableSHA
`c2478e972de2295a79c41cd651461653f38706e02cccd2ceace45c49fe33b01b`,
inventaireexact1 puis UNtest : **0passé,1échec,0ignoré**, code101 à l’assertion
left:[] contre deuxmessages body/label. Ce n’est ni une erreur de compilation ni
un refus de confinement. first-red.json : expected_assertion_red_observed.

[Brut](../16-11-2026/preuves/indexed-warning-red-0c795/logs/test.log),
[manifeste](../16-11-2026/preuves/indexed-warning-red-0c795/SHA256SUMS),
[quiescence](../16-11-2026/preuves/indexed-warning-red-0c795/quiescence.json).
Portée ROUGEcode1/erreur=null/restes[]/nettoyétrue, autoritéterminée ; scanPID/start
vide et sourcepropre. END1180583.723555907, observationfinale1180124.091534846,
aucunreset. Quotaeffectif5CPU→j4,32G/HIGHmax. Target crééexclusivement, provenance
nonce/UID/source ; aucune adoption. Pas de retry, de corpus, de libcore entière,
de service ni de baseLucie. Rouge conservé avant tout correctif.
Reste déjàprévu : implémenter schema_warnings/call/doc selonconception04, puis
validationverte sur source/pilote/portée frais admis ; aucune réactivation duGOrouge.
