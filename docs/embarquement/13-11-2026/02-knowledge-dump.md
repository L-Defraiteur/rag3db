# Reprise technique — activation dense locale

L'état et les worktrees sont dans [le rapport](01-rapport-de-session.md).
Le contrat public est dans ../12-11-2026/03-activation-des-modeles.md ; le
[ticket unique](../tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md)
conserve rouges, refus, budgets, preuves et suites non réalisées.

- ModelSource.activation → serveur::activation → Serveur/Attache. Registry
  Weak intraprocessus et verrou fichier par endpoint/répertoire entre processus.
  Worker propriétaire, annulation propre à chaque demandeur, échéances absolues.
- Backend::connect_embedder est l'appelant existant. Le wrapper garde le Bail,
  contrôle l'attestation HTTP et restaure le délai normal après la sonde finale.
  L'ancien service sans attestation est refusé si l'activation est déclarée.
- Chauffe séparée par capacité : complète/partielle/désactivée/non exécutée,
  formes/erreurs/durée. Profil/périphérique déclarés ne prouvent pas le GPU réel.
- Pas d'éviction/TTL implicite. La politique maintenir laisse vivre le fournisseur ;
  les processus privés de banc ont leur propriétaire et leur arrêt explicite.
- Recettes scripts/qualifier_activation_modeles.sh, qualifier_activation_e2e.sh,
  requalifier_activation_apres_cache.sh ; l'union CPU est qualifier-union.py,
  reprise bornée par reprendre-union.sh. Toujours portée commune après vrai poste,
  memory.max32G/high0, bus privé, j8, target unique et lib29 complète épinglée.
- Source union verte138268398, validation-5wmntc4y, F5/lib2273/NAV40/4tx25.
  Les66 ignorés de lib sont structurels, jamais qualifiés. Le compagnon test
  ED08c/c44 synchronise un index réel via un fournisseur CPU factice, sans
  changement produit. Aucun rejeu GPU après rebase documentaire équivalente.
- Preuves et rouges hors target dans ~/.cache/embarquement-notes/activation.
  Binaires réels mesurés345a402d0 et avant-rebase3 gardés immuables avec SHA256.
  Les scripts d'observation utilisent les helpers figés du banc ; inconnus /proc
  restent inconnus. Ne pas supprimer les preuves avec le cache.
- Livraison : Pubbc1 puis ED43 puis notre tour explicite. Chaque rebase prend
  une nouvelle branche, aucun force-push. Nettoyage seulement après archives
  fidèles de toutes les administrations privées et garde actuelle VERT ; jamais
  retirer un sous-module partagé, un target d'autrui ou un service installé.
