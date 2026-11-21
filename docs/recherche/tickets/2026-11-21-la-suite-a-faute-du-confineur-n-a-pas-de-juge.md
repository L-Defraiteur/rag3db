# La suite à faute injectée du confineur n'a pas de juge

- **État** : ouvert (21 novembre 2026, session recherche, à la demande de c7).
- **Gravité** : un témoin qui ne se joue jamais : la vraie portée crevée (`test_vraie_portee_systemd_crevee_dit_l_enveloppe`)
  ne vit plus que dans `scripts/test_confiner_faute_injectee.py`, que rien ne lance.
- **Atteignable en service** : non (outillage des passes).
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/`, l'étape d'une passe).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : c7.

## Le pas exact

Le lot `recherche-confineur-faute-injectee` (21 nov.) sort la vraie portée systemd (MemoryMax=100M, un vrai oom_kill)
du fichier ordinaire des témoins du confineur, après deux jeux du fichier entier à la tablette sous une session
graphique (un oom_kill à 05 h 43 min 17 s). La nouvelle suite saute ce témoin en le disant sous une session graphique
active (`loginctl`). Mais aucune passe ne la joue : le témoin ne prouve plus rien.

## Pour le fermer

Une étape de `passe_epinglee` à luciepc, après les e2e, qui joue `scripts/test_confiner_faute_injectee.py` :
- sous une session graphique active, un SKIP dit, compté « non jugé » dans le résumé de la passe, jamais vert par
  défaut, et la passe le dit en première ligne ;
- le pousseur ne la joue pas (il tourne pendant que Lucie travaille sur seat1).

Une étape, un SKIP dit, un compteur ; rien de plus.
