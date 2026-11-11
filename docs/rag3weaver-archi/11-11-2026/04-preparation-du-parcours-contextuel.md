# Parcours contextuel : deux dépôts, clic et reprise

Préparation de témoin sur `cx-archi-ui-contextuel`, code `dbf3f7419`, depuis master
`6efd22915`. **Aucune primitive ni capacité nouvelle livrée.** Aucun push master pendant
la fenêtre recherche. ED tient runtime/contrat/gabarits ; archi tient le parcours et le rendu.

## Frontière convenue avec ED

Le contrat est préparé dans
`docs/everything-declarative/11-11-2026/04-contrat-invocation-contextuelle.md`, branche
`cx-ed-invocation-contextuelle`, précision JSON `8cb38b8e3`.

- Recherche : `search_code` reçoit
  `{context:{target:{kind:"backend",id:"<id résolu>"}},query:"repere",options:{}}`.
- Outline : `outline` reçoit
  `{context:{target:{kind:"backend",id:"<id résolu>"}},path:"src/lib.rs"}`.
- Le reçu porte `metadata.invocation` : `target.kind/id/label`, `parent_run`, `run`,
  puis éventuellement une vue effective. La vue est omise dans le premier lot.
- Cible espace ou vue sans résolveur : refus observable, pas de repli silencieux.
  Les arguments de l'outil cible ne sont pas renommés par le wrapper.

La provenance doit être attachée aux résultats, liens et actions. Le rendu actuel de
`ToolResultView` appelle `renderMarkdown` avec l'id de conversation : cela ne suffira pas
pour un résultat servi par un autre dépôt. Ne pas confondre le contexte effectif du résultat
avec celui de la conversation parent ; ne pas changer l'autorité au passage.

## Témoin préparé, sans résultat ni lien simulés

Source : `extension/rag3weaver/src/daemon_host/chat_contextuel.rs`, module isolé branché dans
`daemon_host.rs` (ED confirme ne pas modifier cette ligne). Le témoin :

1. crée deux vrais dépôts git `depot-a` / `depot-b`, chacun avec `src/lib.rs` et `fn repere`,
   mais un contenu distinct (`CONTENU_A_PREUVE` / `CONTENU_B_PREUVE`) ;
2. les attache par l'outil livré de l'atelier, attend la fin des index réels et relève les
   identifiants des contextes réellement résolus par l'hôte ;
3. envoie une demande depuis Chrome dans le chat de l'atelier, d'abord vers B puis A ;
4. capture les outils/schémas proposés à la frontière du modèle. Le faux modèle ne rend
   aucun résultat de recherche : il demande les vrais `search_code` puis `outline` déclarés ;
5. relit `metadata.invocation` dans les véritables retours d'outils, contrôle cible, label
   et identités de runs ;
6. clique uniquement un lien généré depuis la présentation du vrai résultat, vérifie route
   et contenu du dépôt demandé ; aucun href n'est ajouté par le témoin ;
7. recharge la session, contrôle le même lien et le même contexte sans nouvel appel modèle,
   puis reclique vers le contenu exact.

Le faux modèle est piloté pour choisir l'id observé de A/B ; ce témoin ne prouve pas que le
modèle a découvert seul tous les contextes. Il vérifie exposition, exécution du vrai relais,
rendu et reprise. Les invariants d'autorité, cycles, annulation et refus de cible restent
les preuves runtime d'ED ; le parcours ambigu / inconnu sera complété avec sa primitive.

## Vérification et point de reprise

- `rustfmt --check` et `git diff --check` : verts (syntaxe/format uniquement).
- Compilation des tests **VERTE** à luciepc sur `dbf3f7419`, en 1 min 31 s, sous
  `poste lourd`, `-j8`, jeu `rag3db-native,code,daemon,navigateur`, **sans exécuter le témoin**. Nouveau target unique
  `~/.cache/rag3weaver-build/cx-archi-target`, après nettoyage du précédent.
- Source et résultat du check :
  `~/.cache/rag3weaver-build/cx-ui-contextuel-dbf3f7419-check-20261111/`.
- Baseline **ROUGE d'exposition observée** sur `dbf3f7419` : 0 réussi, 1 échec, 0 ignoré,
  3,78 s. Les deux dépôts réels ont été attachés/indexés et la demande Chrome atteint le
  modèle ; l'assertion dit exactement `search_code n'atteint pas le modèle`.
  Journal : `cx-ui-contextuel-dbf3f7419-rouge-20261111/navigateur.log` à luciepc.
- Le relais n'est pas encore implémenté : **exécution du relais, clic et reprise non atteints**.
  Dès disponibilité annoncée, intégrer la primitive au point sûr puis jouer le même témoin
  pour observer l'étape suivante, sans rejouer une absence inchangée ni affaiblir les assertions.
- Ne pas fixer des liens à la main, réécrire un résultat pour le faire passer, ni compter
  une compilation ou un témoin ignoré comme preuve du parcours.

Fichiers de rendu réservés au futur correctif archi : `ui/ide/src/views/Chat.tsx` et
`ui/ide/lib/toolview.tsx`. Aucune modification à ces deux fichiers à ce stade ; aucun
changement à `code.rs`, au gabarit code ou aux fichiers du lot remaining d'ED.

## Provenance et coupure du texte

ED et archi ont confirmé que la provenance nécessaire aux actions doit voyager structurellement
jusqu'aux tours, événements et sessions (ou par référence stable), hors du budget du texte du
modèle. Si la présentation est coupée, la cible doit rester connue ; sinon l'action contextuelle
refuse ou reste inactive, sans repli sur la conversation. Le premier petit cas du témoin relit
le vrai reçu dans le transport actuel ; il ne constitue pas encore une preuve de ce cas de coupure.
