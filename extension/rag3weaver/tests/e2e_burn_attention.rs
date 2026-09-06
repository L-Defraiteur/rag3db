//! **La flash attention de burn à la forme du lot 256** (chantier D du 03
//! optimiseur) : `[256, 12, 512, 64]`, celle de granite-278m sur un lot de
//! 256 séquences de 512 jetons. Les candidats flash accélérés y demandent
//! 3 Gio (`failed to reserve 3221225472 bytes`) — exactement un masque
//! `[256 × 12, 512, 512]` en 4 octets par élément — et l'autotune retombe
//! sur le noyau « unit », dix fois plus lent. Ces sondes disent *qui*
//! matérialise le masque, en changeant seulement la façon de le passer.
//!
//! ```text
//! RAG3WEAVER_BURN_DEVICE=gpu:1 cargo test --features burn-embedder \
//!     --test e2e_burn_attention -- --ignored --nocapture --exact <sonde>
//! ```
#![cfg(feature = "burn-embedder")]

use burn::tensor::{Bool, Distribution, Tensor, TensorData};

/// La forme, surchargeable par `RAG3WEAVER_SONDE_B/H/S/D` pour isoler ce que
/// l'allocation mesure (3 Gio = B·H·S·S·4 = 8 × B·H·S·D·4 à 256/12/512/64).
fn dim(nom: &str, defaut: usize) -> usize {
    std::env::var(nom).ok().and_then(|v| v.parse().ok()).unwrap_or(defaut)
}
#[allow(non_snake_case)]
fn dims() -> (usize, usize, usize, usize) {
    (dim("RAG3WEAVER_SONDE_B", 256), dim("RAG3WEAVER_SONDE_H", 12), dim("RAG3WEAVER_SONDE_S", 512), dim("RAG3WEAVER_SONDE_D", 64))
}

#[allow(non_snake_case)]
fn qkv(device: &burn::tensor::Device) -> (Tensor<4>, Tensor<4>, Tensor<4>) {
    let (B, H, S, D) = dims();
    eprintln!("[attention] forme [{B}, {H}, {S}, {D}]");
    let q = Tensor::<4>::random([B, H, S, D], Distribution::Normal(0.0, 0.2), device);
    let k = Tensor::<4>::random([B, H, S, D], Distribution::Normal(0.0, 0.2), device);
    let v = Tensor::<4>::random([B, H, S, D], Distribution::Normal(0.0, 0.2), device);
    (q, k, v)
}

/// Le masque du graphe : `[b, 1, 1, sk]`, vrai = masqué (le dernier quart de
/// chaque séquence), construit comme dans le code généré (`lower_elem(0.0)`
/// d'un flottant).
#[allow(non_snake_case)]
fn masque_base(device: &burn::tensor::Device) -> Tensor<4, Bool> {
    let (B, _, S, _) = dims();
    let mut m = vec![1.0f32; B * S];
    for b in 0..B { for j in (S * 3 / 4)..S { m[b * S + j] = 0.0; } }
    Tensor::<4>::from_data(TensorData::new(m, [B, 1, 1, S]), device).lower_elem(0.0)
}

