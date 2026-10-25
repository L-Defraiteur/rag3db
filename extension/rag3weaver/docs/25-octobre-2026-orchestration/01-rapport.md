# Orchestration — rapport du 25 octobre 2026

*Session rag3db-c7 (« orchestration sessions »). Reprise à 13 h après six jours
d'arrêt ; écrit à 19 h 40, contexte presque plein. Le fil complet est dans
`docs/11-octobre-2026-10h51/orchestration/01-les-decisions-du-matin.md`
(entrées « 25 octobre »), et les capacités atteintes dans `docs/nouveautes.md`.*

## Le point de départ

Master était resté à dcbaef7b1 (19 oct., 8 h 41) : toute la file du 19 était
dans les worktrees des deux postes, jamais poussée. Toutes les sessions ont
été relancées de zéro à 13 h, sans mémoire de travail. La file a été reprise
dans l'ordre du 19, un lot à la fois, chacun rejoué sur le master du moment.

## Ce qui est sur master ce soir (par ordre d'atterrissage)

Crate rag3weaver : l'IR du Plan et le PlanNode (c1e6f7dd1) ; les échecs
d'index nommés (f6d19317a) ; les quatre lots de la mémoire (b6270fc19,
2c44d9f38, 02454ac98, c3fdcb979) ; l'allègement des langages (610d41cf4) ;
le contournement 1 Tio retiré et les planchers du magasin (2fef5fb90) ; C4,
un seul chemin d'arrêt (95c985c4c) ; le chat de l'atelier dans l'IDE, K1-K3
(a13b22417, 657e568bd, a454d6a91) ; la pagination de l'IR (5881f809e) ; le
vérificateur de planchers (82c4f7ff9, 0ddb28945) ; le retrait du filet
copier_les_liens (388f2260b) ; le patch lucivy temporaire (44c790c20) puis
son ré-épinglage sur 4.3.1 (277f8b11d) ; le témoin du chat et le relais de
l'IDE sur ses fils (067bb8814, 8ef9791dd) ; outline, premier Plan déclaré
(89b5d5dbd) ; l'ordre par extrême de l'IR (beda3d8f1) ; cited_by
(665a4a97c) ; le repli de la recherche (d6abc051f) ; l'embarqueur qui
retente et les comptes du plein texte (2a5a38cb0, 4d1a4a0ee) ; la ligne d'un
Plan refusée en la nommant (9b7f09e70) ; le schéma d'un paramètre liste
(195358024) ; tree-sitter 0.27 et la sonde PG (dfb706b07) ; la pièce 1 de
la recherche de l'IDE (0427c2c2d), le reclassement à la demande (1c430cdfc),
la page de recherche (b0e5451d0) ; R1, le budget de raisonnement (527a0356c)
; le rendu JSON des Plans et la vue outline (cbb41d312) ; le JS de l'IDE en
fichiers (ca53f8e3f) ; la mémoire en fichiers (d72bcf5e0), la version du
format et le manifeste du dossier (bea6c6a98) ; le chat qui code dans une
tâche (ae127d634) ; l'étape navigateur (3ec37ef1e).

Moteur : les trois correctifs du 19 (EXISTS-paramètre 8b98f19f7, COPY NULL
dcbaef7b1, COMMIT sur transaction partie 58b46a9bf) ; le format de stockage
41 (f84433579) et les colonnes stables, format 42 (4ae060070) ; le
rattrapage HNSW (6fe9ffe9b) ; les requêtes préparées qui portent leur
préparation (fb4d83b65 : 7,1 Go au lieu de 30 pour quatre lecteurs) ; la
ligne MSVC/AppleClang (04063ee06, qui rouvre Windows et macOS) ; V2-1b pièce
1, le segment neuf au point de reprise (6b63e4922, amplification mesurée :
neutre sur une ligne, gagnante à 1 % et 10 %) ; le cassert de geo et le
contrôle qui exclut le code vendu (9bf7aca5c, 0aebf4703).

## Les deux bascules

- 17 h 41-17 h 43 : lib unique 04063ee06 (format 41/42, requêtes préparées,
  ligne MSVC), ABI changée ; hôtes bâtis contre elle, essayés sur 7991,
  arrêtés proprement, instantanés reflink des quatre bases partagées
  (memoire-longue, code-rag3db-depot ; code-principal, code-lourd),
  réveil sur RAG3DB_LIBRARY_DIR explicite, IDE de Lucie vérifié au
  navigateur, `courant` posé ensuite aux deux postes après la fin de la
  dernière passe qui le lisait. code-rag3db-depot est en 42 depuis 18 h 32 ;
  memoire-longue passera en 42 à son premier point de reprise. Retour
  seulement par RETOUR.sh.
