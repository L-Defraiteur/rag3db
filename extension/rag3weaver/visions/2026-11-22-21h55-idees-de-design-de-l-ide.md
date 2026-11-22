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

---

## Les fonctions devenues des contraintes

*Ajout du 22 novembre, 22 h 45. Lucie : « identifie tout ce qu'on a fait comme fonctionnalité, mais qui s'est
transformé en contrainte d'UX plutôt qu'en expressivité latente, et enrichis le plan : comment les amener dans un IDE
non bloquant pour l'utilisateur. »*

Une fonction est **latente** quand elle agit sans rien demander, et qu'on la découvre le jour où on en a besoin. Elle
est devenue une **contrainte** quand il faut la connaître, la régler ou la nommer avant de pouvoir avancer. L'essai
du 22 au soir a buté sur sept d'entre elles en une heure.

### Six règles du non bloquant

1. **Rien à préparer avant la première phrase.** Aucun fichier à modifier, aucun commit, aucun serveur à lancer,
   aucun nom à donner.
2. **Une exigence devient une proposition.** Ce que le produit exige (une tâche, une racine, un prix, un domaine),
   l'agent le propose sur une carte, avec un bouton. Jamais un formulaire, jamais une commande à taper à côté.
3. **Le défaut est sûr, et il marche.** L'hôte choisit une valeur prudente et continue. Le réglage existe, il n'est
   pas un péage.
4. **Ce qui est long se fait en fond.** L'index, une repasse, un bâti : on continue de parler, et une ligne calme dit
   où ça en est.
5. **Un refus offre le geste.** Il dit en une phrase ce qui manque, et propose de le faire. Ou l'agent se corrige
   seul, sans montrer le refus.
6. **Jamais de panne muette.** Quand une protection ou une fonction n'est pas active (le juge éteint, une entrée non
   humaine), l'écran le dit.

Et la plomberie reste lisible, pour qui la cherche, dans « Avancé ».

Pour chaque fonction : ce qu'elle exige aujourd'hui, ce qu'elle devient, et son premier lot. « Vu » : vécu à l'essai
du 22 ou relevé par l'audit de recherche. « À vérifier » : je le déduis, sans l'avoir joué.

### I. Avant la première phrase

**1. La signature humaine.** Vu.
- *Exige* : lancer `rag3weaver-backend ide` dans un terminal pour obtenir une adresse à usage unique.
- *Correction d'archi (22 h 5x), plus grave que ce que j'avais écrit* : entrer autrement marche, et **un oui non
  humain accorde quand même** : la route d'accord ne lit pas qui répond. Un client qui tient le jeton de l'hôte, un
  agent compris, peut accepter une carte. Le lot `human-yes-only` passe avant tout le reste : un oui ne vaut accord
  que s'il est signé humain ; sinon la carte dit « pas accordé : seule ta réponse depuis ta session compte ».
- *Devient* : ouvrir l'IDE depuis sa propre session est humain d'office (un lanceur du bureau, un cookie qui dure).
  Si l'entrée n'est pas humaine, un bandeau le dit, avec « c'est bien moi ».
- *Premier lot* : le bandeau « tes réponses ne compteront pas », puis le lanceur. (archi)

**2. Ouvrir un projet.** Vu.
- *Exige* : taper un chemin absolu, sans `~`. Puis rien ne se passe. Un dossier créé par l'IDE n'a pas de commit, et
  la suite réclame `git commit --allow-empty`.
- *Devient* : on parle, et l'agent d'accueil propose le dossier (récents, dépôts trouvés, sélecteur). L'hôte fait le
  premier commit. Ouvrir mène au chat.
- *Premier lot* : en passe (`~`, ouvrir mène au chat, premier commit, sélecteur). (recherche)

**3. La tâche.** Vu.
- *Exige* : savoir que le chat d'un projet ne code pas, trouver « tâches », donner un nom, cliquer « Ouvrir », puis
  parler dans une autre page.
- *Devient* : la branche de travail isolée reste, mais elle s'ouvre seule à la première demande de code, nommée par
  l'hôte. Le mot « tâche » n'apparaît que dans la barre des agents. À la fin : « Garder » ou « Jeter ».
- *Premier lot* : `open_code_task` et la bascule dans le même fil. (archi, recherche)

**4. Le terminal.** Vu.
- *Exige* : `--repo` et `--task`. Tant que la tâche n'existe pas, il montre le fil d'un autre agent et refuse les
  réponses.
