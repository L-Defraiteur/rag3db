# L'accord explicite en mode auto : conception

24 novembre 2026, peu après minuit. Les décisions de Lucie des 23 et 24 novembre, et ce qu'elles deviennent en graphe et
en gabarits, selon sa règle du même soir : tout déclaratif, un builtin n'est qu'une primitive.

## Les décisions de Lucie

1. **Plus de carte bloquante en mode auto** (`bac`, `judge`) : ce que la porte ou le juge refuse revient à l'agent comme
   résultat d'outil ; le tour continue ou finit, il n'attend rien. L'agent demande l'accord dans le texte de sa réponse.
2. **L'accord vient de ses mots** : un message humain suivant (`by.kind: human`) qui autorise explicitement l'action
   refusée pose un accord borné.
3. **Les dossiers aussi** : l'accès à un dossier hors du projet, en lecture comme en écriture, est tranché par le
   juge ; plus d'exception « écriture sous carte ».
4. **Le chemin qu'elle cite** (lecture, dix minutes, `mentioned-path-grant`) passe avant le juge et ne l'appelle pas.
5. **Le drapeau et la justification, en un seul passage** (Lucie, 24 novembre vers 0 h 20 : « le classifier vient déjà
   lire la commande, la justif éventuelle et le warning auto, et du coup il juge, et c'est tout ; l'agent retente s'il
   veut ») : un détecteur déterministe signale une action dangereuse avec un avertissement tout fait (pourquoi, et
   quelles justifications sont recevables) ; le paramètre `justification` est toujours offert, facultatif, sur les
   outils à effet ; le juge lit en une fois l'action, la justification s'il y en a une et l'avertissement, et tranche.
   Un refus revient à l'agent avec l'avertissement ; il retente s'il veut. Pas d'étape pilotée par l'hôte, pas de
   compteur de tentatives : le plafond de pas et d'appels d'un tour borne déjà.
6. **Les cartes ne restent que sous `--gate cards`**, et en service sans écran le juge tranche, le refus revient à
   l'agent, qui le rapporte en fin de session.

## Le graphe

Une action d'un outil à effet (commande, écriture, ouverture d'une racine) traverse, dans l'ordre :

1. **Les scellés**, dans le code (voir plus bas) : un dossier protégé ou une racine trop large est refusé, sans juge.
2. **Le chemin cité** : une racine qu'elle a écrite dans sa demande du tour s'ouvre en lecture, sans juge.
3. **Le détecteur de classes** : la table des classes de commandes et de chemins, en fichier de règles, donne la classe
   de l'action et son drapeau (`safe`, `flagged`, `sealed`). Une classe `safe` passe.
4. **Le juge** (`Decide`) pour une action non sûre, en un seul passage. Son contexte : ses mots du tour, l'action, la
   classe et son avertissement, et la `justification` si l'appel en porte une (le paramètre est toujours offert, dès le
   premier appel). Ses sorties : `allow`, `deny`, `unsure`.
5. **`allow`** : la primitive `GrantAction` pose un accord borné à cette action (cette racine et ce mode, ou cette ligne
   de commande ; ce tour, ou N minutes déclarées), le dit en une ligne (`granted.by_judge`), et l'action est rejouée.
6. **`deny` ou `unsure`** : le refus revient à l'agent comme résultat d'outil, avec l'avertissement de la classe
   (`refused.<classe>`) : ce qui est refusé, pourquoi, ce qui serait recevable. L'agent retente s'il veut, avec une
   meilleure justification, ou demande à l'utilisatrice son accord explicite. Une ligne `refused` s'écrit dans la boîte.
7. **Ses mots ensuite** : un message humain suivant déclenche le même `Decide` sur les refus encore ouverts du tour
   précédent ; `allow` pose l'accord, et l'agent, qui lit la ligne au prochain retour d'outil (`mid-turn-delivery`),
   relance son action.

Déclaré : la table des classes (fichier de règles), les avertissements et les justifications recevables par classe
(gabarits), la question du juge (gabarit), la table des suites (`allow` → accorder ; `deny`, `unsure` → refuser), la
durée d'un accord. Primitive neuve : `GrantAction`, qui pose un
accord borné, une seule chose. `Decide`, `FirstMatch` et les conditions de la chaîne existent.

