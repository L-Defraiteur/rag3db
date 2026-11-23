# poste compte les cœurs, pas la mémoire : oomd a tué la session graphique de luciepc

- **État** : ouvert (23 novembre 2026, écrit par G)
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : `poste`)
- **Gravité** : haute. La session graphique de Lucie fermée pendant cinq heures, et avec elle le trousseau : le jeton
  `gh` invalide, trente lots sans PR (ticket « un jeton gh perdu se dit en une ligne par lot »).
- **Atteignable en service** : non, le poste de travail seulement
- **Touche rag3weaver** : non (`extension/rag3weaver/scripts/poste`)

## Ce que c'est

Le 23 novembre 2026 à 18 h 47 min 45 s, `systemd-oomd` a tué `plasma-kwin_wayland.service` à luciepc : « memory used
(95 638 777 856) / total (100 462 469 120) and swap used (90 599 727 104) / total (100 461 965 312) ». C'est la
règle du swap (`ManagedOOMSwap=kill`, posée par `/usr/lib/systemd/user/slice.d/10-oomd-per-slice-defaults.conf`) :
au-delà de 90 % de swap utilisé, oomd tue le cgroup qui tient le plus de swap, ici kwin avec 27,2 Gio. Juste avant,
quatre places de passe tournaient (portées à 5,4, 5,7, 6,5 et 9,8 Gio de pic dans les cinq minutes d'avant).

`poste` compte des places de cœurs, jamais la mémoire. Les pics des passes, relevés par le pousseur sur 398 passes
depuis le 20 novembre : **médiane 10,7 Gio, p90 13,2, maximum 17,7.** Quatre places au maximum font 71 Gio ; avec le
fond du poste (≈ 20 Gio de mémoire et ≈ 34 Gio de swap déjà tenus, le soir du 23), la mémoire déborde dans le swap
compressé, qui dépasse 90 %.

**`ManagedOOMPreference=avoid` sur la session graphique ne l'aurait pas sauvée** : la page de
`systemd.resource-control` dit que, pour le swap, oomd ne lit cette préférence que sur un cgroup possédé par root, ce
qu'une unité de la session utilisateur n'est pas.

## Ce qu'il faut

1. **Un budget mémoire dans `poste`** : un lourd n'entre que si `MemAvailable` dépasse son pic attendu (le p90 des
   vingt derniers pics de même nature, 13 Gio pour une passe, 16 par défaut), plus une réserve de 12 Gio pour le
   poste à écran, plus ce que les lourds en cours n'ont pas encore atteint de leur propre pic. La place de cœurs reste
   la première condition.
2. **Une enveloppe par portée** (`systemd-run --scope`) : `MemoryHigh=18G`, `MemoryMax=24G`, `MemorySwapMax=4G`. Une
   passe qui s'emballe ralentit puis meurt seule, dans sa portée, au lieu de pousser la session dans le swap.
3. **Témoins, rouges d'abord** : un faux `/proc/meminfo` (variable `POSTE_MEMINFO`) sous le seuil : `poste` attend et
   le dit ; au-dessus : il entre ; la portée créée porte les trois limites.
4. **En attendant** : 3 places à luciepc la nuit du 23 au 24 (posé jusqu'au 24 à 9 h). Trois fois 17,7 Gio plus le
   fond tiennent dans la mémoire vive.

## Ce qui est le geste de Lucie

- **`wireplumber` tient 17,6 Gio de swap** (le soir du 23 à 23 h 5x) : une fuite probable, et le prochain plus gros
  détenteur de swap, donc la prochaine victime d'oomd. La relance coupe le son une seconde :
  `systemctl --user restart wireplumber pipewire pipewire-pulse`
- Côté système, rien n'est requis si les points 1 et 2 tiennent. Elle seule peut choisir de relever le seuil de swap
  d'oomd (`SwapUsedLimit=` dans `/etc/systemd/oomd.conf`).
