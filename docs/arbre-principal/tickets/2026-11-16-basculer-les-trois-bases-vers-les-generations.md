# Basculer les trois bases de code de Lucie vers les générations

- **État** : ouvert (16 novembre 2026), préparation active ; autorisation Lucie relayée par orchestration, opérations après FINI de G et contrôles, aucune nouvelle permission routinière requise.
- **Famille** : produit
- **Gravité** : bascule des données partagées préparée et répétée, pas encore effectuée ; conserver l'intégrité, la recherche et le retour.
- **Atteignable en service** : oui (code-rag3db-depot au portable, code-principal et code-lourd à luciepc).
- **Touche rag3weaver** : non pour le code produit ; opérations et preuves privées de migration.
- **Pour** : arbre principal (migrations et contrôles), G (lib, hôtes, fermeture/réouverture des services), banc (oracle des copies finales).
- **Signalé par** : reprise autorisée par orchestration le 16 novembre, après l'audit RO du périmètre.

Plan maître : `docs/optimiseur/16-11-2026/01-plan-de-la-bascule.md` ; recette des bases : `docs/arbre-principal/15-11-2026/02-la-fenetre-de-la-bascule.md` ; parité et tolérances : `docs/banc/16-11-2026/01-la-parite-de-la-migration.md`, §12.

Source audit927941a67a5805450d49f1e04b39fdd542a6059b ; lib préparée par G59ac9c86bc2b3529b590a7462ab7051e3ad26b8a et hôtes rebâtis927/Lucivy4.3.9. Attendre ses artefacts réellement vérifiés/FINI, jamais employer ancien3177/373b/794 par défaut. A1 suspendu, aucune modification produit simultanée.

Pour fermer : pour chaque base, identité exacte, snapshot cohérent et RETOUR conservés72h, maintain frais borné, deux sens verts, versions1b lots2000 puis générations0publiée/1lu, repassage terminé (dernière ligne du journal, pas busy), comptes visibles/mirroirs0/0/vecteurs et oracle contemporain jugés, refus ou migrations partielles conservés sans faux vert. Retour et données d'avant gardés, aucun retrait maintenant. Une base non traitée ou oracle incomplet reste explicitement non qualifié.

Rapport et recette d'exécution : `docs/arbre-principal/16-11-2026/03-preparation-bascule-bases.md`.
