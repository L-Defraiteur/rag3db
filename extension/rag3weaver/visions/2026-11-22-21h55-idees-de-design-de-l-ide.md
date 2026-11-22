# Idées de design pour l'IDE, champ libre

*22 novembre 2026, 21 h 55. Lucie : « construis-nous des idées de design depuis les capacités de notre produit ; si tu
avais champ libre, que ferais-tu d'ingénieux et de cool ? Elles seront prioritaires pour la suite. »*

Cette page ne refait pas l'audit de recherche (page 04 du 22) ni la page de l'agent d'accueil d'archi. Elle part de ce
que le produit sait faire, et propose ce qu'on en montrerait.

## Le cadre

Les deux règles de Lucie :

1. l'IDE est simple à utiliser, mais propose des choses très avancées, et met le produit en valeur ;
2. quelqu'un qui n'y connaît rien arrive, parle à un agent, et ça commence pour lui, sans chemin ni réglage à taper.

L'interface principale, ce sont des agents. Le chat est au centre. L'agent fait surgir des composants dans le fil,
toujours par des gabarits déclarés (`show(component, data)`). L'agent d'accueil est la page d'accueil.

## Le principe : ce que nous seuls savons faire doit se voir

Trois choses que les autres outils à agents n'ont pas ensemble :

- **un graphe vivant du code**, avec une vue par branche de travail, suivie à l'événement près ;
- **une mémoire signée**, où chaque fiche dit quel agent l'a écrite et d'où elle vient ;
- **une autonomie gouvernée et chiffrée** : la porte, ses cartes, le juge, le registre des dépenses.

Chaque idée ci-dessous rend l'une des trois visible, par l'agent, dans le fil.

Et un seul mécanisme tient les deux règles ensemble : **le fil fait surgir, le bord garde.** Tout ce qui surgit dans
le chat peut s'épingler. L'écran d'un débutant est vide, donc simple. Celui d'un habitué est la somme de ce qu'il a
épinglé, donc avancé. Personne ne règle rien.

Pour chaque idée : ce qu'on voit, sur quoi elle s'appuie (existe, en cours, à construire), et le premier pas.

---

## A. Le socle

### 1. L'accueil qui sait

- **Ce qu'on voit** : l'agent d'accueil n'ouvre pas sur « comment puis-je aider ? ». Il dit ce qui a bougé depuis ta
  dernière visite : « Depuis hier 23 h : deux tâches finies, une t'attend pour un accord, 1,40 $ dépensés ce mois sur
  200, ton projet est à jour. » Chaque morceau est un lien. En dessous, trois gestes : « Reprendre où j'en étais »,
  « Commencer quelque chose », « Montre-moi ».