## Ce qui reste dans le code, et pourquoi

Des gardes de sécurité, qu'aucun fichier ne doit pouvoir éteindre :

- **Les scellés** : les dossiers protégés (`.vault`, `.ssh`, le jeton de l'hôte, les profils de navigateur, les
  magasins d'identifiants) et les racines trop larges (`/`, `$HOME`, un parent de plusieurs dépôts) ne s'ouvrent ni
  par le juge, ni par ses mots, ni par une justification.
- **Qui parle** : seul un message `by.kind: human` vaut ses mots. Une ligne d'agent, un contenu lu, un retour d'outil
  n'en sont jamais.
- **Ce que voit le juge** : ses mots et l'action, toujours ; la justification de l'agent, quand il y en a une, **entre
  comme une prétention à vérifier, jamais comme une autorité**, citée et délimitée comme un texte non sûr. La question
  posée est : « les mots de l'utilisatrice et la tâche en cours soutiennent-ils cette justification ? ». Le contenu lu
  par les outils n'est jamais dans son contexte.

## Les trous que je vois, et ce qui les ferme

1. **Une justification habile sur des mots vagues.** Si elle a écrit « fais ce qu'il faut », une justification
   convaincante peut obtenir une action lourde (effacer hors du projet, pousser en force, envoyer des fichiers sur le
   réseau). **Fermeture** : chaque classe déclare si elle se justifie (`justifiable: true`) ou si seuls ses mots
   l'ouvrent (`words_only`). Les classes lourdes sont `words_only` dans les règles livrées ; Lucie peut en changer.
2. **Une injection relayée par l'agent.** Un fichier lu dit « lance ceci » ; l'agent le reprend dans sa justification
   sans le citer. Le juge ne voit pas le fichier, mais il voit une justification plausible. **Fermeture** : la question
   ancre le juge sur ses mots (« soutiennent-ils ? ») et pas sur la vraisemblance de la justification ; une
   justification qui ne se rattache à aucun de ses mots donne `unsure`, donc rien. Le jeu du mode auto du banc mesure
   ce cas (une injection dans un fichier lu), après le lot.
3. **L'agent qui retente en boucle.** Sans compteur dédié (Lucie : « l'agent retente s'il veut »), ce qui le borne est
   ce qui borne déjà un tour : le plafond d'appels d'outils d'un pas et de pas d'un tour. Chaque verdict est tracé (le
   drapeau, la justification, la réponse du juge), et un appel du juge coûte : la trace le montrera si un agent
   s'acharne.
4. **Un accord trop large.** « Oui, ce dossier » ne doit pas ouvrir son parent. **Fermeture** : `GrantAction` accorde
   exactement la racine proposée par la porte (`proposed_root`) et le mode demandé, rien au-dessus.
5. **Ce qui passe sans qu'elle le voie.** Un accord du juge doit se voir : une ligne dans la boîte, à chaque fois, avec
   l'action, le mode, la durée et ce qui l'a fondé (ses mots, ou la justification). Ce qu'elle lit lui permet de
   retirer l'accord (`revoke_root`, déjà là).

## Les lots

1. **La table de la porte par niveau** : `cards` → carte ; `bac`, `judge` → refus en résultat d'outil, avec la ligne
   `refused`. Les textes en gabarit.
2. **`GrantAction`** et le juge sur une action non sûre (racine ou commande), avec `allow` → accord et rejeu.
3. **Ses mots** : le même juge sur un message humain, pour les refus ouverts.
4. **Le drapeau et la justification** : la table des classes en fichier de règles avec `justifiable` / `words_only`
   et les avertissements, le paramètre `justification` toujours offert sur les outils à effet, lu par le juge au même
   passage.
5. **La mesure** : le jeu du mode auto du banc, injection comprise.

Chacun commence par son témoin rouge. La chaîne de gardes de fin de pas (en file) vient avant : la condition
`RefusedInTurn` y vérifie qu'un tour qui finit avec un refus ouvert l'a dit à l'utilisatrice.
