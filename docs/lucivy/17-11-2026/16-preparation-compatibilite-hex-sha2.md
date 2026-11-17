# Compatibilité hex sha2 — source préparée, non compilée

[Ticket FS](../tickets/2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md).
Le refus E0277 sur8b est publié3759c4cebd et archivé deux postes **avant** cette
correction. Aucun retry du GOv2, aucun rouge d'admission FS prétendu acquis.

Source **0f610f437a341f745e5936492114142ff3afedd4**, parent8b94353, poussée sur
`cx-lucivy-fs-admission`. Deux fichiers de tests, six lignes ajoutées/une retirée :

- sha256 continue d'appeler Sha256::digest de la dépendance exacte0.11.0 ;
  chaque octet est ensuite formaté en deux chiffres hex minuscules par la
  primitive standard, puis collecté. Aucun calcul cryptographique maison,
  nouvelle dépendance ou modification de version/lock.
- Le positif existant vérifie aussi les valeurs connues SHA256 des octets
  vides et de abc, notamment les zéros de tête dans les octets du digest.
  Sept noms de tests inchangés, pas de juge ajouté ni de négatif affaibli.

[Invariants statiques et valeurs de référence](preuves/hex-compatibility-preparation/source-invariants.json).
Panel15, capture_baseline, with_v3_contains et tout le préfixe des six négatifs
sont octet pour octet inchangés. La gate reste délibérément fautive avant les
rouges Rust : erreurs avalées, vue vide acceptée, couverture non contrôlée,
sink inutilisé et oracle natif encore reread. Aucun refus n'est réparé ici.

Les valeurs de référence ont été confirmées par hashlib pendant la préparation,
pas par un nouveau test Rust. **Aucun Cargo/rustfmt/Rust/test après cette
correction**, compilation non vérifiée. Les trois verts CPU de l'exporteur
51a9 restent limités aux raccords simulés et ne sont pas rejoués.

Le W PC est encore8b et l'ancien Qv2 clos, inchangé. T3552 conservé, trois ELF
historiques RO archivés. Le lock PC effectif est maintenant dbdb560...37f6,
direct sha2 déjà présent ; un futur paquet doit l'épingler explicitement et
exiger son invariance (--offline --locked envisageable), pas réutiliser la
recette de delta depuis ce719 sans changement. L'ancien Common de référence
est désormais265273/start126184561 terminé101/nettoyé, pas1021297.

Suite : RO exacte de cette source, puis nouveau paquet/pilote/préflight/nonce
et recette complète relus avant GO distinct. Le début/fin portable devra
être journalisé dans l'enveloppe physique ; cette lacune d'observation n'est
pas comblée à la main dans le refus précédent. Aucun nouveau GO Cargo/native,
pas de corps groundtruth, aucun corpus réel, main/tag/publication.
