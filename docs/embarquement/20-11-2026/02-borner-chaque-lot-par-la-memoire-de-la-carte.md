# Embarquement : borner chaque lot par la mémoire de la carte, avant l'appel

Ticket `2026-11-20-un-lot-accepte-peut-depasser-la-memoire-libre-et-arreter-le-service`, demande de c7 (20 novembre) :
**le service borne chaque lot à la mémoire libre lue avant l'appel, et recoupe.** Une requête acceptée ne doit plus
pouvoir arrêter le service.

## Le constat

Le 20 novembre à 20 h 35, une requête de 32 textes (4 à 1 742 jetons) a fait réserver 4,36 Gio à bge-m3 sur card2 ;
la réservation a été refusée, et le serveur de la carte empoisonné : arrêt, code 70. Le 7879 n'a aucun budget de lot
posé (régime plein). Le découpage d'une requête (`par_lots` : `lot_max` textes sous un budget d'octets) ne connaît ni
la mémoire que le lot demandera, ni celle que la carte a de libre.

**Ce qui a vraiment refusé** (mesuré le soir même) : sous radv, une **allocation unique** est limitée à
`maxMemoryAllocationSize` = 0xfffffffc octets (4 Gio), sur card2 comme sur l'APU de la tablette. Les 4 357 506 816
octets refusés au 7879 dépassaient cette limite, pas la mémoire libre : card2 avait alors 12 Gio de libres.

**Une allocation refusée empoisonne cubecl pour de bon** (essai jetable sur l'APU, bge-m3 dans le processus) : un lot
de 32 × 1 700 mots demande 5,95 Gio en un seul tampon. Il est refusé, puis « Parent device is lost », puis « The server
is in an invalid state ». Les trois petits appels suivants paniquent tous. **Recouper et réessayer est donc
impossible** : la sûreté doit venir d'une estimation faite avant l'appel.

Ordre de grandeur : la mémoire d'un lot est dominée par l'attention, lot × têtes × jetons² × 4 octets. Pour bge-m3
(16 têtes), un texte de 1 742 jetons demande environ 194 Mo, et quelques textes longs dans un même lot demandent
des gigaoctets.

## La forme (acceptée par c7, 20 novembre)

1. **Deux bornes, estimées avant chaque lot** :
   - **le plus grand tampon du lot** (l'attention : lot × têtes × jetons² × 4 octets) sous la limite d'allocation de
     l'adaptateur (`max_buffer_size` de wgpu), avec une marge ;
   - **le total du lot** sous la moitié de la mémoire libre de la carte : `mem_info_vram_total − mem_info_vram_used`
     (amdgpu, `/sys/class/drm/cardN/device/`), sur la carte que l'adresse PCI de l'adaptateur désigne, comme
     `regime::driver_card_types`. Sans sysfs (Intel) : seule la première borne, et l'identité le dit. La lecture est en
     course avec les autres locataires de la carte (le 7878, l'hôte) : la moitié est cette marge.
2. **Le coût d'un lot, estimé par le modèle** : une méthode de l'`Embedder`, `lot_bytes(textes, jetons)` (le plus grand
   tampon et le total), que chaque modèle burn déclare d'après sa forme. Les jetons sont majorés par les octets (le
   pire ratio mesuré est d'un jeton par caractère), plafonnés à la fenêtre du modèle.
3. **Le recoupage, avant l'appel** : dans `par_lots`, un lot qui dépasse une borne est coupé en deux, puis encore,
   jusqu'à tenir. **Pas de réessai après coup** : une allocation refusée empoisonne le serveur. Un texte seul qui ne
   tient pas reçoit un 503 nommé (« <N> octets estimés, <limite> »), sans calcul : le service continue.
4. **L'identité** : `memory_bound` (la limite d'allocation, la carte lue, la mémoire libre au dernier lot, les lots
   recoupés, les textes refusés), dans `/sante`.

## Les témoins, rouges d'abord

1. **Unité** : un factice qui panique comme cubecl quand lot × jetons² dépasse un seuil, et une mémoire libre
   injectée. Une requête de 32 textes longs l'arrête aujourd'hui ; elle doit être recoupée et servie entière, dans
   l'ordre.
2. **Unité** : un texte seul qui ne tient pas reçoit un 503 nommé, et le service continue de servir.
3. **Le chemin de l'hôte**, à la tablette, sur le service d'essai 7995 (APU, régime plein) : la requête du 20 novembre
   (32 textes de la référence) est servie. Jamais sur le 7879.

## À calibrer

Les coefficients de `lot_bytes` se mesurent sur l'APU de la tablette : lots croissants de bge-m3 et de granite,
mémoire relevée par sysfs avant et après, puis l'estimation se compare au relevé. L'estimation doit majorer ce qu'on
mesure. **Puis passivement sur card2** : lecture seule de `used` autour d'un lot réel du 7879 pendant une passe qui
embarque dessus, sans rien lui envoyer ; l'estimation doit majorer ce relevé aussi.

## Taille

Une journée, calibrage compris.

## Ce qui est livré, et prouvé (20 novembre, 23 h 30)

- **Le lot `memory-bound`** : `src/lot_memory.rs` (l'estimation, les deux bornes, le recoupage, la mémoire libre lue en
  sysfs), `Embedder::memory_shape`, `token_counts` et `memory_bounds`, déclarés par granite et bge-m3,
  `BurnDevice::adapter_facts`, le recoupage dans `par_lots` sur ses deux chemins, `EmbedError::Refused` rendu en 503, et
  `memory_bound` dans `/sante`.
- **Un APU compte la GTT** : sur celui de la tablette, la VRAM n'est qu'une réserve de 4 Gio, dont 3 sont pris, et les
  calculs vivent dans la GTT (112 Gio). Pour un adaptateur intégré, la mémoire libre est donc la VRAM plus la GTT.
- **La limite que wgpu rend** est de 2 147 483 647 octets (2 Gio moins un), plus stricte que les 4 Gio de Vulkan : la
  borne est prudente.
- **La preuve par le vrai binaire**, sur l'APU de la tablette (bge-m3, port 7995, budget d'octets levé) :
  - la requête du 20 novembre (32 × 1 700 mots, un tampon de 5,95 Gio sans le lot) est **servie, en un lot recoupé** ;
  - un texte dense (11 000 « é », plus de 8 192 jetons) reçoit un **503 nommé** : « un texte de 8192 jetons ne tient
    pas sur la carte : un tampon de 4294967296 octets estimés, au-delà de 90 % de la limite d'allocation de la carte » ;
  - la requête suivante est servie : le service n'est pas empoisonné.
- **Le calibrage sur l'APU** : le pic mesuré est de 5,97 Go au-dessus du repos, et l'estimation, pour des lots de
  8 × 1 704 jetons, donne 6,25 Go. Elle majore, de 5 %.
- **Reste à prouver** : la majoration sur card2, passivement, par lecture de sysfs autour d'un lot réel du 7879.

