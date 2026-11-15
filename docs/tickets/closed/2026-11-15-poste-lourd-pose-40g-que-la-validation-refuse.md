# `poste lourd` pose par défaut un MemoryMax de 40G que la validation refuse

- **État** : fait (15 novembre 2026, branche poste-plafond-validation, voie 2 au mot de c7) : poste refuse avant la file, code 2, une validation sous un plafond au-dessus des 32 Gio du confineur, et dit « lancer sous POSTE_MEM_MAX=32G » ; le défaut de 40G ne descendra (voie 1) qu après le relevé du pic d un bâti de la lib à -j8 et d un banc, avec le cœur
- **Famille** : produit
- **Gravité** : une passe attend sa place, parfois longtemps, puis rougit en une seconde sur une règle d'outillage,
  sans avoir joué un seul test. Le même soir, deux sessions (archi et hotfixs) sont tombées dans ce piège.
- **Atteignable en service** : non (outillage des passes)
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/confiner_validation.py`) et l'outil `poste`
- **Pour** : G

## Ce qu'on voit

Sans `POSTE_MEM_MAX`, `poste` pose `MEM="${POSTE_MEM_MAX:-40G}"` sur le scope de la commande. La validation de
`run_e2e.sh`, `scripts/confiner_validation.py`, exige un plafond effectif d'au plus 32 Gio
(`MAX_MEMOIRE = 32 * 1024**3`), et refuse sinon :
`ROUGE validation : aucun plafond effectif <= 32 Gio`. Relevé à luciepc le 15 nov., à 21 h 55 : le lot
extension-non-chargee-dite (053cdad28) a attendu 5 min 23 s en file, puis il a rougi en 0,18 s. Les sessions qui le
savent lancent la passe sous `POSTE_MEM_MAX=32G`. Les autres l'apprennent par ce rouge.

## Pour fermer

L'un des deux, au choix de G :

1. le défaut de `poste lourd` devient le seuil de la validation (32G), avec une seule source pour la valeur ;
2. ou `poste` refuse avant la file une commande dont la validation refusera le plafond, et le dit.

Un témoin de script : une passe sans `POSTE_MEM_MAX` ne rougit plus sur le plafond.
