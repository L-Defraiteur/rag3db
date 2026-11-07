# Rapport de la session hooks-essai (7 novembre 2026)

Session Claude Code d'essai des hooks de rag3weaver vers la mémoire longue,
qui a aussi porté des lots produit pour Lucie, sous l'orchestration de c7. Le
fil complet, tour par tour, est dans `01-journal-de-la-session.md`.

## Ce qui est sur master

| hash | lot | ce que ça change |
|---|---|---|
| e2d5c9c88 | chercher sans dossier | la recherche de l'IDE se tape dès l'accueil, le dossier n'est qu'un filtre (`scope`) |
| 3d09b32c2 | journal de l'hôte, chat Rust, page du chat (27 commits) | `daemon.log` horodaté et attribué ; `at_ms`, `result_excerpt`, `role`, `context`, `memory_writes` ; la page en parties, le défilement, la barre « Dépôts / Mémoire / Projets » |
| 23757c7d6 | premier-jeton (11 commits) | le flux de l'IDE ne perd plus de trame ; `kind` des backends (gabarit, puis repli) ; agent_ports sans horloge |
| e72044b41, 178b5dbbe, 95ff12238 | nouveautés | une ligne par lot |

**En file** (en dernier, après le pont d'archi) : `appel-a-delai` (350eaabe7),
`deadline_ms` sur `/call`.

## Les tickets

- **Fermés : 11.**
  - le journal de l'hôte ;
  - les sept tickets du chat (ordre, heure, scroll en flux et au dépliage,
    résultat déplié, écran large, usage et archiviste) ;
  - le premier jeton perdu en direct ;
  - la recherche continuée après l'abandon de son client (dans
    `appel-a-delai`).
- **Ouvert : 1.** « Un arrêt ne coupe pas une requête du moteur en cours ».
  C'est une conception (une connexion par run), écartée de ce lot par c7.

## Les hooks (le double but de la session)

- **Premier binaire** : un bloc d'identité utile, puis un rappel en timeout à
  chaque tour (`timeout: global`, 5 s pour des appels en série).
- **3 ter (a)** : un rappel rapide (79 à 334 ms), mais une extraction de requête
  qui ratait le sujet. L'échelle était jugée avant l'exclusion de ma fiche, d'où
  un bloc hors sujet, et le déjà-vu ne comptait qu'après le seuil.
- **3 ter (b)**, posé à 18 h 48 :
  - un bloc juste (« régime GPU », plein texte 0.93) ;
  - des silences qui disent leur raison ;
  - des tours de notification muets ;
  - aucun bloc hors sujet ;
  - aucun appel du juge différé : la mémoire n'a que deux fiches, dont la
    mienne, exclue.
- **La limite de l'essai** : la mémoire à deux fiches. Rien du bloc n'a servi
  au code ; le repérage est venu d'agents, et les accords de messages entre
  sessions.
- **Le bruit restant** dans la requête extraite : mon propre nom, « c7 », les
  nombres, les mots de consigne. Transmis à archi.

## Les leçons

- **Rouge d'abord, toujours.** Deux rouges mal montés m'ont appris à vérifier
  le rouge lui-même : un dossier manquant, un filtre de test qui ne visait pas
  le module. Le vrai rouge s'est pris sur une branche de preuve.
- **Un flottement se mesure avant de se corriger.** La page en parties perdait
  un jeton une passe sur deux, l'ancienne jamais en trois. Le relevé des rangs
  reçus et posés a désigné le flux, pas la page, contre deux hypothèses
  plausibles.
- **La forme réelle d'une configuration bat la forme du banc.** La pose chez
  Lucie a montré un manifeste hors dépôt, que le banc ne reproduisait pas.
- **Le verrou de push ne couvre que le dernier pas.** Mon premier script
  bloquait sur tout code arrivé ; la procédure ne demande de rejouer que pour
  du code sous `extension/rag3weaver`.