- **S'appuie sur** : l'état de la machine, la boîte des tâches, le registre, la mémoire (existent) ; la date de la
  dernière visite (à garder par l'hôte).
- **Premier pas** : un gabarit `briefing`, rempli par `machine_state`.

### 2. Épingler : le tableau de bord se construit en parlant

- **Ce qu'on voit** : chaque composant surgi dans le fil (tableau, jauge, plan, carte) porte une épingle. Épinglé, il
  passe dans le bord droit et reste vivant. On peut aussi le dire : « garde-moi ça sous les yeux ». Le bord se range,
  se replie, se vide.
- **S'appuie sur** : les gabarits de composants (en cours chez archi) ; leur rafraîchissement et le bord par
  utilisateur (à construire).
- **Premier pas** : un champ `refresh` dans le schéma d'un gabarit, et un outil typé `pin`.

### 3. Le travail raconté, pas déversé

- **Ce qu'on voit** : l'agent qui travaille laisse des lignes calmes : « a lu 4 fichiers », « a lancé les tests :
  12 s, verts », « a modifié `total.ts` (+18, −3) ». Chaque ligne se déplie sur le détail. Aucun nom d'outil, aucune
  milliseconde, aucun JSON au premier niveau.
- **S'appuie sur** : la boîte de la tâche et ses lignes signées (existent).
- **Premier pas** : un gabarit `activity` qui regroupe les appels d'un pas en phrases.

### 4. Une demande, une équipe, un devis

- **Ce qu'on voit** : « j'aimerais qu'on regarde ce dépôt » rend une carte : ce que l'agent va faire (cloner, indexer,
  créer un agent de code), où, avec quel modèle, et **pour combien environ**. « Vas-y » ou « Change ». Pour une grosse
  demande, il propose une petite équipe (un qui cartographie, un qui écrit, un qui relit), avec le devis par membre.
- **S'appuie sur** : le registre et ses prix datés, les clés d'agents stables, `estimate` pour l'index (existent) ;
  l'estimation d'une tâche (à construire : la moyenne des tâches du même genre, sinon une fourchette large, dite
  comme telle).
- **Premier pas** : le devis d'un index et d'un clone sur la carte de l'agent d'accueil.

### 5. Le plan partagé

- **Ce qu'on voit** : le plan de l'agent, en tête du chat, comme une liste cochée, le pas en cours allumé. On peut
  rayer un pas, en ajouter un, en remonter un. L'agent le prend au pas suivant.
- **S'appuie sur** : les plans de l'équipier et la signature humaine (existent) ; la modification d'un élément par
  l'utilisatrice (à construire).
- **Premier pas** : un gabarit `plan` en lecture, épinglé d'office dans une tâche.

### 6. Les mots du produit, gardés par un témoin

- **Ce qu'on voit** : jamais « backend », « manifeste », « nœud », « ReadFileNode », « bm25 », « reflink » à l'écran.
  Les signaux de recherche se disent « par le sens », « par les mots », « nom exact ». Une erreur tient en une phrase
  humaine et une action.
- **S'appuie sur** : la garde des identifiants du pousseur (existe) : la même idée, pour l'écran.
- **Premier pas** : une liste de mots de plomberie interdits à l'écran, et un témoin navigateur qui balaie chaque
  page et rougit s'il en voit un hors de « Avancé ».

---

## B. Montrer le graphe

### 7. Les calques

- **Ce qu'on voit** : une pile de calques, comme dans un logiciel de dessin. « Le projet », puis un calque par branche
  de travail d'agent. On coche un calque : le code, la recherche et la carte montrent le projet tel que cet agent le
  voit, ses changements en surbrillance. Deux calques cochés : leurs différences, et leurs conflits avant qu'ils
  existent.
- **S'appuie sur** : un index par dépôt, des vues par worktree, leurs diffs, le paramètre `view` de chaque outil
  (existent). C'est notre architecture même.
- **Premier pas** : un sélecteur de calque sur la page d'un fichier et sur la recherche.

### 8. Le rayon d'impact sur chaque carte

- **Ce qu'on voit** : avant de fusionner ou d'accorder, la carte dit ce que ça touche : « 3 fonctions, appelées depuis
  14 endroits, dans 5 fichiers ; 2 n'ont pas de test ». Un petit graphe, un clic pour la liste. Pour une destruction :
  ce qui serait perdu et qui n'existe nulle part ailleurs.
- **S'appuie sur** : `impact`, `usages`, `include_members`, les positions des occurrences (existent).
- **Premier pas** : un gabarit `impact` sur la carte de fusion d'une tâche.

### 9. Les portes : chaque affirmation mène à sa preuve

- **Ce qu'on voit** : un nom de symbole, un fichier, une fiche, une tâche ou un agent cité dans une réponse devient
  une porte, seulement s'il se résout. Au survol, un aperçu : la signature, la doc, le nombre d'usages, le dernier
  agent à y avoir touché, la fiche liée. Au clic, on y est. Ce qui ne se résout pas reste du texte.
- **S'appuie sur** : la résolution des symboles cités et la mémoire signée (existent) ; `POST /ide/chemins` (en cours).
- **Premier pas** : étendre la résolution des chemins aux symboles, aux fiches, aux tâches et aux agents.

### 10. La carte vivante du projet

- **Ce qu'on voit** : une carte zoomable (dossiers, fichiers, fonctions) où l'on voit les agents travailler : une zone
  lue luit faiblement, une zone écrite fort, un test passe au vert ou au rouge. Une couleur par agent. Un clic sur une
  zone préremplit « explique-moi ce coin ».
- **S'appuie sur** : le graphe et les suiveurs des vues (existent) ; le flux d'activité par zone (à construire).
- **Premier pas** : la carte statique d'un dépôt, puis les événements.

---

## C. Montrer la confiance

### 11. Tes mots font loi

- **Ce qu'on voit** : une page « Ce que j'ai autorisé ». Chaque accord y est **ta propre phrase, citée**, avec sa date,
  ce qu'elle a ouvert, à quel agent, et combien de fois elle a servi. Un clic sur la phrase la retire. Une décision du
  juge montre la phrase sur laquelle il s'est appuyé.
- **S'appuie sur** : la porte garde la citation et le tour comme preuve ; les lignes signées (existent).
- **Premier pas** : un gabarit `grants`, lu dans les racines accordées et la boîte.

### 12. Le cadran d'autonomie

- **Ce qu'on voit** : par agent, trois positions en mots simples : « Demande-moi tout », « Fais, et demande pour ce
  qui casse ou ce qui sort », « Ici tu as carte blanche » (un dossier, nommé). Sous chaque position, trois exemples
  concrets. Changer de position est une carte, pas un fichier à modifier.
- **S'appuie sur** : les classes de la porte, le bac, les déclarations versionnées (existent). Ce soir, passer en mode
  auto demande un commit à la main dans un dépôt de réglages : c'est ce que cette idée supprime.
- **Premier pas** : l'agent d'accueil fait le changement et son commit, sur une carte.

### 13. Tout se défait

- **Ce qu'on voit** : la promesse écrite en haut d'une tâche : « rien ne touche ton projet avant ton oui ». Sur chaque
  pas qui a changé des fichiers, « Annuler ce pas ». À la fin, « Garder » ou « Jeter », jamais « Fusionner » en
  premier.
- **S'appuie sur** : une tâche vit sur sa branche et dans son arbre ; la fusion et le rejet (existent) ; l'annulation
  d'un seul pas (à construire).
- **Premier pas** : renommer et réordonner les boutons de la page d'une tâche.

### 14. La jauge de dépense

- **Ce qu'on voit** : chaque tâche et chaque agent ont une petite jauge. « Pas plus de 2 $ là-dessus » devient un
  budget. À 80 %, l'agent le dit et demande. Le mois entier est dans le point de l'accueil.
- **S'appuie sur** : le registre, sa réservation avant l'appel, `usage_report`, les alertes (existent) ; le budget par
  tâche (à construire).
- **Premier pas** : la dépense d'une tâche sur sa page.

---

## D. Montrer le travail

### 15. L'app à côté

- **Ce qu'on voit** : quand un service de l'agent est prêt sur un port, une tuile « ton app tourne ». Un clic l'ouvre
  dans un panneau à côté du chat. L'agent modifie, le rechargement à chaud met la page à jour sous tes yeux. Et
  l'agent lit la même page (son arbre d'accessibilité, sa console) : il voit l'erreur que tu vois.
