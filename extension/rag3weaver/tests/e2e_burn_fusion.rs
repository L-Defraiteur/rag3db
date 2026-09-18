//! **La fusion élémentaire en Flex32, chaîne par chaîne, contre f32.** Le
//! détecteur OCR en Flex32 est exact quand chaque opération est synchronisée
//! et faux dès que la fusion enchaîne plusieurs opérations dans un noyau
//! (chantier C, 18 septembre 2026). Ici des chaînes minimales sur les mêmes
//! octets, en Flex32 (`TensorData::from_bytes(…, Flex32)`) et en f32.
#![cfg(feature = "burn-embedder")]

use burn::tensor::{DType, Distribution, Tensor, TensorData};

fn paire(dims: [usize; 4], device: &burn::tensor::Device) -> (Tensor<4>, Tensor<4>) {
    let f = Tensor::<4>::random(dims, Distribution::Normal(0.0, 2.0), device).cast(DType::F32);
    let d = f.clone().into_data();
    let flex = Tensor::<4>::from_data(TensorData::from_bytes(d.bytes, d.shape, DType::Flex32), device);
    (f, flex)
}

fn constante(n: usize, v: f32, dtype: DType, device: &burn::tensor::Device) -> Tensor<4> {
    let d = TensorData::new(vec![v; n], [1, n, 1, 1]);
    match dtype {
        DType::Flex32 => Tensor::<4>::from_data(TensorData::from_bytes(d.bytes, d.shape, DType::Flex32), device),
        _ => Tensor::<4>::from_data(d, device),
    }
}

fn ecart(nom: &str, a: Tensor<4>, b: Tensor<4>) {
    let (da, db) = (a.dtype(), b.dtype());
    let a = a.into_data().convert::<f32>().try_into_vec::<f32>().unwrap();
    let b = b.into_data().convert::<f32>().try_into_vec::<f32>().unwrap();
    let max = a.iter().zip(&b).map(|(x, y)| (x - y).abs()).fold(0f32, f32::max);
    let mx = a.iter().fold(0f32, |m, x| m.max(x.abs()));
    eprintln!("[fusion] {nom:32} {da:?} vs {db:?} : écart absolu max {max:.6} (|max| {mx:.4})");
}

#[test]
#[ignore]
fn chaines_elementaires_flex32_contre_f32() {
    let device = rag3weaver::burn_device::BurnDevice::for_role(rag3weaver::burn_device::BurnRole::Ocr).resolve();
    let dims = [1, 8, 64, 64];
    let (f, x) = paire(dims, &device);
    // une op seule
    ecart("relu", burn::tensor::activation::relu(f.clone()), burn::tensor::activation::relu(x.clone()));
    ecart("erf", f.clone().erf(), x.clone().erf());
    // deux ops
    ecart("relu(x*2+1)", burn::tensor::activation::relu(f.clone().mul_scalar(2.0).add_scalar(1.0)), burn::tensor::activation::relu(x.clone().mul_scalar(2.0).add_scalar(1.0)));
    ecart("erf(x).add(1)", f.clone().erf().add_scalar(1.0), x.clone().erf().add_scalar(1.0));
    ecart("x.mul(erf(x))", f.clone().mul(f.clone().erf()), x.clone().mul(x.clone().erf()));
    // GELU du graphe, constantes diffusées [1, C, 1, 1] du même dtype
    let (cf, cx) = (constante(8, 1.4142135, DType::F32, &device), constante(8, 1.4142135, DType::Flex32, &device));
    let (uf, ux) = (constante(8, 1.0, DType::F32, &device), constante(8, 1.0, DType::Flex32, &device));
    let (hf, hx) = (constante(8, 0.5, DType::F32, &device), constante(8, 0.5, DType::Flex32, &device));
    let gelu = |t: Tensor<4>, c: Tensor<4>, u: Tensor<4>, h: Tensor<4>| t.clone().div(c).erf().add(u).mul(t).mul(h);
    ecart("gelu, constantes diffusées", gelu(f.clone(), cf.clone(), uf.clone(), hf.clone()), gelu(x.clone(), cx.clone(), ux.clone(), hx.clone()));
    ecart("x.div(c) seul", f.clone().div(cf.clone()), x.clone().div(cx.clone()));
    ecart("x.div(c).erf()", f.clone().div(cf.clone()).erf(), x.clone().div(cx.clone()).erf());
    ecart("x.div(c).add(u)", f.clone().div(cf.clone()).add(uf.clone()), x.clone().div(cx.clone()).add(ux.clone()));
    ecart("x.mul(u).mul(h)", f.clone().mul(uf.clone()).mul(hf.clone()), x.clone().mul(ux.clone()).mul(hx.clone()));
    // hard_sigmoid puis produit (bloc SE)
    let hs = |t: Tensor<4>| t.clone().mul(burn::tensor::activation::hard_sigmoid(t, 0.16666667, 0.5));
    ecart("x.mul(hard_sigmoid(x))", hs(f.clone()), hs(x.clone()));
    // une chaîne longue
    let longue = |t: Tensor<4>| burn::tensor::activation::relu(t.clone().mul_scalar(0.5).add_scalar(0.25).erf().mul(t).sub_scalar(0.1));
    ecart("chaîne de 6", longue(f.clone()), longue(x.clone()));
}

