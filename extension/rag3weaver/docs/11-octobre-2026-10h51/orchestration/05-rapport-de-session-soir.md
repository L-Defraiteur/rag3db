# Rapport de la session d'orchestration — 11 octobre 2026, 21 h (soir)

Suite de `02-rapport-de-session.md` (midi). Pour la reprise depuis une session
vierge : lire d'abord `04-les-buts.md` (ce qu'on vise), puis ce rapport (où on
en est), puis `06-knowledge-dump-soir.md` (ce qu'il faut savoir), puis le fil
des décisions dans `01-les-decisions-du-matin.md`.

## 1. Ce qui est sur master depuis midi

| Lot | Session | Hash |
|---|---|---|
| Le mode multi-écrivains par défaut (A9), bascule prouvée par le banc sans écart et par 116 e2e du produit | cœur | `a0a79016e` |
| B2 : le second processus écrivain attend au fichier (`fileLockWaitMs`, 5 s par défaut), garde prise avant toute lecture, C9 retourné | cœur | `0a630b764` |
| A7 (la file des points de reprise), A8 (le dernier à partir écrit), C7 (relations pendantes après arrêt brutal), le ticket des verrous sous index, `CALL build_id()` | cœur, hotfixs | `c0f50f453`, `f024578d5`, `2c7704b27`, `b4655da39`, `48b74ead1` |
| La marche 1 bis du démon en quatre pas (cellule par la requête, index par cellule, réacteur par cellule, migration (a) de l'identité), le filet contre la perte des cellules | arbre principal | `9dd4baedd`, `bac552c22`, `8bb2af2d7`, `cc1254ac9`, `06980485f` |
| Les marches 3, 4 et 5 du démon : client mince, découverte et lancement par le premier client, outil « état du démon », client vers un hôte distant avec son jeton | mémoire | `d25cad18b`, `2b1818d68`, `e60f2342e` |
| Les routes de l'hôte : ouvrir (relit un manifeste modifié) et fermer un backend | arbre principal | `a0f3e3653` |
| La session d'un client mort relevée ; build_id dans l'hôte ; la cellule des bouts d'un lien (`to_cell`) ; review_anchored 40 drains → 1 ; la forme A de l'enveloppe | mémoire | `e9387d545`, `e9d690915`, `bfec631cc`, `cd919b053` |
| Le banc des fiches classées, deux tentatives datées | mémoire | `30ce3aadc` |
| L'échappement systématique des identifiants ; le chaînage des formes (type, vérification, repli, fusion rag3db, saut et CTE PostgreSQL) ; le plancher pg_trgm par la requête ; le voisinage sur PostgreSQL ; le Cypher brut de l'indexation converti (AllOf, In, Detach, crochets) ; impact sur run_plan et la capacité `fanout` (fausse par défaut, mesurée) ; la mesure des 2,2 ms (l'exécution domine, les fils) | F | `4cc23f781`, `76cf86cbc`…`7da725287`, `56a680169`, `17cf90306`, `68dbcfc4b`…`c8fae292a`, `0f62e1d60`, `821d70a63` |
| Le lot 0 de l'exécuteur de flux au complet (le saut, PortKind et Posture, le canal et le détecteur d'interblocage, l'exécuteur concurrent, les points de reprise d'un flux, le sous-graphe relais, le pont vivant) ; check_features en une ligne ; garde du target ; dossiers par suite | recherche | `522fa6780`, `feeb1932b`, `439ca3c9c`, `3f13b1657`, `eb49aba8d`, `0cacde894`, `2306d30f9`, `d432b56d3`, `a39ec19ec` |
| Les agents qui se parlent, lots 1 à 3 (les sept ports de la boucle, Decide et Route, send et l'annuaire) ; la montre de fichiers W1-W3 (source, réaction, retard dit) | proto | `d3aa6f281`, `7b5cda799`, `a73a7941d`, `c1b6ceca1` |
| Les anonymes C++ (287 → 0), le nom seul, l'envoi virtuel (les trois appelants de la question du cœur, « index : racine @ commit » en tête de chaque réponse), les chaînes et commentaires hors références, les réexportations, le compteur des absents (externes / internes manquées) | codeparsers (+ arbre principal) | `66752a44e`, `cad384056`, `faa00f279`, codeparsers `f17202b` |
| Le rendu stable d'impact ; le cinquième jeu de features ; le recompte des tickets par script ; la garde de feature ; l'extension vector par le nom | arbre principal | `b92d809ae`, `eb389901c`, `1fb69401b`, `1a1ee04b0` |
| La course de port_libre des tests et des e2e du démon ; `poste push` (avec `--lourd`, le relais de luciepc, la page) ; l'unité systemd de l'hôte (branche paquet-npm) ; l'hôte sur luciepc | G | `2d3009efc`, `ab1365bc0`, `6c6a12d82`, `277267434`, `3f7ef3102` |
| Le lecteur du journal ; les témoins de B2 et du visited set ; les trois rouges de master en known_red | banc | `c44b444bb`, `82ff8181c` |

## 2. Ce qui est en vol à 21 h

- **La priorité de Lucie** (le worktree de chaque session indexé et à jour) :
  réconciliation de la montre (proto, en push, avec le balayage par dates et
  les trois défauts de luciepc) ; B4 identités relatives (G, écrit et vert,
  derrière son lot A) ; `code-tree add|remove` (codeparsers, sur les routes).
- **Les 6 secondes** : mesure par phase (proto, en tête de sa file) ; le
  rendez-vous par les symboles du fichier et les arêtes externes marquées
  (arbre principal) ; le délai de l'embarquement et le morceau en cours dans
  l'état (arbre principal, en compilation ; un échec avalé devient un ticket).
- **Le planificateur** (cœur) : la surcharge qui protège l'ABI, puis les fils
  proportionnés à la cardinalité (instrument écrit, mesure de F), puis le
  balayage sous masque (71 657 lignes lues pour 117).
- **Le binaire de Lucie** à rebâtir contre la lib B2 (mémoire) avant tout
  redémarrage de son hôte ; le témoin du client qui survit à un redémarrage.
- **G** : lot A (embarqueur absent) en passe, puis B4, B1-B3, C (la marche 4
  portable), archives alpha.3 sans publier.
- **Codeparsers** : les réindexations du soir annoncées (code-lourd et
  code-principal sur luciepc, code-rag3db ici) quand le cœur ne travaille
  plus ; la question du cœur rejouée ensuite ; le ticket du figement (deux
  occurrences à 18 % des vecteurs).
- **Recherche** : batterie complète pour clore le lot 0, puis la page du
  magasin de libs par identifiant de bâti.
- **Mémoire** : tentative 3 des fiches classées quand Decide a `max_tokens`.

## 3. Les défauts trouvés aujourd'hui, et leur état

Fermés : le CREATE TABLE … AS sous le mode ; l'erreur de verrou mal nommée ;
la même fiche dans deux cellules (filet puis migration) ; le plein texte
hors cellule ; le manifeste non relu et le backend abandonné ; la session
d'un client mort ; le plancher pg_trgm par session du pool ; le témoin des
transactions PostgreSQL ; la course de port_libre ; le Cypher brut de
l'indexation ; le voisinage refusé par PostgreSQL ; le rendu d'impact
instable ; l'extension vector relative au gabarit ; les anonymes C++ ; la
garde de feature ; le binaire e2e sans burn-embedder ; deux rouges du corpus
vivant (relevés mis à jour).

Ouverts, à leur propriétaire : le figement de l'index à 18 % des vecteurs
(codeparsers → arbre principal, délai d'embarquement) ; un appel d'outil qui
attend derrière un index sans le dire (proto, arbre principal) ; l'arrêt de
l'hôte qui attend la fin d'une réaction, la racine disparue, « catalogue
occupé » (proto, en push) ; la montre à 0,7 cœur au repos (proto) ; e2e_impact
et e2e_usages qui dépendent de la forme du code vivant (codeparsers / arbre
principal, plus tard) ; search_code confiant sur un jeton absent, le nom
qualifié, les tests par requêtes non suivis, la portée (arbre principal) ;
l'hôte qui hérite de l'environnement du premier client (arbre principal) ;
la marche 4 qui ne tient que sous Linux (G, lot C) ; l'ABI changée par B2 et
le contrôle de bâti qui ne couvre pas le binaire (cœur) ; l'index vectoriel
sous écrivains simultanés (hotfixs, correctif écrit) ; un fichier Rust 2,6
fois plus lent à la ligne (codeparsers) ; les trois rouges de master en
known_red (banc).

## 4. Les règles nées aujourd'hui

Le tour de push (`poste push`, `--lourd`, quatre règles, le relais de luciepc,
la dispense des docs, la boucle sous le verrou) ; « poussé » après trois
contrôles ; master lu par le FETCH_HEAD de son propre fetch ; vert = aucune
ligne ROUGE et le vrai code de sortie, jamais un compte ; check_features à
cinq jeux et une ligne finale ; check_tickets avant tout push de docs qui
touche l'index ; un sous-dossier de target par arbre et la garde du target ;
`git submodule update` après tout rebase ; un rebâti de la lib commune passe
par codeparsers (l'hôte arrêté avant, relancé après) et dit « ABI changée »
quand l'API publique bouge ; un redémarrage d'hôte s'annonce aux sessions qui
l'interrogent ; aucune réindexation entière d'un backend qu'une session
interroge pendant qu'elle travaille ; un worktree et sa base vivent et
meurent ensemble ; aucun arrêt brutal sans Lucie (trois fois ce soir, à sa
main, sur preuve) ; les fiches mémoire en régime d'expérimentation.

## 5. Les fautes de l'orchestration aujourd'hui

- Une réindexation entière de `code-principal` lancée sous le cœur pendant
  qu'il travaillait (deux minutes de blocage) : règle posée.
- Deux hypothèses dites avant la mesure et tombées : « le relevé par le nom
  balaie » (c'était le planificateur) ; « codeparsers réanalyse tout le
  projet » (il n'analyse que le fichier). Les deux dites comme hypothèses,
  les deux retirées à la mesure.
- Deux erreurs d'adressage de sessions (banc / G ; banc de concurrence /
  banc des fiches) : la table des noms est dans le knowledge dump.
- Un pari sur l'ordre (réindexer après le binaire neuf) qui a coûté deux
  arrêts forcés à Lucie : la règle de l'annonce et de l'heure convenue.

## 6. Ce qui attend Lucie au réveil (mis à jour le 12 à 1 h 15)

- **Tes sessions d'ici marchent** : ton hôte a été redémarré à 0 h 26 sur un
  binaire avec la montre, les défenses contre la boucle et le client qui se
  rétablit (md5 3e95f0302023) ; sain à 1 h 01. **Un geste de toi au réveil** :
  la mémoire n'a pas eu la permission d'arrêter ton hôte à 2 h 56 ; la
  séquence (arrêt propre, binaire ≥ B4 avec le correctif du balayage,
  code-rag3db effacé et réindexé, redémarrage) attend ton `kill -TERM` sur
  le pid de l'hôte, commande exacte dans le rapport de la mémoire
  (`419b87a36`), ou ton autorisation dans sa session.
- **Le P1 (20 % de morceaux « sans texte ») est résolu dans sa cause et tes
  textes sont intacts** : un balayage filtré des chaînes du moteur rendait
  vide ou faux (string_column.cpp:240) ; correctif du cœur en cours, lib à
  rebâtir ; c'était aussi la cause de la boucle de luciepc.
- **luciepc, dans l'ordre** (section « Pour Lucie » du rapport de codeparsers,
  `docs/10-octobre-2026-18h20/codeparsers/01-rapport-de-session.md`, et la
  note de hotfixs `ba63aa135`) : terminer le client d'index 132918 (libère
  la place du poste, la mesure de hotfixs passe seule) ; arrêt propre de
  l'hôte figé (`systemctl --user stop rag3weaver-daemon`, pile gdb
  facultative) ; compter base fermée les Scope_Chunk sans texte de
  code-lourd ; remettre la montre ; installer le binaire prêt de G (≥ B4) ;
  **effacer puis réindexer** code-lourd et code-principal ; relancer, /sante ;
  hotfixs rebâtit la lib commune après l'arrêt.
- **Deux questions** : (1) un morceau sans texte doit-il exister ou ne jamais
  naître (ticket P1 `6cdef9585`, 20 % du code, texte perdu à l'écriture,
  cause chez l'arbre principal) ; (2) la forme en deux temps d'usages
  (page 06 de F) si le lot des fils ne suffit pas : décidée « oui si > 100 ms »,
  renversable.
- La séance npm (quatre OTP) quand tu le voudras ; le ménage des branches
  distantes (liste à produire).
- Tout le reste est dans le fil de nuit de `01-les-decisions-du-matin.md`
  (depuis « 21 h – 21 h 40 ») et dans `06-knowledge-dump-soir.md` §2.