fn lancer(nom: &str, q: Tensor<4>, k: Tensor<4>, v: Tensor<4>, masque: Option<Tensor<4, Bool>>) {
    let mut durees = Vec::new();
    for _ in 0..3 {
        let t = std::time::Instant::now();
        let o = burn::tensor::module::attention(q.clone(), k.clone(), v.clone(), masque.clone(), None, Default::default());
        let _ = o.slice([0..1, 0..1, 0..1, 0..1]).into_data();
        durees.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    eprintln!("[attention] {nom} : {:.0} / {:.0} / {:.0} ms", durees[0], durees[1], durees[2]);
}

/// Le masque étendu par une vue à stride 0, comme le fait `patch_attention.py`.
#[test]
#[ignore]
#[allow(non_snake_case)]
fn masque_etendu_par_une_vue() {
    let device = rag3weaver::burn_device::BurnDevice::for_role(rag3weaver::burn_device::BurnRole::Embedder).resolve();
    let (q, k, v) = qkv(&device);
    let (B, H, S, _) = dims();
    let masque = masque_base(&device).expand([B, H, S, S]);
    lancer("masque étendu (vue)", q, k, v, Some(masque));
}

/// Le masque `[b, 1, 1, sk]` passé tel quel, sans expand : si cubek lit les
/// strides, il devrait diffuser tout seul.
#[test]
#[ignore]
fn masque_sans_expand() {
    let device = rag3weaver::burn_device::BurnDevice::for_role(rag3weaver::burn_device::BurnRole::Embedder).resolve();
    let (q, k, v) = qkv(&device);
    let masque = masque_base(&device);
    lancer("masque [b,1,1,sk] sans expand", q, k, v, Some(masque));
}

/// Sans masque du tout : la référence de vitesse.
#[test]
#[ignore]
fn sans_masque() {
    let device = rag3weaver::burn_device::BurnDevice::for_role(rag3weaver::burn_device::BurnRole::Embedder).resolve();
    let (q, k, v) = qkv(&device);
    lancer("sans masque", q, k, v, None);
}

/// La même sonde à 128 séquences, là où ça passe : pour comparer.
#[test]
#[ignore]
#[allow(non_snake_case)]
fn masque_etendu_a_128() {
    let device = rag3weaver::burn_device::BurnDevice::for_role(rag3weaver::burn_device::BurnRole::Embedder).resolve();
    let (q, k, v) = qkv(&device);
    let (_, H, S, _) = dims();
    let (q, k, v) = (q.slice([0..128]), k.slice([0..128]), v.slice([0..128]));
    let masque = masque_base(&device).slice([0..128]).expand([128, H, S, S]);
    lancer("masque étendu (vue), 128", q, k, v, Some(masque));
}

/// **La flash contre la voie naïve, au bit près** (écart absolu max, cosinus
/// minimal), sur les mêmes tenseurs, contigus puis arrivés par `permute`
/// comme dans les graphes générés. `attention_fallback` est la référence de
/// burn lui-même. Formes surchargeables ; par défaut celle de BGE-M3 (16 têtes).
fn parite(nom: &str, q: Tensor<4>, k: Tensor<4>, v: Tensor<4>, masque: Option<Tensor<4, Bool>>) {
    let flash = burn::tensor::module::attention(q.clone(), k.clone(), v.clone(), masque.clone(), None, Default::default());
    let naif = burn::tensor::module::attention_fallback(q, k, v, masque, None, Default::default());
    let a = flash.into_data().convert::<f32>().try_into_vec::<f32>().unwrap();
    let b = naif.into_data().convert::<f32>().try_into_vec::<f32>().unwrap();
    let ecart = a.iter().zip(&b).map(|(x, y)| (x - y).abs()).fold(0f32, f32::max);
    let (mut ab, mut aa, mut bb) = (0f64, 0f64, 0f64);
    for (x, y) in a.iter().zip(&b) { ab += (*x as f64) * (*y as f64); aa += (*x as f64) * (*x as f64); bb += (*y as f64) * (*y as f64); }
    let nan = a.iter().filter(|x| !x.is_finite()).count();
    eprintln!("[parité] {nom} : écart absolu max {ecart:.5}, cosinus {:.6}, non finis {nan}", ab / (aa.sqrt() * bb.sqrt()));
}

#[test]
#[ignore]
#[allow(non_snake_case)]
fn parite_flash_contre_naif_contigu() {
    let device = rag3weaver::burn_device::BurnDevice::for_role(rag3weaver::burn_device::BurnRole::Embedder).resolve();
    let (B, H, S, D) = (dim("RAG3WEAVER_SONDE_B", 4), dim("RAG3WEAVER_SONDE_H", 16), dim("RAG3WEAVER_SONDE_S", 512), dim("RAG3WEAVER_SONDE_D", 64));
    let q = Tensor::<4>::random([B, H, S, D], Distribution::Normal(0.0, 1.0), &device);
    let k = Tensor::<4>::random([B, H, S, D], Distribution::Normal(0.0, 1.0), &device);
    let v = Tensor::<4>::random([B, H, S, D], Distribution::Normal(0.0, 1.0), &device);
    let mut m = vec![1.0f32; B * S];
    for b in 0..B { for j in (S * 3 / 4)..S { m[b * S + j] = 0.0; } }
    let masque = Tensor::<4>::from_data(TensorData::new(m, [B, 1, 1, S]), &device).lower_elem(0.0).expand([B, H, S, S]);
    parite(&format!("contigu [{B}, {H}, {S}, {D}] sans masque"), q.clone(), k.clone(), v.clone(), None);
    parite(&format!("contigu [{B}, {H}, {S}, {D}] avec masque"), q, k, v, Some(masque));
}

#[test]
#[ignore]
#[allow(non_snake_case)]
fn parite_flash_contre_naif_permute() {
    let device = rag3weaver::burn_device::BurnDevice::for_role(rag3weaver::burn_device::BurnRole::Embedder).resolve();
    let (B, H, S, D) = (dim("RAG3WEAVER_SONDE_B", 4), dim("RAG3WEAVER_SONDE_H", 16), dim("RAG3WEAVER_SONDE_S", 512), dim("RAG3WEAVER_SONDE_D", 64));
    // [b, s, h, d] comme à la sortie des linéaires, puis permute comme le graphe.
    let q = Tensor::<4>::random([B, S, H, D], Distribution::Normal(0.0, 1.0), &device).permute([0, 2, 1, 3]);
    let k = Tensor::<4>::random([B, S, H, D], Distribution::Normal(0.0, 1.0), &device).permute([0, 2, 3, 1]).permute([0, 1, 3, 2]);
    let v = Tensor::<4>::random([B, S, H, D], Distribution::Normal(0.0, 1.0), &device).permute([0, 2, 1, 3]);
    let mut m = vec![1.0f32; B * S];
    for b in 0..B { for j in (S * 3 / 4)..S { m[b * S + j] = 0.0; } }
    let masque = Tensor::<4>::from_data(TensorData::new(m, [B, 1, 1, S]), &device).lower_elem(0.0).expand([B, H, S, S]);
    parite(&format!("permuté [{B}, {H}, {S}, {D}] sans masque"), q.clone(), k.clone(), v.clone(), None);
    parite(&format!("permuté [{B}, {H}, {S}, {D}] avec masque"), q, k, v, Some(masque));
}
