# Une frappe tapée avant l'invite d'une commande `!` se voit

- **État** : ouvert, souhait. Relevé par la session tty le 15 nov. 2026 (lot tty-13), dit dans `docs/tty/15-11-2026/04-les-commandes.md`.
- **Famille** : produit
- **Gravité** : gêne, et un risque léger : un mot de passe tapé trop tôt s'affiche à l'écran, et entre dans la sortie gardée, donc dans la consigne envoyée à l'équipier.
- **Atteignable en service** : oui, si l'on tape le mot de passe d'un `!sudo …` avant que `sudo` ait affiché son invite (et coupé l'écho).
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`, `src/coquille.rs`).

## Ce que c'est

Une commande `!` tourne dans un pseudo-terminal à elle, dont l'écho est ouvert jusqu'à ce que la commande le coupe : `sudo` le coupe au moment où il demande le mot de passe. Une frappe arrivée avant ce moment est renvoyée par le pseudo-terminal, comme par tout terminal : elle s'affiche, et elle entre dans la sortie gardée, puisque le terminal garde tout ce que la commande écrit, l'écho compris.

## La recette

Taper `!sudo true`, Entrée, puis tout de suite le mot de passe, avant l'invite « [sudo] password ». Le test `coquille::tests::a_password_read_with_echo_off_never_reaches_the_kept_output` le montre si on lui retire son attente de 500 ms : le secret paraît dans la sortie.

## Pistes

- ne garder de la sortie que ce qui suit la dernière invite de mot de passe reconnue, ou retirer d'une sortie l'écho d'une frappe qui précède une coupure d'écho (le terminal sait ce qu'il a relayé) ;
- lire l'état d'écho du pseudo-terminal (`tcgetattr` sur le maître) avant de relayer une frappe, et retenir les frappes tant que la commande n'a pas lu ;
- au moins, dire dans la consigne qu'une frappe a précédé l'invite.

## Observation privée du16 novembre, à attribuer sans confondre les causes

Harnaisseul91e5, phase0008 fauxsudo : invite et retour attendus nonvus,
marqueur de mot de passe **factice** dans journal/raw, pas dans la consigne.
Le harnais a été interrompu automatiquement au premier rouge, CommonTERMINÉ/
restes[]/nettoye true. Ce constat ne prouve pas encore la course de frappe
précoce décrite ci-dessus : l'invite elle-même est absente, la commande pourrait
avoir échoué avant l'invite. Aucune cause alias ou nouveau défaut produit déduite.

Preuves détaillées/traces désormais persistées et suivi d'attribution dans
`2026-11-16-le-harnais-perd-les-traces-des-seances-et-juge-la-fin-sur-les-octets.md`.
Aucun secret réel, aucune exécution sur l'hôte de Lucie, pas de réessai/fix.

## Attribution RO91e5 levée : fixture absente, pas preuve de ce défaut produit

Raw et consigne de phase0008 confirment code127/No such file : faux_sudo.sh
avait été créé phase0007 avant le changement DOSSIER. Le fauxsudo n'a jamais
coupé l'écho ou affiché d'invite ; le marqueur FACTICE a ensuite été écrit dans
la vue redevenue normale et posté comme answer. Cette observation91e5 relève
du ticket HARNESSEXISTANT, pas d'un correctif de coquille.rs. Le défaut historique
possible de frappe avant invite reste ouvert avec sa propre recette ; aucune
réussite ni correction produit déduite de ce diagnostic. Aucun nouveau test.
