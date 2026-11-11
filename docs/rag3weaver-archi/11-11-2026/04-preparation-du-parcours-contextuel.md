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

## Transport annoncé par ED, pas encore validé

ED prépare sur `cx-ed-relais` un champ `Turn.metadata: Option<Value>` (serde compatible
avec les anciennes sessions), copié depuis le reçu, relayé en `ToolCallFinished.metadata`
et persisté avec le tour. La clé autoritaire sera donc `turn.metadata.invocation` /
`event.metadata.invocation`, hors du texte soumis à coupure. Ce transport et la primitive
ne sont pas annoncés comme compilés/disponibles.

Le témoin `dbf3f7419` s'arrête à l'exposition, avant tout reçu. Une fois cette base disponible,
remplacer son helper temporaire lisant le transport textuel par la lecture du champ structuré
uniquement ; aucune reconstitution depuis un chemin, un label, la conversation ou un texte
tronqué. Le rouge de 3,78 s reste conservé sous son SHA d'origine. Aucune compilation n'est
à relancer sur une source ED encore non validée pour prétendre que le relais fonctionne.


## Intégration diagnostique disponible — base ED `14585ca4a`

Le transport annoncé ci-dessus est maintenant intégré et observé. Branche archi
`cx-archi-ui-contextuel-relais-2`, worktree sur les deux postes :
`~/.cache/rag3weaver-build/cx-wt-ui-contextuel`, target unique sur luciepc :
`~/.cache/rag3weaver-build/cx-archi-target`. Cette base diagnostique n'est pas la base finale
courante : attendre celle d'ED avant la passe commune et un éventuel push master.

- `861e178b7` : outils `search_code` et `outline` effectivement exposés et exécutés depuis Chrome ;
  reçus structurés complets observés. Rouge ensuite : aucun lien sur le vrai `src/lib.rs:1-3`.
- `23dde03d5` : reçu réel conservé ; le diagnostic HTML a lui-même une erreur d'échappement,
  corrigée au commit suivant. Ce passage n'est pas une preuve du rendu.
- `ef43d6ead` : les plages de lignes sont reconnues ; rouge précis en 3,80 s : le reçu désigne B,
  mais le href désigne la conversation. Texte et HTML réels conservés.
- `657f51f12` : **VERT ciblé navigateur 1/0/0 en 65,72 s** : recherche + outline réels, B puis A,
  mêmes chemin et symbole mais contenus distincts, clic correct puis relecture de session et
  clic correct sans nouvel appel modèle. Inventaire atelier/dépôt/tâche : **1/0/0 en 2,17 s**,
  incluant les huit lectures contextuelles et `wait_output` dans l'atelier.
- `3dd0af8ad` : garde de contexte JS verte : reçu autoritaire, ancien outil local inchangé,
  absence/inconnu/vue non prise en charge inactifs, aucun repli sur la conversation.
- `b57313175` : nouveaux témoins de vraie coupure de recherche et commande différée après accord,
  **en cours de validation, pas encore verts**. Aucun résultat fabriqué ni lien injecté par eux.

Le rendu relit exclusivement `metadata.invocation`, conservé dans les tours et les événements,
indépendamment du texte borné. Un contexte explicite sans reçu exploitable n'active ni lien ni
composant d'action. Les outils locaux ordinaires gardent leur contexte local. La plage de ligne
conserve son texte et ouvre la première ligne dans la vue du fichier.

Preuves séparées sous `~/.cache/rag3weaver-build/cx-ui-contextuel-<SHA>-<étape>-20261111/`
sur luciepc, avec source-head, navigateur.log et exit-code. Les rouges initiaux ne sont pas écrasés.
TypeScript vérifié et bundle rebâti. Le succès sur ED145 est diagnostique : aucune passe complète
commune, aucune livraison master, aucune découverte autonome des cibles par le modèle n'est prouvée.


## Base commune et compléments (en validation)

Rebase sans conflit sur la base ED figée `3a0679b7f` (elle-même sur master `418e47a2e`,
avec arrêt `80b467dd3`) ; nouvelle branche `cx-archi-ui-contextuel-relais-3`.
La passe diagnostique `b57313175` fait 1 réussi, 2 échecs, 0 ignoré en 150,28 s :
le parcours court reste vert ; la commande hors domaine redemande l'accord (ticket archi
`2026-11-11-un-accord-de-commande-hors-domaine-redemande-le-meme-accord.md`) ; le parcours
tronqué atteint le reçu coupé et les clics B, puis échoue faute de reçu observé au contrôle suivant ; ce diagnostic initial ne distingue
pas à lui seul l'issue réelle du tour. Le contrôle explicite de l'issue dans la passe `3de12fd0b`
révèle ensuite un débordement de la fenêtre du MockLlm interne (4790 jetons pour 4096). Ces deux rouges ne constituent pas une passe verte.

Les fixtures suivantes gardent le script sous la racine et attendent l'issue du **nouveau**
tour. Le faux modèle déclare une fenêtre de 131072 jetons pour permettre les deux tours de
résultats réellement coupés à la borne de 32768 octets du chat ; cette borne n'est pas modifiée.
Le modèle vérifie la coupure réelle et l'absence de `Metadata:` dans le texte, tout en exigeant
le reçu structuré. La commande attend un fichier signal, créé seulement après l'annonce du
vrai appel wait_output dans le flux. Résultats de cette nouvelle combinaison encore attendus.