- **S'appuie sur** : les services (en cours chez tty), les outils de navigateur de l'équipier (existent).
- **Premier pas** : la tuile d'un service, avec son lien ; puis le panneau.

### 16. Montre du doigt

- **Ce qu'on voit** : on sélectionne trois lignes de code, ou on clique un élément de l'app à côté, et on écrit
  « ça ». La sélection part avec le message, comme une référence exacte.
- **S'appuie sur** : les positions `fichier:ligne:colonne`, les `[ref=…]` de l'arbre d'accessibilité (existent).
- **Premier pas** : une sélection de code devient une puce dans la saisie.

### 17. Remonter le temps, et repartir d'un point

- **Ce qu'on voit** : une frise sous la tâche. On la fait glisser : la conversation, le plan et les fichiers
  reviennent à ce moment. « Repartir d'ici » ouvre une autre branche de travail avec une autre consigne.
- **S'appuie sur** : les générations des vues, le journal des runs, la boîte signée (existent) ; le départ d'une tâche
  depuis une génération (à vérifier).
- **Premier pas** : la frise en lecture seule.

### 18. Le duel

- **Ce qu'on voit** : « essaie avec deux modèles ». Deux branches de travail partent sur la même consigne. Une carte
  les met côte à côte : le diff, les tests, le temps, le prix. « Je garde celui-ci. »
