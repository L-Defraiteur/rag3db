# Un disque presque plein ne retient aucune passe, et rien ne le dit avant les gels

- **État** : ouvert (21 novembre 2026, écrit par G à la demande de c7)
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : `poste`, le relevé de charge)
- **Gravité** : haute. Le 21 novembre, le disque de la tablette est tombé à 4,6 Go libres ; l'écran de Lucie a gelé.
  c7 a effacé 21 targets libres, et un target de G (39 Go) tenait encore une passe.
- **Atteignable en service** : oui, le poste entier
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/poste`, `tools/poste/poste_load.py`)

## Ce que c'est

Chaque passe bâtit dans un target de 10 à 40 Go. `poste` admet une passe sur ses places et sa mémoire, jamais sur le
disque ; le relevé de charge ne porte pas l'espace libre. Rien ne prévient avant que le disque soit plein.

## Ce qu'il faut

- `poste cpu|gpu|lourd` refuse une passe neuve quand le disque de `~/.cache` a moins de N Go libres (50 par défaut,
  un fichier du dossier des verrous pour le régler), en le disant, avant la file ;
- le relevé de charge (`poste-load.tsv`) porte l'espace libre de ce disque ;
- témoin : un faux `df` sous le seuil refuse, au-dessus admet.
