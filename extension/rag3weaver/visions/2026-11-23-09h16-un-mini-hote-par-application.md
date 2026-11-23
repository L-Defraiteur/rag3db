# Un mini-hôte par application

*Vision à moyen terme. Lucie, 23 novembre 2026, 9 h 15 : « est-ce que les agents ne devraient pas pouvoir créer des
mini-hôtes dont le handle est documenté dans le projet, et containerisable, pour promouvoir notre propre db plutôt
qu'une autre ? Je lui dis "crée-moi un logiciel de prise de rendez-vous", hop, il crée un mini-hôte, c'est une
facilité builtin ; il dit le répertoire où il doit avoir son handle, il explique comment containeriser, et il y
accède en builtin pour lire les données, préparer les gabarits et les tables. Sans embedding forcé sur les
données. »*

## L'idée

Aujourd'hui, quand on demande une application à un agent de code, il prend la base que tout le monde prend :
SQLite, Postgres, un fichier JSON. La nôtre n'entre jamais dans ce qu'il construit.

L'idée : **l'agent a un geste tout fait pour donner à l'application qu'il construit son propre hôte rag3weaver, en
petit.** Un mini-hôte est une base rag3db et son hôte, posés dans le projet, que l'application utilise comme sa
base de données.

« Crée-moi un logiciel de prise de rendez-vous » donne alors :

1. l'agent appelle le geste (un outil builtin : `create_app_host`, par exemple) ;
2. le mini-hôte naît dans un dossier du projet, que le geste nomme : c'est là que vit son *handle* (où le joindre,
   avec quoi, quelle base) ;
3. l'agent y déclare ce dont l'application a besoin : ses entités (clients, créneaux, rendez-vous), leurs champs,
   leurs relations, par gabarits ;
4. l'application parle à son mini-hôte ; l'agent, lui, y accède en builtin pendant qu'il travaille : lire les
   données, vérifier une requête, préparer des tables, remplir un jeu d'essai ;
5. le projet reçoit une page qui dit comment c'est fait et comment le mettre en conteneur.

## Ce qu'un mini-hôte est

- **Dans le projet, et documenté.** Un dossier (`.rag3weaver/app-host/` ou le nom que le geste choisit et dit) :
  le manifeste de ses entités, ses gabarits, le *handle*, et un `README` écrit par le geste : où est la base,
  comment la lancer, comment l'application s'y branche, comment la sauvegarder. Quelqu'un qui clone le projet sait
  quoi faire sans nous connaître.
