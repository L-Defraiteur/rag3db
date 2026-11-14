# Un Symbol que plus rien ne relie reste après le retrait de son fichier

- **État** : ouvert ; constat, pas de correctif. Rétention voulue ou fuite, c'est à décider.
- **Famille** : rag3weaver, schéma de code
- **Gravité** : faible aujourd'hui. Les lignes restent et ne se reprennent jamais.
- **Atteignable en service** : oui, à chaque fichier supprimé ou renommé, et à chaque nom qui
  disparaît du code.
- **Touche rag3weaver** : oui, `code.rs`, l'ingestion des symboles.
- **Pour** : everything declarative (au mot de c7, 14 nov. 23 h 3x).
- **Signalé par** : banc, relevé des orphelins du 14 novembre (vérification demandée par c7).
- **Ouvert le** : 14 novembre 2026

## Le constat

Sur une copie de la base migrée, au commit 9d90edbb6, forme à versions sans générations,
moteur 373b5477b, `RAG3WEAVER_MOTEUR_ANCIEN=1` :

- après l'index de la source entière, 20 Symbol sont reliés à un Scope de
  `extension/rag3weaver/src/arbre.rs`. Trois d'entre eux ne sont reliés par aucun Scope d'un autre
  fichier ;
- après un index sans ce fichier, ses 7 Scope et son File sont effacés. Les 3 Symbol exclusifs
  restent, **sans aucune relation** :
  - `5b31ac2a-5f68-9280-9824-7a5437e97146`, `le_libelle_est_en_casse_de_phrase` ;
  - `9530c855-87ed-f7e3-fd1d-bd89d51ff6ca`, `relation_label` ;
  - `63f65d43-edf9-a90d-8557-4a956e5416d9`, `la_forme_du_graphe_de_dependances` ;
- leur plein texte reste aussi : 86 172 documents pour 86 172 lignes avant comme après.

## Pourquoi

Un Symbol est un rendez-vous par nom. L'ingestion le pose par `hashsafe`, de façon idempotente,
sans session d'instantané : aucune fin d'instantané ne le reprend. Ce n'est le cas ni de Scope ni de
File.

## Ce qu'il faut décider

- **Rétention voulue** : un nom reste connu. À écrire alors dans le schéma, et à tenir hors
  des réponses : `usages` ou une recherche sur Symbol rendent un nom qui n'existe plus dans le code.
- **Fuite** : la table grandit à chaque nom disparu. Le remède serait de reprendre, à la fin de
  l'instantané du code, les Symbol sans aucune relation.

## La lecture de l'orchestration (c7, 14 nov. 23 h 3x), telle quelle

Un Symbol est un rendez-vous par nom, hors session : son retrait n'appartient à aucune fin
d'instantané. Ce n'est donc pas une fuite du retrait, c'est **une rétention sans règle**.

- **La règle se pose dans l'écrivain de 2a′-2** : un Symbol que plus aucune relation vivante ne
  relie se ferme à la publication (w). Il se rouvre si le nom revient, avec le même uuid, par
  `hashsafe`.
- **Sous la forme sans générations, on ne touche à rien** : trois Symbol par fichier retiré, c'est
  une poussière. Un ménage à part l'effacerait sans histoire.

Everything declarative tranche.

## La preuve

- Le test : `releve_les_comptes_du_plein_texte_apres_un_effacement`, dans
  `extension/rag3weaver/tests/sonde_parite_search_code.rs`, branche `banc-parite-search-code`, à
  partir de 69a713362e.
- Le relevé : `banc-parite/sorties/orphelins-symboles/parent-comptes.{log,json}`, à luciepc.

Fin : la décision écrite. Puis, si c'est une fuite, un témoin rouge de la reprise, vert après le
correctif.
