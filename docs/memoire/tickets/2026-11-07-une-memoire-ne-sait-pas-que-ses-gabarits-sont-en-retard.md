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

## Reprise et dépendances au 11 novembre

Porteur : session mémoire. La [page de conception M8](../../../extension/rag3weaver/docs/09-novembre-2026-memoire-longue/02-m8-la-montee-des-gabarits.md) attend la relecture d’everything declarative ; aucun lot de code n’est ouvert par cette régularisation. Le rapport du 11 novembre décrit aussi une remise documentaire de l’ancien worktree `wt-mf-96` à relire avant de la reprendre : ne pas l’appliquer aveuglément ni confondre cet état historique avec un travail déjà livré.

Le correctif archive `41526b29a` ne monte pas la configuration réelle de Lucie. [L’archivage à blanc restant](2026-11-11-terminer-l-archivage-a-blanc-des-documents-sur-copie.md) est séparé ; [le raccord automatique M9](2026-11-11-raccorder-l-archiviste-automatique-au-chat.md) ne l’active pas non plus. Après relecture, préciser adoption/fusion/migration et leurs refus, poser les témoins puis la passe avant de demander un tour de code. La montée réelle garde sa décision explicite et son instantané.

## Relecture du 12 novembre 2026

**État : ouvert ; porteur mémoire.** La [revue ED sourcée](../12-11-2026/01-relecture-m8-adoption-fusion-et-reprise.md)
distingue constats, recommandations et choix non tranchés. Aucun lot de code M8 n'est ouvert.
La conception courante est révisée : identité Gmail, distinction du suivi vivant et de l'épingle,
base historique vérifiée, fusion avec conflits et maintenance/reprise par contrats communs.

À trancher avant un lot : politique vivant/épinglé, conservation des bases, qualification des
transitions et exclusion/reprise multi-ressources. Première tranche proposée : plan à blanc,
fusion pure et validation en recouvrement ; aucun apply ni livre écrit. Le diagnostic 1b n'est
pas une application de migration livrée. Les critères de fermeture restent ceux du ticket :
implémentation et témoins livrés, puis montée réelle distinctement autorisée et prouvée.
Aucune base réelle modifiée, aucun test relancé ; Gemini rouge inchangé.
