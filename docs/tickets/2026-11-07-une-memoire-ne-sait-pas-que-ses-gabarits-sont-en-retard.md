# Une mémoire ne sait pas que ses gabarits sont en retard

- **État** : ouvert, le 7 novembre 2026. Forme tranchée par l'orchestration sur le mot de Lucie
  (« pense le plus cool et scalable ») ; pour la session mémoire (96), lot M8, après le pas 3 de M4.
- **Gravité** : blocage. Un backend ouvert sur une copie de `templates/` ne reçoit jamais les
  outils ajoutés au gabarit ensuite, et rien ne le dit.
- **Atteignable en service** : oui. La mémoire de Lucie (`~/.config/rag3weaver/memoire-longue/`)
  est une copie du 25 octobre 2026 au soir : `whois`, `select_sources`, `select_apports` et bientôt
  `archive` n'y sont pas.
- **Touche rag3weaver** : oui (le manifeste, l'hôte, la ligne de commande).
- **Famille** : produit

## Ce que c'est

Le dossier de configuration d'un backend est une copie de `templates/` faite à la main, sans
version : l'hôte ne peut pas dire qu'il est en retard sur les gabarits de son binaire, et la mise à
jour est une recopie à la main (la recette du 25 octobre, page 05 de la mémoire longue).

## Ce qu'il faut (la forme tranchée)

- Le manifeste d'un backend **épingle la version de ses gabarits** : `templates.version`, le commit
  ou l'empreinte du dossier `templates/` copié.
- À l'ouverture, l'hôte compare à `templates/` du binaire et dit, au journal et dans
  `daemon_status`, « gabarits en retard : +N outils (…), −M (…) », **sans rien changer**.
- `rag3weaver-backend templates upgrade <backend>` fait la montée : montre le diff des outils, ferme
  le backend par l'hôte, prend l'instantané, copie `templates/` à neuf (en gardant les champs
  propres au backend : `database`, `files_root`, `vector_extension`, `embeddings.address`), pose la
  nouvelle version, rouvre, vérifie (`index_state`, `verify_files`), et écrit la ligne dans la page
  du jour. Le même geste pour cent mémoires ; un geste par mémoire, rien d'implicite.

## Le témoin

Une config épinglée sur une version plus vieille que le gabarit : l'ouverture dit le retard et sert
les outils d'avant ; `templates upgrade` : les outils neufs sont servis, la base est intacte champ à
champ, l'instantané existe, les champs propres sont gardés.

## Pour le fermer

Le lot M8 sur master avec son témoin ; puis la mémoire de Lucie montée une fois par la commande, au
mot de l'orchestration, quand `archive` est sur master.
