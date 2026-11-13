# run_e2e exige un plafond de 32 Gio, poste en donne 40 par défaut

- **État** : ouvert (13 novembre 2026), relevé par archi pendant la passe du lot du README ; question
  pour l'orchestration.
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
