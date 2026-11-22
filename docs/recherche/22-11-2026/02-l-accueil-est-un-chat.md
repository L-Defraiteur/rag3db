# L'accueil est un chat — maquette

22 novembre 2026, session recherche, à la demande de c7 après le premier essai de Lucie : « je comprends même pas
pourquoi j'ai pas direct un chat d'agent si j'ai encore rien lancé ; un bouton ouvrir ? sur un chemin entré à la
main ? ».

## L'écran d'accueil, rien d'ouvert

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ rag3weaver                                                   ⚙ Avancé  ◐ thème │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   Bonjour. Je suis ton agent. Dis-moi ce que tu veux faire :                  │
│   écrire du code, chercher dans un projet, en commencer un.                   │
│                                                                              │
│   ┌ Projets récents ─────────────────────────────────────────────┐            │
│   │  rag3db            ~/git_workspaces/rag3db        il y a 2 h │            │
│   │  essai-auto        ~/essai-auto                   hier       │            │
│   │  + Choisir un dossier…      + Nouveau projet                  │            │
│   └──────────────────────────────────────────────────────────────┘            │
│                                                                              │
│ ┌──────────────────────────────────────────────────────────────────────────┐ │
│ │ Écris à ton agent…                                                  ➤    │ │
│ └──────────────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────────┘
```

- **La saisie a le focus** dès l'arrivée : on écrit sans rien choisir. L'agent part neutre (règle de Lucie du 21) :
  il peut répondre, expliquer, proposer.
- **Quand il a besoin d'un projet**, il le demande dans le fil, avec une carte d'action : « Je travaille dans quel
  dossier ? [Choisir un dossier…] [Nouveau projet] ». Rien n'est ouvert sans un clic.
- **Nouveau projet** : un nom suffit ; le dossier `~/git_workspaces/projets/<nom>` est créé, `git init`, ouvert.
- **Projets récents** : ceux que l'hôte a ouverts, du plus récent au plus ancien, un clic les rouvre.
- **Avancé** (en haut à droite) : l'arbre des agents, la plomberie du poste, le champ texte d'un chemin (qui comprend
  `~`). Rien de cela sur l'accueil.

## Le sélecteur de dossiers

```
┌ Choisir un dossier ─────────────────────────────────────────────┐
│ ~ › git_workspaces                                    [Ouvrir ici] │
│                                                                    │
│  Dépôts git trouvés                                                │
│   ◆ rag3db              ◆ lucivy             ◆ projets/essai       │
│                                                                    │
│  Dossiers                                                          │
│   📁 projets                                              ›        │
│   📁 recherche                                            ›        │
│   📁 sairen                       (protégé : l'hôte n'y ouvre rien) │
│                                                                    │
│  [Annuler]                                         [Ouvrir ce dossier] │
└────────────────────────────────────────────────────────────────────┘
```

- Servi par l'hôte : il liste les sous-dossiers à partir du dossier personnel, sans fichiers, et marque d'avance ce
  qu'il refusera (l'arbre de Lucie, sairen, le dossier d'une autre session) au lieu de refuser après coup.
- En tête, les **dépôts git trouvés** sous `~/git_workspaces` (un niveau, deux au plus).
- Le fil d'Ariane remonte ; « Ouvrir ici » ouvre le dossier courant.

## Les lots, dans l'ordre

1. **`~` compris côté hôte** (fait, en passe : `recherche-open-folder-tilde`).
2. **L'hôte sert la navigation** : `GET /ide/dossiers?chemin=~/…` (sous-dossiers, dépôts git, refus d'avance) et
   `GET /ide/recents`. Témoins de lib.
3. **Le sélecteur** dans l'IDE, sur ces deux routes. Témoin navigateur : naviguer, ouvrir un dépôt, voir le refus
   d'avance.
4. **L'accueil est le chat** : sans projet, la page est le chat de l'atelier (focus dans la saisie), les récents au-dessus,
   l'arbre des agents et la plomberie derrière « Avancé ». Témoin navigateur : arriver, écrire, recevoir une réponse.
5. **La carte « je travaille dans quel dossier ? »** : l'agent la pose quand un outil demande un projet ; un clic ouvre
   le sélecteur ou crée le projet. Témoin navigateur.
