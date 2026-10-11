# La mémoire par MCP — ce qui existe, ce qui classe, et par où commencer

11 octobre 2026, session mémoire longue. Page demandée par l'orchestration sur
les mots de Lucie : « on pourrait lui demander les fiches mémoire aussi, voir ce
qu'on pourrait en envisager en MCP ; ou la mémoire abstraite, avec
classification ; par LLM c'est bien aussi, sortie structurée tout simplement qui
décide où ranger les trucs avec un bon prompt ; ou un hack avec le reranker ».

Page seule, pas de code. Relu avant d'écrire :
`visions/2026-10-03-20h37-memoire-longue.md` et
`visions/2026-10-04-00h09-memoires-par-gabarit-et-fiches-qui-pointent-vers-tout.md`
(§9.2 surtout : l'archiviste à deux rythmes et la porte à deux temps).

## 1. Ce que le gabarit `memory` sait déjà, et ce que Claude verrait

Sept outils, quatre entités, trois relations — et **ils deviennent des outils MCP
sans une ligne de code**, puisque la liste et les schémas viennent du manifeste
(éprouvé sur le gabarit `code` : `templates/mcp/`, `e2e_mcp_stdio`).

| ce que Claude Code demanderait | l'outil qui existe |
|---|---|
| se souvenir | `put_memory` |
| relire une mémoire précise | `get_memory` |
| retrouver par le sens | `recall` |
| nommer un fil de travail | `put_subject` |
| lister par thème | `recall_subjects` |
| citer une chose du monde | `add_ref` |
| apprendre un genre de chose | `create_ref_type` |

**Et quatre propriétés qui ne se voient pas dans cette liste, mais qui décident
de tout ce qui suit :**

**`Ref` est `hashsafe` sur `(genre, valeur)`.** L'uuid d'une référence se
**recalcule**, donc deux sessions qui citent la même chose convergent sur le
**même nœud**, sans se parler. C'est la pièce qui rend une mémoire partageable
avant même qu'on ait réglé le partage de la base.

**Un genre porte un harnais, et un harnais raté ne jette rien.** `RefType` peut
porter une expression rhai qui normalise une valeur et rend
`{ok, pourquoi, normal, empreinte}`. Un script absent ou cassé donne « citée,
**non vérifiée** » — la référence entre quand même. C'est la règle « jamais un
refus, une information ».

**`create_ref_type` demande déjà avant de créer.** Son premier appel n'écrit
rien : il **montre les genres existants** et demande lequel on retient
(`confirm: false` par défaut), et il refuse un harnais qui reconnaît ce qu'un
genre existant reconnaît déjà. La règle que Lucie veut pour la classification est
donc **déjà implémentée une fois**, sur les genres — il reste à l'appliquer aux
sujets.

**`Ref` a un cycle de vie** (`vue`, `a_revoir`, `introuvable`, `retiree`) : une
cible qui bouge passe « à revoir », une cible disparue devient « introuvable ».
« Cité, introuvable » est une information, pas une perte.

**Ce qui manque et qu'il faut dire** : `ANCHORED_TO` ne connaît que des **lignes**
de la base. Ancrer une mémoire sur le **schéma** (la table, pas une de ses
lignes) ou sur quelque chose **hors de la base** (un chemin, une PR, un commit)
passe aujourd'hui par un `Ref` — ce qui marche, et ce que la vision du 4 octobre
voulait unifier sous une seule relation à genres. Décision en attente de Lucie.

## 2. Ranger : le classeur est le client, et c'est la découverte de cette page

Lucie propose « une sortie structurée qui décide où ranger les trucs avec un bon
prompt ». **En MCP, ce modèle est déjà dans la boucle : c'est le client.**

C'est un renversement par rapport à l'archiviste de la vision, qui appelait un
modèle *depuis* le produit. Par MCP, Claude Code **est** le modèle qui range, et
notre travail n'est plus d'appeler un classificateur mais de **lui donner la
bonne signature d'outil et ce qu'il lui faut sous les yeux** :

- un outil `put_memory` qui accepte le classement en arguments typés (gabarit,
  genre, thème(s), liens) — la sortie structurée que Lucie décrit **est** le
  schéma d'entrée de l'outil, et `tools/list` le lui donne déjà ;
- le **sommaire** des sujets existants, petit et stable, pour qu'il choisisse
  parmi ce qui existe plutôt que d'inventer. C'est exactement ce que
  `recall_subjects` rend ;
- et la règle de Lucie, **« jamais un refus, une confirmation »** : un thème
  inconnu ne fait pas échouer l'appel — l'outil écrit la mémoire, **montre les
  trois sujets les plus proches** et dit « si tu voulais l'un de ceux-là,
  rappelle-moi avec son nom ; sinon confirme la création ». C'est le deux-temps
  de `create_ref_type`, appliqué aux sujets.

**Ce que ça change pour le coût** : aucun serveur de modèle à lancer, aucun
seuil à calibrer, aucune dépendance de plus. Le classement vaut ce que vaut le
modèle du client — et pour Lucie, le client est Claude.

**Ce que ça ne couvre pas** : le rangement **automatique**, sans qu'on le
demande. Un agent qui travaille ne pense pas à ranger ; c'est tout l'objet de
l'archiviste (§4). Par MCP, le rangement est **volontaire**.

## 3. Les modèles de décision : option mesurée, jamais le défaut

Lucie cite `perplexity-ai/pplx-decider-v1.1-27b` et « un hack avec le reranker »
(notre bge-v2-m3, déjà servi sur 7980).

**Rappel de la décision du 10 octobre** : les modèles de décision et GLiNER sont
**mis de côté** — « pas encore fiable l'open source dessus ». Et la mesure qui
l'a motivée dit précisément pourquoi, en des termes qui valent aussi pour le
reranker : **un score ordonne bien et juge mal, et un seuil ne se transporte pas
d'un jeu à l'autre.** Un reranker rend un ordre ; « faut-il ranger ici » demande
une décision, donc un seuil, donc un étalonnage qui ne survit pas au changement
de corpus.

Donc, si on y revient un jour, **trois conditions, et je ne les relâcherais pas** :

1. **sur un jeu qui n'est pas de notre main.** Un banc dont nous écrivons les
   questions et les réponses mesure notre intuition, pas le modèle ;
2. **jamais le défaut**, et déclaré comme option — la classification par le
   client marche sans lui ;
3. **mesuré contre la voie 2**, pas dans l'absolu : la question n'est pas « ce
   décideur est-il bon » mais « fait-il mieux que le modèle qui est déjà dans la
   boucle, pour le coût d'un serveur de plus ».

Et la règle du dépôt tient : **Gemini pour juger le produit, les modèles locaux
pour éprouver les protocoles.** Un décideur local sert à vérifier qu'un protocole
de décision tient ; il ne sert pas à promettre du tout-local.

## 4. La mémoire abstraite : ce qui est faisable par MCP, et ce qui attend

L'archiviste (vision §9.2) a **deux rythmes**, et seul le premier est atteignable
par MCP aujourd'hui.

**Rythme lent — classer à chaque compression.** À 30 % de contexte, la tranche
écoulée est résumée (un **L1**), l'archiviste le classe par date et sujet(s), et
un sujet qui accumule reçoit un **L2**. Chaque L1 garde l'adresse de la tranche
brute : on redescend du L2 au L1, du L1 aux tours.

**Par MCP, aujourd'hui** : rien n'empêche Claude d'appeler `put_memory` avec son
propre résumé quand son contexte se compresse — **sauf qu'il ne sait pas quand ça
arrive**. Un client MCP n'a aucun signal de compression. Donc le rythme lent est
**faisable à la main** (« range ce qu'on vient de faire ») et **pas automatique**.
Ce qui le rendrait automatique est la **marche 1 des fiches de contexte**, pas
MCP.

**Rythme rapide — la porte à deux temps, à chaque tour.** « Faut-il chercher ? »
puis « faut-il injecter ? ». **Hors de portée de MCP, et c'est structurel** : un
serveur MCP ne voit pas les tours de l'agent, il ne voit que les appels qu'on lui
fait. La porte suppose un hôte qui observe chaque tour — c'est le harnais, pas un
serveur d'outils.

**Ce qui est donc vrai, et à dire à Lucie sans l'arrondir** : par MCP, on obtient
une mémoire **que l'agent consulte et nourrit quand il y pense**. L'archiviste
qui range sans qu'on le demande et rappelle sans qu'on le demande est un chantier
du harnais. La bonne nouvelle est que les deux partagent **le même classeur** :
ce qu'on écrit par MCP aujourd'hui est exactement ce que l'archiviste lira demain.

## 5. La portée : la cellule existe, le partage non

`Scope { org, project }` existe, avec ses colonnes système sur toute table, ses
index à part, et le fan-out `SearchOptions.scopes` qui fusionne par rang —
**branché**, pas déclaré (`catalog.rs`).

- **« ce projet »** = la cellule `(org, project)` du projet ;
- **« globale à tous les agents »** = une cellule convenue, `(org, "global")` ;
- **« les deux d'un coup »** = `options.scopes = [projet, global]`, projet
  d'abord.

Et sa limite, dite par le code : les scores ne sont **pas comparables** entre
cellules (IDF distincts), d'où la fusion par rang. Un `recall` à deux cellules
rend un ordre raisonnable, **pas un score interprétable** — à écrire dans la
description de l'outil, sinon un modèle lira les scores comme s'ils voulaient
dire quelque chose.

**Ce qui bloque le partage réel** : une base rag3db n'a qu'un écrivain, et Claude
Code lance **un processus MCP par session**. Le mode `--demon` est la forme
prévue, et il est **ni validé ni réfuté** — ticket
`2026-10-11-mode-demon-ni-valide-ni-refute.md`, trois questions ouvertes dont une
qui pourrait être un défaut de `rag3daemon`.

**Donc, aujourd'hui** : une mémoire **par session** fonctionne (la recette
`04-une-base-par-session-recette-jouee.md`), une mémoire **partagée** attend ce
ticket. Et l'exposition par clés (vision §7) décidera de ce qui part avec un
produit publié : la mémoire de la construction vit dans une cellule `dev` qui ne
suit jamais un site publié. `--keys` est lu ; le filtrage viendra avec elle.

## 6. Trois ordres, leur coût, et ce que je recommande

**A — Brancher le gabarit `memory` tel quel, par session.** *Coût : une passe.*
Le manifeste existe, la recette existe, les sept outils sortent sans code. On
obtient : se souvenir, retrouver, lister, citer, apprendre un genre — **une base
par session**, rien de partagé. Ce qui manque : le rangement guidé (pas de
sommaire poussé à l'agent), et la mémoire reste locale à la session.

**B — A, plus « ranger » soigné.** *Coût : deux à trois passes.* Le deux-temps de
`create_ref_type` porté aux **sujets** (montrer les proches, demander avant de
créer), le sommaire des sujets dans la description de `put_memory`, et le
classement en arguments typés. C'est là que la sortie structurée de Lucie se
réalise, **sans modèle de plus**. Demande de toucher le gabarit et un nœud, pas
le moteur.

**C — B, plus la mémoire partagée.** *Coût : B, plus les trois énigmes du ticket
`--demon`, plus un témoin.* Deux sessions sur une mémoire globale. Dépend d'un
défaut possible de `rag3daemon` qu'on n'a pas élucidé.

**Je recommande A maintenant, puis B.** Parce que A est gratuit et vérifiable
cette semaine, et parce que **B ne se conçoit bien qu'après avoir vu A à
l'usage** : on ne sait pas encore si Claude range spontanément quand il a
`put_memory` et `recall_subjects` sous la main, ni ce qu'il met dedans. La
réponse change la signature qu'on lui donnera en B — et c'est le genre de
question que trois passes d'essai tranchent mieux qu'une page.

**C en dernier, et pas avant que le ticket `--demon` soit fermé** : une mémoire
partagée dont la base peut se verrouiller ou perdre un index serait plus
coûteuse qu'une mémoire par session.

## Ce que je ne sais pas

- **Est-ce que Claude rangera de lui-même ?** A le dira. S'il ne range pas
  spontanément, B devra pousser plus fort qu'un sommaire — par exemple un outil
  `ranger` qu'on appelle explicitement en fin de tâche.
- **Combien de sujets avant que le sommaire cesse d'être petit ?** La vision dit
  « petit et stable » ; à cent sujets, un sommaire d'une ligne chacun fait déjà
  cent lignes dans chaque description d'outil. Il faudra un seuil, et je ne sais
  pas où.
- **Ce qu'une mémoire veut citer d'un run** : « le run » ou « ce que le run a
  rendu » ? La question est ouverte depuis la page sur l'exécution adressable, et
  elle se reposera dès qu'on citera des exécutions.