- 20 h 30 (prévu) : rebâti des deux hôtes sur 3ec37ef1e, même lib (lucivy
  4.3.1, outline, page.js, page de recherche, chat qui code), sans
  instantané ; G a l'essai vert aux deux postes.

## Les règles posées aujourd'hui (toutes en mémoire)

- Chaque commit se pousse dans la minute sur une branche à son nom ; master
  seul attend le vert (Lucie).
- Les docs d'orchestration s'écrivent directement dans l'arbre de Lucie ;
  jamais un Write dans un worktree (le fichier s'ouvre dans son IDE).
- La lib entière d'un tour se joue sous le jeu de features le plus large,
  openai-llm compris (1 841 tests contre 1 761).
- Une passe épingle RAG3DB_LIB ; une bascule ne change rien sous elle ;
  après un C++ du cœur, MOTEUR_ANCIEN=1 avec la lib épinglée, dit dans le
  commit ; la lib se rebâtit par bascule, pas par commit.
- Un rouge connu sur master nu, ticketé et étranger au lot, ne bloque pas le
  push s'il est dit ; un known red devenu vert n'est pas un rouge.
- Les témoins navigateur sont une étape de passe à part.
- Un worktree par script lourd ; un target par passe.
- Aucun modèle d'essai à côté du vrai sur la carte de luciepc ; une bascule
  de service se fait en place, parité sur un vecteur noté.
- Pas de compilateur dans le produit ; clangd en oracle de mesure, étendu à
  Rust, TypeScript, Python ; mais le juge avait un défaut (chemins de deux
  arbres) et tout est en rejugement.
- docs/nouveautes.md : une ligne par grande étape, tenue par c7.
- cleanupPeriodDays = 3650 sur les deux postes (les sessions lucivy avaient
  été effacées à 30 jours).
- Les tunnels d'embarquement du portable sont une unité systemd --user.

## En cours à 19 h 40

- G : rebâti des hôtes à 20 h 30 ; alpha.3 sur 277f8b11d (trois
  plateformes, en cours ; la série d'avant était verte ×3 sur 04063ee06).
- Mémoire : migration de la fiche de Lucie après le rebâti, dossier à
  choisir par Lucie (`~/memoire-longue/` proposé) ; l'archiviste (M4)
  ensuite.
- Everything declarative : le juge de session (journal des runs, agent
  déclaré, fiche de bilan, déclenchement par `bilans`) ; restent la vue
  « bilans » et le témoin Gemini ; puis le banc qualitatif.
- Codeparsers : rejugement complet d'INCLUDES avec un juge corrigé (la
  définition elle-même, sans règle) ; INCLUDES hors de master ; la mesure
  7e/9e sur le portable à minuit, à mon mot.
- Cœur : V2-1c (le point de reprise ne fige plus les lecteurs), passe en
  cours, verdict vers 20 h 30 ; puis V2-2.
- Hotfixs : le CREATE concurrent (A), lots 1+3 fondus, format 43 pour une
  prochaine bascule, parade du crate relue par recherche.
- Recherche : prochain lot à proposer demain.
- 21 h 30 : fenêtre calme de luciepc : embarquement (unités, 7878/7880/7879
  en place), banc (pente ×3, lignes hors du graphe, vingt clients), hotfixs
  (témoin de (A) ×3, rattrapage ancien/nouveau ×3).

## À Lucie

- Le dossier de ses fiches (`~/memoire-longue/` ?).
- lucivy : fusionner 4.3.1 dans main (déploie la démo corrigée) ; le tag
  v4.3.1 publie, après le rouge Python réglé (sa session lucivy l'a fait).
- Le reranker reste à la demande ; le défaut se décide sur les chiffres
  (+0,5 s, gain inégal).
- Ouvrir ou non la liste blanche des réactions aux agents (le juge est
  déclenché par l'hôte en attendant).
- Le chat qui code : jamais d'écriture hors tâche, pas de commandes ; le
  chat n'ouvre pas de tâche lui-même.
