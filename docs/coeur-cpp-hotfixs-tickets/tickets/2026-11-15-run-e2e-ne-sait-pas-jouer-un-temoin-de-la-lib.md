# `run_e2e.sh` ne sait pas jouer un test de la lib comme témoin

- **État** : ouvert (15 novembre 2026, hotfixs)
- **Famille** : produit
- **Gravité** : petite. Un lot dont le témoin est un test unitaire de la lib (`cargo test --lib`) ne peut pas le
  désigner à l'étape « témoin » de `passe_epinglee.sh`. Cette étape se remplit alors d'un e2e voisin, et le vrai témoin
  ne se lit que dans le journal de la lib entière.
- **Atteignable en service** : non (outillage des passes)
- **Touche rag3weaver** : oui (`extension/rag3weaver/run_e2e.sh`, et `passe_epinglee.sh`)
- **Pour** : G

## Ce qu'on voit

`passe_epinglee.sh` joue son témoin par
`./run_e2e.sh --features … --test ${SUITE_DU_TEMOIN:-e2e_lectures} "$FILTRE"`. `run_e2e.sh` traduit `--test <nom>` en
`cargo test --test <nom>`, et n'a pas de forme pour `--lib`. Le 15 nov., le lot extension-non-chargee-dite
(témoin `backend::…::une_extension_notee_qui_ne_se_charge_pas_se_dit_a_l_ouverture`, dans `src/backend.rs`) a dû mettre à
cette étape un e2e voisin, `la_premiere_recherche_apres_l_ouverture_dit_ou_est_le_plein_texte` : un remplissage.

## Pour fermer

`run_e2e.sh --lib <filtre>` (ou `SUITE_DU_TEMOIN=lib`) joue `cargo test --lib -- <filtre>` sous la même lib et le même
garde-fou. Un filtre qui ne joue aucun test est rouge. Un témoin de script le montre.