## Preuves finales archi sur la source commune `a92b21233`

- Trois ciblés **3/0/0 en 143,02 s** : nominal B/A, recherche réellement coupée B/A,
  commande réellement accordée puis suivie par wait_output sur sortie différée.
- Étape navigateur sous jeu large **39 réussis, 0 échec, 0 ignoré en 137,71 s**.
  Les trois nouveaux témoins sont bien détectés par le script de l'étape.
- ELF `rag3weaver-057fdf86676710c7`, SHA256
  `4ad3a31b5742b01e20bbf14a5e848d8c2006061b6a271090fb6511a6b70b7f3e` :
  la table référencée par son fichier `.d` pointe vers les 220 gabarits actuels ;
  leurs 220 contenus sont présents dans l'ELF, aucun absent. app.js SHA256
  `fff528ffdcbfb08c022c1249d0ec5ecc82a4caf4a010eaad4447ca7c3081bc00`.
- TypeScript, test JS de la frontière de contexte, bundle identique aux sources et tickets verts.

Preuves sur portable et luciepc :
`~/.cache/rag3weaver-build/cx-ui-contextuel-a92b21233-navigateur-entier-20261111/`
(`source-head`, `navigateur.log`, `exit-code`, `attestation-elf.json`, table utilisée).
Le premier vérificateur ELF a été arrêté et corrigé : il parcourait inefficacement le mmap et
avait d'abord inclus des tables d'autres jeux de features ; aucun témoin n'a été arrêté.
Le résultat retenu sélectionne uniquement la table du binaire effectivement testé et utilise
la recherche de sous-chaîne du mmap. Les anciennes tables non utilisées restent dans le target.

Tout ce travail et la suite de validation/intégration sont reliés au ticket existant
`docs/everything-declarative/tickets/2026-11-11-le-chat-de-l-atelier-n-expose-pas-les-outils-de-recherche-ni-wait-output.md`
(porteurs ED intégration et archi UI), complété sans doublon. Le défaut d'accord extérieur est
distinct, pris en charge par ED, hors de ce lot, ticket archi conservé ouvert jusqu'au transfert.

**Reste** : verdict complet ED sur le même a92, puis attendre l'intégration du lot recherche
4.3.3 annoncé par orchestration, relire son diff, reposer la combinaison dans une branche neuve
et rejouer les contrôles requis avant toute demande de tour. Les résultats a92 ne valent pas
un succès de cette combinaison future. Aucun push master de ce lot n'est encore réalisé.
Le worktree PC reste exactement sur a92 pendant la passe ED ; les compléments suivants sont
uniquement documentaires sur la branche archi. Target archi 8,6 Go, conservé pour la suite du lot.


## Reprise après combinaison recherche 4.3.3 — non livrée

Source testée exacte `458e50b46a277618f08bb0f37d5bcbebe1250db7`, branche archi
`cx-archi-ui-contextuel-commun-4` (base commune ED `cx-ed-relais-commun-3`), mêmes worktrees
`~/.cache/rag3weaver-build/cx-wt-ui-contextuel` sur les deux postes, target unique PC
`cx-archi-target` conservé. Diff depuis a92 relu : dépendances Lucivy 4.3.3, témoin FTS et
Markdown ; aucun changement du code UI/runtime/gabarits du relais.

Trois ciblés navigateur sous jeu large **3 réussis, 0 échec, 0 ignoré en 135,93 s** :
nominal B/A, vraie coupure du texte B/A avec bons clics/rechargement, commande réelle accordée
puis wait_output sur sortie différée. ELF testé SHA256
`701a37d153c7cacf376852d48631aa8b86c6695466f1771c8595cd01ee1cc91f` :
table de son `.d`, 220 chemins actuels et 220 contenus présents, zéro absent, JS/manifeste
identiques à a92. Preuves sur les deux postes dans
`cx-ui-contextuel-458e50b46-cibles-final-20261111/` du cache de build. Aucun processus du
target archi encore présent après contrôle de `/proc`. Les 39 navigateurs verts a92 restent
une preuve distincte, pas présentée comme un nouveau passage 39 sur 458.

Le ticket commun reste ouvert :
`docs/everything-declarative/tickets/2026-11-11-le-chat-de-l-atelier-n-expose-pas-les-outils-de-recherche-ni-wait-output.md`
(porteurs ED intégration et archi UI). **Avant intégration**, attendre le verdict ED sur 458
et le départage ED du ticket de revue
`docs/everything-declarative/tickets/2026-11-11-une-invocation-peut-executer-une-autre-version-que-celle-verifiee.md`.
L'enchaînement statique invoke_read → call_in → call → call_tool relit prepared ; cela ne
constitue pas ici une reproduction concurrente. Aucun correctif archi de ce point, aucune
modification de source pendant les passes, aucune clôture ni push master anticipé. Le défaut
d'accord extérieur est désormais transféré ouvert à
`docs/everything-declarative/tickets/2026-11-11-un-accord-de-commande-hors-domaine-redemande-le-meme-accord.md`.
