# Un chat au centre, les agents à gauche — maquette

22 novembre 2026, vers 22 h, la forme voulue par Lucie : « faut qu'il y ait principalement un chat, et des UI de
navigation quand même entre les agents, actifs ou non, ça en sidebar gauche ; et le chat, même si j'ai encore rien
ouvert de rien, il est là pour justement me donner les stats ou me sortir un lien vers un chat d'agent, ou m'en setup un
nouveau ». Remplace la page 02 (l'accueil n'est plus une page à cartes, c'est le chat de l'agent d'accueil). La
déclaration de l'agent d'accueil, ses outils et ses limites sont la part d'archi.

## L'écran, toujours le même

```
┌─────────────────────┬────────────────────────────────────────────────────────────┐
│ rag3weaver        ⚙ │  Accueil                                    ◐   ⚙ Avancé   │
│ ┌─────────────────┐ │────────────────────────────────────────────────────────────│
│ │ 🔍 un agent…    │ │                                                            │
│ └─────────────────┘ │  Bonjour. Je connais l'état de cette machine : 3 dépôts,  │
│                     │  2 agents en cours, 1 accord qui t'attend. Je n'ai pas     │
│ ● Accueil           │  d'outils de code ; je peux te mener à un agent, ou t'en   │
│                     │  préparer un.                                              │
│ ACTIFS              │                                                            │
│ ● rag3db · lot B    │                     tu : j'aimerais qu'on regarde le repo  │
│   en cours · 2 min  │                          github.com/x/y                     │
│ ◐ essai-auto        │                                                            │
│   attend ton accord │  Je n'ai pas d'outils de code. Je peux le préparer :       │
│                     │  ┌──────────────────────────────────────────────────────┐ │
│ AUTRES              │  │ Cloner github.com/x/y dans ~/git_workspaces/y,      │ │
│ ○ lucivy            │  │ l'ouvrir, et créer un agent de code dessus           │ │
│   hier              │  │                         [Refuser]  [Accorder]        │ │
│ ○ notes             │  └──────────────────────────────────────────────────────┘ │
│   il y a 3 j        │                                                            │
│                     │  ✓ Prêt : y est cloné et ouvert. → Ouvrir le chat de y     │
│ + Nouvel agent      │                                                            │
│                     │ ┌────────────────────────────────────────────────────────┐ │
│                     │ │ Écris…                                            ➤   │ │
│                     │ └────────────────────────────────────────────────────────┘ │
└─────────────────────┴────────────────────────────────────────────────────────────┘
```

- **Le chat est au centre, toujours.** À l'arrivée, c'est celui de l'**agent d'accueil**, la saisie prête. Il n'y a plus
  de page de formulaires.
- **La barre latérale gauche liste les agents** : « Accueil » en tête, puis les **actifs** (un tour en cours, un accord
  qui attend : le point le dit), puis les **autres**, du plus récent au plus ancien. Chaque agent se nomme par son projet
  et sa tâche, avec son état en une ligne. Un clic ouvre son chat dans la zone centrale ; l'adresse suit
  (`/ide/code/<id>/chat`), on revient en arrière. Une recherche filtre la liste, et « + Nouvel agent » demande la même
  chose à l'agent d'accueil.
- **L'agent d'accueil n'a pas d'outils de code.** Il dit l'état de la machine (dépôts, agents, tâches, dépenses du
  registre, index), répond par des **liens vers les chats d'agents**, et **prépare un agent** sur demande : il clone (sur
  carte d'accord), ouvre le dossier, crée l'agent de code et donne son lien. Ce qu'il ne sait pas faire, il le dit.
- **Le sélecteur de dossiers** est un outil de l'agent d'accueil : « Ouvre-moi un dossier » montre le sélecteur dans le
  fil (navigation depuis `~`, dépôts git trouvés en tête, ce que l'hôte refuse marqué d'avance). Il est aussi dans
  « Avancé ».
- **« Avancé »** (en haut à droite) : la plomberie du poste, l'arbre complet des agents et des sous-agents, le champ de
  chemin (qui comprend `~`), les réglages.

## Dans le chat d'un agent de code

Même écran : la barre latérale reste, le chat est celui de l'agent, avec en tête son projet, sa branche et ses vues. Le
mot d'accueil d'un chat vide dit ce qui vient d'être ouvert et ce que l'agent peut faire (lot « ouvrir mène au chat »,
en passe).

## Les lots, dans l'ordre (part recherche)

1. `~` compris par l'hôte — en passe.
2. Ouvrir un dossier mène au chat du projet — en passe, sur le précédent.
3. **La mise en page** : la barre latérale des agents (actifs, autres, recherche, clic → chat), le chat au centre,
   l'accueil remplacé par le chat de l'agent d'accueil, l'ancienne page d'accueil derrière « Avancé ». Témoin
   navigateur : arriver sur le chat d'accueil ; cliquer un agent de la barre et voir son chat.
4. **Le sélecteur de dossiers servi par l'hôte** (`GET /ide/dossiers?chemin=`), comme carte dans le fil et dans
   « Avancé ». Témoin navigateur.
5. **Les cartes de l'agent d'accueil** dans le fil : un lien vers un chat d'agent, la carte d'accord « cloner et
   préparer », la carte « prêt → ouvrir le chat ». Rendu seulement : les outils et la déclaration sont à archi.

## Des agents autonomes, qui se font surgir et se parlent (Lucie, 23 h 15)

« Des agents ultra autonomes, qui savent faire pop d'autres sessions, qui communiquent entre elles ; arrêtons de trop
mettre de sécu : elle sert de points de validation au mode auto, pas à exiger l'utilisateur H24. »

```
│ ACTIFS                    │
│ ● rag3db · lot B      ⋯   │   ⋯ = Reprendre · Dupliquer
│   └ ● cartographe         │   un agent surgi est sous son parent
│   └ ○ relecteur  fini     │
│ ◐ essai-auto              │
```

- **Sur chaque agent de la barre, un menu « ⋯ »** : **Reprendre** ouvre sa session dans le chat central, même si elle
  est déjà ouverte ailleurs (le pendant de `--pop-agent` au terminal), et **Dupliquer** crée une copie indépendante,
  avec son propre chat (le pendant de `--fork-agent`). La copie apparaît aussitôt dans la barre, « copie de … ».
- **Un agent qui en fait surgir un autre** : le nouveau apparaît sous son parent, relié par un trait, avec son état. Un
  clic ouvre son chat.
- **Les messages entre agents se lisent dans les deux fils** : dans le fil de l'émetteur, une ligne « → au cartographe :
  … » ; dans celui du destinataire, « ← de rag3db · lot B : … ». Chaque ligne mène à l'autre fil.
- **Les cartes deviennent rares** : ce que le juge accorde, ou ce que le survivant tranche, se dit en **ligne calme**
  dans le fil (« ✓ accordé par le juge : `git push origin lot` — d'après « pousse quand c'est vert » »), avec la phrase
  citée et un lien pour révoquer. Une carte ne s'ouvre que lorsqu'aucune règle ne tranche, et elle n'interrompt pas les
  autres agents.

**Lots, après l'accueil-chat** : le menu « Reprendre / Dupliquer » (sur les routes d'archi, les pendants des options de
tty) ; la barre en arbre parent → enfants ; les lignes entre agents dans les deux fils ; les accords tranchés en lignes
calmes, la carte en dernier recours.
