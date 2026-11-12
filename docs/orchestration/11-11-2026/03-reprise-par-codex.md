# Reprise par un autre agent d'orchestration (Codex), 11 novembre 2026

Cette page est faite pour être donnée telle quelle à une session d'orchestration neuve, sans aucune
mémoire de nos échanges. Elle dit ce qu'elle doit lire, ce que sont les sessions qu'elle orchestre,
comment elle les démarre, et ce qui est en attente. Les amorces de la fin se collent dans chaque
session de travail.

## 1. Ce que l'orchestrateur lit d'abord, dans cet ordre

1. `docs/README.md` : l'organisation et les règles qui ne se discutent pas.
2. `docs/orchestration/11-11-2026/02-knowledge-dump.md` : les dépôts, les postes, la lib épinglée, la
   passe, le push, les verrous, les mesures, les décisions de Lucie.
3. `docs/orchestration/11-11-2026/01-rapport-de-session.md` : l'état au 11 novembre, session par session.
4. `docs/procedures/push-d-un-lot.md` : la procédure de passe et de push, à faire respecter mot pour mot.
5. `docs/nouveautes.md` (ce qui est atteint) et `docs/orchestration/visions/README.md` (où on va).

## 2. Le rôle de l'orchestrateur

- Il tient **la file des pushes sur master** : un lot atterrit à la fois ; il donne le tour par le mot
  « à toi » ; la session répond « poussé » avec le hash et les trois contrôles lus (status propre,
  aucun rebase en cours, `origin/master` = HEAD). Les docs seules se poussent sans tour.
- Il fait **relire master avant chaque push** : si du Rust est arrivé sous `extension/rag3weaver`
  depuis la passe, la session rejoue court ; du C++ du moteur ou des docs seuls : rebase sans rejeu.
- Il **nomme la lib** à toutes les sessions quand du C++ du moteur atterrit (une lib neuve au magasin,
  identifiant complet, posée aux deux postes par la session optimiseur).
- Il tient **le rapport d'orchestration** à jour après chaque atterrissage (hash, ce que ça change) dans
  `docs/orchestration/<date>/01-rapport-de-session.md`, et pousse ses docs lui-même.
- Il **ne code pas** et ne lance pas de passe : il lit, décide, relaie, et dit à Lucie quand regarder.
- Il **dit non** à ce que les règles interdisent (push en force, attribution, amont, tag lucivy, sudo,
  effacer sans le mot de Lucie) et remonte à Lucie tout refus de permission, sans le contourner par
  une autre session.

## 2 bis. Comment se prennent les décisions

Trois niveaux, et chacun sait lequel est le sien.

1. **La session décide seule** de ce qui reste dans son périmètre et ne change pas la conception : la
   forme du code, le découpage d'un lot, ses témoins, un nom, un ticket à ouvrir, l'ordre de ses pas.
   Elle n'attend personne pour ça ; elle dit ce qu'elle a fait dans son rapport.
2. **L'orchestrateur tranche** ce qui dépasse une session : qui prend un lot, l'ordre de la file, un
   désaccord de forme entre deux sessions, l'usage des ressources partagées (la carte, les lourds, la
   lib à épingler, une pose), un rouge à départager entre environnement et lot. Il dit sa décision à la
   session, l'écrit dans son rapport avec sa raison, et la marque « à relire par Lucie » quand elle
   touche à une conception. Il ne code pas : il décide et relaie.
3. **Lucie décide** de tout ce qui change la conception ou qui lui appartient : une règle nouvelle, un
   outil qui change ce que l'utilisateur voit, une direction de produit ; son arbre, ses bases, ses
   réglages, son sudo, sa carte de jeu ; toute suppression (fichiers, worktrees finis, branches
   distantes), toute fusion dans lucivy, tout tag, toute publication ; le moment d'une pose quand
   elle travaille. **Un symptôme signalé est une enquête, pas un changement de conception.**

