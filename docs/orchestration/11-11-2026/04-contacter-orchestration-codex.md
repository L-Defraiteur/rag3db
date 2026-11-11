# Sessions Codex — contacter l’orchestration

Lucie vous demande de prendre contact avec la session Codex qui reprend l’orchestration de rag3db.
Faites-le dès la lecture de ce document, puis poursuivez votre travail selon votre dossier de reprise.

## Destinataire

- Titre : `orchestration-sessions`
- Identifiant de session : `01a1cb5c-62b9-7ea0-8b7e-cf38398ece09`
- Dépôt : `/home/lucied/git_workspaces/rag3db`
- Reprise : `docs/orchestration/11-11-2026/03-reprise-par-codex.md`

L’orchestration ne voit pour l’instant que certaines sessions. Ne créez pas de nouvelle session :
contactez celle indiquée ci-dessus depuis votre session actuelle.

## Envoyer le premier message

Cherchez parmi vos outils la capacité `mcp__codex_tui__send_message_to_thread` (son préfixe peut varier).
Elle envoie un message à une session existante. Si nécessaire, découvrez les outils disponibles en
filtrant `ALL_TOOLS` sur `send_message_to_thread`, `list_threads` et `read_thread`.

Appelez `send_message_to_thread` avec :

```json
{
  "threadId": "01a1cb5c-62b9-7ea0-8b7e-cf38398ece09",
  "prompt": "Prise de contact — session <nom>. Mon identifiant : <id si connu>. Dossier : docs/<session>/. État : <en cours / prête / bloquée>. Lot : <branche, worktree, hash>. Vérifications : <non jouées / rouge observé / passe verte et preuve>. Blocage ou permission en attente : <détail ou aucun>. Prochain pas : <action>."
}
```

Remplacez les champs entre chevrons. Le message doit rester inférieur à **1 000 octets UTF-8**.
Omettez le paramètre `model`. Si vous ignorez votre identifiant, cherchez votre session avec
`list_threads` à partir de son titre et de son dossier ; sinon signalez simplement que l’id est inconnu.

Si l’outil manque ou si l’envoi échoue, dites-le à Lucie dans votre session avec l’erreur exacte.
N’écrivez pas directement dans les fichiers internes de Codex pour contourner ce problème.

## Après le contact

Lisez `docs/README.md`, votre rapport et votre knowledge dump du 11 novembre, puis vos tickets.
Les priorités de chaque session sont dans la table du document de reprise de l’orchestration.
Signalez ensuite à l’orchestration :

- « rouge posé », avec le témoin et sa cause ;
- « vert », avec les vérifications réellement jouées et leur preuve ;
- une demande de tour de push quand le lot est prêt ;
- « poussé <hash> », avec les trois contrôles lus : status propre, aucun rebase en cours,
  `origin/master` égal à HEAD ;
- tout blocage ou refus de permission dès qu’il survient.

Un push de code sur master attend le mot **« à toi »** de l’orchestration et suit
`docs/procedures/push-d-un-lot.md`. Les docs seules suivent leur exception habituelle.
Un refus de permission remonte à Lucie ; aucune autre session ne le contourne.

Ce document demande une prise de contact ; il n’accorde aucun tour de push et ne remplace pas
les règles du dépôt. L’orchestration tient la file et les rapports ; chaque session garde son périmètre.
