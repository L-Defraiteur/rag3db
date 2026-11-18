# tty-25 : la cause d'une attente dans le style du thème

Session tty, 18 novembre 2026. Première moitié du ticket `docs/tty/tickets/2026-11-15-la-cause-d-une-attente-dans-le-style-du-theme.md`. Lucie, 15 nov. : « en gros à côté, mais là aussi des glitchs ou selon le thème ».

## Ce qui change (`src/anim.rs`, `cause_spans`)

La cause (« le modèle répond… (gemini-3.8-flash, 12 s) », « run_command cargo check travaille… (40 s) ») ne s'écrit plus en sourdine, d'un seul style :
- **animation en glitch** (les écritures, l'alien) : un caractère du verbe marqué de « … » (« travaille… », « répond… ») devient un glyphe du thème, dans la couleur du halo. Il change une image sur trois, tiré par la graine. Ce que la cause nomme (l'outil, sa cible, le modèle, la durée) reste lisible ;
- **cercle et transmutation** (la réflexion, les lectures) : la cause dans la couleur vive du halo, précédée du premier glyphe de l'animation ;
- **ASCII**, ou terminal sans couleurs : la cause telle quelle, en sourdine.

## Le plafond d'octets, gardé

`tui::tests::mesure_des_octets_par_image_d_animation` fixe les octets de 100 images ; il n'a pas été relâché, et le rendu s'y est plié :
- un glyphe d'encadrement qui changeait à chaque image portait le cercle à 11 239 octets, plafond 10 500 : le glyphe est fixe, et le cercle redescend à 8 609 ;
- un glitch de la cause à chaque image portait le glitch à 23 259 octets, plafond 20 000 ; une image sur trois donnait encore 20 524. Avec un seul caractère à la fois, une image sur trois, on mesure 19 342.

Le glitch a aussi d'abord mangé le nom de l'outil (« ru∫_command »). Un test de la vue l'a vu, d'où le verbe seul.

## Les témoins

- `anim::tests::a_glitch_animation_glitches_its_cause_and_keeps_it_readable` et `a_circle_frames_its_cause_in_the_halo_and_ascii_leaves_it_plain` : vus rouges (700ee1d2c0), puis verts.
- La mesure d'octets et `vue::tests::the_animation_follows_the_state_and_the_tool_policy` (le nom de l'outil suit l'animation) restent verts.
- Le crate passe 116 + 1 tests, et 22 sans vue ; clippy `-D warnings` ne signale rien.

## Ce qui reste au ticket

Les attentes que seul l'hôte connaît (un quota avec son délai, l'index qui termine, une commande en fond), par un événement `waiting {cause, since, retry_in?}` d'archi et d'ED. Le terminal les rendra par le même `cause_spans`.
