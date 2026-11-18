# Évaluation du travail de Codex dans le périmètre tty — 18 novembre 2026

Session tty, à la reprise du 18 nov. 2026, au mot de Lucie (« d'abord chaque session évalue la pertinence de ce qu'a fait Codex »). Codex a tenu le rôle tty du 16 nov. (11 h) au 17 nov. (21 h 10).

## Ce qui a été vérifié, et comment

- **master** : aucun commit de Codex ne touche `extension/rag3weaver/tty` ni `docs/tty` (`git log 927941a67a..origin/master -- extension/rag3weaver/tty docs/tty` est vide).
- **La branche `cx-tty-english-aliases-7`** (tête af6a7447ce, 80 commits, déjà reposée sur master 99fef92ce6) : relue commit par commit pour le code, par listes pour les docs. Puis jouée par tty, dans un worktree et un target à elle (`cx-tty-eval-codex`, `target-cx-tty-wt/cx-tty-eval-codex`, ligne `Compiling rag3weaver-tty … (cx-tty-eval-codex)` vérifiée) :
  - crate : 114 + 1 (`main.rs`) tests, et 22 sans vue ; tous verts ;
  - clippy `-D warnings` : rien ;
  - harnais entier, avec les modifications de Codex au harnais, sous `tmux-local-ouvert` : VERT ;
  - témoin T1 `mesures/plain_stays_attached.py` : VERT.
- **Rouge d'abord** : le premier commit (69009e5a02) n'ajoute que les tests des alias ; le rouge est consigné (274c9df805), puis le code (a69b11a6f5, fc029fb512). Même ordre pour le défaut de Tab : témoin (8544ccbf0e), rouge consigné (9eab76ff61), puis correctif (36b121e6c0).
- **Identité** : 80 commits sous l'adresse gmail, aucune attribution.
- **Les branches `cx-tty-english-aliases`, `-2` à `-6`** : d'anciennes versions rebasées ; chacun de leurs commits a son équivalent dans `-7` (`git cherry` : 0 non repris). Rien n'y est perdu.

## 1. Le tableau

| Ce qui | Commits | Verdict |
|---|---|---|
| Les alias anglais des `/commandes` : `/help`, `/quit`, `/stop`, `/task`, `/repo`, `/log`, `/clear` (`/clear history`), `/steps`, `/name`, à côté des noms français qui restent ; `/aide` montre les deux, l'anglais d'abord avec « (ancien : …) » ; l'affichage reste en français | 69009e5a02 (tests), 274c9df805 (rouge), a69b11a6f5 | **utile, à garder** |
| Les options anglaises : `--repo`, `--task`, `--request`, `--log`, `--steps`, `--address`, `--since`, à côté des anciennes (lues en silence) ; la lecture des options extraite en `parse_options`, testable, avec son test | a69b11a6f5, fc029fb512 | **utile, à garder**. Un détail : le message d'erreur de `--since` dit encore « --depuis : … » |
| La complétion par Tab de l'argument de `/task` et `/repo` (avant : seuls `/tache` et `/depot` cherchaient le catalogue, par préfixe) ; `sources_de` choisit ses sources par la commande lue | 8544ccbf0e, 041660a71e, bc8ab07581, 9eab76ff61, 36b121e6c0 | **utile, à garder** : un vrai défaut, trouvé par lui |
| Le README du crate : les noms anglais d'abord, les anciens dits | dans la branche | **utile, à garder** |
| Le harnais instrumenté : archive d'observations par séance (`mesures/harness_observation.py`, `test_harness_observation.py`), lecture de l'écran tmux (`attendre_visible`), traces des entrées et sorties | be548eefee à 73b528ffa8 (16 nov., 18 h 56 à 20 h 16) | **utile mais à requalifier** : le harnais reste vert, et lire l'écran plutôt que les octets est une vraie amélioration de l'oracle ; mais c'est 250 lignes de plus que personne n'a demandées. À reprendre dans un lot à part, s'il sert, pas avec les alias |
| L'appareil de « qualification » sous `docs/tty/17-11-2026/` : `qualification*.py`, contrats et plans JSON « inactifs », preuves de provenance ELF, profils SDK, manifestes et archives de deux postes (31 fichiers, 348 Ko, dont deux JSON de 2 000 lignes) | 3583e68a9b à 44ae9af3bc (17 nov.) | **inutile pour ce lot, et nuisible s'il entre sur master** : un lot de 3 heures a été bloqué deux jours sur un « producteur physique des images exécutées » qu'aucune règle n'exige. Il ne se fusionne pas ; la branche le garde |
| Le ticket des alias gonflé de 785 lignes (journal de qualification, coupons, GO) | plusieurs | **utile mais à requalifier** : à ramener à son État, à ce qui est fait et à ce qui reste |
| Les ajouts aux tickets tmux et « frappe avant l'invite » (+23 lignes chacun) | plusieurs | **à requalifier** : des observations de qualification, qui ne changent ni le défaut ni sa piste |
| Le ticket neuf `2026-11-16-le-harnais-perd-les-traces-des-seances-et-juge-la-fin-sur-les-octets.md` | 16 nov. | **à requalifier** : son premier rouge (« après o, la fin ne paraît pas ») ne se reproduit pas, le harnais est vert le 18 nov. ; la remarque sur l'oracle par octets est juste, elle va avec le lot du harnais instrumenté |
| La passation `docs/tty/17-11-2026/99-passation-pour-claude.md` | af6a7447ce | voir § 4 |