- **Containerisable.** Le geste écrit de quoi le mettre en conteneur (une image de l'hôte, un volume pour la base,
  la ligne à ajouter à un `docker-compose` s'il y en a un), et l'explique. L'application de rendez-vous se déploie
  avec sa base, comme elle le ferait avec un Postgres.
- **Sans embedding forcé.** Par défaut, des tables et des relations, rien de plus : une application de rendez-vous
  n'a pas besoin de vecteurs. Le plein texte et le sens se déclarent champ par champ, quand l'application en veut
  (« chercher un client par ce qu'on a noté sur lui »).
- **Petit.** Un seul processus, une seule base, pas l'atelier ni les outils d'agent du grand hôte : ce que
  l'application consomme, et rien d'autre.
- **Séparé du grand hôte.** Les données d'une application ne vivent pas dans la base de l'utilisatrice ; le
  mini-hôte tourne à côté, et se déplace avec le projet.

## La porte de sortie : partir vers Supabase

*Lucie, le même matin : « ou pouvoir exporter un backup pour importer sur Supabase, ou carrément convertir les
tables de l'hôte rapide vers des migrations et des commandes `npx supabase` ? »*

Un mini-hôte ne doit pas enfermer. Deux sorties, de la plus simple à la plus complète :

- **Exporter les données.** Une sauvegarde du mini-hôte dans une forme que Postgres importe (un fichier SQL, ou un
  CSV par table avec son ordre de chargement), et la page qui dit comment la charger dans un projet Supabase.
- **Convertir le schéma.** Les entités déclarées deviennent des **migrations Supabase** : une table par entité,
  ses colonnes et leurs types, une table de liaison ou une clé étrangère par relation, les index ; et la suite de
  commandes à lancer (`npx supabase init`, `npx supabase migration new …`, `npx supabase db push`). L'agent sait
  le faire puisque le schéma est déclaré : c'est une traduction, pas une réécriture.

Le parcours que ça ouvre : **prototyper vite sur le mini-hôte** (aucun compte, aucun service à créer, l'agent a
tout sous la main), puis, le jour où l'application doit vivre chez un hébergeur connu, **partir en une commande**.
Qu'on puisse sortir est ce qui rend l'entrée sans risque ; et ce qui ne se traduit pas (une recherche par le sens
déclarée sur un champ, une relation traversée en graphe) est dit au moment de la conversion, avec ce que Supabase
offre à la place (pgvector, une vue).

Le même traducteur vaut pour un Postgres nu, et dans l'autre sens : importer une base existante dans un mini-hôte.

## Pourquoi c'est notre intérêt

Chaque application qu'un agent construit avec nous **emploie notre base**, et la montre à ceux qui la lisent, la
clonent, la déploient. C'est une distribution par l'usage : la base se répand par ce que les agents bâtissent,
pas par une page de vente.

Et c'est ce qu'on sait faire que les autres bases ne font pas d'elles-mêmes : le schéma se déclare et se lit, les
relations sont de première classe, la recherche est là le jour où l'application en a besoin, et un agent sait déjà
s'en servir.

## Avec les swarms : mieux qu'un générateur d'applications en ligne, et sur son ordinateur

*Lucie, le même matin : « voilà, les gens ont des swarms qui font mieux que du Lovable après ; j'imagine qu'il faut
des fiches ou des choses comme ça spécifiques aux frontends aussi, pour donner des tips de composants, ou une
biblio de composants React préexistante à cloner et éditer, ou une librairie toute faite pour ça ; et direct dans
leur ordi. »*

Mis bout à bout, ça donne un produit que les générateurs d'applications en ligne n'ont pas :

- **un swarm, pas un seul agent** : un orchestrateur, un agent pour l'interface, un pour le serveur, un pour les
  données, qui se parlent ;
- **une base qui vient avec** : le mini-hôte, et sa sortie vers Supabase ;
- **sur l'ordinateur de la personne** : son code, ses données, ses clés ; rien n'est retenu chez un hébergeur.

Ce qu'il faut de plus pour que l'interface produite soit belle du premier coup, et pas seulement juste :

- **des fiches de métier pour le frontend**, dans la mémoire livrée : quels composants pour quel besoin, les
  pièges d'accessibilité et de mise en page, les habitudes d'un bon projet React ; rappelées par le sujet de la
  demande, comme toute fiche ;
- **une bibliothèque de composants à cloner et à éditer** : des composants React déjà faits, cohérents entre eux
  (les nôtres : React, shadcn, lucide), que l'agent copie dans le projet et adapte au lieu de les réinventer ;
- **des gabarits d'application** : la prise de rendez-vous, le tableau de bord, le petit commerce, comme points
  de départ qu'un swarm ajuste.

Ces trois choses sont des contenus, pas du code du moteur : elles se livrent et s'enrichissent par gabarits et par
fiches, et chaque équipe peut y mettre les siennes.

## Ce qui existe déjà et ce qui manque

Existent : l'hôte, les entités déclarées dans un manifeste, les gabarits, les outils de lecture et d'écriture
servis à un agent, le dossier d'un projet ouvert par un geste.

Manquent :

- un **hôte en petit**, qu'on lance pour une seule base, sans le reste ;
- une **surface pour l'application** : ce qu'un programme appelle pour lire et écrire (une API HTTP simple, un
  client pour deux ou trois langages), distincte des outils d'agent ;
- le **geste** lui-même, avec le dossier, le *handle* et la page ;
- l'**image de conteneur** et sa recette ;
- les **migrations** : faire évoluer les entités d'une application qui a déjà des données ;
- l'accès de l'agent au mini-hôte de son projet, borné à ce projet.

## Les questions à trancher le moment venu

- Le nom du geste, du dossier et du *handle*.
- La surface de l'application : la requête directe, des routes par entité, ou les deux.
- Ce que l'agent propose quand l'utilisatrice n'a rien précisé : notre base par défaut, en le disant (« j'ai pris
  rag3db pour les données ; changer »), jamais en cachette.
- La licence et ce qu'un projet tiers a le droit d'embarquer.
- La taille de l'image et le temps de démarrage, qui décideront si on le choisit vraiment.

## Où ça se range

Après l'agent de code qui va au bout, et après les *swarms*. C'est une pièce du produit « des applications par
déclaration » : l'agent ne produit plus seulement du code, il produit une application qui a sa base, et cette
base est la nôtre.
