// archive_reglages — le record de la Source, plus les réglages de la décision, pour que chaque
// section les porte (SplitSections recopie les champs de son entrée sur chaque section).
// Contournement dit (page 06, 7 nov. 2026) : un graphe cité qui déclare d'autres paramètres que
// `element` est refusé au chargement (ticket du 7 nov.) ; ses réglages voyagent donc par l'élément.
// input : #{ candidates, allow_create, instruction, search_options, value: le record de la Source }
let r = input.value;
r.candidates = input.candidates;
r.allow_create = input.allow_create;
r.instruction = input.instruction;
r.search_options = input.search_options;
r
