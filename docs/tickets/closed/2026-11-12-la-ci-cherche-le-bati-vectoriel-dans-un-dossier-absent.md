# La CI cherche le bâti vectoriel dans un dossier absent

- **État** : corrigé 567d2de50, fixture workflow d03ef6fc2 ; rouge reproduit puis 20 fixtures CPU vertes aux deux postes, revue favorable de l’orchestration ; clôture livrée avec le lot d’intégration coordonné, CI réelle non rejouée.
- **Famille** : produit
- **Gravité** : CI rag3weaver bloquée avant l'exécution des tests unitaires.
- **Atteignable en service** : non, workflow de validation.
- **Touche rag3weaver** : oui, .github/workflows/rag3weaver-workflow.yml.
- **Pour** : optimiseur, coordination orchestration pour l'intégration.
- **Signalé par** : orchestration, vérification de la CI après l'intégration du lot A.
- **Ouvert le** : 12 novembre 2026

## Preuve relue, sans rerun

[Run rag3weaver43287259623](https://github.com/L-Defraiteur/rag3db/actions/runs/43287259623),
source6dd116e8f70fd328acf3b45a89401e06f71365cb, job131426574887,
étape8 « L'extension vector, du même bâti », échec le12novembre19:46:35UTC.
Le connecteur fournit les métadonnées du job ; le journal a été lu via gh API,
conservé avec une copie sans séquences ANSI dans
`~/.cache/orchestration-notes/ci-12nov-2103/`.

La commande cherche
`extension/rag3weaver/target/debug/build/rag3db-*/out/build` par ls/mtime.
Le glob ne trouve rien ; CMake reçoit un dossier vide après -B :

```text
ls: cannot access 'extension/rag3weaver/target/debug/build/rag3db-*/out/build': No such file or directory
CMake Error: No build directory specified for -B
```

Le même journal montre la compilation de rag3db0.11.1 depuis tools/rust_api,
puis un exécutable de tests sous
`target/debug/build/rag3weaver/024bb70f754e3a61/out/`.
Cela est un indice de disposition différente, pas la preuve du chemin CMake
effectif. Ne pas remplacer le glob par un autre chemin deviné ni choisir un
cache ancien seulement par sa date. Aucun défaut du code A ou des index réels
n'est établi par cet échec antérieur à leur intégration.

## Travail borné et fin attendue

Relire le workflow et le producteur Cargo/CMake réel ; identifier le dossier
du **même** moteur que celui lié aux tests. Composer les informations existantes
(notamment artefacts et build-script-executed Cargo si pertinents), avec refus
nommé quand la provenance est absente, périmée ou ambiguë. Publier un diff limité
au workflow et à l'éventuel helper commun nécessaire ; préserver source moteur,
configuration, ABI et extension produite, sans nouvelle lib posée sur les postes.

Éprouver la sélection et les refus par fixtures CPU représentatives, à partir
des faits du producteur observé ; ne pas écrire de témoin qui valide seulement
le nouveau glob choisi. Aucun rebâti C++, Cargo complet, service, modèle ou GPU
nécessaire au diagnostic initial. Premier rouge inattendu : arrêt et diagnostic,
pas de rejeu inchangé. Tout besoin d'une validation lourde se coordonne après
le diff ; la CI automatique future jugera le commit intégré. Aucun rerun des
anciens jobs ni publication de paquet ou modification d'identifiants.

Fin : dossier sélectionné avec provenance explicite du même bâti, refus des
cas absents/ambigus/périmés, diff et preuves relus puis intégrés. Le ticket se
clôture avec ce verdict, sans présenter une validation CPU comme une CI verte.

## Diagnostic du producteur et recette figée

`tools/rust_api/build.rs::build_bundled_cmake` utilise `cmake::Config` sans
remplacer son out_dir. La dépendance verrouillée cmake 0.1.57 (source locale
du registre relue) prend `OUT_DIR` et configure `OUT_DIR/build` ; le producteur
annonce `cargo:rustc-link-search=native=<OUT_DIR>/build/src` et une liaison
statique rag3db. Ce contrat, et non la forme des dossiers de Cargo, identifie
le bâti. Le journal CI original a aussi été relu dans le cache annoncé.

Format Cargo de référence :
https://doc.rust-lang.org/cargo/reference/external-tools.html#json-messages
(`build-script-executed` fournit out_dir et linked_paths même sur cache valide ;
`compiler-artifact` décrit le profil test et l’exécutable ; build-finished clôt
l’invocation). Le package_id est repris exactement depuis cargo metadata.

Diff borné publié avant essais : workflow et deux scripts Python, aucune
source moteur/Rust hôte. La commande no-run conserve les mêmes features,
capture metadata et messages JSON, précédés du contexte source/commit/recette.
Le sélecteur refuse contexte périmé, invocation incomplète ou concaténée,
OUT_DIR absent/ambigu, lien natif incohérent, cache CMake déplacé ou étranger,
archive ou artefacts absents. Aucun glob et aucune comparaison de mtime.
Une preuve des chemins et empreintes est émise dans le journal CI futur.

Recette de validation CPU prévue après publication de cette source :

```bash
WORKFLOW_ANCIEN=<copie workflow avant correctif> python3 extension/rag3weaver/scripts/test_bati_vector_ci.py Provenance.test_out_dir_nouveaux_dossiers_et_artefact_cache_valide
# Rouge attendu du vrai ancien sélecteur ls, sans exécuter CMake.
python3 extension/rag3weaver/scripts/test_bati_vector_ci.py -v
```

Fixtures : disposition OUT_DIR imbriquée et disposition historique, cache
leurre plus récent, mêmes package_id de metadata, chemin natif et cache du
producteur, artefacts de l’unique invocation. Refus absents/ambigus/périmés et
échec Cargo ; aucun C++, Cargo complet, GPU ou service lancé localement.

## Vérifications réellement jouées

Sources/recette publiées en 567d2de50 avant les premiers essais ; fixture des
deux vrais blocs shell publiée en d03ef6fc2 avant son exécution. Empreintes
des trois fichiers figées avant les tests et relues après. Seuls workflow et
scripts de validation changent ; aucune source Rust hôte ni C++ modifiée.

Rouge reproduit au portable et à luciepc : le vrai ancien sélecteur ls,
extrait du workflow conservé, rend code 2 sur une fixture OUT_DIR imbriquée ;
le test échoue sur `No such file or directory` / sa traduction française.
Aucune ligne CMake du workflow ancien n’est exécutée. Sur luciepc, le contrôle
initial du journal attendait le texte anglais : il a été arrêté et diagnostiqué
(journal français), puis la preuve déjà produite a été vérifiée par code 2,
chemin littéral et assertion, sans relancer le rouge inchangé.

**20 fixtures CPU vertes aux deux postes**, aucune sautée ; luciepc sous
`poste lourd`. La fixture d’intégration exécute les blocs shell réels du
workflow et le vrai CLI du sélecteur ; seuls les producteurs Cargo/CMake sont
factices, avec leurs messages et cache de fixture. Elle vérifie le même -B
et le même --build, sans compilation ni chargement d’extension.

Preuves aux deux postes : `~/.cache/optimiseur-notes/ci-vector-12nov/`
(`workflow-ancien.yml`, `rouge.log`, `fixtures.log`, `sources-avant-tests.sha256`).
Sources exactes conservées à luciepc sous le sous-dossier `source/`.
Contrôles de tickets et diff --check également verts.

Aucun ancien job relancé, aucun Cargo/CMake complet réel lancé, aucune nouvelle
lib posée et aucun service/GPU sollicité. Les fixtures prouvent la sélection
et ses refus, **pas une CI verte** : la prochaine CI automatique du commit
intégré doit encore juger son environnement réel et les tests unitaires.

## Revue et clôture du correctif

L’orchestration a relu le helper, les vingt fixtures et les blocs réels du
workflow, sans objection concrète : provenance metadata/package_id,
invocation JSON unique, out_dir et liaison native, cache CMake, archive et
artefact de test. Cette revue est distincte du verdict du prochain job CI.

Empreintes des trois fichiers éprouvés, concordantes aux deux postes :

- workflow : `b4b452aa3d43f0651846644c20f4e09db1fb792fc1358a8d7661041bb81e4621` ;
- helper : `4c35df44df54373ee52aef3b05101ec090d7e38feb41209864c878e509e34997` ;
- fixtures : `2e85b443a3f52ea9dfeba9609cce24fbfd88eabd4522804e31b6783e89fdc662`.

La clôture ne modifie aucun de ces octets. Dernier fetch de préparation :
master 82e812764, inchangé depuis la base du lot ; rebase sans changement
sur la branche neuve `cx-optimiseur-ci-vector-2`. Aucun rejeu Cargo/C++ ni
nouvelle exécution de fixture inchangée nécessaire pour ce pas documentaire.
Ticket déplacé dans closed avec cet État et les preuves dans le même commit.