/// **Une chaîne suivie d'une convolution** : c'est la forme du détecteur
/// (relu/GELU puis conv), et c'est là que la fusion `nhwc_relayout` joue.
#[test]
#[ignore]
fn chaine_puis_convolution_flex32_contre_f32() {
    use burn::tensor::module::conv2d;
    use burn::tensor::ops::ConvOptions;
    let device = rag3weaver::burn_device::BurnDevice::for_role(rag3weaver::burn_device::BurnRole::Ocr).resolve();
    let (xf, xx) = paire([1, 32, 184, 616], &device);
    let (wf, wx) = paire([32, 32, 3, 3], &device);
    let (w2f, w2x) = paire([32, 32, 1, 1], &device);
    let opts = || ConvOptions::new([1, 1], [1, 1], [1, 1], 1);
    let resync = |t: Tensor<4>| { let d = t.into_data(); Tensor::<4>::from_data(d, &device) };
    ecart("conv seule 3×3", conv2d(xf.clone(), wf.clone(), None, opts()), conv2d(xx.clone(), wx.clone(), None, opts()));
    ecart("conv seule 1×1", conv2d(xf.clone(), w2f.clone(), None, opts()), conv2d(xx.clone(), w2x.clone(), None, opts()));
    let chaine = |t: Tensor<4>| burn::tensor::activation::relu(t.mul_scalar(2.0).add_scalar(1.0));
    ecart("relu(x*2+1) → conv 3×3", conv2d(chaine(xf.clone()), wf.clone(), None, opts()), conv2d(chaine(xx.clone()), wx.clone(), None, opts()));
    ecart("relu(x*2+1) → sync → conv 3×3", conv2d(resync(chaine(xf.clone())), wf.clone(), None, opts()), conv2d(resync(chaine(xx.clone())), wx.clone(), None, opts()));
    ecart("relu(x*2+1) → conv 1×1", conv2d(chaine(xf.clone()), w2f.clone(), None, opts()), conv2d(chaine(xx.clone()), w2x.clone(), None, opts()));
    ecart("conv 3×3 → relu → conv 1×1", conv2d(burn::tensor::activation::relu(conv2d(xf.clone(), wf.clone(), None, opts())), w2f.clone(), None, opts()), conv2d(burn::tensor::activation::relu(conv2d(xx.clone(), wx.clone(), None, opts())), w2x.clone(), None, opts()));
    ecart("conv 3×3 → conv 1×1", conv2d(conv2d(xf.clone(), wf.clone(), None, opts()), w2f.clone(), None, opts()), conv2d(conv2d(xx.clone(), wx.clone(), None, opts()), w2x.clone(), None, opts()));
    ecart("x*2 → conv 3×3 (sans relu)", conv2d(xf.clone().mul_scalar(2.0), wf.clone(), None, opts()), conv2d(xx.clone().mul_scalar(2.0), wx.clone(), None, opts()));
}

/// **Le bloc SE** : moyenne sur H×W, deux 1×1, hard_sigmoid, puis produit
/// diffusé sur le tenseur d'origine — un opérande `[1, C, 1, 1]` *calculé*.
#[test]
#[ignore]
fn bloc_se_flex32_contre_f32() {
    use burn::tensor::module::conv2d;
    use burn::tensor::ops::ConvOptions;
    let device = rag3weaver::burn_device::BurnDevice::for_role(rag3weaver::burn_device::BurnRole::Ocr).resolve();
    let (xf, xx) = paire([1, 32, 46, 154], &device);
    let (w1f, w1x) = paire([8, 32, 1, 1], &device);
    let (w2f, w2x) = paire([32, 8, 1, 1], &device);
    let opts = || ConvOptions::new([1, 1], [0, 0], [1, 1], 1);
    let hs = |t: Tensor<4>| burn::tensor::activation::hard_sigmoid(t, 0.16666667, 0.5);
    let moy = |t: Tensor<4>| t.mean_dim(2).mean_dim(3);
    ecart("moyenne H×W", moy(xf.clone()), moy(xx.clone()));
    ecart("x.mul(moyenne(x))", xf.clone().mul(moy(xf.clone())), xx.clone().mul(moy(xx.clone())));
    ecart("x.mul(hard_sigmoid(moyenne(x)))", xf.clone().mul(hs(moy(xf.clone()))), xx.clone().mul(hs(moy(xx.clone()))));
    let se = |t: Tensor<4>, w1: Tensor<4>, w2: Tensor<4>| {
        let m = moy(t.clone());
        let a = burn::tensor::activation::relu(conv2d(m, w1, None, opts()));
        let g = hs(conv2d(a, w2, None, opts()));
        t.mul(g)
    };
    ecart("bloc SE complet", se(xf.clone(), w1f.clone(), w2f.clone()), se(xx.clone(), w1x.clone(), w2x.clone()));
    let hs_seul = |t: Tensor<4>, w1: Tensor<4>, w2: Tensor<4>| hs(conv2d(burn::tensor::activation::relu(conv2d(moy(t), w1, None, opts())), w2, None, opts()));
    ecart("hard_sigmoid(conv(relu(conv(moy))))", hs_seul(xf.clone(), w1f.clone(), w2f.clone()), hs_seul(xx.clone(), w1x.clone(), w2x.clone()));
}

