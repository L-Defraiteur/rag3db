# L'IDE et le chat : ce qui reste des demandes de Lucie du 7 au soir

Relevé par archi le 8 novembre 2026 vers 12 h 40, à la demande de c7. Sources : le rapport
`07-novembre-2026-orchestration/02-rapport-de-session.md` (de 21 h 45 à 23 h 10), les tickets du
7 novembre et le journal de master. Les hashs sont ceux de master : la branche chat-lisible a été
rebasée, et ses hashs cités dans les tickets (48abbd62d, a03de7d7f) ne sont pas sur master.

| Demande | État | Où |
|---|---|---|
| L'ordre des parties (mots, appel, mots, appel, réponse) pendant le tour et après une relecture | fait | f7cfbef4e (Rust), 34f9fc427 (page) |
| L'heure d'un appel prise à l'appel, celle du résultat à son retour | fait | f7cfbef4e, 34f9fc427 |
| Le défilement : ne suit le flux que collé en bas ; déplier un outil ne le déplace pas | fait | 34f9fc427 |
| Un outil déplié montre son résultat (pas « fait ») | fait | f7cfbef4e (`result_excerpt`), 34f9fc427 |
| La largeur : barres aux bords, colonne fluide, aide et consommation sur leur ligne ; texte des bulles sans largeur fixe | fait | 34f9fc427 (le cadre), 384f6fac5 (les bulles) |
| L'usage du modèle (jetons du tour et des tours récents) et la jauge du contexte sur le maximum déclaré | fait | a5ff087c9, 34f9fc427 (la jauge) ; b129247a9, 2daf2e332 (l'usage) |
| L'archiviste dans la barre, en une ligne par tâche | fait | 34f9fc427 (en direct) ; b129247a9 (après un rechargement) |
| Le chat sur toute la hauteur | fait | 34f9fc427 |
| Sous chaque conversation, le récapitulatif et le bouton qui déplie les fiches écrites ; la conversation active dépliée | fait | e92f71d14 (les fiches), b129247a9, 2daf2e332 (le récapitulatif) |
| Le code coloré, le code en ligne, un chemin cité en lien | fait | 384f6fac5 (A, highlight.js), 5ec68e9a5 (B, les grammaires de l'hôte) |
| Un symbole cité (`code.truc`, `Backend::run`) en lien vers sa définition | **pas fait** | ticket `2026-11-07-le-code-dans-le-chat-n-est-pas-colore-ni-lie` (ouvert) ; il faut résoudre le nom par l'index |
| Les retours d'outils mis en page sans perte, avec le bouton « brut » | fait | 097640ecb |
| La barre : session › dépôts › worktrees (branche, chargé, rag), et les sous-sessions récursives | fait | 056ed09eb (`chat_sessions`, `tasks` avec `parent_task`, `TaskNode` récursif) ; à l'écran, les sous-sessions s'appellent « Tâches », d'après la décision de 22 h 50 (session › tâches › espaces) |
| « tenu par », visible des deux côtés | fait | 056ed09eb |
| Le fil d'Ariane | fait | 056ed09eb (`Breadcrumb`) |
| Le filtre « actifs seulement » et le menu « par session / par dépôt », dont le choix est gardé | fait | 056ed09eb, 2750a19d3 |
| Le vocabulaire : dépôt, worktree, branche, jamais « backend » à l'écran | fait | 73c1edd29, c83409c51 (`kind`, dépôt et branche par l'espace de travail), 932197e79 ; les témoins de la barre le vérifient |
| « Dépôts » et « Disponibles » ; un worktree attaché paraît sans rechargement | fait | 056ed09eb (`available_repos`) ; 1005c5544 (`RepoAttached`) |
| Le thème : l'IDE comme un terminal avant-gardiste, le même thème que le terminal, « Daemon Code Summoning » pendant que le modèle réfléchit | **partiel** | Le terminal lit `templates/themes/daemon.json` (f2e6e2521, 3696a71cb). L'IDE ne le lit pas encore : rien dans `ui/ide/` ni dans `src/ide.rs` (constaté dans 72f17733a). L'IDE garde son clair et son sombre, sans animation pendant la réflexion. |

## À côté (pas dans la liste de Lucie, mais au même endroit)

- Deux tickets ouverts sur ce que le **modèle** reçoit (pas sur ce que l'écran montre) :
  `2026-11-07-le-chat-envoie-au-modele-la-reponse-json-entiere-de-trois-outils` et
  `2026-11-07-la-section-liens-de-search-code-n-arrive-pas-au-modele`.
- `2026-11-07-le-temoin-du-chat-dans-la-page-rougit-a-la-premiere-execution-apres-un-bati` :
  ouvert, chez everything declarative.

## Ce que prend archi

Le premier « pas fait » visible : **le thème commun dans l'IDE**. L'IDE lit
`templates/themes/daemon.json` (servi par l'hôte, mêmes tokens que le terminal) et rend
l'animation « réfléchit » du thème pendant que le modèle réfléchit, dans la page du chat. Lucie a
dit le 8 novembre qu'un thème se branche et se montre sans accord préalable. Le clair et le sombre
restent des choix du menu ; le thème daemon en devient un troisième. Le lien vers un symbole cité
vient après, en lot à part.
