# Bilan tty : aliases anglais et harnais privé, 16 novembre 2026

Le **harnais entier tmux-local-ouvert est VERT** sur source934f3c810a, ELFtty
cb24 déjà bâti, sourceRustproduit6d identique. Scope et verdict composé VERT0,
aucun résidu, autorité terminée. F8 canonique8/8 porté de6d et buildtty0 porté
de9d, pas rejoués. Le lot d'alias reste **ouvert** : relecture du raccord réel
TUI/Tab révèle une acquisition de candidats encore limitée aux noms français.
Aucun push master ni pose du binaire de Lucie effectué.

## Preuves de qualification du harnais

GO C7-TTY-HARNESS-934F-16nov1200 ; start855836.130261483,
end857036.130261483, final855982.790141532, soit146,660s tout compris.
Watchdog extérieur autour de l'ensemble file/poste/Common/final/pub,
verdict composé après Common TERMINÉ/restes[]/nettoye true ; empreintes
SDK/ELF/contrôleurs/target et source propre contrôlées.

- Accord de la phase2 corrélé à l'unique approvalID : protocole et écran jugés.
- Kitty et sans Kitty sous tmux ouvert : deux textes multiligne exacts,
  aideMaj-Entrée vue par capture-pane ; demandeKittyfalse admise sous tmux.
- Commande !/fauxsudo : invite et retour vus, une consigne portant le retour,
  marqueur FACTICE absent de consigne/journal/raw, zéro lancement sans frappe.
- Cinq thèmes, dix passages couleurs ; toutes les autres séances du harnais
  achevées, verdict final VERT, liste rouges vide.

Preuves sélectionnées versionnées : [manifest](preuves-harnais-934f/manifest.json),
[verdict composé](preuves-harnais-934f/composed-verdict.json),
[résultat du harnais](preuves-harnais-934f/harness-result.json),
[verdict de portée](preuves-harnais-934f/scope-verdict.json).
Archives complètes aux deux postes, hors target :
`~/.cache/rag3weaver-build/archives/cx-tty-harness-934f-green/evidence.tar`,
SHA3c361191aa9a1de916d93f209b98c02e316e801e8221fe430ae8add0119fec43.
Les trois rouges antérieurs restent archivés et ne sont pas transformés en verts.

## Reste réel du ticket d'alias

`tty/src/tui.rs` : Action::Completer transmet le début original à sources_de,
puis à commandes::completer. Le second connaît task/repo, mais sources_de ne
fait GET/ide/etat que si le début commence par /tache ou /depot. Les tests
anglais actuels fournissent Sources à la main ; le harnais natif utilise les
anciens noms. Les commandes anglaises marchent, mais la complétion dynamique
des tâches/dépôts en anglais n'est pas qualifiée et son raccord manque.

Même ticket porteur tty :
`docs/tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md`.
Avant correction : réserver tui.rs auprès de l'orchestration comme annoncé au
départ, puis témoin du véritable sources_de sur faux catalogue privé/callback
injecté, rouge avant fix. Composer le parseur existant commandes::lire pour
choisir la capacité de complétion évite une seconde liste manuelle d'aliases.
Proposer un budget ciblé et source/pilote après réservation ; aucun Cargo/HTTP/
service supplémentaire autorisé par ce bilan. Ce reste n'est pas masqué par
le vert du harnais de compatibilité français.
