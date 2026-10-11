# La stèle du produit — ce qui doit être vrai de rag3weaver

*Arbre principal et codeparsers, 11 octobre 2026. Décision de Lucie, le même jour : la stèle du
moteur (`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`) se juge sur ses propres tickets. Les
tickets « perte ou réponse fausse » du produit vont dans une stèle à part : « c'est une stèle
différente de celle du moteur ».*

Le produit, c'est ce qui vit dans `extension/rag3weaver`, codeparsers, lucivy et les outils : la
table « Ouverts — produit » de `docs/tickets/README.md`. Comme pour le moteur, la stèle est un
point d'arrêt. Quand elle est tenue, on ne revient au produit que sur un défaut **constaté**, plus
par précaution.

## 1. Ce qui doit être vrai

| # | Condition | Ce que « fait » veut dire |
|---|---|---|
| P1 | **Rien n'est rendu comme sûr qui ne l'est pas** | chaque lien, usage ou impact dit comment il a été trouvé (par le type, dans le fichier, par le nom) ; un lien deviné ne passe jamais pour un lien lu |
| P2 | **Aucune perte à l'indexation ni à la synchronisation** | ce qui est écrit est relu ; un arrêt, propre ou brutal, se reprend aux comptes d'une passe sans arrêt ; une écriture ne tombe pas dans une autre cellule ou un autre état |
| P3 | **Aucun refus muet** | une écriture qui n'a pas eu lieu ne rend pas `Ok` ; une erreur du moteur n'est pas avalée par un repli ; un échec a un lecteur |
| P4 | **Ce que l'outil ne sait pas est dit** | une définition anonyme, un appel non résolu ou un langage mal couvert sont comptés et nommés dans la réponse, pas laissés hors du résultat |

Ce que la stèle **n'exige pas** : la dette de rendu, les lenteurs sans perte, les blocages qui se
disent (ils restent en tickets), et tout ce qui touche un montage PostgreSQL tant qu'il n'a pas
d'usage.

## 2. Les tickets, rangés sous les conditions

Ce sont les tickets ouverts du produit dont la gravité est « perte », « réponse fausse »,
« réponse incomplète » ou « inconnue » (table « Ouverts — produit », 11 octobre, midi). On en
compte **20**, et non 21 comme annoncé : la différence est à vérifier quand la table est relue.

### P1 — rien n'est rendu comme sûr qui ne l'est pas

| ticket | ce qui le ferme |
|---|---|
| `2026-10-04-une-arete-ne-dit-pas-comment-elle-a-ete-resolue.md` | les marques de résolution, en cours (filtre dans Liens et impact sur master ; « nom » sur les liens par le nom seul, `718cfbe1d`) ; reste : les réexportations |
| `2026-10-04-des-receveurs-restent-sans-type.md` | codeparsers (B2d) ; les appels sur une locale non typée sont visibles et jamais reliés (`6afc2976a`) |
| `2026-10-04-un-chemin-vers-une-reexportation-reste-par-le-nom.md` | décidé, attend `code.rs` |
| `2026-10-04-nom-seul-et-segments-de-module.md` | codeparsers, « pas pour maintenant » à revoir |
| `2026-10-04-prototypes-c-au-niveau-du-fichier.md` | codeparsers |
| `2026-10-04-type-d-un-champ-declare-dans-un-autre-fichier.md` | le rendez-vous des types entre fichiers (codeparsers et `code.rs`) |

### P2 — aucune perte à l'indexation ni à la synchronisation

| ticket | ce qui le ferme |
|---|---|
| `2026-10-11-indexation-enfant-du-serveur-mcp.md` | la forme C. La marche 0 (`75de6618f`) règle déjà le serveur qui se ferme proprement, et la mémoire longue a ajouté SIGTERM (`59718e806`) ; reste le serveur tué, que règle le démon |
| `2026-10-11-mode-demon-ni-valide-ni-refute.md` | la forme C : `--demon` est retiré à sa marche 7 |
| `2026-10-11-une-reaction-ecrit-dans-la-cellule-qu-un-appel-a-basculee.md` | la forme C, marche 1 bis (les sessions : plus de cellule courante) |
| `2026-10-11-un-put-sans-le-champ-de-cycle-de-vie-reecrit-l-etat-initial.md` | mémoire longue |
| `2026-10-04-portee-person-ecrite-jamais-rappelee.md` | mémoire longue (le crochet de rappel), condition de sortie écrite au ticket |
| `2026-10-04-journaux-d-annulation-laisses-dans-tmp.md` | le dataflow : ses journaux d'annulation sous le cache, avec un ménage |
| `2026-10-10-postgresql-transaction-de-paquet-sur-deux-sessions.md` | corrigé dans le texte, témoin vivant à jouer (montage PostgreSQL) |
| `2026-10-11-postgresql-defaire-un-lien-ne-trouve-rien.md` | corrigé dans le texte, témoin vivant à jouer (montage PostgreSQL) |

### P3 — aucun refus muet

| ticket | ce qui le ferme |
|---|---|
| `2026-10-11-un-lien-vers-un-bout-absent-rend-ok-sans-rien-ecrire.md` | mémoire longue (le lien général) : un refus nommé |
| `2026-10-11-echecs-de-groupe-d-une-reaction-sans-lecteur.md` | mémoire longue : un lecteur des échecs, et une borne |
| `2026-10-05-erreurs-du-moteur-avalees-par-les-replis.md` | arbre principal : relire les replis un par un, chacun dit son erreur ou s'arrête |
| `2026-10-04-analyze-racine-relative-rend-zero-scope.md` | codeparsers : refuser une racine relative, ou la résoudre en le disant |

### P4 — ce que l'outil ne sait pas est dit

| ticket | ce qui le ferme |
|---|---|
| `2026-10-04-fonctions-anonymes-cpp-restantes.md` | codeparsers |
| `2026-10-11-appel-a-une-fermeture-locale-ecarte.md` | codeparsers, T5 (`d90a1f6`, branche `t5-fermetures`, en vérification) |

**Hors de la stèle**, parce qu'ils se disent ou ne perdent rien :
- la reprise qui exige `takeover`, un blocage qui se dit. La marche 0 l'a déjà levée pour l'arrêt propre, et la forme C la règle pour le reste ;
- la confirmation sous un petit tampon : un blocage, et en blobs il dépend du moteur (le COPY borné en octets) ;
- les services de modèles sans authentification ;
- les trois rendus d'arbre ;
- le démon de test sous charge ;
- le rebâti du plein texte sur PostgreSQL.

## 3. Ce qui la ferme, dans l'ordre

1. **Codeparsers « au maximum »** : P1 et P4 sont presque tous à lui (`code.rs` compris). *À écrire
   par codeparsers : l'ordre de ses lots et ce qui reste après chacun.*
2. **La forme C** ferme trois tickets de P2 par ses marches 1 bis, 3 et 7.
3. **La mémoire longue** ferme quatre tickets (le lien vers un bout absent, les échecs de réaction,
   le `put` sans état, la portée `person`).
4. **L'arbre principal** relit les replis qui avalent les erreurs (P3) et range les journaux
   d'annulation (P2).
5. **Les deux tickets PostgreSQL** ne demandent qu'un témoin vivant, à jouer quand un montage
   PostgreSQL a un usage.

**La preuve** : chaque ticket fermé avec son témoin, comme pour le moteur. Et une passe de
contrôle sur ce dépôt, qui compte les liens par marque de résolution (`e2e_banc_relations`) et dit
ce qui reste « par le nom ».
