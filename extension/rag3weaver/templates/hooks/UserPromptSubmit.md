# UserPromptSubmit — les blocs du hook de Claude Code

Relu à chaque appel ; une section `## <clé>` par bloc. Trous : `{rubriques}`, `{fiche}` (la
fiche d'identité trouvée, ou la consigne du handle), `{contexte}` et `{plafond}` (en milliers de
jetons), `{comment}` (d'où vient la mesure).

## defi_identity

Ta session a été compactée. Avant toute autre chose, écris ton apport `identity` sur ta fiche d'identité{fiche} : un seul L1 de sept lignes, une rubrique par ligne ({rubriques}), par `put_apport` (section "identity", via "claude-code", author = ta clé), puis `relier_apport`.

## rappel_identity

Rappel, une fois : ton apport `identity` n'est pas écrit depuis la compaction. Écris-le maintenant ({rubriques}).

## suggestion_contexte

Ton contexte est à ~{contexte} k jetons sur {plafond} k ({comment}) : range maintenant ce qui doit l'être — un apport, un plan clos — plutôt que d'attendre la compaction.
