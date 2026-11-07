# SessionStart — les blocs du hook de Claude Code

Un gabarit déclaré, relu à chaque appel du hook (`~/.config/rag3weaver/hooks/SessionStart.md`,
posé depuis `templates/hooks/` par `rag3weaver-backend hook --poser-les-gabarits`) : corriger un
texte ici suffit, aucune session ne redémarre. Une section `## <clé>` par bloc ; les trous
`{rubriques}`, `{id}`, `{genre}` sont remplis par le hook. Ce qui précède la première section est
un commentaire.

## sans_fiche

Aucune fiche d'identité ne cite cette session. Si tu as une clé (ton nom court, celui que les autres sessions emploient), écris ton L1 `handle` : `put_apport` {fiche: <clé>, source: "sessions", section: "handle", texte: "session:{id}", niveau: "L1", via: "claude-code", author: <clé>} puis `relier_apport` {links: [{from: {name: <clé>}, to: {fiche: <clé>, source: "sessions", section: "handle"}}]} et `citer_source` {genre: "handle", valeur: "session:{id}", fiche: {fiche: <clé>, source: "sessions", section: "handle"}}{genre}. Si tu es nouvelle, crée d'abord ta fiche d'identité : `put_fiche` {name: <clé>, about: <une ligne>, state: "current", author: <clé>}. Passe toujours `author` = ta clé (M7 n'est pas là pour le faire). Rien n'est créé à ta place.

## session_compactee

Ta session vient d'être compactée. Au premier tour, avant toute autre chose, écris ton apport `identity` sur ta fiche d'identité : un seul L1 de sept lignes, une rubrique par ligne ({rubriques}), par `put_apport` (section "identity", via "claude-code", author = ta clé), puis `relier_apport` : c'est à toi de dire qui tu es, pas au harnais.

## compaction_l0_seul

La dernière compaction est rangée en L0 seul (aucune fiche de tâche ne te cite) : nomme ta tâche (`put_fiche`, puis `fiche:<nom>` dans la rubrique `doing` de ton identity) et le prochain résumé s'y rangera.

## sans_identity

- Aucun apport `identity` sur ta fiche : écris-en un, sept lignes, une rubrique par ligne ({rubriques}), `put_apport` section "identity", via "claude-code", author = ta clé, puis `relier_apport`.
