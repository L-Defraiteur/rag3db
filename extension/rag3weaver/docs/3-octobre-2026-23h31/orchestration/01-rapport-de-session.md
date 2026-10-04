# Orchestration — rapport de session

Mis à jour le 5 octobre 2026 à 0 h 20. Ce fichier se met à jour sur place.

## 1. Le rôle, et l'adresse

La session d'orchestration cadre les autres sessions par messages, relit par
git, tient le journal des décisions et ne remonte à Lucie que les vrais choix.
Lucie délègue les choix réversibles (« prendre le meilleur chemin possible et
laisser la voie aux autres options »).

**Adresse** : depuis le 4 octobre vers 15 h (session reprise en fork),
l'orchestration répond à `rag3db-76 [ccdb1b]`, plus à `rag3db-9f`. Le banc
porte le même nom avec un autre repère : `rag3db-76 [155bbd]`. Toujours écrire
le repère.

## 2. La priorité : la stèle du moteur

Décision de Lucie, 4 octobre : le moteur a un point d'arrêt écrit
(`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`), puis « on se focus
sur la stèle ». Quatre conditions ; l'ordre a été changé le soir même par
Lucie (« on l'a attaqué du bon côté ? ») : le chargement journalisé passe
avant le câblage des verrous.

| Condition | État au 5 octobre, 0 h 20 |
|---|---|
| plus de défaut connu qui corrompt ou qui perd | neuf tickets fermés le 4 ; reste l'ordre de synchronisation au point de reprise (banc) et le vrai remède du COPY après des insertions (refus nommé posé en attendant) |
| le chargement en masse journalisé | page acceptée ; étape 1 (mesure) et étape 2 (nœuds) sur master `0f4a54b2c`, derrière `force_checkpoint_on_copy=false` ; étape 3 (relations) en cours |
| les verrous | V1 (le gestionnaire seul) sur master `022c78402` ; A3′, A4′, V2, index au commit après le chargement journalisé |
| les écritures parallèles | en dernier |

Deux sessions travaillent le moteur : le cœur C++ (le COPY, puis les verrous)
et le banc (les tickets bloquants, les témoins). Elles se disent leurs
fichiers, un seul rebâti exclusif à la fois, relecture croisée de ce qui
touche la reprise.

## 3. Ce que le 4 octobre a trouvé dans le moteur

Tous d'origine (présents chez Vela ou Ladybug), sauf mention :

1. Recréation d'index vectoriel bornée par une cardinalité estimée
   (`1ea49837f`).
2. Annulation d'un ajout : le compte du bloc revient, pas ses colonnes ; le
   point de reprise suivant déborde du tas (`05788a868`).
3. Après un COPY refusé, la clé d'origine sort de l'index de clé primaire
   (`05788a868`, puis `5c8507577` — le premier correctif avait créé des clés
   fantômes : une régression à nous, attrapée par le test du ROLLBACK).
4. Point de reprise après `ALTER TABLE … DROP` : identifiant de colonne pris
   pour une position (`308ebd17e`).
5. Balayage de plusieurs tables de relations en transaction (`88d937160`).
6. Une erreur rendue sur une écriture validée, et un COPY validé puis perdu à
   l'arrêt : la validation tenait le verrou public pendant l'attente du point
   de reprise (`68ff9d5e2`).
7. **La corruption rare d'e2e_code** : deux exemplaires de la même base
   ouverts en écriture dans un même processus (le verrou `fcntl` n'exclut
   qu'un autre processus) ; l'ancien exemplaire était retenu par les acteurs
   de lucivy. Garde du moteur `57c8389b4`, filet de rag3weaver `ad220d0eb`.
   La « réouverture intermittente sous charge » était le même défaut.
8. Arrêts au mauvais instant : ordre des suppressions à la reprise, fsync
   après le rejeu (`a86d4e6a8`).
9. **Insertions puis COPY dans la même table et la même transaction : écriture
   sur la mauvaise ligne, validée, silencieuse.** Refus nommé `0f4a54b2c` ; le
   vrai remède après l'étape 3. Les relations, quatre formes éprouvées :
   justes.

## 4. Le produit

| Sujet | État |
|---|---|
| premier index de ce dépôt, sur disque | 753 s → 110 s (médiane, transaction par paquet K = 4) ; 95 s en mode « plein texte hors de la base » sur un poste calme. Cible de Lucie : 90 s. |
| transaction par paquet | derrière sa variable, **mesures seulement**. Conditions de l'allumage par défaut : le test de comparaison ligne à ligne (sous transaction / sans, reprise comprise) — en cours chez l'arbre principal. « Mêmes comptes » ne prouvait pas « même graphe ». |
| plein texte hors de la base (`FtsStorage::Files`) | étape A sur master `16ab78413` (mémoire ÷ 1,7, réouverture ÷ 4, 4 Gio de tampon passe) ; étape B (marque de génération, reprise) en cours ; le défaut reste en base |
| section « Liens » | allumée, en arbre (`~ Consumes ~`), filtre des arêtes devinées |
| graphe de code | marque `resolution` sur les arêtes ; « nom » 11 769 → 9 210 ; filtre allumable dans impact (branche `filtre-impact` à fusionner) |
| mémoire longue | références (`add_ref`, `create_ref_type`) de bout en bout ; graphes réactifs au manifeste à proposer |
| modèles de décision | **mis de côté par Lucie** (4 octobre) ; essai « note d'utilité » négatif |

## 5. Les visions

Réunies dans `extension/rag3weaver/visions/`, datées dans leur nom, avec
`00-vision-generale.md` (l'idée, les étages, ce qu'on vend, les principes, la
largeur, la stèle). Une vision nouvelle va là. Rien n'y est décidé sauf ce qui
est marqué « retenu par Lucie ».

## 6. Ce qui attend Lucie

- Le changement dans lucivy pour une fermeture synchrone (demi-page :
  `3-octobre-2026-23h31/arbre-principal/03-lucivy-fermeture-synchrone.md`) —
  plus nécessaire depuis le filet, reste une amélioration.
- Retirer `content_boost` et `special_ops` (recommandé).
- L'allumage par défaut de la transaction par paquet, puis du mode hors
  base : à lui représenter quand leurs preuves sont complètes.
- Toujours en attente depuis le 3 : la réécriture de l'historique
  (`5ea14e1aa`, à lancer par elle), « envoie » ou non pour tracel-ai, le
  ménage des branches distantes, le statut du dépôt.

## 7. Méthode : ce que la journée a corrigé

- **Une estimation de session en « jours » ne se rend pas telle quelle** : la
  compter en passes (rebâtis, listes C++, passes instrumentées, mesures).
- **Une mesure se rend en médiane sur trois passes alternées**, avec l'état
  du poste ; un meilleur chiffre isolé (97 s) n'a pas tenu.
- **« Mêmes comptes » n'est pas « même graphe »** : comparer les lignes et
  leurs propriétés.
- **Attaquer la cause, pas le symptôme** : le COPY hors journal était derrière
  le point de reprise forcé, les défauts de l'annulation, la perte après
  délai et un cycle avec les verrous.
- **Pour une corruption, obtenir le fichier abîmé et lire ses pages** avant
  une quatrième hypothèse ; et vérifier d'abord qu'aucune réouverture n'a eu
  lieu pendant qu'un exemplaire vivait.
- **Pousser après un rebasage qui apporte du code sans rejouer** : trois
  écarts le 4, trois sessions, rattrapés dans les minutes — la règle reste.

## 8. Comment reprendre

1. Lire la stèle (§3 : le tri et l'ordre), puis `docs/journal-des-chantiers.md`.
2. Lire ce dossier, puis celui de la session dont on reprend le sujet.
3. `git log origin/master --since=<dernière lecture>` : tout se livre sur
   master, en avance rapide, sans force.
4. `ListAgents` pour les noms du jour ; relancer une session au repos par un
   message qui nomme son prochain lot.
