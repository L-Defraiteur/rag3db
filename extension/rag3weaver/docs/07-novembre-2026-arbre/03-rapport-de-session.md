# Rapport de session — l'arbre principal, 7 novembre 2026

Session « arbre principal » (73). Aucune consigne de Lucie dans la session : la direction vient
de c7 (orchestration) et des pairs (le cœur, hotfixs, G, mémoire, archi, recherche, everything
declarative). Ce qui suit est ce qui est fait, poussé et vérifié, puis ce qui attend.

## Sur master

| quoi | commit | vérifié par |
|---|---|---|
| `whois { agent \| handle \| alias }` dans le gabarit memory (page 04 §8, pièce 3), relu par 96 | 393d4b0c0..2f4ed7841 | e2e_whois, e2e_cited_by, e2e_plan_declare, lib entière sous le jeu large, check_features 5/5 ; navigateur 11/12, le témoin du chat rejoué seul vert (ticket du 7 nov.) |
| la procédure de bascule : le contrôle des deux sens (étape 8), `e2e_lectures` au verdict, l'essai de l'IDE avant le posé, la borne dans le lourd | 9dc626c70, b99b67de0, cf14a7771 | le contrôle joué sur l'instantané du 25 oct. (écart connu retrouvé), rc 7 sur un dossier vide |
| la lecture « ce que le moteur doit au modèle des vues » | 9bd377476, ddb856e21 | — (une lecture ; M1 mesurée sur git) |

Et la **lib unique de la bascule 43** (page 02 de ce dossier) : bâtie, nommée, ABI lue, verdict
vert, copiée aux deux postes, « fini » donné à G à 19 h 12.

## Sur des branches

- `arbre-principal/juge-claude-code-2` (ec329009e) : le juge d'un tour de Claude Code — ses appels
  de l'hôte lus par leur serveur (`mcp__<serveur>__<outil>`, table `serveurs` du journal), les
  outils propres de Claude Code laissés, le coût « non mesuré », la fiche prise par `whois
  {handle: session}`. Témoin de bout en bout ajouté à `e2e_juge`. Va sur master **après la
  bascule 43**, par une passe ordinaire.
- `arbre-principal/mesure-vues` (b826a1147) : le témoin `e2e_mesure_vues` (M3 à M5, en Cypher brut
  sur une copie de base).

## Ce qui a été appris

- **La recherche dense filtrée ne tourne pas sur une base ouverte en lecture seule** :
  `PROJECT_GRAPH_CYPHER` y est refusé (« Cannot execute write operations in a read-only
  database! »), pour la même raison que sous `READ ONLY` (un `CALL` autonome est classé écriture).
- **Une mesure en attente bloque la file** : `poste mesure` a la priorité exclusive même en
  attente ; à luciepc, sept lourds attendaient derrière la mienne. Retirée à la demande de c7.
- **Deux tickets du même défaut** : le sparse filtré résout ses offsets sur le parent et cherche
  dans les morceaux (lu dans le code, confirmé par recherche, ticket de recherche 2d7f11067) ;
  le mien, en double, retiré (ddb856e21). Mon script de push de docs rejoue `check_tickets.py`
  après le rebase.
- **Une vue se définit contre la base de sa tâche** (M1 sur git, tranché par everything
  declarative) : 0 à 8 chemins contre la base, 5 à 53 contre le master du moment.
- **Un contrôle qui ne trouve rien à juger est rouge** (hotfixs) : le contrôle des deux sens
  compte ses bases.
- Les défauts du hook signalés à archi (le `cited_by` préfixé par le genre, la source unique des
  handles) : corrigés dans son lot 3.

## Ce qui attend

1. La pose de la bascule 43, par G (étapes 6 à 11) ; puis le lot du juge sur master.
2. Les mesures M3, M4, M5 des vues : seulement sur une file vide (luciepc après 22 h, au mot de
   c7, ou ici demain), sous 17a094d12 une fois le format 43 en service, sur les copies reflink
   déjà prêtes à luciepc (`~/.local/share/rag3weaver/mesure-vues/lecture` et `m5-plus-tard`).
3. Le `said` de whois qui cite la valeur demandée, au pas 3 de 96 (RhaiNode à port `value` qui
   s'ajoute aux champs par clé).
4. Le prototype vertical de dimanche, avec recherche et everything declarative.
5. En fin de session : effacer wt-juge, target-plan-local ici ; target-A-plan, wt-lib-arbre,
   wt-mesure-vues et les copies de mesure à luciepc (en le disant, la taille rendue ici).