Comment on lui demande : par **une page de conception** (ce qui existe, la forme proposée, les
questions à trancher, numérotées), poussée sur master en docs seules, et une ligne dans le rapport
« à lire par Lucie ». Rien ne se code avant son mot. En attendant, la session fait tout ce qui n'en
dépend pas (règle de Lucie du 7 novembre : « ne pas s'empêcher de progresser ») : mesures, témoins
rouges, pages. Son mot se donne dans la session concernée (lucivy pour lucivy) ou à l'orchestrateur,
qui le relaie mot pour mot. Un refus de permission (le sien, ou celui de la plateforme) vaut pour
toutes les sessions : on le remonte, on ne le contourne pas par une autre session ni en reformulant.

Comment on garde la trace : la décision et sa raison dans le rapport d'orchestration du jour ; la
conception dans sa page, avec les questions et les réponses ; un défaut connu non corrigé dans un
ticket ; ce qui est atteint dans `docs/nouveautes.md`. Quand c'est posé et que ça vaut son regard,
on le dit à Lucie en une ligne (« dire à Lucie quand regarder ») ; on ne la dérange pas pour le reste.

## 2 ter. Le modèle de décision local (où on en est, et comment on a travaillé)

Lucie veut un **modèle local qui décide** à deux endroits : dans la mémoire longue (garder ou non une
fiche rappelée) et dans l'atelier (accorder ou non une action : un chemin hors de la racine, une
commande). Le travail s'est fait par **bancs**, jamais par intuition, dans la session banc :

- **Le juge local du rappel** (8 et 11 novembre) : un jeu de 440 paires (contexte d'agent, bout de
  mémoire) dont 88 positives vérifiées à la main ; chaque modèle répond « oui / non » à « ce bout de
  mémoire est-il pertinent avec le contexte de l'agent ? » et on lit **P(oui) / P(non) dans les logits**,
  pas le texte ; on mesure exactitude et AUC, et le temps par cas. Résultats : Gemini 0,81–0,84 à 4,4 s ;
  le dense seul 0,41 ; Qwen2.5-1.5B 0,65 ; **Qwen3-4B 0,76 / AUC 0,92**, 504 ms en bf16, et en GGUF Q8_0
  avec cache de préfixe la même chose à 83 ms par fiche une fois le contexte en cache (1,3 s pour k = 5) ;
  Q4_K_M perd en forme courte. Les petits modèles sans exemples sont au hasard. Leçons : **un classeur,
  pas un seuil** (on garde les 1 ou 2 meilleures sur 5) ; le contexte utile est les deux derniers tours
  et leurs résultats d'outils, borné à 2 k jetons ; la liste A–E ne marche pas (biais de position) ; les
  licences se lisent (Qwen3 Apache 2.0 ; Qwen2.5-3B refusé, licence non commerciale).
  Pages : `extension/rag3weaver/docs/08-novembre-2026-banc/` (01 à 03), `docs/banc/11-11-2026/03-le-juge-en-gguf.md`,
  et la conception du branchement `extension/rag3weaver/docs/09-novembre-2026-memoire-longue/03-le-classeur-local-dans-le-rappel.md`
  (en attente du mot de Lucie ; la mesure dit qu'aujourd'hui le contenu manque plus que le juge).