- *Devient* : `rag3weaver-tty` dans un dossier, et on parle. `--continue` reprend la dernière conversation.
- *Premier lot* : en cours (`cx-tty-task-only`, puis le départ sans argument). (tty)

**5. Un modèle branché.** À vérifier.
- *Exige* : un régime de modèle déclaré et des identifiants posés à la main. Une nouvelle venue n'en a pas, et
  l'agent d'accueil lui-même a besoin d'un modèle pour lui répondre.
- *Devient* : au premier lancement, un écran sans modèle (le seul) propose trois voies : un modèle local, s'il y en a
  un ; une clé d'un fournisseur ; notre inférence. Ensuite, changer de modèle se dit à l'agent.
- *Premier lot* : jouer une arrivée sur un poste neuf, sans rien, et écrire ce qui manque. (recherche, archi)

### II. Pour que l'agent ait le droit d'agir

**6. Le mode auto.** Vu.
- *Exige* : modifier `workspace.commands` dans un fichier de réglages, puis faire un commit dans un dépôt à part.
  Sans le commit, rien ne change, et rien ne le dit.
- *Devient* : le cadran d'autonomie (idée 12). Un projet neuf part sur « fais, et demande pour ce qui casse ou ce qui
  sort ». Changer est une carte.
- *Premier lot* : la posture par défaut d'un projet neuf, et la carte qui la change. (archi)

**7. Le juge.** Vu.
- *Exige* : un fichier `rules.json` posé à la main et un serveur de modèle lancé à la main, qui s'éteint à l'heure de
  son délai. Sans lui, toute commande jugée devient une carte, sans explication.
- *Devient* : un service de l'hôte, déclaré, démarré à la demande et arrêté au repos. S'il manque, la carte le dit :
  « le juge est éteint, je te demande donc ».
- *Premier lot* : la ligne « juge éteint » sur la carte ; puis le juge en service de l'hôte. (archi, optimiseur)

**8. Le bac, les racines, les dossiers protégés.** Vu.
- *Exige* : rien, quand ça marche. Mais un `git status` a été classé « protégé », et une carte ne disait pas sa
  commande.
- *Devient* : l'agent demande la racine du projet une fois, avec sa raison. La carte dit la commande, la racine, le
  mode, et l'avertissement d'une racine large.
- *Premier lot* : en passe (`task-root-not-protected`, les cartes de racine). (archi, recherche, tty)

**9. Les domaines du navigateur.** À vérifier.
- *Exige* : déclarer à la main les domaines que l'agent peut lire.
- *Devient* : une carte au premier besoin : « je veux lire `localhost:5173`, ton app. Pour cette conversation, ou
  toujours ? »
- *Premier lot* : la carte de classe navigateur. (archi)

**10. Le plafond et les prix.** Connu.
- *Exige* : un prix déclaré pour chaque modèle. Un modèle sans prix est refusé net.
- *Devient* : le refus offre le geste : « ce modèle n'a pas de prix. Je le cherche sur la page du fournisseur, ou tu
  le donnes ? » Le plafond approché se dit dans le fil, avant de bloquer.
- *Premier lot* : le refus d'un modèle sans prix devient une carte. (everything declarative)

### III. Pour que l'agent sache

**11. L'index.** Vu.
- *Exige* : comprendre « index : à jour », « catalogue tenu », « repasse de l'analyse », et trois avertissements en
  tête d'une recherche.
- *Devient* : l'agent propose l'index avec sa durée et travaille depuis le disque en attendant. Une repasse est une
  ligne calme, « je relis le projet, 40 % », jamais un pavé.
- *Premier lot* : les avertissements de la recherche repliés sous « comment j'ai cherché ». (recherche)

**12. La recherche.** Vu.
- *Exige* : lire « bm25 n°61 (11.020) · bm25_exact n°1 », cocher « reclasser ».
- *Devient* : « par le sens », « par les mots », « nom exact ». La relecture se déclenche seule sur une question en
  phrases. La recherche est d'abord un outil de l'agent ; sa page est dans « Avancé ».
- *Premier lot* : les étiquettes humaines des signaux. (recherche)

**13. Les vues.** Connu.
- *Exige* : ouvrir, choisir et fermer une vue pour lire le projet tel qu'une branche de travail le voit.
- *Devient* : une vue s'ouvre quand un agent travaille dans une branche, et se ferme à son atterrissage. Pour
  l'utilisatrice, ce sont les calques (idée 7).