/// **Le bloc SE du graphe, à l'identique** : depthwise 3×3 (32 groupes, biais),
/// moyenne, 1×1 (biais), relu, 1×1 (biais), hard_sigmoid, produit diffusé.
#[test]
#[ignore]
fn bloc_se_apres_depthwise_flex32_contre_f32() {
    use burn::tensor::module::conv2d;
    use burn::tensor::ops::ConvOptions;
    let device = rag3weaver::burn_device::BurnDevice::for_role(rag3weaver::burn_device::BurnRole::Ocr).resolve();
    let (xf, xx) = paire([1, 32, 184, 616], &device);
    let (wdf, wdx) = paire([32, 1, 3, 3], &device);
    let (w1f, w1x) = paire([8, 32, 1, 1], &device);
    let (w2f, w2x) = paire([32, 8, 1, 1], &device);
    let biais = |n: usize, dtype: DType| { let d = TensorData::new((0..n).map(|i| (i as f32) * 0.01 - 0.1).collect::<Vec<_>>(), [n]); match dtype { DType::Flex32 => Tensor::<1>::from_data(TensorData::from_bytes(d.bytes, d.shape, DType::Flex32), &device), _ => Tensor::<1>::from_data(d, &device) } };
    let (bdf, bdx) = (biais(32, DType::F32), biais(32, DType::Flex32));
    let (b1f, b1x) = (biais(8, DType::F32), biais(8, DType::Flex32));
    let (b2f, b2x) = (biais(32, DType::F32), biais(32, DType::Flex32));
    let dw = || ConvOptions::new([1, 1], [1, 1], [1, 1], 32);
    let un = || ConvOptions::new([1, 1], [0, 0], [1, 1], 1);
    let hs = |t: Tensor<4>| burn::tensor::activation::hard_sigmoid(t, 0.16666667, 0.5);
    let moy = |t: Tensor<4>| t.mean_dim(2).mean_dim(3);
    let bloc = |x: Tensor<4>, wd: Tensor<4>, bd: Option<Tensor<1>>, w1: Tensor<4>, b1: Option<Tensor<1>>, w2: Tensor<4>, b2: Option<Tensor<1>>, groupes: usize| {
        let opts_d = ConvOptions::new([1, 1], [1, 1], [1, 1], groupes);
        let wd = if groupes == 1 { wd.repeat_dim(1, 32) } else { wd };
        let y = conv2d(x, wd, bd, opts_d);
        let m = moy(y.clone());
        let a = burn::tensor::activation::relu(conv2d(m, w1, b1, un()));
        let g = hs(conv2d(a, w2, b2, un()));
        y.mul(g)
    };
    ecart("depthwise+biais → SE(biais) → mul", bloc(xf.clone(), wdf.clone(), Some(bdf.clone()), w1f.clone(), Some(b1f.clone()), w2f.clone(), Some(b2f.clone()), 32), bloc(xx.clone(), wdx.clone(), Some(bdx.clone()), w1x.clone(), Some(b1x.clone()), w2x.clone(), Some(b2x.clone()), 32));
    ecart("depthwise sans biais → SE sans biais", bloc(xf.clone(), wdf.clone(), None, w1f.clone(), None, w2f.clone(), None, 32), bloc(xx.clone(), wdx.clone(), None, w1x.clone(), None, w2x.clone(), None, 32));
    ecart("depthwise+biais → SE sans biais", bloc(xf.clone(), wdf.clone(), Some(bdf.clone()), w1f.clone(), None, w2f.clone(), None, 32), bloc(xx.clone(), wdx.clone(), Some(bdx.clone()), w1x.clone(), None, w2x.clone(), None, 32));
    ecart("depthwise sans biais → SE(biais)", bloc(xf.clone(), wdf.clone(), None, w1f.clone(), Some(b1f.clone()), w2f.clone(), Some(b2f.clone()), 32), bloc(xx.clone(), wdx.clone(), None, w1x.clone(), Some(b1x.clone()), w2x.clone(), Some(b2x.clone()), 32));
    ecart("conv pleine+biais → SE(biais)", bloc(xf.clone(), wdf.clone(), Some(bdf.clone()), w1f.clone(), Some(b1f.clone()), w2f.clone(), Some(b2f.clone()), 1), bloc(xx.clone(), wdx.clone(), Some(bdx.clone()), w1x.clone(), Some(b1x.clone()), w2x.clone(), Some(b2x.clone()), 1));
    ecart("depthwise+biais seule", conv2d(xf.clone(), wdf.clone(), Some(bdf.clone()), dw()), conv2d(xx.clone(), wdx.clone(), Some(bdx.clone()), dw()));
    let _ = un;
}