- **Le décideur d'actions de l'atelier** : pas encore de modèle. La matière s'accumule d'abord :
  chaque accord ou refus de Lucie sur une carte (chemin, commande) va dans `<runs>/decisions.jsonl`
  avec la demande, la raison du modèle et la décision (lot d'archi du 9 novembre). Règle fixée : **le
  décideur local escalade, il n'accorde jamais à la place de l'utilisatrice** ; il sert à refuser vite
  l'évident et à ne déranger que pour le reste. Quand il y aura quelques centaines de décisions, le banc
  les rejoue avec Qwen3-4B (même forme oui/non) et mesure avant tout branchement. Graine de Lucie du
  7 novembre : un classifieur ModernBERT (« Laya ») si un modèle plus petit suffit.
- **La carte** : le banc tourne sur la carte des sessions (`card2`, `ROCR_VISIBLE_DEVICES=0`), jamais
  sur celle de seat1 ; llama.cpp de Lucie déjà bâti en HIP à luciepc (`~/git_workspaces/llama.cpp/build-hip`).

## 3. Les sessions qu'il orchestre

Chaque session porte le nom de sa barre chez Lucie, et son dossier `docs/<nom-en-kebab-case>/`. Elle
commence par lire `docs/README.md`, puis son `11-11-2026/01-rapport-de-session.md` (section
« Reprendre » : chaque worktree inachevé, son chemin, sa branche, son dernier commit, les commandes),
puis son `02-knowledge-dump.md`, puis ses tickets dans `docs/<nom>/tickets/`.

| session | dossier | premier travail (au 11 novembre) |
|---|---|---|
| everything declarative | `docs/everything-declarative/` | **le correctif des réclamations du rattrapage**, branche `reclamations-rendues` (9da8e4433), jamais compilé ni joué : rouge d'abord, passe, push — c'est lui qui rougit la CI par intermittence |
| recherche | `docs/recherche/` | arret-du-backend (branche `arret-du-backend-7`) : trancher le rouge des trois CSV de COPY d'un processus tué, rebase, rejeu court, push ; puis le journal des runs écrit après l'arrêt ; puis le Surveillant ; et le lot « lucivy-core 4.3.3 » (Cargo.toml et Cargo.lock, passe entière : un close() en échec arrête quand même ses pools, lié au SIGSEGV de sortie des e2e) |
| rag3weaver archi | `docs/rag3weaver-archi/` | `ide-par-le-flux` (wt-flux, a9c5a3108) : rejouer la passe tuée (code 137), départager la lib entière lente, push ; puis le découpage au survol de la jauge ; relecture d'ED de l'atelier |
| coeur c++ | `docs/coeur-cpp/` | le témoin de l'annulation sous pool plein (`annulation-sous-pool-plein`, 6e7f11680, jamais joué) : le jouer rouge, puis le correctif ; fermer le ticket de l'index complet quand hotfixs a rejoué sa trace |
| coeur c++/hotfixs/tickets | `docs/coeur-cpp-hotfixs-tickets/` | rejouer la trace exacte des deux sens sous la lib 29a7bc52a (service 7882, hôte 7992 ; les outils dans `11-11-2026/outils/`) : c'est le vert qui ferme le ticket de l'index complet |
| mémoire | `docs/memoire/` | l'archivage à blanc des docs sur une copie de la base de Lucie (le 400 de Vertex se relit maintenant, lot 2d6a08a65) ; M8 après relecture d'ED ; le classeur local après le mot de Lucie |
| optimiseur | `docs/optimiseur/` | **poser le binaire de la tête de master** aux deux postes (lib 29a7bc52a inchangée), une coupure, préavis d'une minute ; tenir les libs, les services, les tunnels, le disque de luciepc |
| embarquement | `docs/embarquement/` | rien d'inachevé ; son ticket (granite-278m arrêté huit fois, cause illisible) |
| codeparsers | `docs/codeparsers/` | le lot require-js (pointeur 78888f5, branche `codeparsers/require-js`) : décider sur la mesure du 9 (lecture dans le rapport), passe, push ; le ticket import_binding |
| banc | `docs/banc/` | le banc Qwen3-4B en GGUF est fait (96a98a452 : Q8_0 en cache de préfixe égale bf16, 0,76 / AUC 0,92, 1,3 s pour k = 5 ; Q4_K_M perd en forme courte) ; ensuite le décideur d'actions quand il y aura des accords journalisés, et le classeur dans le rappel au mot de Lucie |
| arbre principal | `docs/arbre-principal/` | wt-deplace (le garde-fou) et les mesures M3 à M5, décrits dans « Reprendre » |
| lucivy | `docs/lucivy/` | la 4.3.3 est publiée le 11 (tag v4.3.3, main 663975a ; #22, #23, #24, #25 fusionnées au mot de Lucie) ; rien d'autre sans son mot. Côté rag3db, `extension/rag3weaver/Cargo.toml` prend encore `lucivy-core = "4.3.2"` : monter à 4.3.3 est un lot Rust de la session recherche (Cargo.lock, passe, push) |
| tty | `docs/tty/` | rien d'inachevé ; les thèmes suivants de l'interface terminal au premier fichier |
| hooks-essai (fermée) | `extension/rag3weaver/docs/07-novembre-2026-hooks-essai/` | sa page « la recherche de code par les hooks » attend le mot de Lucie ; à rouvrir seulement s'il le donne |

## 4. Ce qui attend Lucie (à lui rappeler, pas à faire à sa place)

- Un index complet de ses bases de code pour réparer l'écart des deux sens (le correctif empêche, il ne
  répare pas) ; sa carte, son heure.
- Lire : la page des vues par worktree, celle de l'atelier, celle des hooks, celle du classeur local.
- Le ménage des gros dossiers laissés sur disque (listés dans chaque rapport) et des 476 `livre.json` de
  témoins (ticket de mémoire) ; la suppression des branches distantes fusionnées.

## 5. Les postes au 11 novembre

- Lib aux deux postes : `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
- Binaire posé : 0b0ff221a ; master a avancé depuis (a6637eeee, be319fa75, 3e8c87aa5, ab8272deb, 2d6a08a65 et les docs) : une pose est due.
- Services à luciepc (card2) : 7878 granite-278m, 7879 bge-m3, 7880 granite-107m, 7882 granite privé
  d'hotfixs (laissé en route pour le rejeu). Tunnels du portable : 7979 → 7878, 7980 → 7879, 7981 → 7880.
  Jamais un cinquième service sur card2 ; card0 au mot de Lucie.
- `lourds-max` à luciepc : 4. fstrim hebdomadaire avec `--minimum 16M`, le dimanche à 4 h (posé le 11).

## 6. Amorces à coller dans chaque session de travail

Remplacer `<nom>` par le dossier de la session. Le texte tient en quelques lignes, exprès.

> Tu es la session « <nom de la barre> » du projet rag3db (`~/git_workspaces/rag3db`). Lis d'abord
> `docs/README.md`, puis `docs/<nom>/11-11-2026/01-rapport-de-session.md` (sa section « Reprendre » dit
> où reprendre, et avec quelles commandes), puis `docs/<nom>/11-11-2026/02-knowledge-dump.md`, puis
> tes tickets dans `docs/<nom>/tickets/`. Règles qui ne se discutent pas : identité git
> `Lucie Defraiteur <luciedefraiteur@gmail.com>`, aucune ligne d'attribution ; jamais l'arbre
> `~/git_workspaces/rag3db` lui-même (un worktree par lot) ; chaque commit poussé sur sa branche dans la
> minute ; master seulement par la procédure `docs/procedures/push-d-un-lot.md`, au mot « à toi » de
> l'orchestration, dans `if grep -qx VERT`, après les trois contrôles ; lib épinglée par son identifiant
> complet ; rouge d'abord ; un défaut connu non corrigé est un ticket. Dis à l'orchestration « rouge
> posé », « vert », « poussé <hash> », et tout blocage dans la minute. Premier travail : <la ligne de la
> table ci-dessus>.

Pour la session optimiseur, ajouter : « tu poses les libs et les binaires aux deux postes (luciepc puis
le portable), avec instantané et retour, annonce une minute avant ; tu nommes la lib à l'orchestration ».
Pour lucivy, ajouter : « aucune fusion dans main ni aucun tag sans le mot de Lucie dans cette session ».

## 7. Gemini comme ressource normale du projet — précision de Lucie, 12 novembre

Lucie confirme que les ressources cloud du projet doivent servir à le faire
avancer : utiliser Gemini quand son apport est pertinent, sans hésitation de
principe ni nouvelle demande d'autorisation pour chaque appel dans le travail
confié. Cela vaut pour une seconde lecture de contrats, une analyse difficile,
la comparaison de raisonnements et les qualifications qui nécessitent un vrai
modèle. Les modèles locaux restent également des ressources normales selon leur
pertinence ; il ne s'agit pas de choisir un fournisseur pour toute tâche.

L'appel répond à une question précise, avec contexte utile, source identifiée,
borne de durée et résultat conservé dans le ticket du travail. Une seconde
opinion distingue défaut concret, hypothèse et limite de preuve ; elle complète
les relectures et tests. Elle n'est pas une nouvelle formalité obligatoire avant
chaque intégration. Une revue parallèle d'un lot déjà qualifié ne suspend pas
son atterrissage par défaut ; un défaut concret est ensuite traité et ticketé.

Utiliser les voies déjà configurées du projet, identifiants par leur chemin,
sans lire, afficher ni copier .env ou .vault. Une revue de code prend les sources
et contrats concernés ; elle n'implique pas d'ouvrir les bases ou d'envoyer les
conversations et données réelles de Lucie. Éviter les appels sans question utile
et les répétitions inchangées ; conserver les erreurs et leur diagnostic.

## 8. Nuit du12au13novembre : lots et second GPU

Lucie autorise les suites utiles parallèles, notamment visions et correctifs
du cœur, en demandant de découper les lots.
[Répartition effective](../12-11-2026/02-lots-independants-pour-la-nuit.md).
Card0 est disponible **cette nuit le13novembre00:00–08:00 Europe/Paris**,
en plus de card2. L'entrée vérifie l'heure/placement/consommateurs, les seuls
modèles privés ont une échéance effective avant08:00 et aucune acquisition
tardive ne les réactive. Optimiseur tient ce créneau ; aucune nouvelle demande
GPU à Lucie pour un besoin pertinent autorisé. Les modèles existants ne sont
pas déchargés sans attribution et les autres nuits ne sont pas présumées.

Pour Gemini Decision1, Lucie confirme au contraire d'attendre **Vertex/Google
Cloud** avec ses crédits : aucune voie AI Studio ou facturation alternative
n'est créée. La disponibilité Cloud annoncée ultérieurement reste à vérifier.
