# Un index échoué laisse sa session de synchronisation ouverte

- **État** : ouvert le 23 novembre 2026, sur un essai de recherche à la tablette. Il revient aux propriétaires de la synchronisation des couches et de ses sessions, à départager.
- **Famille** : produit
- **Gravité** : après un seul échec d'indexation, un dépôt ne s'indexe plus, même l'hôte relancé ; rien ne dit à l'utilisatrice quoi faire.
- **Atteignable en service** : oui, constaté sur l'hôte de service de la tablette.
- **Touche rag3weaver** : oui (la synchronisation par `Scope` et ses sessions, l'outil `index`).

## Ce qui s'est passé

1. Le 23 novembre à 05 h 4x, l'index d'un dépôt de 2 183 fichiers ouvert par `/dossiers/ouvrir` échoue : « IO exception: Cannot open file /tmp/rag3weaver-liens-…-ScopeContent_CHUNKED_FROM-….csv: Too many open files — le moteur a refusé le COPY … la base doit être rouverte ». La cause de cet échec-là est corrigée depuis 06 h 07 : l'unité de l'hôte a `LimitNOFILE=65536`, au lieu d'une limite douce de 1 024.
2. L'hôte est relancé par une pose (pid 169094). À 08 h 1x, l'outil `index` (`confirm: true`) sur le même dépôt est refusé : « synchronisation refusée : Scope : une session est déjà ouverte sur ce périmètre ('1795408619025-649aaa963fdd', ouverte depuis 2 h 36 min) ; une seule à la fois — la finir, l'abandonner, ou takeover pour la reprendre ».
3. L'outil `index` n'offre ni « abandonner » ni « takeover ». La recherche sert alors un index partiel, sans le dire.

Ce ticket porte sur la session laissée ouverte, pas sur les descripteurs de fichiers.

## Ce qu'il faut

- Un échec de synchronisation ferme ou abandonne sa session `Scope`, sur tous les chemins d'erreur, y compris un COPY refusé par le moteur.
- Une session orpheline, dont le processus qui l'a ouverte n'existe plus (hôte relancé), est reprise ou abandonnée d'elle-même au prochain index, et cela se dit dans le journal.
- Quand un index part sur un dépôt dont la synchronisation a échoué, l'outil `index` le dit, ainsi que ce qu'il fait.
- La recherche sur un index partiel le signale.
- Les témoins sont rouges d'abord. Un index qui échoue au milieu d'un COPY (faute injectée), puis un second index sur le même dépôt, après une relance, doivent finir par « indexation terminée ».

## Pour reproduire

Le dépôt de l'essai est `~/essai6-sans-historique/d`, à la tablette (manifeste `code-d`), laissé tel quel. Les journaux sont dans `~/.cache/rag3weaver/commandes/index/` : `1795408618967-3804001-3.out` pour l'échec, `1795418002284-169094-1.out` pour le refus.
