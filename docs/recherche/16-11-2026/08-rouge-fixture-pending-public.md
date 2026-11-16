# Rouge de fixture : le mapping public n’est pas admissible

Admission distincte C7, source préfixe `1f816fd3b366380a1f7d034a540cfbbc2aee7bd7`, pilote `c306291a78158d3925eb57d5b2f18eed5fb7438a`. Un seul juge : `catalog::tests::existing_fts_pending_public_entity_refuses_its_physical_target`.

Compilation réussie29.058014s, puis résultat0passé/1échoué/0ignoré,1851filtrés. **La panne arrive dans la fixture AVANT l’appel à open_existing_fts_snapshot**, à resolve_search_target :

```
SchemaError("Document : main doit être une dérivée par contenu de cette cible")
```

Le prérequis rouge pending N’EST PAS levé. Aucun retry, aucune autre fonction, aucun vert a92e lancé. Ce rouge ne réfute ni ne qualifie le fix pending.

Diagnostic RO : `main` construit par make_test_config est une dérivée historique identityRoot ; ajouter seulement Document.search_index=main ne satisfait pas validate_search_index_context. Il faut une vraie DerivedIdentity::Content, fromDocument, render non vide et champs cherchables compatibles source/cible ; changer uniquement identity pourrait encore laisser un champ `content` absent côté source. Future fixture doit valider DerivedConfig ET validate_search_index_context avant de résoudre, puis préparer main99 et Document41 et tester la marque pendingDocument sans pendingmain. Les autres juges utilisant ce mapping doivent être corrigés aussi. Cette page précède toute correction de fixture ; aucune compilation supplémentaire.

SuperviseurPID90292/start117654519, délai1200 de1176545.24028511 à1177745.24028511 ; elapsed29.544531s. Cgroup réel max34359738368/highmax/swap0/**cpu500000/100000**, quota par place sans override800. Reprise du target par garde commune réelle, marker désormais W préfixe, ancien W187 et preuves conservés. Magasin réel/pin/so+vectorSHA contrôlés.

Verdict communROUGE/code101/erreur=null/restes[]/nettoye=true/reçu terminé. Poste libre0/4 après fin. Target gardé1593823256octets, ELF SHAe7a9e6d90a25ca3ea02b2b31ba4bd4d40e787942ce8fcf3bc43202556ae9d5f0.

Archives PC/local `~/.cache/rag3weaver-build/cx-archive-recherche-fts-pending-red-v1` :26fichiers identiques après réhash local, manifeste SHA256 `19c174ee157c7720299fba7560005fcb4b777572f6b18b25b66c2a315ea2bcdb`. Tous logs bruts, commandes, profils, inventaires/ledger, reçu/recette/enfants/bus/verdict et contrôleurs conservés. Aucun service/modèle/7970/base de Lucie ouvert.