- **S'appuie sur** : les tâches isolées, les calques, le registre (existent).
- **Premier pas** : dupliquer une tâche sous un autre modèle.

### 19. La piste d'atterrissage

- **Ce qu'on voit** : sur un projet à plusieurs agents, une bande : ce qui attend, ce qui se vérifie, ce qui a
  atterri. Un badge : « preuve : vu rouge, puis vert ». La promesse « rien n'atterrit sans preuve » se voit.
- **S'appuie sur** : notre propre façon de travailler, la file des lots. À offrir au produit.
- **Premier pas** : la fusion d'une tâche passe par une vérification déclarée (les tests du projet), et la montre.

---

## E. Montrer la mémoire

### 20. Le carnet du projet

- **Ce qu'on voit** : la mémoire comme un carnet qu'on feuillette : des sujets, des fiches en cartes, chacune avec le
  signe de l'agent qui l'a écrite et ses sources. En tête : « ce que le projet a appris cette semaine ».
- **S'appuie sur** : fiches, sources, apports, signatures, miroirs (existent).
- **Premier pas** : le gabarit `memory_cards` du catalogue, en page entière.

### 21. « Je note ça »

- **Ce qu'on voit** : quand un agent écrit en mémoire, une puce discrète dans le fil. Un clic ouvre la fiche. « Non,
  c'est faux » la corrige, sous ta signature. « Souviens-toi que… » et « oublie… » sont des gestes du chat.
- **S'appuie sur** : l'écriture signée par l'hôte, les apports humains (existent).
- **Premier pas** : la puce, avec son lien.

### 22. Le retour d'absence

- **Ce qu'on voit** : on rouvre une tâche après une pause : trois lignes, « là où on en était », et « ce que j'ai fait
  pendant ton absence ».
- **S'appuie sur** : les résumés de l'archiviste (existent).
- **Premier pas** : un gabarit `recap` à la réouverture d'un chat.

---

## F. Le confort

### 23. La palette

- **Ce qu'on voit** : Ctrl+K, un seul champ : le code, la mémoire, les tâches, les agents, les actions. Chaque
  résultat a son geste. C'est toute la barre latérale d'un habitué, dans une touche.
- **Premier pas** : la palette sur les projets, les agents et les tâches.

### 24. Les lieux

- **Ce qu'on voit** : les postes sont des lieux (« ici », « le gros PC »). On voit où tourne chaque agent, et si le
  lieu est occupé. « Envoie ça sur le gros PC. »
- **S'appuie sur** : le client à plusieurs hôtes, `use_host`, le verrou du poste (existent).
- **Premier pas** : le lieu de chaque agent dans la barre gauche.

### 25. La voix

- **Ce qu'on voit** : on parle au lieu de taper. Une commande de fond qui finit le dit à voix courte : « tes tests
  sont verts ».
- **S'appuie sur** : la graine de la voix sur le chat (à construire).

### 26. Le premier quart d'heure

- **Ce qu'on voit** : « Montre-moi » lance une vraie tâche sur un petit projet jouet. En cinq minutes, la personne
  voit une carte d'accord, une note de mémoire, l'app à côté. La démonstration est le produit, pas une vidéo.
- **Premier pas** : un projet jouet livré avec l'IDE, et un parcours écrit pour l'agent d'accueil.

### 27. L'identité : la forge

- **Ce qu'on voit** : on garde le noir et le rouge de braise. L'activité se lit comme une braise : plus un agent
  travaille, plus sa pastille rougeoie ; ce qui a atterri refroidit en une teinte calme. Un signe par agent. Un thème
  clair pour qui le veut.

