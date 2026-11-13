# run_e2e exige un plafond de 32 Gio, poste en donne 40 par défaut

- **État** : fermé (13 novembre 2026) sur `4b4233853` de G : la règle de c7 est en tête de « La
  passe » dans `docs/procedures/push-d-un-lot.md` (32 Gio pour toute validation, 40 Gio par défaut
  de `poste` pour les bâtis).
- **Famille** : produit
- **Gravité** : passe refusée d'entrée. `run_e2e.sh` rougit « aucun plafond effectif <= 32 Gio » sous un
  `poste lourd` sans réglage.
- **Atteignable en service** : non (l'outillage des passes).
- **Touche rag3weaver** : oui (`scripts/confiner_validation.py`, `MAX_MEMOIRE`) ; et le script
  `~/.cache/rag3weaver-build/poste` (`MEM="${POSTE_MEM_MAX:-40G}"`), hors dépôt.

## Ce que c'est

`533924ee6` (12 novembre 2026, « Partager la portée confinée des validations ») a posé
`MAX_MEMOIRE = 32 * 1024**3` dans `scripts/confiner_validation.py`. La procédure
(`docs/procedures/push-d-un-lot.md`) dit de poser `POSTE_MEM_MAX=32G` avant `poste`. Mais
`poste` plafonne toujours à 40 Gio par défaut, et toute passe lancée comme avant le 12 novembre
rougit.

## Ce qu'il faut pour le fermer

Une seule valeur, décidée par l'orchestration : `poste` à 32 Gio par défaut, ou `MAX_MEMOIRE` à
40 Gio, avec la raison. Puis le script, la procédure et ce ticket accordés.

## La décision (c7, 13 novembre 2026)

- **32 Gio pour toute validation** : c'est l'invariant du confineur (`MAX_MEMOIRE`), il ne change pas.
- **40 Gio reste le défaut de `poste`**, pour les bâtis.
- Les appelants posent `POSTE_MEM_MAX=32G` devant un `poste lourd … run_e2e.sh`, ou appellent
  `run_e2e.sh` seul.
- G l'écrit dans la procédure ; ce ticket se ferme quand elle le dit.