- *Premier lot* : la vue d'une tâche ouverte et fermée par l'hôte. (arbre principal)

**14. La mémoire.** Connu.
- *Exige* : une trentaine d'outils (fiche, source, apport, sujet, ancrer, scinder, rejoindre…).
- *Devient* : trois gestes, « souviens-toi », « oublie », « qu'est-ce qu'on sait sur… ». L'archiviste range. Le
  carnet (idée 20) montre le reste.
- *Premier lot* : la puce « je note ça » et sa correction. (mémoire)

**15. Le service d'embarquement.** À vérifier.
- *Exige* : un service de vecteurs en marche, avec ses réglages de carte graphique.
- *Devient* : l'hôte le démarre à la demande, retombe sur un petit modèle sans carte graphique, et dit la vitesse
  attendue avant d'indexer.
- *Premier lot* : jouer un index sur un poste sans carte graphique. (embarquement)

### IV. Pour que le travail aboutisse

**16. Les déclarations.** Vu.
- *Exige* : rien, en principe. Mais l'agent du projet a présenté ses réglages comme les fichiers du projet, et a
  « proposé » un README de sa propre machinerie.
- *Devient* : les réglages sont un sujet à part, qu'on aborde en le demandant (« change ta façon de travailler »).
  Le graphe de l'agent se regarde par « voir comment il réfléchit ».
- *Premier lot* : en passe (`declarations-not-project`, `chat-project-first`). (archi)

**17. La preuve.** Vu.
- *Exige* : lire « Témoins », « Fusionner », « Équipier », « Boîte aux lettres » sur la page d'une tâche.
- *Devient* : l'agent trouve les tests du projet et propose de les lancer avant de finir. Un badge, « preuve : vu
  rouge, puis vert ». Sans tests, il le dit, sans rien exiger.
- *Premier lot* : les mots de la page d'une tâche (idée 13 : « Garder », « Jeter »). (recherche)

**18. Les commandes de fond.** Connu.
- *Exige* : pour l'agent, un délai, une borne, une seule commande par ligne.
- *Devient* : un refus de ce genre ne remonte jamais à l'utilisatrice : l'agent se corrige. Elle voit les tuiles
  « en cours », « prêt », « arrêtée après 10 min ».
- *Premier lot* : les tuiles des commandes de fond et des services. (tty, recherche)

### V. Pour que la machine tourne

**19. La pose de l'hôte.** Vu.
- *Exige* : attendre un hôte neuf pour avoir une nouveauté, une coupure des bases, une repasse, des clients à
  relancer.
- *Devient* : l'hôte se met à jour au repos et le dit en une ligne. Une conversation en cours n'est jamais coupée.
- *Premier lot* : la mise à jour au repos, sans coupure visible. (optimiseur)

**20. Plusieurs postes.** Connu.
- *Exige* : des options `ssh` et un tunnel pour parler à l'autre poste depuis le terminal.
- *Devient* : les lieux (idée 24). Un poste se déclare une fois, par son nom.
- *Premier lot* : `--host <nom>` au terminal, lu dans la liste des hôtes. (tty, optimiseur)

**21. La plomberie à l'accueil.** Vu.
- *Exige* : traverser les sessions, leurs numéros de processus, les orphelines, les gabarits locaux.
- *Devient* : « Avancé ».
- *Premier lot* : en cours (les orphelines hors du chat, l'accueil en chat). (recherche)

### Ce que ça change à l'ordre

Avant le socle des idées, un palier : **la première minute**. Tant qu'il n'est pas franchi, aucune idée ne se voit.

1. En passe ce soir : la tâche qui s'ouvre seule (3), le projet proposé (2), le terminal sans argument (4), les
   réglages qui ne passent plus pour le projet (16), `git status` sans carte (8).
2. À lancer ensuite : l'entrée humaine d'office et son bandeau (1), la posture par défaut et son cadran (6), le juge
   en service de l'hôte (7), l'arrivée sur un poste neuf, sans modèle (5).
3. Puis les refus qui offrent le geste : un prix manquant (10), un domaine (9), l'index (11).

Le critère : **une personne qui n'a jamais vu le produit lance une commande ou ouvre une page, écrit une demande de
code, et voit l'agent travailler puis lui demander un accord, sans rien avoir tapé d'autre.** Ce parcours se joue
dans le vrai navigateur et dans le vrai terminal, sur un poste neuf, avant de dire « prêt ».