---

## Par quoi commencer

L'audit de recherche fixe l'ordre des réparations. Pour les idées, je prendrais cet ordre :

1. **Le socle** : l'accueil qui sait (1), le travail raconté (3), les mots du produit avec leur témoin (6). Ce sont
   eux qui font qu'un débutant reste.
2. **Épingler (2) et le plan partagé (5)**, dès que le catalogue de gabarits et `show` ont atterri.
3. **La carte d'accord enrichie** : le rayon d'impact (8), tes mots font loi (11), tout se défait (13). C'est la
   sûreté du produit, rendue belle.
4. **L'app à côté (15)**, avec les services de tty : c'est la démonstration la plus parlante.
5. **Les calques (7)** : notre différence la plus difficile à copier.
6. Puis le devis (4), la jauge (14), les portes (9), le carnet (20), le duel (18).
7. Le reste, au fil des lots.

## Comment juger une idée livrée

Trois questions, posées dans le vrai navigateur, en jouant le parcours d'une nouvelle venue :

1. sait-elle quoi faire sans rien lire ?
2. y a-t-il un mot de plomberie, ou un champ à taper ?
3. voit-on quelque chose que seul ce produit peut montrer ?

Un écran qui ne passe pas les deux premières n'est pas prêt. Un écran qui ne passe pas la troisième est prêt, mais
ne vend rien.

---

## Corrections après relecture (archi et recherche, 22 h 2x)

Les deux sessions qui tiennent le code ont relu la page. Ce qui était faux, ou plus dur qu'écrit :

- **2, épingler** : pas de champ `refresh` dans un gabarit. Un composant épinglé garde **sa source** (`{tool, args}`)
  pour se remplir de nouveau, et seuls les outils **en lecture** peuvent nourrir un composant vivant ; sinon une
  épingle relancerait une écriture. Chaque gabarit déclare les sujets du flux qu'il écoute, ce qui le rend vivant sans
  sondage. Le bord se replie sur un écran étroit.
- **3, le travail raconté** : les appels d'outils d'un pas sont au journal des runs, pas dans la boîte. `activity` lit
  le journal.
- **6, les mots du produit** : le témoin balaie l'habillage de l'IDE, pas le texte du modèle ni les blocs de code,
  sinon il rougirait au hasard d'une réponse.
- **7, les calques** : « les conflits avant qu'ils existent » se calcule à la demande (un `merge-tree` entre deux
  branches), pas en continu.
- **11, tes mots font loi** : trois choses manquent. Quand le juge laisse passer une commande, **rien ne l'écrit**
  aujourd'hui : il faut une ligne `command_granted` (par le juge, avec la citation). Le nombre d'usages d'un accord
  n'est compté nulle part. « Un clic retire » marche pour une racine (`revoke_root`), mais un accord de ligne vit dans
  la garde de la tâche, sans route pour le retirer.
- **12, le cadran** : sous le mode d'approbation des déclarations, le changement passe par `propose`, puis `apply`
  après le oui, sur la carte.
- **13, annuler un pas, et 17, repartir d'un point** : ils supposent qu'un pas d'équipier commite sur la branche de sa
  tâche, ce qui n'est pas fait. Une génération de vue est un état de l'index, pas un commit : repartir d'une
  génération ne rendrait pas les fichiers. Le chemin : un commit par pas, puis repartir d'un commit.
- **1, l'accueil qui sait** : la dernière visite se tient par le cookie de l'entrée de l'utilisatrice.
- **10, la carte vivante** : la plus lourde, à garder pour la fin.

Deux lots en sortent, à faire tôt parce qu'ils servent la sûreté autant que l'écran : **la ligne `command_granted`**
(une commande passée par le juge laisse une trace signée) et **un commit par pas d'équipier**.

L'ordre retenu avec recherche : d'abord les réparations de son audit, avec (6), (3), les boutons de (13) et (1)
fondus dedans ; puis (2) et (5) après `show` ; puis (8) et (11) sur la carte d'accord ; puis (15) ; puis (7).