## 2. Verdict d'ensemble

1. Le travail utile est petit et bon : les alias des commandes et des options, un vrai défaut de Tab corrigé, un README juste ; rouge d'abord, et vert vérifié par tty (crate, clippy, harnais, T1).
2. Le lot est prêt à entrer sur master, **sans** l'appareil de qualification : environ 200 lignes de code et de tests, sur les 13 130 de la branche.
3. Codex a pris l'exigence de preuve pour une fin en soi : contrats, provenance d'exécutables, coupons, au point de déclarer « non terminé » un lot vert.
4. Rien n'est nuisible tant que rien ne quitte la branche ; master n'a rien reçu de lui dans ce périmètre.
5. À retenir pour les prochaines passations : la passe d'un lot du terminal, c'est le crate, `check_features` et le harnais ; rien d'autre n'est à construire.

## 3. Ce que ça change à la page du 16 (`docs/tty/16-11-2026/02-passation-pour-codex.md`)

- **§ 2.1, les commandes et options en anglais** : fait en branche, à reprendre en lot propre (tty-21). Seuls le code, les tests et le README de `cx-tty-english-aliases-7` sont repris, sur le master courant, dans une branche neuve (`cx-tty-21`). Le message de `--since` passe en même temps. Une page courte, et le ticket ramené à son État, fermé au push.
- **Le harnais instrumenté** s'ajoute à la liste, en lot à part après tty-21, s'il sert : son ticket neuf y est rattaché.
- Le reste de la liste est inchangé : la cause dans le style du thème, la frappe avant l'invite, le mode plain face à un pas relancé, les messages entre sessions, l'agent pair, la copie de l'hôte par un champ.
- Une règle de plus pour une prochaine passation : « La passe d'un lot du terminal est celle du knowledge dump ; pas de contrat, de provenance d'exécutable ni de pilote de qualification. »

## 4. Lisibilité de sa passation

Mauvaise. 241 lignes, écrites pour un lecteur qui aurait vécu ses deux jours : « coupons », « GO », « Common », « F8 », « Qv2/c003 », « P17–P20 », des SHA de manifestes, des PID. On n'y trouve qu'avec peine les trois faits utiles :
- la branche `cx-tty-english-aliases-7` ;
- les alias faits et verts ;
- le correctif de Tab.

Elle respecte en revanche l'arrêt de Lucie : aucun calcul ni nettoyage, rien sur master. Elle dit honnêtement ce qui n'a pas été vérifié.
