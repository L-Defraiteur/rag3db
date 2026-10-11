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
compte **20**, et non 21 comme annoncé. Le 21ᵉ est « Une réaction peut remonter une relation
qu'aucun outil n'écrit » (une déclaration inatteignable acceptée en silence, donc P3), fermé à
11 h 28 par la marche 4 du lien générique (`ad51a6630`), une demi-heure avant que la page ne soit
écrite.

### P1 — rien n'est rendu comme sûr qui ne l'est pas

| ticket | ce qui le ferme |
|---|---|
| `2026-10-04-une-arete-ne-dit-pas-comment-elle-a-ete-resolue.md` | les marques de résolution, en cours (filtre dans Liens et impact sur master ; « nom » sur les liens par le nom seul, `718cfbe1d`) ; reste : les réexportations |
| `2026-10-04-des-receveurs-restent-sans-type.md` | **P1 tenue** : un receveur sans type est soit marqué « nom » (par le seul définisseur, ou par l'analyseur dans le fichier, `718cfbe1d`), soit visible et jamais relié (`6afc2976a`). Ce qui reste (typer davantage : retours d'appel, paramètres de fermeture) est du rappel, hors de la stèle. À fermer pour la stèle avec ces deux témoins |
| `2026-10-04-un-chemin-vers-une-reexportation-reste-par-le-nom.md` | décidé, attend `code.rs`. La sonde B9 de la session recherche voit pourtant traverser `pub use interne::secrete` (appel attribué) : rejouer la recette du ticket avant d'écrire du code |
| `2026-10-04-nom-seul-et-segments-de-module.md` | codeparsers, « pas pour maintenant » à revoir |
| `2026-10-04-prototypes-c-au-niveau-du-fichier.md` | codeparsers |
| `2026-10-04-type-d-un-champ-declare-dans-un-autre-fichier.md` | probablement déjà fermé par B2b (les types différés résolus au rendez-vous, `6002c3ccd`, témoin `e2e_types_differes`) : rejouer la recette du ticket, puis le fermer |

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
| `2026-10-11-appel-a-une-fermeture-locale-ecarte.md` | codeparsers, T5 (`d90a1f6`, branche `t5-fermetures`, en vérification) ; le ticket est fermé dans la branche |

**Hors de la stèle**, parce qu'ils se disent ou ne perdent rien :
- la reprise qui exige `takeover`, un blocage qui se dit. La marche 0 l'a déjà levée pour l'arrêt propre, et la forme C la règle pour le reste ;
- la confirmation sous un petit tampon : un blocage, et en blobs il dépend du moteur (le COPY borné en octets) ;
- les services de modèles sans authentification ;
- les trois rendus d'arbre ;
- le démon de test sous charge ;
- le rebâti du plein texte sur PostgreSQL.

## 3. Ce qui la ferme, dans l'ordre

1. **Codeparsers « au maximum »** : P1 et P4 sont presque tous à lui (`code.rs` compris). Dans
   l'ordre, chaque lot avec son témoin rouge, le banc sans recul réel (arêtes comparées une à une
   quand des liens bougent), et le pointeur à l'arbre principal :
   1. **T5** (fermeture locale, `map(aide)`) : fait, branche `t5-fermetures` en vérification.
      Reste ensuite en P4 : les C++ anonymes.
   2. **Rejouer deux recettes** — fait (branche `recettes-p1`) : le type d'un champ déclaré dans
      un autre fichier est **fermé** (B2b, témoin posé) ; le chemin vers une réexportation est
      **rouge**, plus net que le ticket (avec un homonyme : aucune arête). Il devient le lot 2 bis.
   2 bis. **La réexportation** — fait (codeparsers `77c8ea6`, branche `reexportations-2`) : un
      import suit les `pub use` du fichier qu'il désigne, trois sauts au plus, marque « import ».
      Ticket fermé. Restent hors de la stèle tant qu'aucun cas ne les montre : `pub use x::*`, le
      renommage, `export … from` (TypeScript), `__init__.py` (Python). Reste ensuite en P1 : le nom seul et les segments de module, et les
      prototypes C.
   3. **`analyze` en racine relative** (P3) — fait (branche `analyze-racine`) : la racine se
      résout et se dit (le défaut avait changé de forme : des chemins « absolus » relatifs et
      doublés). Pas fait, faute de cas : le fichier non rendu dans `skipped`. Reste ensuite :
      rien en P3 côté codeparsers.
   4. **Les prototypes C au niveau du fichier** (P1) — fait (codeparsers `9531860`, branche
      `prototypes-c`) : membres du scope de fichier, noms hors des références ; et un `struct`
      sans corps n'est plus une seconde définition. Arêtes de rag3db/src identiques.
   5. **Les C++ anonymes** (P4) — fait (codeparsers `35faa0f`, branche `cpp-anonymes`) : 287 →
      0 sur rag3db. La cause majeure était les classes exportées (`class RAG3DB_API Foo`), lues
      comme des fonctions ; avec elles, les méthodes gabarits retrouvent leur scope. Appels
      reliés entre fichiers sur rag3db/src : 41,5 → 50,7 %. Reste ensuite en P4 : rien.
   6. **Le nom seul et les segments de module** (P1) : un paramètre nommé dans une signature et un
      segment de chemin qui nomme un module ne sont plus des références par le nom. C'est le lot
      le plus large. Reste ensuite en P1 : la passe de contrôle.
   7. **Fermer « une arête ne dit pas comment elle a été résolue » et « des receveurs restent sans
      type »** sur la passe de contrôle : `e2e_banc_relations` compte les liens par marque. Plus
      aucun lien deviné n'y passe pour un lien lu, et ce qui reste « par le nom » est compté
      (banc codeparsers : `relations.consumes.par_le_nom`).

   Hors de la stèle, après elle : typer davantage de receveurs (le rappel, pas la vérité du
   rendu : retours d'appel, paramètres de fermeture, éléments d'itérateurs).
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
