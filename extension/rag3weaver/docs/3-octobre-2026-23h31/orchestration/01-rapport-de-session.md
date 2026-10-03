# Orchestration — rapport de session

Mis à jour le 3 octobre 2026 à 23 h 35. Ce fichier se met à jour sur place.

## 1. Le rôle

La session d'orchestration cadre les autres sessions par messages, relit par
git, tient le journal des décisions et ne remonte à Lucie que les vrais choix.
Depuis le 3 octobre au soir, Lucie délègue les choix réversibles (« prendre le
meilleur chemin possible et laisser la voie aux autres options ») et laisse
les sessions travailler seules plusieurs heures.

## 2. Où en est chaque chantier

Les noms de sessions changent à chaque relance ; se fier au sujet.

| Sujet | État | Suite |
|---|---|---|
| **Cœur C++** | T0, A2, A5, A5 bis livrées. `DROP_VECTOR_INDEX` au rejeu livré (`f5acca417`). **Garde 1** de la reprise avec index d'extension prouvée (`d21045c40`, locale), en passe Rust avant livraison. | Vérifier si « la mise à jour d'un vecteur perd des lignes dans l'index » couvre le `SET` depuis NULL (le chemin principal de rag3weaver) ; garde 2 ; les deux défauts de rappel du HNSW ; puis V1 (gestionnaire de verrous, ressource générique, genre « index »), A3′, A4′, V2, maintenance de l'index au commit ; H4 ; port de l'opérateur de Ladybug pour `UNWIND … MATCH`. |
| **Banc de concurrence** | Témoins des verrous (§6 de la note), cas du verrou d'index, famille « arrêt brutal, un seul écrivain », marche « reprise avec index d'extension » : tout sur master (`80e2e810a`). À l'arrêt. | Reprendre quand V1 arrive, ou pour un nouveau témoin. |
| **Arbre principal** | Synchronisation par source, chargement en masse des relations, seuil du COPY à 200, édition pendant l'indexation, pile codeparsers 4 → 7, réécriture des 17 requêtes par lot (`b3db4244c`), replis muets comptés (`218faf854`). | Accès de champ et pointeur 8 de codeparsers ; **report de la poussée des blobs plein texte** avec sa marque durable ; reconnaître « is behind its table » à l'ouverture et rebâtir ; renommer le test de chargement interrompu. |
| **Embarquements** | Service distant (trois modèles d'embarquement, modèle de décision, petit modèle de langage sur l'autre poste), `estimate`, `index`, états d'index (global et par entité), modèles déclarés lots 1 à 4, profils d'ingestion. | Lot 5 (routes de démon du relecteur et de l'OCR) ; remesure du dépôt entier après chaque gain. |
| **Recherche** | Backend de code (deux manifestes), politique par outil, recherche adaptative, garde des liens, faille `run_command` fermée, passes d'agent (faible et Gemini) jouées et écrites, crochet après outil lot 1. | Client « le même motif existe ailleurs » ; branche `sparse` dans les graphes de gabarit ; lot 6 des modèles (LLM du chat) avec la session embarquements. |
| **Codeparsers** | Genre d'usage, fausses arêtes, déterminisme, appels typés, fonctions de module Rust, marque de test, blocs de test TS, imports, `usages`, `impact`, banc des relations. | `LinksNode` (liens entre résultats, par le crochet après outil) ; `CohesionNode` (poids 0 par défaut), mesuré au banc. |
| **Mémoire longue** | Proposition, banc scripté, gabarit `memory` (zéro Rust), l'ingestion émet ce qu'elle change ; suite `e2e_arret_brutal`. | Attendus de la garde 1 ; réacteur « à revoir » (lot 4) ; sujets comme entités dérivées. |
| **Optimiseur** | Cinq documents sur les modèles de décision ; arrêté. | Attend « envoie » de Lucie pour tracel-ai. |

## 3. Les faits graves de la soirée, à ne pas perdre

1. **Une base à vecteurs peut planter à l'ouverture après un arrêt brutal.**
   Cause : un point de reprise vide le journal, enregistrement `LOAD EXTENSION`
   compris ; le rejeu rencontre alors un index HNSW connu mais pas chargé.
   Confirmé par le chemin de rag3weaver. Garde 1 en cours de livraison.
   Tant qu'elle n'est pas sur master et reprise par rag3weaver : arrêter les
   backends proprement, ne pas toucher à la base MTG.
2. **Nos tests de reprise ne tuaient rien.** La variante Crash du banc fermait
   la base avant de tuer ; aucun test Rust ne fait « écrire, mourir, rouvrir
   dans un autre processus ». Règle : une reprise se prouve par SIGKILL,
   journal vérifié non vide, réouverture dans un processus **neuf**.
3. **`UNWIND … MATCH` sur une clé prise dans une structure balaie les tables.**
   Contourné dans rag3weaver par `dialect::unwind_par_cle` ; garde-fou
   `e2e_plans_par_lot`.
4. **Un agent fort contourne un refus par un autre outil.** `run_command`
   laissait lire hors du domaine ; fermé, avec la limite écrite : la vraie
   frontière est un bac à sable.
5. **Un outil qui lit le catalogue ne rend jamais un vide sans dire d'où il
   vient**, et l'état d'index se lit par entité (le journal de conversation
   faussait l'état global).

## 4. Ce qui attend Lucie

- L'ordre du cœur C++ : garde 2 et défauts de rappel du HNSW avant les verrous
  (recommandé).
- La ligne d'état en tête de fiche plutôt qu'en avertissement (recommandé).
- L'affichage de la marque de test dans les résultats.
- Remesurer le couple de fusion 0,45/0,55 sur la nouvelle référence.
- La section « Liens » par défaut ou non (exemples à venir).
- « Envoie » pour tracel-ai, dans la fenêtre de l'optimiseur.
- La demande au support GitHub ; le ménage des branches distantes.

## 5. La cible d'indexation

Lucie, 23 h 30 : **1 min 30 au plus** pour qu'un dépôt comme celui-ci soit
cherchable par mots. Mesures : 1 798 s → 523 → 499 → 352 s. Premier poste
restant : la poussée des blobs d'index plein texte (119 s).

## 6. Comment reprendre

1. Lire `docs/journal-des-chantiers.md`, §4 « Décisions du 3 octobre au soir »
   et §6.
2. Lire ce dossier, puis celui de la session dont on reprend le sujet.
3. `git log origin/master --since=<dernière lecture>` : tout se livre sur
   master, en avance rapide, sans force.
4. Relancer une session au repos par un message qui nomme son prochain lot ;
   une session peut s'arrêter sur un compte rendu sans le dire.
