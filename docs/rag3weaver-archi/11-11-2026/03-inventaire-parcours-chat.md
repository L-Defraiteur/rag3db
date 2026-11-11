# Parcours de code du chat : inventaire effectif, 11 novembre 2026

Livré sur master `8d5139f9e` : lot commun `cx-archi-ide-par-le-flux-chat-4`,
source exactement jouée `bc3a1d779`, code identique après rebase de documents. Il réunit flux et correctif chat, après ED réclamations et projection.
Les preuves ciblées historiques ci-dessous restent attachées à `816fc4824`.
Anciens chemins nettoyés après push : worktree `cx-wt-chat-erreurs` et target `flux`
sur les deux postes. Les preuves restent en dehors de ces chemins.

## Ce qui a été observé

Le témoin `le_navigateur_inventorie_les_outils_effectifs_atelier_depot_et_tache`
envoie trois demandes depuis Chrome, sur les gabarits livrés, avec une tâche créée par
`/ide/taches`. Le faux modèle relève `GenOptions.tools`, après filtrage. C'est une preuve
d'exposition au modèle, pas encore l'exécution de chacun de ces outils.

| capacité | atelier | dépôt hors tâche | tâche |
|---|---|---|---|
| lire un fichier | `read_file`, `read_scope` | `read_file` | `read_file` |
| explorer / chercher les relations | absent de la liste | `list_files`, `grep_files`, `outline`, `search_code`, `usages`, `impact`, `callees`, `graph`, `schema` | mêmes outils |
| écrire | `replace_text`, `replace_lines`, `insert_lines` | masqué | `edit_file` |
| exécuter et attendre | `run_command`, mais aucun `wait_output` | `wait_output`, commandes masquées | `run_command`, `wait_output` |
| worktrees et accords | `attach_worktree`, `list_worktrees`, `open_branch`, `ask_permission` | aucun dans cette liste | aucun dans cette liste |
| déclarations | `proposals`, `propose`, `list_declarations`, `read_declaration` | mêmes outils | ajoute `declare`, `declare_set`, `apply`, `reject` |

L'atelier propose aussi `show_progress` ; dépôt et tâche proposent `estimate` et `index`.
La politique déclarée de l'atelier autorise commandes et écritures hors tâche ; le dépôt
passe par sa tâche pour ces deux capacités. L'absence d'outils de recherche dans l'atelier
est transmise à ED, qui tient outils et gabarits ; ticket dédié, sans doublon d'implémentation.

## Premier blocage démontré et corrigé dans la branche

`une_erreur_du_modele_garde_les_actions_du_chat_et_permet_la_reprise` lance une demande
par le formulaire : `read_file` réussit, l'appel modèle suivant échoue. Avant correction
(`1c0631b67`), le témoin est rouge en 0,91 s : la bulle en cours est supprimée, avec le
résultat pourtant obtenu. Les actions ne doivent pas disparaître lorsqu'une étape suivante
échoue.

Le catch conserve maintenant la bulle et les résultats. Le message poussé `ChatPousse`
porte `fin_du_tour`, résultat discriminé de ce tour : succès avec éventuel arrêt, ou erreur
avec son message. Un tour parti ailleurs que dans le formulaire (une relance après carte,
par exemple) utilise ce même contrat, au lieu de passer systématiquement à « fini ».
Pas de polling ni de relecture temporisée ajoutés.

Après correction (`816fc4824`), le même témoin passe en 1,28 s : lecture toujours visible,
erreur dite, nouvelle demande réussie et historique conservé ; un autre tour lancé depuis
le navigateur hors formulaire échoue ensuite et son état reste « Erreur », avec deux lectures
visibles. L'inventaire des trois contextes passe en 1,94 s. Les types TypeScript sont verts
et le bundle est rebâti avec les sources.

## Validation et limites

- Rouge : luciepc `cx-chat-erreurs-rouge-1c0631b67-20261111/`.
- Ciblés verts : `cx-chat-erreurs-816fc4824-20261111-{erreur,inventaire}/` ;
  captures exactes des outils dans `inventaire/navigateur.log`.
- Régression navigateur : **36/36, zéro échec, zéro ignoré, 132,73 s** sur `816fc4824` ;
  `cx-chat-erreurs-816fc4824-20261111-navigateurs/`, verdict `VERT`. Lanceur terminé.
  Cela valide cette étape, pas la passe globale.
- Passe finale commune **VERT** sur `bc3a1d779` : features 5/5, lib 2 100/0 (61 ignorés),
  hooks 2/2, lectures 24/24, PostgreSQL privé 34/34, navigateurs 36/36 (132,10 s).
  Source et journaux : `cx-flux-chat-bc3a1d779-prive-r2-20261111` à luciepc.
  Le témoin historique des 606 vecteurs dus passe après ED ; son ancien rouge est conservé.
- Push master `8d5139f9e` au tour accordé, trois contrôles lus ; nettoyage deux postes et base
  privée terminé, preuves et branches distantes conservées. Aucun nouveau binaire posé par archi.

La chaîne complète explorer → lire → éditer → tester → montrer diff → livrer n'est pas encore
prouvée par ce lot. Les accords, l'arrêt, les longues commandes, la réouverture après panne
et la livraison de tâche doivent avoir leurs preuves de parcours ; les témoins antérieurs
restent des indices à relire, pas un verdict global sur ce cap.
