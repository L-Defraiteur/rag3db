# Le témoin de la vraie portée systemd du confineur échoue parfois sans verdict

- **État** : ouvert (20 novembre 2026, session recherche).
- **Gravité** : confort : un faux rouge de `scripts/test_confiner_validation.py`, qui rend à tort tout lot dont ce
  test est le témoin.
- **Atteignable en service** : non (un test des scripts du poste).
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/test_confiner_validation.py`).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Ce que c'est

`Montage.test_vraie_portee_systemd_crevee_dit_l_enveloppe` lance une vraie portée `systemd-run --user --scope`
plafonnée à 100 Mo, qui crève. Puis il lit `verdict.json` dans le dossier des preuves. À luciepc, le 20 novembre vers
19 h 30, il a échoué **une fois sur cinq sur master**, et une fois sur cinq avec le lot `confineur-grace` : le fichier
`verdict.json` n'existe pas quand le test le lit (`FileNotFoundError` dans `read_text`, l. 294). Ici, à la tablette,
il était vert.

## Pour le fermer

Lire, à l'échec, ce que contient le dossier des preuves. Soit la portée est arrêtée par systemd avant que le
confineur n'écrive son verdict (alors le test doit attendre la fin du confineur, et non lire tout de suite), soit le
dossier attendu n'est pas le bon. Le témoin est le test joué vingt fois de suite à luciepc, toujours vert.
