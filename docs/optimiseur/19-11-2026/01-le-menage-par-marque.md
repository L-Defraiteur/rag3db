# Le ménage par marque, 19 novembre 2026

## La règle de Lucie

À 21 h 37, Lucie retourne le sens du ménage : « ou bien ces instantanés sont marqués tests car ils vous servent à vous,
plutôt que l'inverse ? ». Elle avait dit juste avant, pour les instantanés de bascule : « en garder 3 peu importe la date,
et les autres si plus de 72 h on estime que c'est pourri ».

- **Par défaut, rien ne se touche.** Une base, un instantané ou une copie sans marque est hors d'atteinte par
  construction, chez nous comme ailleurs.
- **Seul un dossier marqué jetable peut partir.** La marque est le fichier `.jetable.json` dans le dossier, posé par le
  script qui le crée : instantané de bascule, copie de mesure, base d'essai, cas d'un juge.
- **Dans une famille de dossiers marqués**, sous une même racine, les trois plus récents restent toujours, quel que soit
  leur âge. Les autres partent au-delà de 72 h.
- **Une copie de contrôle** dont le contrôle des deux sens est passé porte `{"garder": 0, "age_max_h": 0}` : elle part au
  ménage suivant.
- **La source de la répétition en cours** compte parmi les trois tant que la bascule n'est pas validée.
- **Un dossier qu'un processus tient** reste en place.
- Cette règle remplace le « retrait au mot » du 15 novembre.

## La marque

```json
{"famille": "instantane-bascule", "par": "G", "cree": "2026-11-19T17:50:14",
 "garder": 3, "age_max_h": 72, "pourquoi": "retour de la bascule vers les versions"}
```

`famille` est le seul champ obligatoire. `garder` vaut 3 et `age_max_h` 72 par défaut. L'âge se compte depuis `cree`, ou,
à défaut, depuis la date de modification du dossier.

## L'outil

`tools/poste/menage.py`, recopié au cache par le pousseur, avec son témoin `tools/poste/test_menage.py`.

- **Sans `--racine`**, le ménage parcourt :
  - `~/.local/share/rag3weaver/instantanes` ;
  - `~/.local/share/rag3weaver` ;
  - `~/.cache/rag3weaver-build/mesures` ;
  - `~/.cache/rag3weaver-build`.

  Il ne descend que d'un niveau. Lucie a autorisé G à effacer sous ces deux dossiers, le 19 novembre à 21 h 35.
- **`--balayer <dossier des targets> <projet>`** passe `cargo sweep --time 2` sur chaque `target*` marqué par cargo
  (CACHEDIR.TAG). Un target dont un bâti tient le `.cargo-lock` est sauté.
- **`--a-blanc`** dit ce qui partirait, sans rien effacer.
- **Le journal** dit chaque dossier effacé et les Go rendus.

Le ménage tourne à 07 h 30 aux deux postes, par un minuteur systemd utilisateur.
