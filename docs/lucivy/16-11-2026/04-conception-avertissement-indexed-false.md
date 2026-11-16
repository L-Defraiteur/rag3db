# Avertir d’indexed:false ignoré — contrat proposé, avant correctif

Ticket : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
Porteur lucivy. Entrée4 indépendante du lotulp : handle.rs, query.rs, READMEcore
et test_text_field_indexed_false.rs seulement. Aucun changement du BM25, du
format d’index, des options de recherche ni de l’effet existant des déclarations.

## Capacité publique bornée

API existante dans la préparation :
`lucivy_core::handle::schema_warnings(&SchemaConfig) -> Vec<String>`.
Module handle déjà public : pas de nouvelle couche, registre ni option.
Fonction pure sur les déclarations, sans I/O/index/corpus/scheduler ; ordre des
champs conservé. Un message pour chaque champ text/string explicitement false,
aucun pour true/omis ou les numériques qui honorent déjà leur option.
La fonction renseigne ; elle ne valide pas et ne réécrit pas la configuration.

Au correctif APRÈS rouge : build_schema commence par config.validate(), puis
appelle cette même fonction et affiche chaque message via eprintln!("Warning: …").
Pas de logger, dépendance, cacheglobal ni flag neuf. Le nom du champ sera rendu
par formatageDebug pour échapper guillemets et retours à la ligne et garder un
message d’une ligne ; messages en anglais. Un message par champ **par appel de
construction de schéma** : une création multishard construit plusieurs schémas,
pas de promesse de déduplication globale à vie. Aucun avertissement à chaque
requête ni lecture du document.

La doc de FieldDef::indexed dans query.rs et celle des champs du READMEcore
diront explicitement que false n’est pas honoré pour text/string, ces champs
restant indexés ; numériques inchangés. Honorer false serait un autre contrat,
réservé à la décision de Lucie, pas un comportement introduit dans ce lot.

## Preuve préparée, NON jouée

API scaffold renvoyant[] pour permettre un rouge d’assertion, pas un simple
symbole manquant. Témoin1pureconfig exige exactement deux messages pour
bodytextfalse et labelstringfalse ; true/omis/u64/i64/f64false sont les contrôles.
Chaînes attendues préparées en littéraux bruts Rust ; aucune compilation lancée.

Prochain créneau admis : poste lourd, targetprivé, j8maximum/deuxcœurslibres,
test_text_field_indexed_false exact, sans corpus ni index. Voir le rouge avant
implémenter warnings/call/doc ; puis même test et suitecore complète dont le
corpus reste une dépendance à traiter séparément, pas un motif de fauxvert.
Aucun lourd maintenant : essais/verdict G puis coupure ne sont pas déclarésfinis.
