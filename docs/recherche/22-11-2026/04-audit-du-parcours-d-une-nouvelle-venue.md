# Audit : le parcours d'une nouvelle venue

22 novembre 2026, vers 22 h 30, session recherche, à la demande de Lucie (22 h 15 : « jauger ce qui perd un
utilisateur, ce qui ne semble pas correct pour un humain ; il arrive, il parle à un agent et ça commence des choses pour
lui »). Joué dans le Chrome de Lucie (extension claude-in-chrome, un onglet neuf), sur l'hôte posé à la tablette, comme
une personne qui ne connaît rien : arriver, ouvrir un projet, demander une vraie tâche de code, trancher une carte. Les
captures sont sous `audit/`, dans l'ordre du parcours.

**Où le parcours s'arrête** : aux deux cartes d'accord de la tâche `readme`. Les accorder revient à Lucie, pas à un agent
(le garde-fou l'a refusé, à juste titre) : elles l'attendent sur `/ide/code/31199c08b7e8/taches/readme`.

## Bloquant : la personne ne peut pas aller plus loin seule

1. **L'agent du dossier ouvert parle de la machinerie, pas du projet** (captures 04, 05). Dans le chat d'un dossier neuf,
   « crée un README.md qui explique ce projet » : l'agent appelle `list_declarations` et `read_declaration`, répond
   « Ce projet est un backend déclaratif de code pour Rag3weaver… », puis fait une « proposition #1 » qui réécrit un
   README… des manifestes internes (« Le backend de code — un moteur, deux politiques… »). Il ajoute « Sur ce backend
   déclaratif, les modifications ne sont pas directement écrites ». C'est la racine du « tu te fous de moi là ? » de
   Lucie. → archi, `chat-project-first` (l'invite part du dossier ; une demande de changement ouvre la tâche).
2. **Un dossier créé par l'IDE ne peut pas recevoir de tâche** (capture 08). « Ouvrir une tâche » répond : « le dépôt
   n'a encore aucun commit : une tâche part d'un commit. Faites-en un (`git -C … commit --allow-empty -m début`), puis
   rouvrez la tâche ». L'IDE fait lui-même le `git init` ; il doit faire aussi le premier commit. → recherche, petit lot.
3. **Commencer exige un chemin absolu tapé à la main** (captures 01, 02). Le premier élément de la page est un champ de
   chemin ; `~/essai-audit` est refusé (« un chemin absolu est attendu »). → recherche, lots `~` (en passe), sélecteur,
   accueil = chat.
4. **Après « Ouvrir », rien ne se passe** (capture 03) : la page reste sur l'accueil, et le dossier ouvert est un
   « agent inactif » dans une liste. → recherche, lot « ouvrir mène au chat » (en passe).

## Grave : la personne est perdue ou se méfie

5. **L'accueil est de la plomberie** (capture 01) : quatorze « agents » qui sont des sessions de développement
   (`mcp (pid 11756) · pid 11756`), des badges « actif/inactif », « 1 gabarit(s) local(aux) différent(s) du livré »,
   pas de chat. → l'accueil = le chat de l'agent d'accueil, la barre des agents (lot 3 de la maquette), la plomberie
   dans « Avancé ».
6. **Le chat s'ouvre sur un pavé rouge** de onze « session(s) orpheline(s) » avec leurs chemins `~/.local/share/…`
   (capture 04). → lot « orphelines » (en passe).
7. **Le jargon de la barre du chat** : « 0 tour · 0 jetons · 0 fiche écrite », « ce dépôt ne déclare pas de mémoire
   (bilans.memoire) », « rag en cours », « contexte : pas encore de tour », le nom du modèle. Les outils s'affichent par
   leur nom technique avec leur durée (`list_declarations 21:49:05 · 100 ms`, `propose`).
8. **La carte d'accord ne dit pas ce qu'elle accorde** (capture 11). « L'agent attend ton accord : run_command ·
   protected, once », « Racine : /home/lucied/.local/share/rag3weaver/taches/…/readme/code/git · en lecture » ; la
   première carte n'a **pas la commande** (on la lit seulement dans la boîte : `git status`, puis
   `git add README.md && git commit -m "Ajout du README.md"`). Un `git status` dans l'arbre de la tâche elle-même est
   classé « protected ». → la carte dit la commande en tête, en clair, et sa raison en français ; la classe d'un
   `git status` dans l'arbre de la tâche est à revoir (archi).
9. **La page d'une tâche parle en interne** (captures 10, 11) : « Témoins : la branche se charge entière », un bouton
   « Fusionner » avant tout travail, « Équipier », « Boîte aux lettres », un menu « demande », un champ « élément (pour
   une réponse) », « partie de 9757d0204 » (qui n'est pas le commit du dossier, `40d1bd0` : à vérifier, la tâche part
   peut-être d'un autre dépôt). La liste des tâches dit « base : reflink, 4716 Kio, posée 32 ms ».
10. **Nommer une tâche avant de pouvoir parler** (capture 07) : un champ « nom de la tâche » puis « Ouvrir une
    tâche ». Une personne demande une chose ; la tâche et son nom viennent de la demande (`open_code_task`).

## Moyen

11. **La page d'une proposition** montre un diff entier avec « Appliquer / Rejeter » et « par agent — proposée »,
    sans dire où ce fichier vit ni ce que l'application changera.
12. **Les liens de fichiers morts** du chat (signalé par Lucie, rejoué) : un chemin cité devient un lien même s'il
    n'existe pas, et la page dit « ReadFileNode: no such file… ». → lot « liens résolus » (en passe).

## Ce qui met le produit en valeur (à garder, à montrer)

- L'agent répond vite, et la tâche démarre seule après la demande ; la boîte montre le fil horodaté
  (`demande · humain`, puis les demandes d'accord).
- Les cartes d'accord existent et attendent la personne : c'est ce qui rend le produit sûr. Il faut seulement qu'elles
  parlent en clair.
- Le flux en direct (« flux : ouvert ») et le thème donnent une impression d'outil vivant.
- Ce qui manque à l'écran pour mettre en valeur : ce que l'agent sait de la machine (dépôts, index, mémoire, dépenses),
  à faire surgir dans le chat par l'agent d'accueil (les blocs `show` d'archi).

## Les lots, dans l'ordre de l'audit

1. **archi** — `chat-project-first` et `open-code-task` ; **recherche** — la page suit la demande à sa tâche
   (`ide-moves-to-task`, en passe, sur la branche d'archi).
2. **recherche** — un dossier créé par l'IDE reçoit son premier commit.
3. **recherche** — `~` ; ouvrir mène au chat ; orphelines hors du chat ; liens résolus (en passe, en chaîne).
4. **recherche** — la carte d'accord dit la commande et sa raison en clair, en tête ; **archi** — la classe d'une
   commande dans l'arbre de la tâche.
5. **recherche** — l'accueil = le chat de l'agent d'accueil, la barre des agents à gauche, la plomberie dans
   « Avancé » ; le rendu des blocs `show` d'archi (gabarits déclarés seulement).
6. **recherche** — le jargon de la barre du chat et de la page d'une tâche, réécrit en mots de la personne.
7. **recherche** — le sélecteur de dossiers, dans le fil et dans « Avancé ».

Chaque lot ne sera dit prêt qu'une fois ce parcours rejoué de bout en bout dans Chrome (règle du jour de c7).
