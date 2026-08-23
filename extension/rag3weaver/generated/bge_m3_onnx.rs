// ─────────────────────────────────────────────────────────────────────────────
//  CODE GÉNÉRÉ — NE PAS ÉDITER À LA MAIN
// ─────────────────────────────────────────────────────────────────────────────
//
//  Ce fichier n'est pas de nous. C'est la traduction mécanique, par burn-onnx,
//  du graphe ONNX publié par BAAI. Aucune ligne n'a été écrite à la main, et
//  aucune ne doit l'être : toute modification serait perdue à la régénération.
//
//  Modèle source : BAAI/bge-m3        https://huggingface.co/BAAI/bge-m3
//                  XLM-RoBERTa-large, 24 couches, hidden 1024, vocab 250 002
//                  Licence MIT — tout le mérite revient à ses auteurs
//                  (Chen, Xiao, Zhang, Luo, Lian, Liu — BAAI)
//
//  Entrée        : onnx/model.onnx + onnx/model.onnx_data du dépôt ci-dessus
//  Outil         : burn-onnx 0.22.0-pre.1, LoadStrategy::Bytes
//                  (la 0.21 échoue : base_path non propagé sur le chemin mmap
//                   zero-copy de onnx-ir, or ce modèle dépasse la limite de 2 Go
//                   du protobuf ONNX et doit donc utiliser l'external data)
//
//  Poids         : NON inclus ici (2,2 Go). Publiés séparément :
//                  https://huggingface.co/Lucie666/bge-m3-burnpack
//                  Chargement : Model::from_bytes(bytes, &device)
//
//  Régénération  : voir generated/README.md
//
//  Interface     : forward(input_ids: Tensor<2, Int>, attention_mask: Tensor<2, Int>)
//                    -> (Tensor<3>, Tensor<2>)
//                       token_embeddings [B,S,1024] — pour la tête sparse
//                       sentence_embedding [B,1024] — dense, CLS-poolé + L2-normalisé
//
//  Parité vérifiée contre candle-transformers XLMRobertaModel : cosinus 1.00000000
// ─────────────────────────────────────────────────────────────────────────────

use burn::prelude::*;
use burn::nn::Linear;
use burn::nn::LinearConfig;
use burn::tensor::Bytes;
use burn_store::BurnpackStore;
use burn_store::ModuleSnapshot;


#[derive(Module, Debug)]
pub struct Submodule1 {
    constant393: burn::module::Param<Tensor<2, Int>>,
    constant398: burn::module::Param<Tensor<1, Int>>,
    constant399: burn::module::Param<Tensor<1>>,
    constant400: burn::module::Param<Tensor<1>>,
    constant403: burn::module::Param<Tensor<1, Int>>,
    constant1: burn::module::Param<Tensor<2>>,
    constant3: burn::module::Param<Tensor<2>>,
    constant2: burn::module::Param<Tensor<2>>,
    constant404: burn::module::Param<Tensor<1>>,
    constant405: burn::module::Param<Tensor<1>>,
    constant4: burn::module::Param<Tensor<1>>,
    constant5: burn::module::Param<Tensor<1>>,
    linear1: Linear,
    linear2: Linear,
    linear3: Linear,
    constant418: burn::module::Param<Tensor<1>>,
    linear4: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule1 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant393: burn::module::Param<Tensor<2, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
                Int,
            >::zeros([1, 8194], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [1, 8194].into(),
        );
        let constant398: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from([-1i64]),
                (device, burn::tensor::DType::I64),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant399: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant400: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([
                    -340282346638528860000000000000000000000f64,
                ]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant403: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from([1i64]),
                (device, burn::tensor::DType::I64),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([250002, 1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [250002, 1024].into(),
        );
        let constant3: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([1, 1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 1024].into(),
        );
        let constant2: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([8194, 1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [8194, 1024].into(),
        );
        let constant404: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant405: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant4: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant5: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear1 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear2 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear3 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant418: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear4 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant393,
            constant398,
            constant399,
            constant400,
            constant403,
            constant1,
            constant3,
            constant2,
            constant404,
            constant405,
            constant4,
            constant5,
            linear1,
            linear2,
            linear3,
            constant418,
            linear4,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        input_ids: Tensor<2, Int>,
        attention_mask: Tensor<2, Int>,
    ) -> (Tensor<3>, Tensor<4>) {
        let shape1_out1: [i64; 2] = {
            let axes = &input_ids.clone().dims()[0..2];
            let mut output = [0i64; 2];
            for i in 0..2 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather1_out1 = shape1_out1[0] as i64;
        let gather2_out1 = shape1_out1[1] as i64;
        let unsqueeze1_out1 = [gather2_out1 as i64];
        let constant393_out1 = self.constant393.val();
        let slice1_out1 = constant393_out1.slice(s![.., 0..unsqueeze1_out1[0]]);
        let unsqueeze2_out1 = [gather1_out1 as i64];
        let concat1_out1: [i64; 2usize] = [&unsqueeze2_out1[..], &unsqueeze1_out1[..]]
            .concat()
            .try_into()
            .unwrap();
        let reshape1_out1 = concat1_out1;
        let shape3_out1: [i64; 1] = [2i64];
        let constantofshape1_out1 = Tensor::<
            1,
            Int,
        >::from_data(
                burn::tensor::TensorData::from([1i64 as i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .reshape([1])
            .expand(shape3_out1);
        let constant398_out1 = self.constant398.val();
        let mul1_out1 = constantofshape1_out1.clone().mul(constant398_out1);
        let equal1_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(reshape1_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(mul1_out1)
        };
        let where1_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&reshape1_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal1_out1, constantofshape1_out1);
        let expand1_out1 = {
            let onnx_shape: [i64; 2usize] = TryInto::<
                [i64; 2usize],
            >::try_into(where1_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = slice1_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..2usize {
                let dim_offset = 2usize - 2usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            slice1_out1.expand(shape)
        };
        let unsqueeze4_out1: Tensor<3, Int> = attention_mask.unsqueeze_dims::<3>(&[1]);
        let unsqueeze5_out1: Tensor<4, Int> = unsqueeze4_out1.unsqueeze_dims::<4>(&[2]);
        let cast1_out1 = unsqueeze5_out1.float().cast(burn::tensor::DType::F32);
        let constant399_out1 = self.constant399.val();
        let sub1_out1 = (constant399_out1)
            .unsqueeze_dims(&[0isize, 1isize, 2isize])
            .sub(cast1_out1);
        let constant400_out1 = self.constant400.val();
        let mul2_out1 = sub1_out1
            .mul((constant400_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let constant401_out1 = 1i64;
        let equal2_out1 = input_ids.clone().equal_elem(constant401_out1);
        let not1_out1 = equal2_out1.bool_not();
        let cast2_out1 = not1_out1.int().cast(burn::tensor::DType::I32);
        let cumsum1_out1 = cast2_out1.clone().cumsum(1);
        let mul3_out1 = cumsum1_out1.mul(cast2_out1);
        let cast3_out1 = mul3_out1.cast(burn::tensor::DType::I64);
        let constant403_out1 = self.constant403.val();
        let add1_out1 = cast3_out1.add((constant403_out1).unsqueeze_dims(&[0isize]));
        let constant1_out1 = self.constant1.val();
        let gather3_out1 = constant1_out1.take::<2, 3>(0, input_ids);
        let constant3_out1 = self.constant3.val();
        let gather4_out1 = constant3_out1.take::<2, 3>(0, expand1_out1);
        let add2_out1 = gather3_out1.add(gather4_out1);
        let constant2_out1 = self.constant2.val();
        let gather5_out1 = constant2_out1.take::<2, 3>(0, add1_out1);
        let add3_out1 = add2_out1.add(gather5_out1);
        let reducemean1_out1 = { add3_out1.clone().mean_dim(2usize) };
        let sub2_out1 = add3_out1.sub(reducemean1_out1);
        let constant404_out1 = self.constant404.val();
        let pow1_out1 = sub2_out1
            .clone()
            .powf((constant404_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean2_out1 = { pow1_out1.mean_dim(2usize) };
        let constant405_out1 = self.constant405.val();
        let add4_out1 = reducemean2_out1
            .add((constant405_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt1_out1 = add4_out1.sqrt();
        let div1_out1 = sub2_out1.div(sqrt1_out1);
        let constant4_out1 = self.constant4.val();
        let mul4_out1 = div1_out1
            .mul((constant4_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant5_out1 = self.constant5.val();
        let add5_out1 = mul4_out1
            .add((constant5_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear1_out1 = self.linear1.forward(add5_out1.clone());
        let linear2_out1 = self.linear2.forward(add5_out1.clone());
        let shape4_out1: [i64; 3] = {
            let axes = &linear2_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather6_out1 = shape4_out1[0] as i64;
        let gather7_out1 = shape4_out1[1] as i64;
        let unsqueeze6_out1 = [gather6_out1 as i64];
        let unsqueeze7_out1 = [gather7_out1 as i64];
        let constant409_out1: [i64; 1] = [64i64];
        let constant408_out1: [i64; 1] = [16i64];
        let concat2_out1: [i64; 4usize] = [
            &unsqueeze6_out1[..],
            &unsqueeze7_out1[..],
            &constant408_out1[..],
            &constant409_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape2_out1 = linear2_out1.reshape(concat2_out1);
        let linear3_out1 = self.linear3.forward(add5_out1.clone());
        let shape6_out1: [i64; 3] = {
            let axes = &linear3_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather8_out1 = shape6_out1[0] as i64;
        let gather9_out1 = shape6_out1[1] as i64;
        let unsqueeze8_out1 = [gather8_out1 as i64];
        let unsqueeze9_out1 = [gather9_out1 as i64];
        let constant413_out1: [i64; 1] = [64i64];
        let constant412_out1: [i64; 1] = [16i64];
        let concat3_out1: [i64; 4usize] = [
            &unsqueeze8_out1[..],
            &unsqueeze9_out1[..],
            &constant412_out1[..],
            &constant413_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape3_out1 = linear3_out1.reshape(concat3_out1);
        let transpose1_out1 = reshape3_out1.permute([0, 2, 1, 3]);
        let shape8_out1: [i64; 3] = {
            let axes = &linear1_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather10_out1 = shape8_out1[0] as i64;
        let gather11_out1 = shape8_out1[1] as i64;
        let unsqueeze10_out1 = [gather10_out1 as i64];
        let unsqueeze11_out1 = [gather11_out1 as i64];
        let constant417_out1: [i64; 1] = [64i64];
        let constant416_out1: [i64; 1] = [16i64];
        let concat4_out1: [i64; 4usize] = [
            &unsqueeze10_out1[..],
            &unsqueeze11_out1[..],
            &constant416_out1[..],
            &constant417_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape4_out1 = linear1_out1.reshape(concat4_out1);
        let transpose2_out1 = reshape4_out1.permute([0, 2, 1, 3]);
        let transpose3_out1 = reshape2_out1.permute([0, 2, 3, 1]);
        let matmul4_out1 = transpose2_out1.matmul(transpose3_out1);
        let constant418_out1 = self.constant418.val();
        let div2_out1 = matmul4_out1
            .div((constant418_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add6_out1 = div2_out1.add(mul2_out1.clone());
        let softmax1_out1 = burn::tensor::activation::softmax(add6_out1, 3);
        let matmul5_out1 = softmax1_out1.matmul(transpose1_out1);
        let transpose4_out1 = matmul5_out1.permute([0, 2, 1, 3]);
        let shape10_out1: [i64; 4] = {
            let axes = &transpose4_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather12_out1 = shape10_out1[0] as i64;
        let gather13_out1 = shape10_out1[1] as i64;
        let unsqueeze12_out1 = [gather12_out1 as i64];
        let unsqueeze13_out1 = [gather13_out1 as i64];
        let constant421_out1: [i64; 1] = [1024i64];
        let concat5_out1: [i64; 3usize] = [
            &unsqueeze12_out1[..],
            &unsqueeze13_out1[..],
            &constant421_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape5_out1 = transpose4_out1.reshape(concat5_out1);
        let linear4_out1 = self.linear4.forward(reshape5_out1);
        let add7_out1 = linear4_out1.add(add5_out1);
        (add7_out1, mul2_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule2 {
    constant422: burn::module::Param<Tensor<1>>,
    constant423: burn::module::Param<Tensor<1>>,
    constant10: burn::module::Param<Tensor<1>>,
    constant11: burn::module::Param<Tensor<1>>,
    linear5: Linear,
    constant424: burn::module::Param<Tensor<1>>,
    constant425: burn::module::Param<Tensor<1>>,
    constant426: burn::module::Param<Tensor<1>>,
    linear6: Linear,
    constant427: burn::module::Param<Tensor<1>>,
    constant428: burn::module::Param<Tensor<1>>,
    constant14: burn::module::Param<Tensor<1>>,
    constant15: burn::module::Param<Tensor<1>>,
    linear7: Linear,
    linear8: Linear,
    linear9: Linear,
    constant441: burn::module::Param<Tensor<1>>,
    linear10: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule2 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant422: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant423: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant10: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant11: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear5 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant424: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant425: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant426: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear6 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant427: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant428: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant14: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant15: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear7 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear8 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear9 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant441: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear10 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant422,
            constant423,
            constant10,
            constant11,
            linear5,
            constant424,
            constant425,
            constant426,
            linear6,
            constant427,
            constant428,
            constant14,
            constant15,
            linear7,
            linear8,
            linear9,
            constant441,
            linear10,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add7_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean3_out1 = { add7_out1.clone().mean_dim(2usize) };
        let sub3_out1 = add7_out1.sub(reducemean3_out1);
        let constant422_out1 = self.constant422.val();
        let pow2_out1 = sub3_out1
            .clone()
            .powf((constant422_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean4_out1 = { pow2_out1.mean_dim(2usize) };
        let constant423_out1 = self.constant423.val();
        let add8_out1 = reducemean4_out1
            .add((constant423_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt2_out1 = add8_out1.sqrt();
        let div3_out1 = sub3_out1.div(sqrt2_out1);
        let constant10_out1 = self.constant10.val();
        let mul5_out1 = div3_out1
            .mul((constant10_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant11_out1 = self.constant11.val();
        let add9_out1 = mul5_out1
            .add((constant11_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear5_out1 = self.linear5.forward(add9_out1.clone());
        let constant424_out1 = self.constant424.val();
        let div4_out1 = linear5_out1
            .clone()
            .div((constant424_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf1_out1 = div4_out1.erf();
        let constant425_out1 = self.constant425.val();
        let add10_out1 = erf1_out1
            .add((constant425_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul6_out1 = linear5_out1.mul(add10_out1);
        let constant426_out1 = self.constant426.val();
        let mul7_out1 = mul6_out1
            .mul((constant426_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear6_out1 = self.linear6.forward(mul7_out1);
        let add11_out1 = linear6_out1.add(add9_out1);
        let reducemean5_out1 = { add11_out1.clone().mean_dim(2usize) };
        let sub4_out1 = add11_out1.sub(reducemean5_out1);
        let constant427_out1 = self.constant427.val();
        let pow3_out1 = sub4_out1
            .clone()
            .powf((constant427_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean6_out1 = { pow3_out1.mean_dim(2usize) };
        let constant428_out1 = self.constant428.val();
        let add12_out1 = reducemean6_out1
            .add((constant428_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt3_out1 = add12_out1.sqrt();
        let div5_out1 = sub4_out1.div(sqrt3_out1);
        let constant14_out1 = self.constant14.val();
        let mul8_out1 = div5_out1
            .mul((constant14_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant15_out1 = self.constant15.val();
        let add13_out1 = mul8_out1
            .add((constant15_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear7_out1 = self.linear7.forward(add13_out1.clone());
        let linear8_out1 = self.linear8.forward(add13_out1.clone());
        let shape12_out1: [i64; 3] = {
            let axes = &linear8_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather14_out1 = shape12_out1[0] as i64;
        let gather15_out1 = shape12_out1[1] as i64;
        let unsqueeze14_out1 = [gather14_out1 as i64];
        let unsqueeze15_out1 = [gather15_out1 as i64];
        let constant432_out1: [i64; 1] = [64i64];
        let constant431_out1: [i64; 1] = [16i64];
        let concat6_out1: [i64; 4usize] = [
            &unsqueeze14_out1[..],
            &unsqueeze15_out1[..],
            &constant431_out1[..],
            &constant432_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape6_out1 = linear8_out1.reshape(concat6_out1);
        let linear9_out1 = self.linear9.forward(add13_out1.clone());
        let shape14_out1: [i64; 3] = {
            let axes = &linear9_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather16_out1 = shape14_out1[0] as i64;
        let gather17_out1 = shape14_out1[1] as i64;
        let unsqueeze16_out1 = [gather16_out1 as i64];
        let unsqueeze17_out1 = [gather17_out1 as i64];
        let constant436_out1: [i64; 1] = [64i64];
        let constant435_out1: [i64; 1] = [16i64];
        let concat7_out1: [i64; 4usize] = [
            &unsqueeze16_out1[..],
            &unsqueeze17_out1[..],
            &constant435_out1[..],
            &constant436_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape7_out1 = linear9_out1.reshape(concat7_out1);
        let transpose5_out1 = reshape7_out1.permute([0, 2, 1, 3]);
        let shape16_out1: [i64; 3] = {
            let axes = &linear7_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather18_out1 = shape16_out1[0] as i64;
        let gather19_out1 = shape16_out1[1] as i64;
        let unsqueeze18_out1 = [gather18_out1 as i64];
        let unsqueeze19_out1 = [gather19_out1 as i64];
        let constant440_out1: [i64; 1] = [64i64];
        let constant439_out1: [i64; 1] = [16i64];
        let concat8_out1: [i64; 4usize] = [
            &unsqueeze18_out1[..],
            &unsqueeze19_out1[..],
            &constant439_out1[..],
            &constant440_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape8_out1 = linear7_out1.reshape(concat8_out1);
        let transpose6_out1 = reshape8_out1.permute([0, 2, 1, 3]);
        let transpose7_out1 = reshape6_out1.permute([0, 2, 3, 1]);
        let matmul12_out1 = transpose6_out1.matmul(transpose7_out1);
        let constant441_out1 = self.constant441.val();
        let div6_out1 = matmul12_out1
            .div((constant441_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add14_out1 = div6_out1.add(mul2_out1);
        let softmax2_out1 = burn::tensor::activation::softmax(add14_out1, 3);
        let matmul13_out1 = softmax2_out1.matmul(transpose5_out1);
        let transpose8_out1 = matmul13_out1.permute([0, 2, 1, 3]);
        let shape18_out1: [i64; 4] = {
            let axes = &transpose8_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather20_out1 = shape18_out1[0] as i64;
        let gather21_out1 = shape18_out1[1] as i64;
        let unsqueeze20_out1 = [gather20_out1 as i64];
        let unsqueeze21_out1 = [gather21_out1 as i64];
        let constant444_out1: [i64; 1] = [1024i64];
        let concat9_out1: [i64; 3usize] = [
            &unsqueeze20_out1[..],
            &unsqueeze21_out1[..],
            &constant444_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape9_out1 = transpose8_out1.reshape(concat9_out1);
        let linear10_out1 = self.linear10.forward(reshape9_out1);
        let add15_out1 = linear10_out1.add(add13_out1);
        add15_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule3 {
    constant445: burn::module::Param<Tensor<1>>,
    constant446: burn::module::Param<Tensor<1>>,
    constant20: burn::module::Param<Tensor<1>>,
    constant21: burn::module::Param<Tensor<1>>,
    linear11: Linear,
    constant447: burn::module::Param<Tensor<1>>,
    constant448: burn::module::Param<Tensor<1>>,
    constant449: burn::module::Param<Tensor<1>>,
    linear12: Linear,
    constant450: burn::module::Param<Tensor<1>>,
    constant451: burn::module::Param<Tensor<1>>,
    constant24: burn::module::Param<Tensor<1>>,
    constant25: burn::module::Param<Tensor<1>>,
    linear13: Linear,
    linear14: Linear,
    linear15: Linear,
    constant464: burn::module::Param<Tensor<1>>,
    linear16: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule3 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant445: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant446: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant20: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant21: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear11 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant447: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant448: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant449: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear12 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant450: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant451: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant24: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant25: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear13 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear14 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear15 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant464: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear16 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant445,
            constant446,
            constant20,
            constant21,
            linear11,
            constant447,
            constant448,
            constant449,
            linear12,
            constant450,
            constant451,
            constant24,
            constant25,
            linear13,
            linear14,
            linear15,
            constant464,
            linear16,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add15_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean7_out1 = { add15_out1.clone().mean_dim(2usize) };
        let sub5_out1 = add15_out1.sub(reducemean7_out1);
        let constant445_out1 = self.constant445.val();
        let pow4_out1 = sub5_out1
            .clone()
            .powf((constant445_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean8_out1 = { pow4_out1.mean_dim(2usize) };
        let constant446_out1 = self.constant446.val();
        let add16_out1 = reducemean8_out1
            .add((constant446_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt4_out1 = add16_out1.sqrt();
        let div7_out1 = sub5_out1.div(sqrt4_out1);
        let constant20_out1 = self.constant20.val();
        let mul9_out1 = div7_out1
            .mul((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant21_out1 = self.constant21.val();
        let add17_out1 = mul9_out1
            .add((constant21_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear11_out1 = self.linear11.forward(add17_out1.clone());
        let constant447_out1 = self.constant447.val();
        let div8_out1 = linear11_out1
            .clone()
            .div((constant447_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf2_out1 = div8_out1.erf();
        let constant448_out1 = self.constant448.val();
        let add18_out1 = erf2_out1
            .add((constant448_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul10_out1 = linear11_out1.mul(add18_out1);
        let constant449_out1 = self.constant449.val();
        let mul11_out1 = mul10_out1
            .mul((constant449_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear12_out1 = self.linear12.forward(mul11_out1);
        let add19_out1 = linear12_out1.add(add17_out1);
        let reducemean9_out1 = { add19_out1.clone().mean_dim(2usize) };
        let sub6_out1 = add19_out1.sub(reducemean9_out1);
        let constant450_out1 = self.constant450.val();
        let pow5_out1 = sub6_out1
            .clone()
            .powf((constant450_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean10_out1 = { pow5_out1.mean_dim(2usize) };
        let constant451_out1 = self.constant451.val();
        let add20_out1 = reducemean10_out1
            .add((constant451_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt5_out1 = add20_out1.sqrt();
        let div9_out1 = sub6_out1.div(sqrt5_out1);
        let constant24_out1 = self.constant24.val();
        let mul12_out1 = div9_out1
            .mul((constant24_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant25_out1 = self.constant25.val();
        let add21_out1 = mul12_out1
            .add((constant25_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear13_out1 = self.linear13.forward(add21_out1.clone());
        let linear14_out1 = self.linear14.forward(add21_out1.clone());
        let shape20_out1: [i64; 3] = {
            let axes = &linear14_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather22_out1 = shape20_out1[0] as i64;
        let gather23_out1 = shape20_out1[1] as i64;
        let unsqueeze22_out1 = [gather22_out1 as i64];
        let unsqueeze23_out1 = [gather23_out1 as i64];
        let constant455_out1: [i64; 1] = [64i64];
        let constant454_out1: [i64; 1] = [16i64];
        let concat10_out1: [i64; 4usize] = [
            &unsqueeze22_out1[..],
            &unsqueeze23_out1[..],
            &constant454_out1[..],
            &constant455_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape10_out1 = linear14_out1.reshape(concat10_out1);
        let linear15_out1 = self.linear15.forward(add21_out1.clone());
        let shape22_out1: [i64; 3] = {
            let axes = &linear15_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather24_out1 = shape22_out1[0] as i64;
        let gather25_out1 = shape22_out1[1] as i64;
        let unsqueeze24_out1 = [gather24_out1 as i64];
        let unsqueeze25_out1 = [gather25_out1 as i64];
        let constant459_out1: [i64; 1] = [64i64];
        let constant458_out1: [i64; 1] = [16i64];
        let concat11_out1: [i64; 4usize] = [
            &unsqueeze24_out1[..],
            &unsqueeze25_out1[..],
            &constant458_out1[..],
            &constant459_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape11_out1 = linear15_out1.reshape(concat11_out1);
        let transpose9_out1 = reshape11_out1.permute([0, 2, 1, 3]);
        let shape24_out1: [i64; 3] = {
            let axes = &linear13_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather26_out1 = shape24_out1[0] as i64;
        let gather27_out1 = shape24_out1[1] as i64;
        let unsqueeze26_out1 = [gather26_out1 as i64];
        let unsqueeze27_out1 = [gather27_out1 as i64];
        let constant463_out1: [i64; 1] = [64i64];
        let constant462_out1: [i64; 1] = [16i64];
        let concat12_out1: [i64; 4usize] = [
            &unsqueeze26_out1[..],
            &unsqueeze27_out1[..],
            &constant462_out1[..],
            &constant463_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape12_out1 = linear13_out1.reshape(concat12_out1);
        let transpose10_out1 = reshape12_out1.permute([0, 2, 1, 3]);
        let transpose11_out1 = reshape10_out1.permute([0, 2, 3, 1]);
        let matmul20_out1 = transpose10_out1.matmul(transpose11_out1);
        let constant464_out1 = self.constant464.val();
        let div10_out1 = matmul20_out1
            .div((constant464_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add22_out1 = div10_out1.add(mul2_out1);
        let softmax3_out1 = burn::tensor::activation::softmax(add22_out1, 3);
        let matmul21_out1 = softmax3_out1.matmul(transpose9_out1);
        let transpose12_out1 = matmul21_out1.permute([0, 2, 1, 3]);
        let shape26_out1: [i64; 4] = {
            let axes = &transpose12_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather28_out1 = shape26_out1[0] as i64;
        let gather29_out1 = shape26_out1[1] as i64;
        let unsqueeze28_out1 = [gather28_out1 as i64];
        let unsqueeze29_out1 = [gather29_out1 as i64];
        let constant467_out1: [i64; 1] = [1024i64];
        let concat13_out1: [i64; 3usize] = [
            &unsqueeze28_out1[..],
            &unsqueeze29_out1[..],
            &constant467_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape13_out1 = transpose12_out1.reshape(concat13_out1);
        let linear16_out1 = self.linear16.forward(reshape13_out1);
        let add23_out1 = linear16_out1.add(add21_out1);
        add23_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule4 {
    constant468: burn::module::Param<Tensor<1>>,
    constant469: burn::module::Param<Tensor<1>>,
    constant30: burn::module::Param<Tensor<1>>,
    constant31: burn::module::Param<Tensor<1>>,
    linear17: Linear,
    constant470: burn::module::Param<Tensor<1>>,
    constant471: burn::module::Param<Tensor<1>>,
    constant472: burn::module::Param<Tensor<1>>,
    linear18: Linear,
    constant473: burn::module::Param<Tensor<1>>,
    constant474: burn::module::Param<Tensor<1>>,
    constant34: burn::module::Param<Tensor<1>>,
    constant35: burn::module::Param<Tensor<1>>,
    linear19: Linear,
    linear20: Linear,
    linear21: Linear,
    constant487: burn::module::Param<Tensor<1>>,
    linear22: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule4 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant468: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant469: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant30: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant31: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear17 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant470: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant471: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant472: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear18 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant473: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant474: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant34: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant35: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear19 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear20 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear21 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant487: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear22 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant468,
            constant469,
            constant30,
            constant31,
            linear17,
            constant470,
            constant471,
            constant472,
            linear18,
            constant473,
            constant474,
            constant34,
            constant35,
            linear19,
            linear20,
            linear21,
            constant487,
            linear22,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add23_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean11_out1 = { add23_out1.clone().mean_dim(2usize) };
        let sub7_out1 = add23_out1.sub(reducemean11_out1);
        let constant468_out1 = self.constant468.val();
        let pow6_out1 = sub7_out1
            .clone()
            .powf((constant468_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean12_out1 = { pow6_out1.mean_dim(2usize) };
        let constant469_out1 = self.constant469.val();
        let add24_out1 = reducemean12_out1
            .add((constant469_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt6_out1 = add24_out1.sqrt();
        let div11_out1 = sub7_out1.div(sqrt6_out1);
        let constant30_out1 = self.constant30.val();
        let mul13_out1 = div11_out1
            .mul((constant30_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant31_out1 = self.constant31.val();
        let add25_out1 = mul13_out1
            .add((constant31_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear17_out1 = self.linear17.forward(add25_out1.clone());
        let constant470_out1 = self.constant470.val();
        let div12_out1 = linear17_out1
            .clone()
            .div((constant470_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf3_out1 = div12_out1.erf();
        let constant471_out1 = self.constant471.val();
        let add26_out1 = erf3_out1
            .add((constant471_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul14_out1 = linear17_out1.mul(add26_out1);
        let constant472_out1 = self.constant472.val();
        let mul15_out1 = mul14_out1
            .mul((constant472_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear18_out1 = self.linear18.forward(mul15_out1);
        let add27_out1 = linear18_out1.add(add25_out1);
        let reducemean13_out1 = { add27_out1.clone().mean_dim(2usize) };
        let sub8_out1 = add27_out1.sub(reducemean13_out1);
        let constant473_out1 = self.constant473.val();
        let pow7_out1 = sub8_out1
            .clone()
            .powf((constant473_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean14_out1 = { pow7_out1.mean_dim(2usize) };
        let constant474_out1 = self.constant474.val();
        let add28_out1 = reducemean14_out1
            .add((constant474_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt7_out1 = add28_out1.sqrt();
        let div13_out1 = sub8_out1.div(sqrt7_out1);
        let constant34_out1 = self.constant34.val();
        let mul16_out1 = div13_out1
            .mul((constant34_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant35_out1 = self.constant35.val();
        let add29_out1 = mul16_out1
            .add((constant35_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear19_out1 = self.linear19.forward(add29_out1.clone());
        let linear20_out1 = self.linear20.forward(add29_out1.clone());
        let shape28_out1: [i64; 3] = {
            let axes = &linear20_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather30_out1 = shape28_out1[0] as i64;
        let gather31_out1 = shape28_out1[1] as i64;
        let unsqueeze30_out1 = [gather30_out1 as i64];
        let unsqueeze31_out1 = [gather31_out1 as i64];
        let constant478_out1: [i64; 1] = [64i64];
        let constant477_out1: [i64; 1] = [16i64];
        let concat14_out1: [i64; 4usize] = [
            &unsqueeze30_out1[..],
            &unsqueeze31_out1[..],
            &constant477_out1[..],
            &constant478_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape14_out1 = linear20_out1.reshape(concat14_out1);
        let linear21_out1 = self.linear21.forward(add29_out1.clone());
        let shape30_out1: [i64; 3] = {
            let axes = &linear21_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather32_out1 = shape30_out1[0] as i64;
        let gather33_out1 = shape30_out1[1] as i64;
        let unsqueeze32_out1 = [gather32_out1 as i64];
        let unsqueeze33_out1 = [gather33_out1 as i64];
        let constant482_out1: [i64; 1] = [64i64];
        let constant481_out1: [i64; 1] = [16i64];
        let concat15_out1: [i64; 4usize] = [
            &unsqueeze32_out1[..],
            &unsqueeze33_out1[..],
            &constant481_out1[..],
            &constant482_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape15_out1 = linear21_out1.reshape(concat15_out1);
        let transpose13_out1 = reshape15_out1.permute([0, 2, 1, 3]);
        let shape32_out1: [i64; 3] = {
            let axes = &linear19_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather34_out1 = shape32_out1[0] as i64;
        let gather35_out1 = shape32_out1[1] as i64;
        let unsqueeze34_out1 = [gather34_out1 as i64];
        let unsqueeze35_out1 = [gather35_out1 as i64];
        let constant486_out1: [i64; 1] = [64i64];
        let constant485_out1: [i64; 1] = [16i64];
        let concat16_out1: [i64; 4usize] = [
            &unsqueeze34_out1[..],
            &unsqueeze35_out1[..],
            &constant485_out1[..],
            &constant486_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape16_out1 = linear19_out1.reshape(concat16_out1);
        let transpose14_out1 = reshape16_out1.permute([0, 2, 1, 3]);
        let transpose15_out1 = reshape14_out1.permute([0, 2, 3, 1]);
        let matmul28_out1 = transpose14_out1.matmul(transpose15_out1);
        let constant487_out1 = self.constant487.val();
        let div14_out1 = matmul28_out1
            .div((constant487_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add30_out1 = div14_out1.add(mul2_out1);
        let softmax4_out1 = burn::tensor::activation::softmax(add30_out1, 3);
        let matmul29_out1 = softmax4_out1.matmul(transpose13_out1);
        let transpose16_out1 = matmul29_out1.permute([0, 2, 1, 3]);
        let shape34_out1: [i64; 4] = {
            let axes = &transpose16_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather36_out1 = shape34_out1[0] as i64;
        let gather37_out1 = shape34_out1[1] as i64;
        let unsqueeze36_out1 = [gather36_out1 as i64];
        let unsqueeze37_out1 = [gather37_out1 as i64];
        let constant490_out1: [i64; 1] = [1024i64];
        let concat17_out1: [i64; 3usize] = [
            &unsqueeze36_out1[..],
            &unsqueeze37_out1[..],
            &constant490_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape17_out1 = transpose16_out1.reshape(concat17_out1);
        let linear22_out1 = self.linear22.forward(reshape17_out1);
        let add31_out1 = linear22_out1.add(add29_out1);
        add31_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule5 {
    constant491: burn::module::Param<Tensor<1>>,
    constant492: burn::module::Param<Tensor<1>>,
    constant40: burn::module::Param<Tensor<1>>,
    constant41: burn::module::Param<Tensor<1>>,
    linear23: Linear,
    constant493: burn::module::Param<Tensor<1>>,
    constant494: burn::module::Param<Tensor<1>>,
    constant495: burn::module::Param<Tensor<1>>,
    linear24: Linear,
    constant496: burn::module::Param<Tensor<1>>,
    constant497: burn::module::Param<Tensor<1>>,
    constant44: burn::module::Param<Tensor<1>>,
    constant45: burn::module::Param<Tensor<1>>,
    linear25: Linear,
    linear26: Linear,
    linear27: Linear,
    constant510: burn::module::Param<Tensor<1>>,
    linear28: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule5 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant491: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant492: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant40: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant41: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear23 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant493: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant494: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant495: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear24 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant496: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant497: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant44: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant45: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear25 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear26 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear27 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant510: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear28 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant491,
            constant492,
            constant40,
            constant41,
            linear23,
            constant493,
            constant494,
            constant495,
            linear24,
            constant496,
            constant497,
            constant44,
            constant45,
            linear25,
            linear26,
            linear27,
            constant510,
            linear28,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add31_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean15_out1 = { add31_out1.clone().mean_dim(2usize) };
        let sub9_out1 = add31_out1.sub(reducemean15_out1);
        let constant491_out1 = self.constant491.val();
        let pow8_out1 = sub9_out1
            .clone()
            .powf((constant491_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean16_out1 = { pow8_out1.mean_dim(2usize) };
        let constant492_out1 = self.constant492.val();
        let add32_out1 = reducemean16_out1
            .add((constant492_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt8_out1 = add32_out1.sqrt();
        let div15_out1 = sub9_out1.div(sqrt8_out1);
        let constant40_out1 = self.constant40.val();
        let mul17_out1 = div15_out1
            .mul((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant41_out1 = self.constant41.val();
        let add33_out1 = mul17_out1
            .add((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear23_out1 = self.linear23.forward(add33_out1.clone());
        let constant493_out1 = self.constant493.val();
        let div16_out1 = linear23_out1
            .clone()
            .div((constant493_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf4_out1 = div16_out1.erf();
        let constant494_out1 = self.constant494.val();
        let add34_out1 = erf4_out1
            .add((constant494_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul18_out1 = linear23_out1.mul(add34_out1);
        let constant495_out1 = self.constant495.val();
        let mul19_out1 = mul18_out1
            .mul((constant495_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear24_out1 = self.linear24.forward(mul19_out1);
        let add35_out1 = linear24_out1.add(add33_out1);
        let reducemean17_out1 = { add35_out1.clone().mean_dim(2usize) };
        let sub10_out1 = add35_out1.sub(reducemean17_out1);
        let constant496_out1 = self.constant496.val();
        let pow9_out1 = sub10_out1
            .clone()
            .powf((constant496_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean18_out1 = { pow9_out1.mean_dim(2usize) };
        let constant497_out1 = self.constant497.val();
        let add36_out1 = reducemean18_out1
            .add((constant497_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt9_out1 = add36_out1.sqrt();
        let div17_out1 = sub10_out1.div(sqrt9_out1);
        let constant44_out1 = self.constant44.val();
        let mul20_out1 = div17_out1
            .mul((constant44_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant45_out1 = self.constant45.val();
        let add37_out1 = mul20_out1
            .add((constant45_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear25_out1 = self.linear25.forward(add37_out1.clone());
        let linear26_out1 = self.linear26.forward(add37_out1.clone());
        let shape36_out1: [i64; 3] = {
            let axes = &linear26_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather38_out1 = shape36_out1[0] as i64;
        let gather39_out1 = shape36_out1[1] as i64;
        let unsqueeze38_out1 = [gather38_out1 as i64];
        let unsqueeze39_out1 = [gather39_out1 as i64];
        let constant501_out1: [i64; 1] = [64i64];
        let constant500_out1: [i64; 1] = [16i64];
        let concat18_out1: [i64; 4usize] = [
            &unsqueeze38_out1[..],
            &unsqueeze39_out1[..],
            &constant500_out1[..],
            &constant501_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape18_out1 = linear26_out1.reshape(concat18_out1);
        let linear27_out1 = self.linear27.forward(add37_out1.clone());
        let shape38_out1: [i64; 3] = {
            let axes = &linear27_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather40_out1 = shape38_out1[0] as i64;
        let gather41_out1 = shape38_out1[1] as i64;
        let unsqueeze40_out1 = [gather40_out1 as i64];
        let unsqueeze41_out1 = [gather41_out1 as i64];
        let constant505_out1: [i64; 1] = [64i64];
        let constant504_out1: [i64; 1] = [16i64];
        let concat19_out1: [i64; 4usize] = [
            &unsqueeze40_out1[..],
            &unsqueeze41_out1[..],
            &constant504_out1[..],
            &constant505_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape19_out1 = linear27_out1.reshape(concat19_out1);
        let transpose17_out1 = reshape19_out1.permute([0, 2, 1, 3]);
        let shape40_out1: [i64; 3] = {
            let axes = &linear25_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather42_out1 = shape40_out1[0] as i64;
        let gather43_out1 = shape40_out1[1] as i64;
        let unsqueeze42_out1 = [gather42_out1 as i64];
        let unsqueeze43_out1 = [gather43_out1 as i64];
        let constant509_out1: [i64; 1] = [64i64];
        let constant508_out1: [i64; 1] = [16i64];
        let concat20_out1: [i64; 4usize] = [
            &unsqueeze42_out1[..],
            &unsqueeze43_out1[..],
            &constant508_out1[..],
            &constant509_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape20_out1 = linear25_out1.reshape(concat20_out1);
        let transpose18_out1 = reshape20_out1.permute([0, 2, 1, 3]);
        let transpose19_out1 = reshape18_out1.permute([0, 2, 3, 1]);
        let matmul36_out1 = transpose18_out1.matmul(transpose19_out1);
        let constant510_out1 = self.constant510.val();
        let div18_out1 = matmul36_out1
            .div((constant510_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add38_out1 = div18_out1.add(mul2_out1);
        let softmax5_out1 = burn::tensor::activation::softmax(add38_out1, 3);
        let matmul37_out1 = softmax5_out1.matmul(transpose17_out1);
        let transpose20_out1 = matmul37_out1.permute([0, 2, 1, 3]);
        let shape42_out1: [i64; 4] = {
            let axes = &transpose20_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather44_out1 = shape42_out1[0] as i64;
        let gather45_out1 = shape42_out1[1] as i64;
        let unsqueeze44_out1 = [gather44_out1 as i64];
        let unsqueeze45_out1 = [gather45_out1 as i64];
        let constant513_out1: [i64; 1] = [1024i64];
        let concat21_out1: [i64; 3usize] = [
            &unsqueeze44_out1[..],
            &unsqueeze45_out1[..],
            &constant513_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape21_out1 = transpose20_out1.reshape(concat21_out1);
        let linear28_out1 = self.linear28.forward(reshape21_out1);
        let add39_out1 = linear28_out1.add(add37_out1);
        add39_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule6 {
    constant514: burn::module::Param<Tensor<1>>,
    constant515: burn::module::Param<Tensor<1>>,
    constant50: burn::module::Param<Tensor<1>>,
    constant51: burn::module::Param<Tensor<1>>,
    linear29: Linear,
    constant516: burn::module::Param<Tensor<1>>,
    constant517: burn::module::Param<Tensor<1>>,
    constant518: burn::module::Param<Tensor<1>>,
    linear30: Linear,
    constant519: burn::module::Param<Tensor<1>>,
    constant520: burn::module::Param<Tensor<1>>,
    constant54: burn::module::Param<Tensor<1>>,
    constant55: burn::module::Param<Tensor<1>>,
    linear31: Linear,
    linear32: Linear,
    linear33: Linear,
    constant533: burn::module::Param<Tensor<1>>,
    linear34: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule6 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant514: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant515: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant50: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant51: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear29 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant516: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant517: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant518: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear30 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant519: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant520: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant54: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant55: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear31 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear32 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear33 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant533: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear34 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant514,
            constant515,
            constant50,
            constant51,
            linear29,
            constant516,
            constant517,
            constant518,
            linear30,
            constant519,
            constant520,
            constant54,
            constant55,
            linear31,
            linear32,
            linear33,
            constant533,
            linear34,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add39_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean19_out1 = { add39_out1.clone().mean_dim(2usize) };
        let sub11_out1 = add39_out1.sub(reducemean19_out1);
        let constant514_out1 = self.constant514.val();
        let pow10_out1 = sub11_out1
            .clone()
            .powf((constant514_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean20_out1 = { pow10_out1.mean_dim(2usize) };
        let constant515_out1 = self.constant515.val();
        let add40_out1 = reducemean20_out1
            .add((constant515_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt10_out1 = add40_out1.sqrt();
        let div19_out1 = sub11_out1.div(sqrt10_out1);
        let constant50_out1 = self.constant50.val();
        let mul21_out1 = div19_out1
            .mul((constant50_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant51_out1 = self.constant51.val();
        let add41_out1 = mul21_out1
            .add((constant51_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear29_out1 = self.linear29.forward(add41_out1.clone());
        let constant516_out1 = self.constant516.val();
        let div20_out1 = linear29_out1
            .clone()
            .div((constant516_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf5_out1 = div20_out1.erf();
        let constant517_out1 = self.constant517.val();
        let add42_out1 = erf5_out1
            .add((constant517_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul22_out1 = linear29_out1.mul(add42_out1);
        let constant518_out1 = self.constant518.val();
        let mul23_out1 = mul22_out1
            .mul((constant518_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear30_out1 = self.linear30.forward(mul23_out1);
        let add43_out1 = linear30_out1.add(add41_out1);
        let reducemean21_out1 = { add43_out1.clone().mean_dim(2usize) };
        let sub12_out1 = add43_out1.sub(reducemean21_out1);
        let constant519_out1 = self.constant519.val();
        let pow11_out1 = sub12_out1
            .clone()
            .powf((constant519_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean22_out1 = { pow11_out1.mean_dim(2usize) };
        let constant520_out1 = self.constant520.val();
        let add44_out1 = reducemean22_out1
            .add((constant520_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt11_out1 = add44_out1.sqrt();
        let div21_out1 = sub12_out1.div(sqrt11_out1);
        let constant54_out1 = self.constant54.val();
        let mul24_out1 = div21_out1
            .mul((constant54_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant55_out1 = self.constant55.val();
        let add45_out1 = mul24_out1
            .add((constant55_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear31_out1 = self.linear31.forward(add45_out1.clone());
        let linear32_out1 = self.linear32.forward(add45_out1.clone());
        let shape44_out1: [i64; 3] = {
            let axes = &linear32_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather46_out1 = shape44_out1[0] as i64;
        let gather47_out1 = shape44_out1[1] as i64;
        let unsqueeze46_out1 = [gather46_out1 as i64];
        let unsqueeze47_out1 = [gather47_out1 as i64];
        let constant524_out1: [i64; 1] = [64i64];
        let constant523_out1: [i64; 1] = [16i64];
        let concat22_out1: [i64; 4usize] = [
            &unsqueeze46_out1[..],
            &unsqueeze47_out1[..],
            &constant523_out1[..],
            &constant524_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape22_out1 = linear32_out1.reshape(concat22_out1);
        let linear33_out1 = self.linear33.forward(add45_out1.clone());
        let shape46_out1: [i64; 3] = {
            let axes = &linear33_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather48_out1 = shape46_out1[0] as i64;
        let gather49_out1 = shape46_out1[1] as i64;
        let unsqueeze48_out1 = [gather48_out1 as i64];
        let unsqueeze49_out1 = [gather49_out1 as i64];
        let constant528_out1: [i64; 1] = [64i64];
        let constant527_out1: [i64; 1] = [16i64];
        let concat23_out1: [i64; 4usize] = [
            &unsqueeze48_out1[..],
            &unsqueeze49_out1[..],
            &constant527_out1[..],
            &constant528_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape23_out1 = linear33_out1.reshape(concat23_out1);
        let transpose21_out1 = reshape23_out1.permute([0, 2, 1, 3]);
        let shape48_out1: [i64; 3] = {
            let axes = &linear31_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather50_out1 = shape48_out1[0] as i64;
        let gather51_out1 = shape48_out1[1] as i64;
        let unsqueeze50_out1 = [gather50_out1 as i64];
        let unsqueeze51_out1 = [gather51_out1 as i64];
        let constant532_out1: [i64; 1] = [64i64];
        let constant531_out1: [i64; 1] = [16i64];
        let concat24_out1: [i64; 4usize] = [
            &unsqueeze50_out1[..],
            &unsqueeze51_out1[..],
            &constant531_out1[..],
            &constant532_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape24_out1 = linear31_out1.reshape(concat24_out1);
        let transpose22_out1 = reshape24_out1.permute([0, 2, 1, 3]);
        let transpose23_out1 = reshape22_out1.permute([0, 2, 3, 1]);
        let matmul44_out1 = transpose22_out1.matmul(transpose23_out1);
        let constant533_out1 = self.constant533.val();
        let div22_out1 = matmul44_out1
            .div((constant533_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add46_out1 = div22_out1.add(mul2_out1);
        let softmax6_out1 = burn::tensor::activation::softmax(add46_out1, 3);
        let matmul45_out1 = softmax6_out1.matmul(transpose21_out1);
        let transpose24_out1 = matmul45_out1.permute([0, 2, 1, 3]);
        let shape50_out1: [i64; 4] = {
            let axes = &transpose24_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather52_out1 = shape50_out1[0] as i64;
        let gather53_out1 = shape50_out1[1] as i64;
        let unsqueeze52_out1 = [gather52_out1 as i64];
        let unsqueeze53_out1 = [gather53_out1 as i64];
        let constant536_out1: [i64; 1] = [1024i64];
        let concat25_out1: [i64; 3usize] = [
            &unsqueeze52_out1[..],
            &unsqueeze53_out1[..],
            &constant536_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape25_out1 = transpose24_out1.reshape(concat25_out1);
        let linear34_out1 = self.linear34.forward(reshape25_out1);
        let add47_out1 = linear34_out1.add(add45_out1);
        add47_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule7 {
    constant537: burn::module::Param<Tensor<1>>,
    constant538: burn::module::Param<Tensor<1>>,
    constant60: burn::module::Param<Tensor<1>>,
    constant61: burn::module::Param<Tensor<1>>,
    linear35: Linear,
    constant539: burn::module::Param<Tensor<1>>,
    constant540: burn::module::Param<Tensor<1>>,
    constant541: burn::module::Param<Tensor<1>>,
    linear36: Linear,
    constant542: burn::module::Param<Tensor<1>>,
    constant543: burn::module::Param<Tensor<1>>,
    constant64: burn::module::Param<Tensor<1>>,
    constant65: burn::module::Param<Tensor<1>>,
    linear37: Linear,
    linear38: Linear,
    linear39: Linear,
    constant556: burn::module::Param<Tensor<1>>,
    linear40: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule7 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant537: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant538: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant60: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant61: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear35 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant539: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant540: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant541: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear36 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant542: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant543: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant64: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant65: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear37 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear38 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear39 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant556: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear40 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant537,
            constant538,
            constant60,
            constant61,
            linear35,
            constant539,
            constant540,
            constant541,
            linear36,
            constant542,
            constant543,
            constant64,
            constant65,
            linear37,
            linear38,
            linear39,
            constant556,
            linear40,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add47_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean23_out1 = { add47_out1.clone().mean_dim(2usize) };
        let sub13_out1 = add47_out1.sub(reducemean23_out1);
        let constant537_out1 = self.constant537.val();
        let pow12_out1 = sub13_out1
            .clone()
            .powf((constant537_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean24_out1 = { pow12_out1.mean_dim(2usize) };
        let constant538_out1 = self.constant538.val();
        let add48_out1 = reducemean24_out1
            .add((constant538_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt12_out1 = add48_out1.sqrt();
        let div23_out1 = sub13_out1.div(sqrt12_out1);
        let constant60_out1 = self.constant60.val();
        let mul25_out1 = div23_out1
            .mul((constant60_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant61_out1 = self.constant61.val();
        let add49_out1 = mul25_out1
            .add((constant61_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear35_out1 = self.linear35.forward(add49_out1.clone());
        let constant539_out1 = self.constant539.val();
        let div24_out1 = linear35_out1
            .clone()
            .div((constant539_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf6_out1 = div24_out1.erf();
        let constant540_out1 = self.constant540.val();
        let add50_out1 = erf6_out1
            .add((constant540_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul26_out1 = linear35_out1.mul(add50_out1);
        let constant541_out1 = self.constant541.val();
        let mul27_out1 = mul26_out1
            .mul((constant541_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear36_out1 = self.linear36.forward(mul27_out1);
        let add51_out1 = linear36_out1.add(add49_out1);
        let reducemean25_out1 = { add51_out1.clone().mean_dim(2usize) };
        let sub14_out1 = add51_out1.sub(reducemean25_out1);
        let constant542_out1 = self.constant542.val();
        let pow13_out1 = sub14_out1
            .clone()
            .powf((constant542_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean26_out1 = { pow13_out1.mean_dim(2usize) };
        let constant543_out1 = self.constant543.val();
        let add52_out1 = reducemean26_out1
            .add((constant543_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt13_out1 = add52_out1.sqrt();
        let div25_out1 = sub14_out1.div(sqrt13_out1);
        let constant64_out1 = self.constant64.val();
        let mul28_out1 = div25_out1
            .mul((constant64_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant65_out1 = self.constant65.val();
        let add53_out1 = mul28_out1
            .add((constant65_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear37_out1 = self.linear37.forward(add53_out1.clone());
        let linear38_out1 = self.linear38.forward(add53_out1.clone());
        let shape52_out1: [i64; 3] = {
            let axes = &linear38_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather54_out1 = shape52_out1[0] as i64;
        let gather55_out1 = shape52_out1[1] as i64;
        let unsqueeze54_out1 = [gather54_out1 as i64];
        let unsqueeze55_out1 = [gather55_out1 as i64];
        let constant547_out1: [i64; 1] = [64i64];
        let constant546_out1: [i64; 1] = [16i64];
        let concat26_out1: [i64; 4usize] = [
            &unsqueeze54_out1[..],
            &unsqueeze55_out1[..],
            &constant546_out1[..],
            &constant547_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape26_out1 = linear38_out1.reshape(concat26_out1);
        let linear39_out1 = self.linear39.forward(add53_out1.clone());
        let shape54_out1: [i64; 3] = {
            let axes = &linear39_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather56_out1 = shape54_out1[0] as i64;
        let gather57_out1 = shape54_out1[1] as i64;
        let unsqueeze56_out1 = [gather56_out1 as i64];
        let unsqueeze57_out1 = [gather57_out1 as i64];
        let constant551_out1: [i64; 1] = [64i64];
        let constant550_out1: [i64; 1] = [16i64];
        let concat27_out1: [i64; 4usize] = [
            &unsqueeze56_out1[..],
            &unsqueeze57_out1[..],
            &constant550_out1[..],
            &constant551_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape27_out1 = linear39_out1.reshape(concat27_out1);
        let transpose25_out1 = reshape27_out1.permute([0, 2, 1, 3]);
        let shape56_out1: [i64; 3] = {
            let axes = &linear37_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather58_out1 = shape56_out1[0] as i64;
        let gather59_out1 = shape56_out1[1] as i64;
        let unsqueeze58_out1 = [gather58_out1 as i64];
        let unsqueeze59_out1 = [gather59_out1 as i64];
        let constant555_out1: [i64; 1] = [64i64];
        let constant554_out1: [i64; 1] = [16i64];
        let concat28_out1: [i64; 4usize] = [
            &unsqueeze58_out1[..],
            &unsqueeze59_out1[..],
            &constant554_out1[..],
            &constant555_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape28_out1 = linear37_out1.reshape(concat28_out1);
        let transpose26_out1 = reshape28_out1.permute([0, 2, 1, 3]);
        let transpose27_out1 = reshape26_out1.permute([0, 2, 3, 1]);
        let matmul52_out1 = transpose26_out1.matmul(transpose27_out1);
        let constant556_out1 = self.constant556.val();
        let div26_out1 = matmul52_out1
            .div((constant556_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add54_out1 = div26_out1.add(mul2_out1);
        let softmax7_out1 = burn::tensor::activation::softmax(add54_out1, 3);
        let matmul53_out1 = softmax7_out1.matmul(transpose25_out1);
        let transpose28_out1 = matmul53_out1.permute([0, 2, 1, 3]);
        let shape58_out1: [i64; 4] = {
            let axes = &transpose28_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather60_out1 = shape58_out1[0] as i64;
        let gather61_out1 = shape58_out1[1] as i64;
        let unsqueeze60_out1 = [gather60_out1 as i64];
        let unsqueeze61_out1 = [gather61_out1 as i64];
        let constant559_out1: [i64; 1] = [1024i64];
        let concat29_out1: [i64; 3usize] = [
            &unsqueeze60_out1[..],
            &unsqueeze61_out1[..],
            &constant559_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape29_out1 = transpose28_out1.reshape(concat29_out1);
        let linear40_out1 = self.linear40.forward(reshape29_out1);
        let add55_out1 = linear40_out1.add(add53_out1);
        add55_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule8 {
    constant560: burn::module::Param<Tensor<1>>,
    constant561: burn::module::Param<Tensor<1>>,
    constant70: burn::module::Param<Tensor<1>>,
    constant71: burn::module::Param<Tensor<1>>,
    linear41: Linear,
    constant562: burn::module::Param<Tensor<1>>,
    constant563: burn::module::Param<Tensor<1>>,
    constant564: burn::module::Param<Tensor<1>>,
    linear42: Linear,
    constant565: burn::module::Param<Tensor<1>>,
    constant566: burn::module::Param<Tensor<1>>,
    constant74: burn::module::Param<Tensor<1>>,
    constant75: burn::module::Param<Tensor<1>>,
    linear43: Linear,
    linear44: Linear,
    linear45: Linear,
    constant579: burn::module::Param<Tensor<1>>,
    linear46: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule8 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant560: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant561: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant70: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant71: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear41 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant562: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant563: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant564: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear42 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant565: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant566: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant74: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant75: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear43 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear44 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear45 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant579: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear46 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant560,
            constant561,
            constant70,
            constant71,
            linear41,
            constant562,
            constant563,
            constant564,
            linear42,
            constant565,
            constant566,
            constant74,
            constant75,
            linear43,
            linear44,
            linear45,
            constant579,
            linear46,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add55_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean27_out1 = { add55_out1.clone().mean_dim(2usize) };
        let sub15_out1 = add55_out1.sub(reducemean27_out1);
        let constant560_out1 = self.constant560.val();
        let pow14_out1 = sub15_out1
            .clone()
            .powf((constant560_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean28_out1 = { pow14_out1.mean_dim(2usize) };
        let constant561_out1 = self.constant561.val();
        let add56_out1 = reducemean28_out1
            .add((constant561_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt14_out1 = add56_out1.sqrt();
        let div27_out1 = sub15_out1.div(sqrt14_out1);
        let constant70_out1 = self.constant70.val();
        let mul29_out1 = div27_out1
            .mul((constant70_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant71_out1 = self.constant71.val();
        let add57_out1 = mul29_out1
            .add((constant71_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear41_out1 = self.linear41.forward(add57_out1.clone());
        let constant562_out1 = self.constant562.val();
        let div28_out1 = linear41_out1
            .clone()
            .div((constant562_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf7_out1 = div28_out1.erf();
        let constant563_out1 = self.constant563.val();
        let add58_out1 = erf7_out1
            .add((constant563_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul30_out1 = linear41_out1.mul(add58_out1);
        let constant564_out1 = self.constant564.val();
        let mul31_out1 = mul30_out1
            .mul((constant564_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear42_out1 = self.linear42.forward(mul31_out1);
        let add59_out1 = linear42_out1.add(add57_out1);
        let reducemean29_out1 = { add59_out1.clone().mean_dim(2usize) };
        let sub16_out1 = add59_out1.sub(reducemean29_out1);
        let constant565_out1 = self.constant565.val();
        let pow15_out1 = sub16_out1
            .clone()
            .powf((constant565_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean30_out1 = { pow15_out1.mean_dim(2usize) };
        let constant566_out1 = self.constant566.val();
        let add60_out1 = reducemean30_out1
            .add((constant566_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt15_out1 = add60_out1.sqrt();
        let div29_out1 = sub16_out1.div(sqrt15_out1);
        let constant74_out1 = self.constant74.val();
        let mul32_out1 = div29_out1
            .mul((constant74_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant75_out1 = self.constant75.val();
        let add61_out1 = mul32_out1
            .add((constant75_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear43_out1 = self.linear43.forward(add61_out1.clone());
        let linear44_out1 = self.linear44.forward(add61_out1.clone());
        let shape60_out1: [i64; 3] = {
            let axes = &linear44_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather62_out1 = shape60_out1[0] as i64;
        let gather63_out1 = shape60_out1[1] as i64;
        let unsqueeze62_out1 = [gather62_out1 as i64];
        let unsqueeze63_out1 = [gather63_out1 as i64];
        let constant570_out1: [i64; 1] = [64i64];
        let constant569_out1: [i64; 1] = [16i64];
        let concat30_out1: [i64; 4usize] = [
            &unsqueeze62_out1[..],
            &unsqueeze63_out1[..],
            &constant569_out1[..],
            &constant570_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape30_out1 = linear44_out1.reshape(concat30_out1);
        let linear45_out1 = self.linear45.forward(add61_out1.clone());
        let shape62_out1: [i64; 3] = {
            let axes = &linear45_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather64_out1 = shape62_out1[0] as i64;
        let gather65_out1 = shape62_out1[1] as i64;
        let unsqueeze64_out1 = [gather64_out1 as i64];
        let unsqueeze65_out1 = [gather65_out1 as i64];
        let constant574_out1: [i64; 1] = [64i64];
        let constant573_out1: [i64; 1] = [16i64];
        let concat31_out1: [i64; 4usize] = [
            &unsqueeze64_out1[..],
            &unsqueeze65_out1[..],
            &constant573_out1[..],
            &constant574_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape31_out1 = linear45_out1.reshape(concat31_out1);
        let transpose29_out1 = reshape31_out1.permute([0, 2, 1, 3]);
        let shape64_out1: [i64; 3] = {
            let axes = &linear43_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather66_out1 = shape64_out1[0] as i64;
        let gather67_out1 = shape64_out1[1] as i64;
        let unsqueeze66_out1 = [gather66_out1 as i64];
        let unsqueeze67_out1 = [gather67_out1 as i64];
        let constant578_out1: [i64; 1] = [64i64];
        let constant577_out1: [i64; 1] = [16i64];
        let concat32_out1: [i64; 4usize] = [
            &unsqueeze66_out1[..],
            &unsqueeze67_out1[..],
            &constant577_out1[..],
            &constant578_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape32_out1 = linear43_out1.reshape(concat32_out1);
        let transpose30_out1 = reshape32_out1.permute([0, 2, 1, 3]);
        let transpose31_out1 = reshape30_out1.permute([0, 2, 3, 1]);
        let matmul60_out1 = transpose30_out1.matmul(transpose31_out1);
        let constant579_out1 = self.constant579.val();
        let div30_out1 = matmul60_out1
            .div((constant579_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add62_out1 = div30_out1.add(mul2_out1);
        let softmax8_out1 = burn::tensor::activation::softmax(add62_out1, 3);
        let matmul61_out1 = softmax8_out1.matmul(transpose29_out1);
        let transpose32_out1 = matmul61_out1.permute([0, 2, 1, 3]);
        let shape66_out1: [i64; 4] = {
            let axes = &transpose32_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather68_out1 = shape66_out1[0] as i64;
        let gather69_out1 = shape66_out1[1] as i64;
        let unsqueeze68_out1 = [gather68_out1 as i64];
        let unsqueeze69_out1 = [gather69_out1 as i64];
        let constant582_out1: [i64; 1] = [1024i64];
        let concat33_out1: [i64; 3usize] = [
            &unsqueeze68_out1[..],
            &unsqueeze69_out1[..],
            &constant582_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape33_out1 = transpose32_out1.reshape(concat33_out1);
        let linear46_out1 = self.linear46.forward(reshape33_out1);
        let add63_out1 = linear46_out1.add(add61_out1);
        add63_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule9 {
    constant583: burn::module::Param<Tensor<1>>,
    constant584: burn::module::Param<Tensor<1>>,
    constant80: burn::module::Param<Tensor<1>>,
    constant81: burn::module::Param<Tensor<1>>,
    linear47: Linear,
    constant585: burn::module::Param<Tensor<1>>,
    constant586: burn::module::Param<Tensor<1>>,
    constant587: burn::module::Param<Tensor<1>>,
    linear48: Linear,
    constant588: burn::module::Param<Tensor<1>>,
    constant589: burn::module::Param<Tensor<1>>,
    constant84: burn::module::Param<Tensor<1>>,
    constant85: burn::module::Param<Tensor<1>>,
    linear49: Linear,
    linear50: Linear,
    linear51: Linear,
    constant602: burn::module::Param<Tensor<1>>,
    linear52: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule9 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant583: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant584: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant80: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant81: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear47 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant585: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant586: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant587: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear48 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant588: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant589: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant84: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant85: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear49 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear50 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear51 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant602: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear52 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant583,
            constant584,
            constant80,
            constant81,
            linear47,
            constant585,
            constant586,
            constant587,
            linear48,
            constant588,
            constant589,
            constant84,
            constant85,
            linear49,
            linear50,
            linear51,
            constant602,
            linear52,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add63_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean31_out1 = { add63_out1.clone().mean_dim(2usize) };
        let sub17_out1 = add63_out1.sub(reducemean31_out1);
        let constant583_out1 = self.constant583.val();
        let pow16_out1 = sub17_out1
            .clone()
            .powf((constant583_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean32_out1 = { pow16_out1.mean_dim(2usize) };
        let constant584_out1 = self.constant584.val();
        let add64_out1 = reducemean32_out1
            .add((constant584_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt16_out1 = add64_out1.sqrt();
        let div31_out1 = sub17_out1.div(sqrt16_out1);
        let constant80_out1 = self.constant80.val();
        let mul33_out1 = div31_out1
            .mul((constant80_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant81_out1 = self.constant81.val();
        let add65_out1 = mul33_out1
            .add((constant81_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear47_out1 = self.linear47.forward(add65_out1.clone());
        let constant585_out1 = self.constant585.val();
        let div32_out1 = linear47_out1
            .clone()
            .div((constant585_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf8_out1 = div32_out1.erf();
        let constant586_out1 = self.constant586.val();
        let add66_out1 = erf8_out1
            .add((constant586_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul34_out1 = linear47_out1.mul(add66_out1);
        let constant587_out1 = self.constant587.val();
        let mul35_out1 = mul34_out1
            .mul((constant587_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear48_out1 = self.linear48.forward(mul35_out1);
        let add67_out1 = linear48_out1.add(add65_out1);
        let reducemean33_out1 = { add67_out1.clone().mean_dim(2usize) };
        let sub18_out1 = add67_out1.sub(reducemean33_out1);
        let constant588_out1 = self.constant588.val();
        let pow17_out1 = sub18_out1
            .clone()
            .powf((constant588_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean34_out1 = { pow17_out1.mean_dim(2usize) };
        let constant589_out1 = self.constant589.val();
        let add68_out1 = reducemean34_out1
            .add((constant589_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt17_out1 = add68_out1.sqrt();
        let div33_out1 = sub18_out1.div(sqrt17_out1);
        let constant84_out1 = self.constant84.val();
        let mul36_out1 = div33_out1
            .mul((constant84_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant85_out1 = self.constant85.val();
        let add69_out1 = mul36_out1
            .add((constant85_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear49_out1 = self.linear49.forward(add69_out1.clone());
        let linear50_out1 = self.linear50.forward(add69_out1.clone());
        let shape68_out1: [i64; 3] = {
            let axes = &linear50_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather70_out1 = shape68_out1[0] as i64;
        let gather71_out1 = shape68_out1[1] as i64;
        let unsqueeze70_out1 = [gather70_out1 as i64];
        let unsqueeze71_out1 = [gather71_out1 as i64];
        let constant593_out1: [i64; 1] = [64i64];
        let constant592_out1: [i64; 1] = [16i64];
        let concat34_out1: [i64; 4usize] = [
            &unsqueeze70_out1[..],
            &unsqueeze71_out1[..],
            &constant592_out1[..],
            &constant593_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape34_out1 = linear50_out1.reshape(concat34_out1);
        let linear51_out1 = self.linear51.forward(add69_out1.clone());
        let shape70_out1: [i64; 3] = {
            let axes = &linear51_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather72_out1 = shape70_out1[0] as i64;
        let gather73_out1 = shape70_out1[1] as i64;
        let unsqueeze72_out1 = [gather72_out1 as i64];
        let unsqueeze73_out1 = [gather73_out1 as i64];
        let constant597_out1: [i64; 1] = [64i64];
        let constant596_out1: [i64; 1] = [16i64];
        let concat35_out1: [i64; 4usize] = [
            &unsqueeze72_out1[..],
            &unsqueeze73_out1[..],
            &constant596_out1[..],
            &constant597_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape35_out1 = linear51_out1.reshape(concat35_out1);
        let transpose33_out1 = reshape35_out1.permute([0, 2, 1, 3]);
        let shape72_out1: [i64; 3] = {
            let axes = &linear49_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather74_out1 = shape72_out1[0] as i64;
        let gather75_out1 = shape72_out1[1] as i64;
        let unsqueeze74_out1 = [gather74_out1 as i64];
        let unsqueeze75_out1 = [gather75_out1 as i64];
        let constant601_out1: [i64; 1] = [64i64];
        let constant600_out1: [i64; 1] = [16i64];
        let concat36_out1: [i64; 4usize] = [
            &unsqueeze74_out1[..],
            &unsqueeze75_out1[..],
            &constant600_out1[..],
            &constant601_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape36_out1 = linear49_out1.reshape(concat36_out1);
        let transpose34_out1 = reshape36_out1.permute([0, 2, 1, 3]);
        let transpose35_out1 = reshape34_out1.permute([0, 2, 3, 1]);
        let matmul68_out1 = transpose34_out1.matmul(transpose35_out1);
        let constant602_out1 = self.constant602.val();
        let div34_out1 = matmul68_out1
            .div((constant602_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add70_out1 = div34_out1.add(mul2_out1);
        let softmax9_out1 = burn::tensor::activation::softmax(add70_out1, 3);
        let matmul69_out1 = softmax9_out1.matmul(transpose33_out1);
        let transpose36_out1 = matmul69_out1.permute([0, 2, 1, 3]);
        let shape74_out1: [i64; 4] = {
            let axes = &transpose36_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather76_out1 = shape74_out1[0] as i64;
        let gather77_out1 = shape74_out1[1] as i64;
        let unsqueeze76_out1 = [gather76_out1 as i64];
        let unsqueeze77_out1 = [gather77_out1 as i64];
        let constant605_out1: [i64; 1] = [1024i64];
        let concat37_out1: [i64; 3usize] = [
            &unsqueeze76_out1[..],
            &unsqueeze77_out1[..],
            &constant605_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape37_out1 = transpose36_out1.reshape(concat37_out1);
        let linear52_out1 = self.linear52.forward(reshape37_out1);
        let add71_out1 = linear52_out1.add(add69_out1);
        add71_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule10 {
    constant606: burn::module::Param<Tensor<1>>,
    constant607: burn::module::Param<Tensor<1>>,
    constant90: burn::module::Param<Tensor<1>>,
    constant91: burn::module::Param<Tensor<1>>,
    linear53: Linear,
    constant608: burn::module::Param<Tensor<1>>,
    constant609: burn::module::Param<Tensor<1>>,
    constant610: burn::module::Param<Tensor<1>>,
    linear54: Linear,
    constant611: burn::module::Param<Tensor<1>>,
    constant612: burn::module::Param<Tensor<1>>,
    constant94: burn::module::Param<Tensor<1>>,
    constant95: burn::module::Param<Tensor<1>>,
    linear55: Linear,
    linear56: Linear,
    linear57: Linear,
    constant625: burn::module::Param<Tensor<1>>,
    linear58: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule10 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant606: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant607: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant90: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant91: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear53 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant608: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant609: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant610: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear54 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant611: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant612: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant94: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant95: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear55 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear56 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear57 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant625: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear58 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant606,
            constant607,
            constant90,
            constant91,
            linear53,
            constant608,
            constant609,
            constant610,
            linear54,
            constant611,
            constant612,
            constant94,
            constant95,
            linear55,
            linear56,
            linear57,
            constant625,
            linear58,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add71_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean35_out1 = { add71_out1.clone().mean_dim(2usize) };
        let sub19_out1 = add71_out1.sub(reducemean35_out1);
        let constant606_out1 = self.constant606.val();
        let pow18_out1 = sub19_out1
            .clone()
            .powf((constant606_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean36_out1 = { pow18_out1.mean_dim(2usize) };
        let constant607_out1 = self.constant607.val();
        let add72_out1 = reducemean36_out1
            .add((constant607_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt18_out1 = add72_out1.sqrt();
        let div35_out1 = sub19_out1.div(sqrt18_out1);
        let constant90_out1 = self.constant90.val();
        let mul37_out1 = div35_out1
            .mul((constant90_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant91_out1 = self.constant91.val();
        let add73_out1 = mul37_out1
            .add((constant91_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear53_out1 = self.linear53.forward(add73_out1.clone());
        let constant608_out1 = self.constant608.val();
        let div36_out1 = linear53_out1
            .clone()
            .div((constant608_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf9_out1 = div36_out1.erf();
        let constant609_out1 = self.constant609.val();
        let add74_out1 = erf9_out1
            .add((constant609_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul38_out1 = linear53_out1.mul(add74_out1);
        let constant610_out1 = self.constant610.val();
        let mul39_out1 = mul38_out1
            .mul((constant610_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear54_out1 = self.linear54.forward(mul39_out1);
        let add75_out1 = linear54_out1.add(add73_out1);
        let reducemean37_out1 = { add75_out1.clone().mean_dim(2usize) };
        let sub20_out1 = add75_out1.sub(reducemean37_out1);
        let constant611_out1 = self.constant611.val();
        let pow19_out1 = sub20_out1
            .clone()
            .powf((constant611_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean38_out1 = { pow19_out1.mean_dim(2usize) };
        let constant612_out1 = self.constant612.val();
        let add76_out1 = reducemean38_out1
            .add((constant612_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt19_out1 = add76_out1.sqrt();
        let div37_out1 = sub20_out1.div(sqrt19_out1);
        let constant94_out1 = self.constant94.val();
        let mul40_out1 = div37_out1
            .mul((constant94_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant95_out1 = self.constant95.val();
        let add77_out1 = mul40_out1
            .add((constant95_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear55_out1 = self.linear55.forward(add77_out1.clone());
        let linear56_out1 = self.linear56.forward(add77_out1.clone());
        let shape76_out1: [i64; 3] = {
            let axes = &linear56_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather78_out1 = shape76_out1[0] as i64;
        let gather79_out1 = shape76_out1[1] as i64;
        let unsqueeze78_out1 = [gather78_out1 as i64];
        let unsqueeze79_out1 = [gather79_out1 as i64];
        let constant616_out1: [i64; 1] = [64i64];
        let constant615_out1: [i64; 1] = [16i64];
        let concat38_out1: [i64; 4usize] = [
            &unsqueeze78_out1[..],
            &unsqueeze79_out1[..],
            &constant615_out1[..],
            &constant616_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape38_out1 = linear56_out1.reshape(concat38_out1);
        let linear57_out1 = self.linear57.forward(add77_out1.clone());
        let shape78_out1: [i64; 3] = {
            let axes = &linear57_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather80_out1 = shape78_out1[0] as i64;
        let gather81_out1 = shape78_out1[1] as i64;
        let unsqueeze80_out1 = [gather80_out1 as i64];
        let unsqueeze81_out1 = [gather81_out1 as i64];
        let constant620_out1: [i64; 1] = [64i64];
        let constant619_out1: [i64; 1] = [16i64];
        let concat39_out1: [i64; 4usize] = [
            &unsqueeze80_out1[..],
            &unsqueeze81_out1[..],
            &constant619_out1[..],
            &constant620_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape39_out1 = linear57_out1.reshape(concat39_out1);
        let transpose37_out1 = reshape39_out1.permute([0, 2, 1, 3]);
        let shape80_out1: [i64; 3] = {
            let axes = &linear55_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather82_out1 = shape80_out1[0] as i64;
        let gather83_out1 = shape80_out1[1] as i64;
        let unsqueeze82_out1 = [gather82_out1 as i64];
        let unsqueeze83_out1 = [gather83_out1 as i64];
        let constant624_out1: [i64; 1] = [64i64];
        let constant623_out1: [i64; 1] = [16i64];
        let concat40_out1: [i64; 4usize] = [
            &unsqueeze82_out1[..],
            &unsqueeze83_out1[..],
            &constant623_out1[..],
            &constant624_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape40_out1 = linear55_out1.reshape(concat40_out1);
        let transpose38_out1 = reshape40_out1.permute([0, 2, 1, 3]);
        let transpose39_out1 = reshape38_out1.permute([0, 2, 3, 1]);
        let matmul76_out1 = transpose38_out1.matmul(transpose39_out1);
        let constant625_out1 = self.constant625.val();
        let div38_out1 = matmul76_out1
            .div((constant625_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add78_out1 = div38_out1.add(mul2_out1);
        let softmax10_out1 = burn::tensor::activation::softmax(add78_out1, 3);
        let matmul77_out1 = softmax10_out1.matmul(transpose37_out1);
        let transpose40_out1 = matmul77_out1.permute([0, 2, 1, 3]);
        let shape82_out1: [i64; 4] = {
            let axes = &transpose40_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather84_out1 = shape82_out1[0] as i64;
        let gather85_out1 = shape82_out1[1] as i64;
        let unsqueeze84_out1 = [gather84_out1 as i64];
        let unsqueeze85_out1 = [gather85_out1 as i64];
        let constant628_out1: [i64; 1] = [1024i64];
        let concat41_out1: [i64; 3usize] = [
            &unsqueeze84_out1[..],
            &unsqueeze85_out1[..],
            &constant628_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape41_out1 = transpose40_out1.reshape(concat41_out1);
        let linear58_out1 = self.linear58.forward(reshape41_out1);
        let add79_out1 = linear58_out1.add(add77_out1);
        add79_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule11 {
    constant629: burn::module::Param<Tensor<1>>,
    constant630: burn::module::Param<Tensor<1>>,
    constant100: burn::module::Param<Tensor<1>>,
    constant101: burn::module::Param<Tensor<1>>,
    linear59: Linear,
    constant631: burn::module::Param<Tensor<1>>,
    constant632: burn::module::Param<Tensor<1>>,
    constant633: burn::module::Param<Tensor<1>>,
    linear60: Linear,
    constant634: burn::module::Param<Tensor<1>>,
    constant635: burn::module::Param<Tensor<1>>,
    constant104: burn::module::Param<Tensor<1>>,
    constant105: burn::module::Param<Tensor<1>>,
    linear61: Linear,
    linear62: Linear,
    linear63: Linear,
    constant648: burn::module::Param<Tensor<1>>,
    linear64: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule11 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant629: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant630: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant100: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant101: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear59 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant631: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant632: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant633: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear60 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant634: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant635: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant104: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant105: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear61 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear62 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear63 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant648: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear64 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant629,
            constant630,
            constant100,
            constant101,
            linear59,
            constant631,
            constant632,
            constant633,
            linear60,
            constant634,
            constant635,
            constant104,
            constant105,
            linear61,
            linear62,
            linear63,
            constant648,
            linear64,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add79_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean39_out1 = { add79_out1.clone().mean_dim(2usize) };
        let sub21_out1 = add79_out1.sub(reducemean39_out1);
        let constant629_out1 = self.constant629.val();
        let pow20_out1 = sub21_out1
            .clone()
            .powf((constant629_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean40_out1 = { pow20_out1.mean_dim(2usize) };
        let constant630_out1 = self.constant630.val();
        let add80_out1 = reducemean40_out1
            .add((constant630_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt20_out1 = add80_out1.sqrt();
        let div39_out1 = sub21_out1.div(sqrt20_out1);
        let constant100_out1 = self.constant100.val();
        let mul41_out1 = div39_out1
            .mul((constant100_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant101_out1 = self.constant101.val();
        let add81_out1 = mul41_out1
            .add((constant101_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear59_out1 = self.linear59.forward(add81_out1.clone());
        let constant631_out1 = self.constant631.val();
        let div40_out1 = linear59_out1
            .clone()
            .div((constant631_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf10_out1 = div40_out1.erf();
        let constant632_out1 = self.constant632.val();
        let add82_out1 = erf10_out1
            .add((constant632_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul42_out1 = linear59_out1.mul(add82_out1);
        let constant633_out1 = self.constant633.val();
        let mul43_out1 = mul42_out1
            .mul((constant633_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear60_out1 = self.linear60.forward(mul43_out1);
        let add83_out1 = linear60_out1.add(add81_out1);
        let reducemean41_out1 = { add83_out1.clone().mean_dim(2usize) };
        let sub22_out1 = add83_out1.sub(reducemean41_out1);
        let constant634_out1 = self.constant634.val();
        let pow21_out1 = sub22_out1
            .clone()
            .powf((constant634_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean42_out1 = { pow21_out1.mean_dim(2usize) };
        let constant635_out1 = self.constant635.val();
        let add84_out1 = reducemean42_out1
            .add((constant635_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt21_out1 = add84_out1.sqrt();
        let div41_out1 = sub22_out1.div(sqrt21_out1);
        let constant104_out1 = self.constant104.val();
        let mul44_out1 = div41_out1
            .mul((constant104_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant105_out1 = self.constant105.val();
        let add85_out1 = mul44_out1
            .add((constant105_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear61_out1 = self.linear61.forward(add85_out1.clone());
        let linear62_out1 = self.linear62.forward(add85_out1.clone());
        let shape84_out1: [i64; 3] = {
            let axes = &linear62_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather86_out1 = shape84_out1[0] as i64;
        let gather87_out1 = shape84_out1[1] as i64;
        let unsqueeze86_out1 = [gather86_out1 as i64];
        let unsqueeze87_out1 = [gather87_out1 as i64];
        let constant639_out1: [i64; 1] = [64i64];
        let constant638_out1: [i64; 1] = [16i64];
        let concat42_out1: [i64; 4usize] = [
            &unsqueeze86_out1[..],
            &unsqueeze87_out1[..],
            &constant638_out1[..],
            &constant639_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape42_out1 = linear62_out1.reshape(concat42_out1);
        let linear63_out1 = self.linear63.forward(add85_out1.clone());
        let shape86_out1: [i64; 3] = {
            let axes = &linear63_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather88_out1 = shape86_out1[0] as i64;
        let gather89_out1 = shape86_out1[1] as i64;
        let unsqueeze88_out1 = [gather88_out1 as i64];
        let unsqueeze89_out1 = [gather89_out1 as i64];
        let constant643_out1: [i64; 1] = [64i64];
        let constant642_out1: [i64; 1] = [16i64];
        let concat43_out1: [i64; 4usize] = [
            &unsqueeze88_out1[..],
            &unsqueeze89_out1[..],
            &constant642_out1[..],
            &constant643_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape43_out1 = linear63_out1.reshape(concat43_out1);
        let transpose41_out1 = reshape43_out1.permute([0, 2, 1, 3]);
        let shape88_out1: [i64; 3] = {
            let axes = &linear61_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather90_out1 = shape88_out1[0] as i64;
        let gather91_out1 = shape88_out1[1] as i64;
        let unsqueeze90_out1 = [gather90_out1 as i64];
        let unsqueeze91_out1 = [gather91_out1 as i64];
        let constant647_out1: [i64; 1] = [64i64];
        let constant646_out1: [i64; 1] = [16i64];
        let concat44_out1: [i64; 4usize] = [
            &unsqueeze90_out1[..],
            &unsqueeze91_out1[..],
            &constant646_out1[..],
            &constant647_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape44_out1 = linear61_out1.reshape(concat44_out1);
        let transpose42_out1 = reshape44_out1.permute([0, 2, 1, 3]);
        let transpose43_out1 = reshape42_out1.permute([0, 2, 3, 1]);
        let matmul84_out1 = transpose42_out1.matmul(transpose43_out1);
        let constant648_out1 = self.constant648.val();
        let div42_out1 = matmul84_out1
            .div((constant648_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add86_out1 = div42_out1.add(mul2_out1);
        let softmax11_out1 = burn::tensor::activation::softmax(add86_out1, 3);
        let matmul85_out1 = softmax11_out1.matmul(transpose41_out1);
        let transpose44_out1 = matmul85_out1.permute([0, 2, 1, 3]);
        let shape90_out1: [i64; 4] = {
            let axes = &transpose44_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather92_out1 = shape90_out1[0] as i64;
        let gather93_out1 = shape90_out1[1] as i64;
        let unsqueeze92_out1 = [gather92_out1 as i64];
        let unsqueeze93_out1 = [gather93_out1 as i64];
        let constant651_out1: [i64; 1] = [1024i64];
        let concat45_out1: [i64; 3usize] = [
            &unsqueeze92_out1[..],
            &unsqueeze93_out1[..],
            &constant651_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape45_out1 = transpose44_out1.reshape(concat45_out1);
        let linear64_out1 = self.linear64.forward(reshape45_out1);
        let add87_out1 = linear64_out1.add(add85_out1);
        add87_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule12 {
    constant652: burn::module::Param<Tensor<1>>,
    constant653: burn::module::Param<Tensor<1>>,
    constant110: burn::module::Param<Tensor<1>>,
    constant111: burn::module::Param<Tensor<1>>,
    linear65: Linear,
    constant654: burn::module::Param<Tensor<1>>,
    constant655: burn::module::Param<Tensor<1>>,
    constant656: burn::module::Param<Tensor<1>>,
    linear66: Linear,
    constant657: burn::module::Param<Tensor<1>>,
    constant658: burn::module::Param<Tensor<1>>,
    constant114: burn::module::Param<Tensor<1>>,
    constant115: burn::module::Param<Tensor<1>>,
    linear67: Linear,
    linear68: Linear,
    linear69: Linear,
    constant671: burn::module::Param<Tensor<1>>,
    linear70: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule12 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant652: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant653: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant110: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant111: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear65 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant654: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant655: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant656: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear66 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant657: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant658: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant114: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant115: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear67 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear68 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear69 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant671: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear70 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant652,
            constant653,
            constant110,
            constant111,
            linear65,
            constant654,
            constant655,
            constant656,
            linear66,
            constant657,
            constant658,
            constant114,
            constant115,
            linear67,
            linear68,
            linear69,
            constant671,
            linear70,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add87_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean43_out1 = { add87_out1.clone().mean_dim(2usize) };
        let sub23_out1 = add87_out1.sub(reducemean43_out1);
        let constant652_out1 = self.constant652.val();
        let pow22_out1 = sub23_out1
            .clone()
            .powf((constant652_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean44_out1 = { pow22_out1.mean_dim(2usize) };
        let constant653_out1 = self.constant653.val();
        let add88_out1 = reducemean44_out1
            .add((constant653_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt22_out1 = add88_out1.sqrt();
        let div43_out1 = sub23_out1.div(sqrt22_out1);
        let constant110_out1 = self.constant110.val();
        let mul45_out1 = div43_out1
            .mul((constant110_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant111_out1 = self.constant111.val();
        let add89_out1 = mul45_out1
            .add((constant111_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear65_out1 = self.linear65.forward(add89_out1.clone());
        let constant654_out1 = self.constant654.val();
        let div44_out1 = linear65_out1
            .clone()
            .div((constant654_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf11_out1 = div44_out1.erf();
        let constant655_out1 = self.constant655.val();
        let add90_out1 = erf11_out1
            .add((constant655_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul46_out1 = linear65_out1.mul(add90_out1);
        let constant656_out1 = self.constant656.val();
        let mul47_out1 = mul46_out1
            .mul((constant656_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear66_out1 = self.linear66.forward(mul47_out1);
        let add91_out1 = linear66_out1.add(add89_out1);
        let reducemean45_out1 = { add91_out1.clone().mean_dim(2usize) };
        let sub24_out1 = add91_out1.sub(reducemean45_out1);
        let constant657_out1 = self.constant657.val();
        let pow23_out1 = sub24_out1
            .clone()
            .powf((constant657_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean46_out1 = { pow23_out1.mean_dim(2usize) };
        let constant658_out1 = self.constant658.val();
        let add92_out1 = reducemean46_out1
            .add((constant658_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt23_out1 = add92_out1.sqrt();
        let div45_out1 = sub24_out1.div(sqrt23_out1);
        let constant114_out1 = self.constant114.val();
        let mul48_out1 = div45_out1
            .mul((constant114_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant115_out1 = self.constant115.val();
        let add93_out1 = mul48_out1
            .add((constant115_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear67_out1 = self.linear67.forward(add93_out1.clone());
        let linear68_out1 = self.linear68.forward(add93_out1.clone());
        let shape92_out1: [i64; 3] = {
            let axes = &linear68_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather94_out1 = shape92_out1[0] as i64;
        let gather95_out1 = shape92_out1[1] as i64;
        let unsqueeze94_out1 = [gather94_out1 as i64];
        let unsqueeze95_out1 = [gather95_out1 as i64];
        let constant662_out1: [i64; 1] = [64i64];
        let constant661_out1: [i64; 1] = [16i64];
        let concat46_out1: [i64; 4usize] = [
            &unsqueeze94_out1[..],
            &unsqueeze95_out1[..],
            &constant661_out1[..],
            &constant662_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape46_out1 = linear68_out1.reshape(concat46_out1);
        let linear69_out1 = self.linear69.forward(add93_out1.clone());
        let shape94_out1: [i64; 3] = {
            let axes = &linear69_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather96_out1 = shape94_out1[0] as i64;
        let gather97_out1 = shape94_out1[1] as i64;
        let unsqueeze96_out1 = [gather96_out1 as i64];
        let unsqueeze97_out1 = [gather97_out1 as i64];
        let constant666_out1: [i64; 1] = [64i64];
        let constant665_out1: [i64; 1] = [16i64];
        let concat47_out1: [i64; 4usize] = [
            &unsqueeze96_out1[..],
            &unsqueeze97_out1[..],
            &constant665_out1[..],
            &constant666_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape47_out1 = linear69_out1.reshape(concat47_out1);
        let transpose45_out1 = reshape47_out1.permute([0, 2, 1, 3]);
        let shape96_out1: [i64; 3] = {
            let axes = &linear67_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather98_out1 = shape96_out1[0] as i64;
        let gather99_out1 = shape96_out1[1] as i64;
        let unsqueeze98_out1 = [gather98_out1 as i64];
        let unsqueeze99_out1 = [gather99_out1 as i64];
        let constant670_out1: [i64; 1] = [64i64];
        let constant669_out1: [i64; 1] = [16i64];
        let concat48_out1: [i64; 4usize] = [
            &unsqueeze98_out1[..],
            &unsqueeze99_out1[..],
            &constant669_out1[..],
            &constant670_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape48_out1 = linear67_out1.reshape(concat48_out1);
        let transpose46_out1 = reshape48_out1.permute([0, 2, 1, 3]);
        let transpose47_out1 = reshape46_out1.permute([0, 2, 3, 1]);
        let matmul92_out1 = transpose46_out1.matmul(transpose47_out1);
        let constant671_out1 = self.constant671.val();
        let div46_out1 = matmul92_out1
            .div((constant671_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add94_out1 = div46_out1.add(mul2_out1);
        let softmax12_out1 = burn::tensor::activation::softmax(add94_out1, 3);
        let matmul93_out1 = softmax12_out1.matmul(transpose45_out1);
        let transpose48_out1 = matmul93_out1.permute([0, 2, 1, 3]);
        let shape98_out1: [i64; 4] = {
            let axes = &transpose48_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather100_out1 = shape98_out1[0] as i64;
        let gather101_out1 = shape98_out1[1] as i64;
        let unsqueeze100_out1 = [gather100_out1 as i64];
        let unsqueeze101_out1 = [gather101_out1 as i64];
        let constant674_out1: [i64; 1] = [1024i64];
        let concat49_out1: [i64; 3usize] = [
            &unsqueeze100_out1[..],
            &unsqueeze101_out1[..],
            &constant674_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape49_out1 = transpose48_out1.reshape(concat49_out1);
        let linear70_out1 = self.linear70.forward(reshape49_out1);
        let add95_out1 = linear70_out1.add(add93_out1);
        add95_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule13 {
    constant675: burn::module::Param<Tensor<1>>,
    constant676: burn::module::Param<Tensor<1>>,
    constant120: burn::module::Param<Tensor<1>>,
    constant121: burn::module::Param<Tensor<1>>,
    linear71: Linear,
    constant677: burn::module::Param<Tensor<1>>,
    constant678: burn::module::Param<Tensor<1>>,
    constant679: burn::module::Param<Tensor<1>>,
    linear72: Linear,
    constant680: burn::module::Param<Tensor<1>>,
    constant681: burn::module::Param<Tensor<1>>,
    constant124: burn::module::Param<Tensor<1>>,
    constant125: burn::module::Param<Tensor<1>>,
    linear73: Linear,
    linear74: Linear,
    linear75: Linear,
    constant694: burn::module::Param<Tensor<1>>,
    linear76: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule13 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant675: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant676: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant120: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant121: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear71 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant677: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant678: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant679: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear72 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant680: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant681: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant124: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant125: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear73 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear74 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear75 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant694: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear76 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant675,
            constant676,
            constant120,
            constant121,
            linear71,
            constant677,
            constant678,
            constant679,
            linear72,
            constant680,
            constant681,
            constant124,
            constant125,
            linear73,
            linear74,
            linear75,
            constant694,
            linear76,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add95_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean47_out1 = { add95_out1.clone().mean_dim(2usize) };
        let sub25_out1 = add95_out1.sub(reducemean47_out1);
        let constant675_out1 = self.constant675.val();
        let pow24_out1 = sub25_out1
            .clone()
            .powf((constant675_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean48_out1 = { pow24_out1.mean_dim(2usize) };
        let constant676_out1 = self.constant676.val();
        let add96_out1 = reducemean48_out1
            .add((constant676_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt24_out1 = add96_out1.sqrt();
        let div47_out1 = sub25_out1.div(sqrt24_out1);
        let constant120_out1 = self.constant120.val();
        let mul49_out1 = div47_out1
            .mul((constant120_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant121_out1 = self.constant121.val();
        let add97_out1 = mul49_out1
            .add((constant121_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear71_out1 = self.linear71.forward(add97_out1.clone());
        let constant677_out1 = self.constant677.val();
        let div48_out1 = linear71_out1
            .clone()
            .div((constant677_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf12_out1 = div48_out1.erf();
        let constant678_out1 = self.constant678.val();
        let add98_out1 = erf12_out1
            .add((constant678_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul50_out1 = linear71_out1.mul(add98_out1);
        let constant679_out1 = self.constant679.val();
        let mul51_out1 = mul50_out1
            .mul((constant679_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear72_out1 = self.linear72.forward(mul51_out1);
        let add99_out1 = linear72_out1.add(add97_out1);
        let reducemean49_out1 = { add99_out1.clone().mean_dim(2usize) };
        let sub26_out1 = add99_out1.sub(reducemean49_out1);
        let constant680_out1 = self.constant680.val();
        let pow25_out1 = sub26_out1
            .clone()
            .powf((constant680_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean50_out1 = { pow25_out1.mean_dim(2usize) };
        let constant681_out1 = self.constant681.val();
        let add100_out1 = reducemean50_out1
            .add((constant681_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt25_out1 = add100_out1.sqrt();
        let div49_out1 = sub26_out1.div(sqrt25_out1);
        let constant124_out1 = self.constant124.val();
        let mul52_out1 = div49_out1
            .mul((constant124_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant125_out1 = self.constant125.val();
        let add101_out1 = mul52_out1
            .add((constant125_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear73_out1 = self.linear73.forward(add101_out1.clone());
        let linear74_out1 = self.linear74.forward(add101_out1.clone());
        let shape100_out1: [i64; 3] = {
            let axes = &linear74_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather102_out1 = shape100_out1[0] as i64;
        let gather103_out1 = shape100_out1[1] as i64;
        let unsqueeze102_out1 = [gather102_out1 as i64];
        let unsqueeze103_out1 = [gather103_out1 as i64];
        let constant685_out1: [i64; 1] = [64i64];
        let constant684_out1: [i64; 1] = [16i64];
        let concat50_out1: [i64; 4usize] = [
            &unsqueeze102_out1[..],
            &unsqueeze103_out1[..],
            &constant684_out1[..],
            &constant685_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape50_out1 = linear74_out1.reshape(concat50_out1);
        let linear75_out1 = self.linear75.forward(add101_out1.clone());
        let shape102_out1: [i64; 3] = {
            let axes = &linear75_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather104_out1 = shape102_out1[0] as i64;
        let gather105_out1 = shape102_out1[1] as i64;
        let unsqueeze104_out1 = [gather104_out1 as i64];
        let unsqueeze105_out1 = [gather105_out1 as i64];
        let constant689_out1: [i64; 1] = [64i64];
        let constant688_out1: [i64; 1] = [16i64];
        let concat51_out1: [i64; 4usize] = [
            &unsqueeze104_out1[..],
            &unsqueeze105_out1[..],
            &constant688_out1[..],
            &constant689_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape51_out1 = linear75_out1.reshape(concat51_out1);
        let transpose49_out1 = reshape51_out1.permute([0, 2, 1, 3]);
        let shape104_out1: [i64; 3] = {
            let axes = &linear73_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather106_out1 = shape104_out1[0] as i64;
        let gather107_out1 = shape104_out1[1] as i64;
        let unsqueeze106_out1 = [gather106_out1 as i64];
        let unsqueeze107_out1 = [gather107_out1 as i64];
        let constant693_out1: [i64; 1] = [64i64];
        let constant692_out1: [i64; 1] = [16i64];
        let concat52_out1: [i64; 4usize] = [
            &unsqueeze106_out1[..],
            &unsqueeze107_out1[..],
            &constant692_out1[..],
            &constant693_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape52_out1 = linear73_out1.reshape(concat52_out1);
        let transpose50_out1 = reshape52_out1.permute([0, 2, 1, 3]);
        let transpose51_out1 = reshape50_out1.permute([0, 2, 3, 1]);
        let matmul100_out1 = transpose50_out1.matmul(transpose51_out1);
        let constant694_out1 = self.constant694.val();
        let div50_out1 = matmul100_out1
            .div((constant694_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add102_out1 = div50_out1.add(mul2_out1);
        let softmax13_out1 = burn::tensor::activation::softmax(add102_out1, 3);
        let matmul101_out1 = softmax13_out1.matmul(transpose49_out1);
        let transpose52_out1 = matmul101_out1.permute([0, 2, 1, 3]);
        let shape106_out1: [i64; 4] = {
            let axes = &transpose52_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather108_out1 = shape106_out1[0] as i64;
        let gather109_out1 = shape106_out1[1] as i64;
        let unsqueeze108_out1 = [gather108_out1 as i64];
        let unsqueeze109_out1 = [gather109_out1 as i64];
        let constant697_out1: [i64; 1] = [1024i64];
        let concat53_out1: [i64; 3usize] = [
            &unsqueeze108_out1[..],
            &unsqueeze109_out1[..],
            &constant697_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape53_out1 = transpose52_out1.reshape(concat53_out1);
        let linear76_out1 = self.linear76.forward(reshape53_out1);
        let add103_out1 = linear76_out1.add(add101_out1);
        add103_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule14 {
    constant698: burn::module::Param<Tensor<1>>,
    constant699: burn::module::Param<Tensor<1>>,
    constant130: burn::module::Param<Tensor<1>>,
    constant131: burn::module::Param<Tensor<1>>,
    linear77: Linear,
    constant700: burn::module::Param<Tensor<1>>,
    constant701: burn::module::Param<Tensor<1>>,
    constant702: burn::module::Param<Tensor<1>>,
    linear78: Linear,
    constant703: burn::module::Param<Tensor<1>>,
    constant704: burn::module::Param<Tensor<1>>,
    constant134: burn::module::Param<Tensor<1>>,
    constant135: burn::module::Param<Tensor<1>>,
    linear79: Linear,
    linear80: Linear,
    linear81: Linear,
    constant717: burn::module::Param<Tensor<1>>,
    linear82: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule14 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant698: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant699: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant130: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant131: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear77 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant700: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant701: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant702: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear78 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant703: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant704: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant134: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant135: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear79 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear80 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear81 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant717: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear82 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant698,
            constant699,
            constant130,
            constant131,
            linear77,
            constant700,
            constant701,
            constant702,
            linear78,
            constant703,
            constant704,
            constant134,
            constant135,
            linear79,
            linear80,
            linear81,
            constant717,
            linear82,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add103_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean51_out1 = { add103_out1.clone().mean_dim(2usize) };
        let sub27_out1 = add103_out1.sub(reducemean51_out1);
        let constant698_out1 = self.constant698.val();
        let pow26_out1 = sub27_out1
            .clone()
            .powf((constant698_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean52_out1 = { pow26_out1.mean_dim(2usize) };
        let constant699_out1 = self.constant699.val();
        let add104_out1 = reducemean52_out1
            .add((constant699_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt26_out1 = add104_out1.sqrt();
        let div51_out1 = sub27_out1.div(sqrt26_out1);
        let constant130_out1 = self.constant130.val();
        let mul53_out1 = div51_out1
            .mul((constant130_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant131_out1 = self.constant131.val();
        let add105_out1 = mul53_out1
            .add((constant131_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear77_out1 = self.linear77.forward(add105_out1.clone());
        let constant700_out1 = self.constant700.val();
        let div52_out1 = linear77_out1
            .clone()
            .div((constant700_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf13_out1 = div52_out1.erf();
        let constant701_out1 = self.constant701.val();
        let add106_out1 = erf13_out1
            .add((constant701_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul54_out1 = linear77_out1.mul(add106_out1);
        let constant702_out1 = self.constant702.val();
        let mul55_out1 = mul54_out1
            .mul((constant702_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear78_out1 = self.linear78.forward(mul55_out1);
        let add107_out1 = linear78_out1.add(add105_out1);
        let reducemean53_out1 = { add107_out1.clone().mean_dim(2usize) };
        let sub28_out1 = add107_out1.sub(reducemean53_out1);
        let constant703_out1 = self.constant703.val();
        let pow27_out1 = sub28_out1
            .clone()
            .powf((constant703_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean54_out1 = { pow27_out1.mean_dim(2usize) };
        let constant704_out1 = self.constant704.val();
        let add108_out1 = reducemean54_out1
            .add((constant704_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt27_out1 = add108_out1.sqrt();
        let div53_out1 = sub28_out1.div(sqrt27_out1);
        let constant134_out1 = self.constant134.val();
        let mul56_out1 = div53_out1
            .mul((constant134_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant135_out1 = self.constant135.val();
        let add109_out1 = mul56_out1
            .add((constant135_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear79_out1 = self.linear79.forward(add109_out1.clone());
        let linear80_out1 = self.linear80.forward(add109_out1.clone());
        let shape108_out1: [i64; 3] = {
            let axes = &linear80_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather110_out1 = shape108_out1[0] as i64;
        let gather111_out1 = shape108_out1[1] as i64;
        let unsqueeze110_out1 = [gather110_out1 as i64];
        let unsqueeze111_out1 = [gather111_out1 as i64];
        let constant708_out1: [i64; 1] = [64i64];
        let constant707_out1: [i64; 1] = [16i64];
        let concat54_out1: [i64; 4usize] = [
            &unsqueeze110_out1[..],
            &unsqueeze111_out1[..],
            &constant707_out1[..],
            &constant708_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape54_out1 = linear80_out1.reshape(concat54_out1);
        let linear81_out1 = self.linear81.forward(add109_out1.clone());
        let shape110_out1: [i64; 3] = {
            let axes = &linear81_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather112_out1 = shape110_out1[0] as i64;
        let gather113_out1 = shape110_out1[1] as i64;
        let unsqueeze112_out1 = [gather112_out1 as i64];
        let unsqueeze113_out1 = [gather113_out1 as i64];
        let constant712_out1: [i64; 1] = [64i64];
        let constant711_out1: [i64; 1] = [16i64];
        let concat55_out1: [i64; 4usize] = [
            &unsqueeze112_out1[..],
            &unsqueeze113_out1[..],
            &constant711_out1[..],
            &constant712_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape55_out1 = linear81_out1.reshape(concat55_out1);
        let transpose53_out1 = reshape55_out1.permute([0, 2, 1, 3]);
        let shape112_out1: [i64; 3] = {
            let axes = &linear79_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather114_out1 = shape112_out1[0] as i64;
        let gather115_out1 = shape112_out1[1] as i64;
        let unsqueeze114_out1 = [gather114_out1 as i64];
        let unsqueeze115_out1 = [gather115_out1 as i64];
        let constant716_out1: [i64; 1] = [64i64];
        let constant715_out1: [i64; 1] = [16i64];
        let concat56_out1: [i64; 4usize] = [
            &unsqueeze114_out1[..],
            &unsqueeze115_out1[..],
            &constant715_out1[..],
            &constant716_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape56_out1 = linear79_out1.reshape(concat56_out1);
        let transpose54_out1 = reshape56_out1.permute([0, 2, 1, 3]);
        let transpose55_out1 = reshape54_out1.permute([0, 2, 3, 1]);
        let matmul108_out1 = transpose54_out1.matmul(transpose55_out1);
        let constant717_out1 = self.constant717.val();
        let div54_out1 = matmul108_out1
            .div((constant717_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add110_out1 = div54_out1.add(mul2_out1);
        let softmax14_out1 = burn::tensor::activation::softmax(add110_out1, 3);
        let matmul109_out1 = softmax14_out1.matmul(transpose53_out1);
        let transpose56_out1 = matmul109_out1.permute([0, 2, 1, 3]);
        let shape114_out1: [i64; 4] = {
            let axes = &transpose56_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather116_out1 = shape114_out1[0] as i64;
        let gather117_out1 = shape114_out1[1] as i64;
        let unsqueeze116_out1 = [gather116_out1 as i64];
        let unsqueeze117_out1 = [gather117_out1 as i64];
        let constant720_out1: [i64; 1] = [1024i64];
        let concat57_out1: [i64; 3usize] = [
            &unsqueeze116_out1[..],
            &unsqueeze117_out1[..],
            &constant720_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape57_out1 = transpose56_out1.reshape(concat57_out1);
        let linear82_out1 = self.linear82.forward(reshape57_out1);
        let add111_out1 = linear82_out1.add(add109_out1);
        add111_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule15 {
    constant721: burn::module::Param<Tensor<1>>,
    constant722: burn::module::Param<Tensor<1>>,
    constant140: burn::module::Param<Tensor<1>>,
    constant141: burn::module::Param<Tensor<1>>,
    linear83: Linear,
    constant723: burn::module::Param<Tensor<1>>,
    constant724: burn::module::Param<Tensor<1>>,
    constant725: burn::module::Param<Tensor<1>>,
    linear84: Linear,
    constant726: burn::module::Param<Tensor<1>>,
    constant727: burn::module::Param<Tensor<1>>,
    constant144: burn::module::Param<Tensor<1>>,
    constant145: burn::module::Param<Tensor<1>>,
    linear85: Linear,
    linear86: Linear,
    linear87: Linear,
    constant740: burn::module::Param<Tensor<1>>,
    linear88: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule15 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant721: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant722: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant140: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant141: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear83 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant723: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant724: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant725: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear84 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant726: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant727: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant144: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant145: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear85 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear86 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear87 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant740: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear88 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant721,
            constant722,
            constant140,
            constant141,
            linear83,
            constant723,
            constant724,
            constant725,
            linear84,
            constant726,
            constant727,
            constant144,
            constant145,
            linear85,
            linear86,
            linear87,
            constant740,
            linear88,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add111_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean55_out1 = { add111_out1.clone().mean_dim(2usize) };
        let sub29_out1 = add111_out1.sub(reducemean55_out1);
        let constant721_out1 = self.constant721.val();
        let pow28_out1 = sub29_out1
            .clone()
            .powf((constant721_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean56_out1 = { pow28_out1.mean_dim(2usize) };
        let constant722_out1 = self.constant722.val();
        let add112_out1 = reducemean56_out1
            .add((constant722_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt28_out1 = add112_out1.sqrt();
        let div55_out1 = sub29_out1.div(sqrt28_out1);
        let constant140_out1 = self.constant140.val();
        let mul57_out1 = div55_out1
            .mul((constant140_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant141_out1 = self.constant141.val();
        let add113_out1 = mul57_out1
            .add((constant141_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear83_out1 = self.linear83.forward(add113_out1.clone());
        let constant723_out1 = self.constant723.val();
        let div56_out1 = linear83_out1
            .clone()
            .div((constant723_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf14_out1 = div56_out1.erf();
        let constant724_out1 = self.constant724.val();
        let add114_out1 = erf14_out1
            .add((constant724_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul58_out1 = linear83_out1.mul(add114_out1);
        let constant725_out1 = self.constant725.val();
        let mul59_out1 = mul58_out1
            .mul((constant725_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear84_out1 = self.linear84.forward(mul59_out1);
        let add115_out1 = linear84_out1.add(add113_out1);
        let reducemean57_out1 = { add115_out1.clone().mean_dim(2usize) };
        let sub30_out1 = add115_out1.sub(reducemean57_out1);
        let constant726_out1 = self.constant726.val();
        let pow29_out1 = sub30_out1
            .clone()
            .powf((constant726_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean58_out1 = { pow29_out1.mean_dim(2usize) };
        let constant727_out1 = self.constant727.val();
        let add116_out1 = reducemean58_out1
            .add((constant727_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt29_out1 = add116_out1.sqrt();
        let div57_out1 = sub30_out1.div(sqrt29_out1);
        let constant144_out1 = self.constant144.val();
        let mul60_out1 = div57_out1
            .mul((constant144_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant145_out1 = self.constant145.val();
        let add117_out1 = mul60_out1
            .add((constant145_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear85_out1 = self.linear85.forward(add117_out1.clone());
        let linear86_out1 = self.linear86.forward(add117_out1.clone());
        let shape116_out1: [i64; 3] = {
            let axes = &linear86_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather118_out1 = shape116_out1[0] as i64;
        let gather119_out1 = shape116_out1[1] as i64;
        let unsqueeze118_out1 = [gather118_out1 as i64];
        let unsqueeze119_out1 = [gather119_out1 as i64];
        let constant731_out1: [i64; 1] = [64i64];
        let constant730_out1: [i64; 1] = [16i64];
        let concat58_out1: [i64; 4usize] = [
            &unsqueeze118_out1[..],
            &unsqueeze119_out1[..],
            &constant730_out1[..],
            &constant731_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape58_out1 = linear86_out1.reshape(concat58_out1);
        let linear87_out1 = self.linear87.forward(add117_out1.clone());
        let shape118_out1: [i64; 3] = {
            let axes = &linear87_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather120_out1 = shape118_out1[0] as i64;
        let gather121_out1 = shape118_out1[1] as i64;
        let unsqueeze120_out1 = [gather120_out1 as i64];
        let unsqueeze121_out1 = [gather121_out1 as i64];
        let constant735_out1: [i64; 1] = [64i64];
        let constant734_out1: [i64; 1] = [16i64];
        let concat59_out1: [i64; 4usize] = [
            &unsqueeze120_out1[..],
            &unsqueeze121_out1[..],
            &constant734_out1[..],
            &constant735_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape59_out1 = linear87_out1.reshape(concat59_out1);
        let transpose57_out1 = reshape59_out1.permute([0, 2, 1, 3]);
        let shape120_out1: [i64; 3] = {
            let axes = &linear85_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather122_out1 = shape120_out1[0] as i64;
        let gather123_out1 = shape120_out1[1] as i64;
        let unsqueeze122_out1 = [gather122_out1 as i64];
        let unsqueeze123_out1 = [gather123_out1 as i64];
        let constant739_out1: [i64; 1] = [64i64];
        let constant738_out1: [i64; 1] = [16i64];
        let concat60_out1: [i64; 4usize] = [
            &unsqueeze122_out1[..],
            &unsqueeze123_out1[..],
            &constant738_out1[..],
            &constant739_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape60_out1 = linear85_out1.reshape(concat60_out1);
        let transpose58_out1 = reshape60_out1.permute([0, 2, 1, 3]);
        let transpose59_out1 = reshape58_out1.permute([0, 2, 3, 1]);
        let matmul116_out1 = transpose58_out1.matmul(transpose59_out1);
        let constant740_out1 = self.constant740.val();
        let div58_out1 = matmul116_out1
            .div((constant740_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add118_out1 = div58_out1.add(mul2_out1);
        let softmax15_out1 = burn::tensor::activation::softmax(add118_out1, 3);
        let matmul117_out1 = softmax15_out1.matmul(transpose57_out1);
        let transpose60_out1 = matmul117_out1.permute([0, 2, 1, 3]);
        let shape122_out1: [i64; 4] = {
            let axes = &transpose60_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather124_out1 = shape122_out1[0] as i64;
        let gather125_out1 = shape122_out1[1] as i64;
        let unsqueeze124_out1 = [gather124_out1 as i64];
        let unsqueeze125_out1 = [gather125_out1 as i64];
        let constant743_out1: [i64; 1] = [1024i64];
        let concat61_out1: [i64; 3usize] = [
            &unsqueeze124_out1[..],
            &unsqueeze125_out1[..],
            &constant743_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape61_out1 = transpose60_out1.reshape(concat61_out1);
        let linear88_out1 = self.linear88.forward(reshape61_out1);
        let add119_out1 = linear88_out1.add(add117_out1);
        add119_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule16 {
    constant744: burn::module::Param<Tensor<1>>,
    constant745: burn::module::Param<Tensor<1>>,
    constant150: burn::module::Param<Tensor<1>>,
    constant151: burn::module::Param<Tensor<1>>,
    linear89: Linear,
    constant746: burn::module::Param<Tensor<1>>,
    constant747: burn::module::Param<Tensor<1>>,
    constant748: burn::module::Param<Tensor<1>>,
    linear90: Linear,
    constant749: burn::module::Param<Tensor<1>>,
    constant750: burn::module::Param<Tensor<1>>,
    constant154: burn::module::Param<Tensor<1>>,
    constant155: burn::module::Param<Tensor<1>>,
    linear91: Linear,
    linear92: Linear,
    linear93: Linear,
    constant763: burn::module::Param<Tensor<1>>,
    linear94: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule16 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant744: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant745: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant150: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant151: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear89 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant746: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant747: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant748: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear90 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant749: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant750: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant154: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant155: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear91 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear92 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear93 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant763: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear94 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant744,
            constant745,
            constant150,
            constant151,
            linear89,
            constant746,
            constant747,
            constant748,
            linear90,
            constant749,
            constant750,
            constant154,
            constant155,
            linear91,
            linear92,
            linear93,
            constant763,
            linear94,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add119_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean59_out1 = { add119_out1.clone().mean_dim(2usize) };
        let sub31_out1 = add119_out1.sub(reducemean59_out1);
        let constant744_out1 = self.constant744.val();
        let pow30_out1 = sub31_out1
            .clone()
            .powf((constant744_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean60_out1 = { pow30_out1.mean_dim(2usize) };
        let constant745_out1 = self.constant745.val();
        let add120_out1 = reducemean60_out1
            .add((constant745_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt30_out1 = add120_out1.sqrt();
        let div59_out1 = sub31_out1.div(sqrt30_out1);
        let constant150_out1 = self.constant150.val();
        let mul61_out1 = div59_out1
            .mul((constant150_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant151_out1 = self.constant151.val();
        let add121_out1 = mul61_out1
            .add((constant151_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear89_out1 = self.linear89.forward(add121_out1.clone());
        let constant746_out1 = self.constant746.val();
        let div60_out1 = linear89_out1
            .clone()
            .div((constant746_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf15_out1 = div60_out1.erf();
        let constant747_out1 = self.constant747.val();
        let add122_out1 = erf15_out1
            .add((constant747_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul62_out1 = linear89_out1.mul(add122_out1);
        let constant748_out1 = self.constant748.val();
        let mul63_out1 = mul62_out1
            .mul((constant748_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear90_out1 = self.linear90.forward(mul63_out1);
        let add123_out1 = linear90_out1.add(add121_out1);
        let reducemean61_out1 = { add123_out1.clone().mean_dim(2usize) };
        let sub32_out1 = add123_out1.sub(reducemean61_out1);
        let constant749_out1 = self.constant749.val();
        let pow31_out1 = sub32_out1
            .clone()
            .powf((constant749_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean62_out1 = { pow31_out1.mean_dim(2usize) };
        let constant750_out1 = self.constant750.val();
        let add124_out1 = reducemean62_out1
            .add((constant750_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt31_out1 = add124_out1.sqrt();
        let div61_out1 = sub32_out1.div(sqrt31_out1);
        let constant154_out1 = self.constant154.val();
        let mul64_out1 = div61_out1
            .mul((constant154_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant155_out1 = self.constant155.val();
        let add125_out1 = mul64_out1
            .add((constant155_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear91_out1 = self.linear91.forward(add125_out1.clone());
        let linear92_out1 = self.linear92.forward(add125_out1.clone());
        let shape124_out1: [i64; 3] = {
            let axes = &linear92_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather126_out1 = shape124_out1[0] as i64;
        let gather127_out1 = shape124_out1[1] as i64;
        let unsqueeze126_out1 = [gather126_out1 as i64];
        let unsqueeze127_out1 = [gather127_out1 as i64];
        let constant754_out1: [i64; 1] = [64i64];
        let constant753_out1: [i64; 1] = [16i64];
        let concat62_out1: [i64; 4usize] = [
            &unsqueeze126_out1[..],
            &unsqueeze127_out1[..],
            &constant753_out1[..],
            &constant754_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape62_out1 = linear92_out1.reshape(concat62_out1);
        let linear93_out1 = self.linear93.forward(add125_out1.clone());
        let shape126_out1: [i64; 3] = {
            let axes = &linear93_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather128_out1 = shape126_out1[0] as i64;
        let gather129_out1 = shape126_out1[1] as i64;
        let unsqueeze128_out1 = [gather128_out1 as i64];
        let unsqueeze129_out1 = [gather129_out1 as i64];
        let constant758_out1: [i64; 1] = [64i64];
        let constant757_out1: [i64; 1] = [16i64];
        let concat63_out1: [i64; 4usize] = [
            &unsqueeze128_out1[..],
            &unsqueeze129_out1[..],
            &constant757_out1[..],
            &constant758_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape63_out1 = linear93_out1.reshape(concat63_out1);
        let transpose61_out1 = reshape63_out1.permute([0, 2, 1, 3]);
        let shape128_out1: [i64; 3] = {
            let axes = &linear91_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather130_out1 = shape128_out1[0] as i64;
        let gather131_out1 = shape128_out1[1] as i64;
        let unsqueeze130_out1 = [gather130_out1 as i64];
        let unsqueeze131_out1 = [gather131_out1 as i64];
        let constant762_out1: [i64; 1] = [64i64];
        let constant761_out1: [i64; 1] = [16i64];
        let concat64_out1: [i64; 4usize] = [
            &unsqueeze130_out1[..],
            &unsqueeze131_out1[..],
            &constant761_out1[..],
            &constant762_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape64_out1 = linear91_out1.reshape(concat64_out1);
        let transpose62_out1 = reshape64_out1.permute([0, 2, 1, 3]);
        let transpose63_out1 = reshape62_out1.permute([0, 2, 3, 1]);
        let matmul124_out1 = transpose62_out1.matmul(transpose63_out1);
        let constant763_out1 = self.constant763.val();
        let div62_out1 = matmul124_out1
            .div((constant763_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add126_out1 = div62_out1.add(mul2_out1);
        let softmax16_out1 = burn::tensor::activation::softmax(add126_out1, 3);
        let matmul125_out1 = softmax16_out1.matmul(transpose61_out1);
        let transpose64_out1 = matmul125_out1.permute([0, 2, 1, 3]);
        let shape130_out1: [i64; 4] = {
            let axes = &transpose64_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather132_out1 = shape130_out1[0] as i64;
        let gather133_out1 = shape130_out1[1] as i64;
        let unsqueeze132_out1 = [gather132_out1 as i64];
        let unsqueeze133_out1 = [gather133_out1 as i64];
        let constant766_out1: [i64; 1] = [1024i64];
        let concat65_out1: [i64; 3usize] = [
            &unsqueeze132_out1[..],
            &unsqueeze133_out1[..],
            &constant766_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape65_out1 = transpose64_out1.reshape(concat65_out1);
        let linear94_out1 = self.linear94.forward(reshape65_out1);
        let add127_out1 = linear94_out1.add(add125_out1);
        add127_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule17 {
    constant767: burn::module::Param<Tensor<1>>,
    constant768: burn::module::Param<Tensor<1>>,
    constant160: burn::module::Param<Tensor<1>>,
    constant161: burn::module::Param<Tensor<1>>,
    linear95: Linear,
    constant769: burn::module::Param<Tensor<1>>,
    constant770: burn::module::Param<Tensor<1>>,
    constant771: burn::module::Param<Tensor<1>>,
    linear96: Linear,
    constant772: burn::module::Param<Tensor<1>>,
    constant773: burn::module::Param<Tensor<1>>,
    constant164: burn::module::Param<Tensor<1>>,
    constant165: burn::module::Param<Tensor<1>>,
    linear97: Linear,
    linear98: Linear,
    linear99: Linear,
    constant786: burn::module::Param<Tensor<1>>,
    linear100: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule17 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant767: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant768: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant160: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant161: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear95 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant769: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant770: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant771: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear96 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant772: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant773: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant164: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant165: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear97 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear98 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear99 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant786: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear100 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant767,
            constant768,
            constant160,
            constant161,
            linear95,
            constant769,
            constant770,
            constant771,
            linear96,
            constant772,
            constant773,
            constant164,
            constant165,
            linear97,
            linear98,
            linear99,
            constant786,
            linear100,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add127_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean63_out1 = { add127_out1.clone().mean_dim(2usize) };
        let sub33_out1 = add127_out1.sub(reducemean63_out1);
        let constant767_out1 = self.constant767.val();
        let pow32_out1 = sub33_out1
            .clone()
            .powf((constant767_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean64_out1 = { pow32_out1.mean_dim(2usize) };
        let constant768_out1 = self.constant768.val();
        let add128_out1 = reducemean64_out1
            .add((constant768_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt32_out1 = add128_out1.sqrt();
        let div63_out1 = sub33_out1.div(sqrt32_out1);
        let constant160_out1 = self.constant160.val();
        let mul65_out1 = div63_out1
            .mul((constant160_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant161_out1 = self.constant161.val();
        let add129_out1 = mul65_out1
            .add((constant161_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear95_out1 = self.linear95.forward(add129_out1.clone());
        let constant769_out1 = self.constant769.val();
        let div64_out1 = linear95_out1
            .clone()
            .div((constant769_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf16_out1 = div64_out1.erf();
        let constant770_out1 = self.constant770.val();
        let add130_out1 = erf16_out1
            .add((constant770_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul66_out1 = linear95_out1.mul(add130_out1);
        let constant771_out1 = self.constant771.val();
        let mul67_out1 = mul66_out1
            .mul((constant771_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear96_out1 = self.linear96.forward(mul67_out1);
        let add131_out1 = linear96_out1.add(add129_out1);
        let reducemean65_out1 = { add131_out1.clone().mean_dim(2usize) };
        let sub34_out1 = add131_out1.sub(reducemean65_out1);
        let constant772_out1 = self.constant772.val();
        let pow33_out1 = sub34_out1
            .clone()
            .powf((constant772_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean66_out1 = { pow33_out1.mean_dim(2usize) };
        let constant773_out1 = self.constant773.val();
        let add132_out1 = reducemean66_out1
            .add((constant773_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt33_out1 = add132_out1.sqrt();
        let div65_out1 = sub34_out1.div(sqrt33_out1);
        let constant164_out1 = self.constant164.val();
        let mul68_out1 = div65_out1
            .mul((constant164_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant165_out1 = self.constant165.val();
        let add133_out1 = mul68_out1
            .add((constant165_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear97_out1 = self.linear97.forward(add133_out1.clone());
        let linear98_out1 = self.linear98.forward(add133_out1.clone());
        let shape132_out1: [i64; 3] = {
            let axes = &linear98_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather134_out1 = shape132_out1[0] as i64;
        let gather135_out1 = shape132_out1[1] as i64;
        let unsqueeze134_out1 = [gather134_out1 as i64];
        let unsqueeze135_out1 = [gather135_out1 as i64];
        let constant777_out1: [i64; 1] = [64i64];
        let constant776_out1: [i64; 1] = [16i64];
        let concat66_out1: [i64; 4usize] = [
            &unsqueeze134_out1[..],
            &unsqueeze135_out1[..],
            &constant776_out1[..],
            &constant777_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape66_out1 = linear98_out1.reshape(concat66_out1);
        let linear99_out1 = self.linear99.forward(add133_out1.clone());
        let shape134_out1: [i64; 3] = {
            let axes = &linear99_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather136_out1 = shape134_out1[0] as i64;
        let gather137_out1 = shape134_out1[1] as i64;
        let unsqueeze136_out1 = [gather136_out1 as i64];
        let unsqueeze137_out1 = [gather137_out1 as i64];
        let constant781_out1: [i64; 1] = [64i64];
        let constant780_out1: [i64; 1] = [16i64];
        let concat67_out1: [i64; 4usize] = [
            &unsqueeze136_out1[..],
            &unsqueeze137_out1[..],
            &constant780_out1[..],
            &constant781_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape67_out1 = linear99_out1.reshape(concat67_out1);
        let transpose65_out1 = reshape67_out1.permute([0, 2, 1, 3]);
        let shape136_out1: [i64; 3] = {
            let axes = &linear97_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather138_out1 = shape136_out1[0] as i64;
        let gather139_out1 = shape136_out1[1] as i64;
        let unsqueeze138_out1 = [gather138_out1 as i64];
        let unsqueeze139_out1 = [gather139_out1 as i64];
        let constant785_out1: [i64; 1] = [64i64];
        let constant784_out1: [i64; 1] = [16i64];
        let concat68_out1: [i64; 4usize] = [
            &unsqueeze138_out1[..],
            &unsqueeze139_out1[..],
            &constant784_out1[..],
            &constant785_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape68_out1 = linear97_out1.reshape(concat68_out1);
        let transpose66_out1 = reshape68_out1.permute([0, 2, 1, 3]);
        let transpose67_out1 = reshape66_out1.permute([0, 2, 3, 1]);
        let matmul132_out1 = transpose66_out1.matmul(transpose67_out1);
        let constant786_out1 = self.constant786.val();
        let div66_out1 = matmul132_out1
            .div((constant786_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add134_out1 = div66_out1.add(mul2_out1);
        let softmax17_out1 = burn::tensor::activation::softmax(add134_out1, 3);
        let matmul133_out1 = softmax17_out1.matmul(transpose65_out1);
        let transpose68_out1 = matmul133_out1.permute([0, 2, 1, 3]);
        let shape138_out1: [i64; 4] = {
            let axes = &transpose68_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather140_out1 = shape138_out1[0] as i64;
        let gather141_out1 = shape138_out1[1] as i64;
        let unsqueeze140_out1 = [gather140_out1 as i64];
        let unsqueeze141_out1 = [gather141_out1 as i64];
        let constant789_out1: [i64; 1] = [1024i64];
        let concat69_out1: [i64; 3usize] = [
            &unsqueeze140_out1[..],
            &unsqueeze141_out1[..],
            &constant789_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape69_out1 = transpose68_out1.reshape(concat69_out1);
        let linear100_out1 = self.linear100.forward(reshape69_out1);
        let add135_out1 = linear100_out1.add(add133_out1);
        add135_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule18 {
    constant790: burn::module::Param<Tensor<1>>,
    constant791: burn::module::Param<Tensor<1>>,
    constant170: burn::module::Param<Tensor<1>>,
    constant171: burn::module::Param<Tensor<1>>,
    linear101: Linear,
    constant792: burn::module::Param<Tensor<1>>,
    constant793: burn::module::Param<Tensor<1>>,
    constant794: burn::module::Param<Tensor<1>>,
    linear102: Linear,
    constant795: burn::module::Param<Tensor<1>>,
    constant796: burn::module::Param<Tensor<1>>,
    constant174: burn::module::Param<Tensor<1>>,
    constant175: burn::module::Param<Tensor<1>>,
    linear103: Linear,
    linear104: Linear,
    linear105: Linear,
    constant809: burn::module::Param<Tensor<1>>,
    linear106: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule18 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant790: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant791: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant170: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant171: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear101 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant792: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant793: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant794: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear102 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant795: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant796: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant174: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant175: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear103 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear104 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear105 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant809: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear106 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant790,
            constant791,
            constant170,
            constant171,
            linear101,
            constant792,
            constant793,
            constant794,
            linear102,
            constant795,
            constant796,
            constant174,
            constant175,
            linear103,
            linear104,
            linear105,
            constant809,
            linear106,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add135_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean67_out1 = { add135_out1.clone().mean_dim(2usize) };
        let sub35_out1 = add135_out1.sub(reducemean67_out1);
        let constant790_out1 = self.constant790.val();
        let pow34_out1 = sub35_out1
            .clone()
            .powf((constant790_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean68_out1 = { pow34_out1.mean_dim(2usize) };
        let constant791_out1 = self.constant791.val();
        let add136_out1 = reducemean68_out1
            .add((constant791_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt34_out1 = add136_out1.sqrt();
        let div67_out1 = sub35_out1.div(sqrt34_out1);
        let constant170_out1 = self.constant170.val();
        let mul69_out1 = div67_out1
            .mul((constant170_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant171_out1 = self.constant171.val();
        let add137_out1 = mul69_out1
            .add((constant171_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear101_out1 = self.linear101.forward(add137_out1.clone());
        let constant792_out1 = self.constant792.val();
        let div68_out1 = linear101_out1
            .clone()
            .div((constant792_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf17_out1 = div68_out1.erf();
        let constant793_out1 = self.constant793.val();
        let add138_out1 = erf17_out1
            .add((constant793_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul70_out1 = linear101_out1.mul(add138_out1);
        let constant794_out1 = self.constant794.val();
        let mul71_out1 = mul70_out1
            .mul((constant794_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear102_out1 = self.linear102.forward(mul71_out1);
        let add139_out1 = linear102_out1.add(add137_out1);
        let reducemean69_out1 = { add139_out1.clone().mean_dim(2usize) };
        let sub36_out1 = add139_out1.sub(reducemean69_out1);
        let constant795_out1 = self.constant795.val();
        let pow35_out1 = sub36_out1
            .clone()
            .powf((constant795_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean70_out1 = { pow35_out1.mean_dim(2usize) };
        let constant796_out1 = self.constant796.val();
        let add140_out1 = reducemean70_out1
            .add((constant796_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt35_out1 = add140_out1.sqrt();
        let div69_out1 = sub36_out1.div(sqrt35_out1);
        let constant174_out1 = self.constant174.val();
        let mul72_out1 = div69_out1
            .mul((constant174_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant175_out1 = self.constant175.val();
        let add141_out1 = mul72_out1
            .add((constant175_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear103_out1 = self.linear103.forward(add141_out1.clone());
        let linear104_out1 = self.linear104.forward(add141_out1.clone());
        let shape140_out1: [i64; 3] = {
            let axes = &linear104_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather142_out1 = shape140_out1[0] as i64;
        let gather143_out1 = shape140_out1[1] as i64;
        let unsqueeze142_out1 = [gather142_out1 as i64];
        let unsqueeze143_out1 = [gather143_out1 as i64];
        let constant800_out1: [i64; 1] = [64i64];
        let constant799_out1: [i64; 1] = [16i64];
        let concat70_out1: [i64; 4usize] = [
            &unsqueeze142_out1[..],
            &unsqueeze143_out1[..],
            &constant799_out1[..],
            &constant800_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape70_out1 = linear104_out1.reshape(concat70_out1);
        let linear105_out1 = self.linear105.forward(add141_out1.clone());
        let shape142_out1: [i64; 3] = {
            let axes = &linear105_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather144_out1 = shape142_out1[0] as i64;
        let gather145_out1 = shape142_out1[1] as i64;
        let unsqueeze144_out1 = [gather144_out1 as i64];
        let unsqueeze145_out1 = [gather145_out1 as i64];
        let constant804_out1: [i64; 1] = [64i64];
        let constant803_out1: [i64; 1] = [16i64];
        let concat71_out1: [i64; 4usize] = [
            &unsqueeze144_out1[..],
            &unsqueeze145_out1[..],
            &constant803_out1[..],
            &constant804_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape71_out1 = linear105_out1.reshape(concat71_out1);
        let transpose69_out1 = reshape71_out1.permute([0, 2, 1, 3]);
        let shape144_out1: [i64; 3] = {
            let axes = &linear103_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather146_out1 = shape144_out1[0] as i64;
        let gather147_out1 = shape144_out1[1] as i64;
        let unsqueeze146_out1 = [gather146_out1 as i64];
        let unsqueeze147_out1 = [gather147_out1 as i64];
        let constant808_out1: [i64; 1] = [64i64];
        let constant807_out1: [i64; 1] = [16i64];
        let concat72_out1: [i64; 4usize] = [
            &unsqueeze146_out1[..],
            &unsqueeze147_out1[..],
            &constant807_out1[..],
            &constant808_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape72_out1 = linear103_out1.reshape(concat72_out1);
        let transpose70_out1 = reshape72_out1.permute([0, 2, 1, 3]);
        let transpose71_out1 = reshape70_out1.permute([0, 2, 3, 1]);
        let matmul140_out1 = transpose70_out1.matmul(transpose71_out1);
        let constant809_out1 = self.constant809.val();
        let div70_out1 = matmul140_out1
            .div((constant809_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add142_out1 = div70_out1.add(mul2_out1);
        let softmax18_out1 = burn::tensor::activation::softmax(add142_out1, 3);
        let matmul141_out1 = softmax18_out1.matmul(transpose69_out1);
        let transpose72_out1 = matmul141_out1.permute([0, 2, 1, 3]);
        let shape146_out1: [i64; 4] = {
            let axes = &transpose72_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather148_out1 = shape146_out1[0] as i64;
        let gather149_out1 = shape146_out1[1] as i64;
        let unsqueeze148_out1 = [gather148_out1 as i64];
        let unsqueeze149_out1 = [gather149_out1 as i64];
        let constant812_out1: [i64; 1] = [1024i64];
        let concat73_out1: [i64; 3usize] = [
            &unsqueeze148_out1[..],
            &unsqueeze149_out1[..],
            &constant812_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape73_out1 = transpose72_out1.reshape(concat73_out1);
        let linear106_out1 = self.linear106.forward(reshape73_out1);
        let add143_out1 = linear106_out1.add(add141_out1);
        add143_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule19 {
    constant813: burn::module::Param<Tensor<1>>,
    constant814: burn::module::Param<Tensor<1>>,
    constant180: burn::module::Param<Tensor<1>>,
    constant181: burn::module::Param<Tensor<1>>,
    linear107: Linear,
    constant815: burn::module::Param<Tensor<1>>,
    constant816: burn::module::Param<Tensor<1>>,
    constant817: burn::module::Param<Tensor<1>>,
    linear108: Linear,
    constant818: burn::module::Param<Tensor<1>>,
    constant819: burn::module::Param<Tensor<1>>,
    constant184: burn::module::Param<Tensor<1>>,
    constant185: burn::module::Param<Tensor<1>>,
    linear109: Linear,
    linear110: Linear,
    linear111: Linear,
    constant832: burn::module::Param<Tensor<1>>,
    linear112: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule19 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant813: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant814: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant180: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant181: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear107 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant815: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant816: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant817: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear108 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant818: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant819: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant184: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant185: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear109 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear110 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear111 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant832: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear112 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant813,
            constant814,
            constant180,
            constant181,
            linear107,
            constant815,
            constant816,
            constant817,
            linear108,
            constant818,
            constant819,
            constant184,
            constant185,
            linear109,
            linear110,
            linear111,
            constant832,
            linear112,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add143_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean71_out1 = { add143_out1.clone().mean_dim(2usize) };
        let sub37_out1 = add143_out1.sub(reducemean71_out1);
        let constant813_out1 = self.constant813.val();
        let pow36_out1 = sub37_out1
            .clone()
            .powf((constant813_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean72_out1 = { pow36_out1.mean_dim(2usize) };
        let constant814_out1 = self.constant814.val();
        let add144_out1 = reducemean72_out1
            .add((constant814_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt36_out1 = add144_out1.sqrt();
        let div71_out1 = sub37_out1.div(sqrt36_out1);
        let constant180_out1 = self.constant180.val();
        let mul73_out1 = div71_out1
            .mul((constant180_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant181_out1 = self.constant181.val();
        let add145_out1 = mul73_out1
            .add((constant181_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear107_out1 = self.linear107.forward(add145_out1.clone());
        let constant815_out1 = self.constant815.val();
        let div72_out1 = linear107_out1
            .clone()
            .div((constant815_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf18_out1 = div72_out1.erf();
        let constant816_out1 = self.constant816.val();
        let add146_out1 = erf18_out1
            .add((constant816_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul74_out1 = linear107_out1.mul(add146_out1);
        let constant817_out1 = self.constant817.val();
        let mul75_out1 = mul74_out1
            .mul((constant817_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear108_out1 = self.linear108.forward(mul75_out1);
        let add147_out1 = linear108_out1.add(add145_out1);
        let reducemean73_out1 = { add147_out1.clone().mean_dim(2usize) };
        let sub38_out1 = add147_out1.sub(reducemean73_out1);
        let constant818_out1 = self.constant818.val();
        let pow37_out1 = sub38_out1
            .clone()
            .powf((constant818_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean74_out1 = { pow37_out1.mean_dim(2usize) };
        let constant819_out1 = self.constant819.val();
        let add148_out1 = reducemean74_out1
            .add((constant819_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt37_out1 = add148_out1.sqrt();
        let div73_out1 = sub38_out1.div(sqrt37_out1);
        let constant184_out1 = self.constant184.val();
        let mul76_out1 = div73_out1
            .mul((constant184_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant185_out1 = self.constant185.val();
        let add149_out1 = mul76_out1
            .add((constant185_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear109_out1 = self.linear109.forward(add149_out1.clone());
        let linear110_out1 = self.linear110.forward(add149_out1.clone());
        let shape148_out1: [i64; 3] = {
            let axes = &linear110_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather150_out1 = shape148_out1[0] as i64;
        let gather151_out1 = shape148_out1[1] as i64;
        let unsqueeze150_out1 = [gather150_out1 as i64];
        let unsqueeze151_out1 = [gather151_out1 as i64];
        let constant823_out1: [i64; 1] = [64i64];
        let constant822_out1: [i64; 1] = [16i64];
        let concat74_out1: [i64; 4usize] = [
            &unsqueeze150_out1[..],
            &unsqueeze151_out1[..],
            &constant822_out1[..],
            &constant823_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape74_out1 = linear110_out1.reshape(concat74_out1);
        let linear111_out1 = self.linear111.forward(add149_out1.clone());
        let shape150_out1: [i64; 3] = {
            let axes = &linear111_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather152_out1 = shape150_out1[0] as i64;
        let gather153_out1 = shape150_out1[1] as i64;
        let unsqueeze152_out1 = [gather152_out1 as i64];
        let unsqueeze153_out1 = [gather153_out1 as i64];
        let constant827_out1: [i64; 1] = [64i64];
        let constant826_out1: [i64; 1] = [16i64];
        let concat75_out1: [i64; 4usize] = [
            &unsqueeze152_out1[..],
            &unsqueeze153_out1[..],
            &constant826_out1[..],
            &constant827_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape75_out1 = linear111_out1.reshape(concat75_out1);
        let transpose73_out1 = reshape75_out1.permute([0, 2, 1, 3]);
        let shape152_out1: [i64; 3] = {
            let axes = &linear109_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather154_out1 = shape152_out1[0] as i64;
        let gather155_out1 = shape152_out1[1] as i64;
        let unsqueeze154_out1 = [gather154_out1 as i64];
        let unsqueeze155_out1 = [gather155_out1 as i64];
        let constant831_out1: [i64; 1] = [64i64];
        let constant830_out1: [i64; 1] = [16i64];
        let concat76_out1: [i64; 4usize] = [
            &unsqueeze154_out1[..],
            &unsqueeze155_out1[..],
            &constant830_out1[..],
            &constant831_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape76_out1 = linear109_out1.reshape(concat76_out1);
        let transpose74_out1 = reshape76_out1.permute([0, 2, 1, 3]);
        let transpose75_out1 = reshape74_out1.permute([0, 2, 3, 1]);
        let matmul148_out1 = transpose74_out1.matmul(transpose75_out1);
        let constant832_out1 = self.constant832.val();
        let div74_out1 = matmul148_out1
            .div((constant832_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add150_out1 = div74_out1.add(mul2_out1);
        let softmax19_out1 = burn::tensor::activation::softmax(add150_out1, 3);
        let matmul149_out1 = softmax19_out1.matmul(transpose73_out1);
        let transpose76_out1 = matmul149_out1.permute([0, 2, 1, 3]);
        let shape154_out1: [i64; 4] = {
            let axes = &transpose76_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather156_out1 = shape154_out1[0] as i64;
        let gather157_out1 = shape154_out1[1] as i64;
        let unsqueeze156_out1 = [gather156_out1 as i64];
        let unsqueeze157_out1 = [gather157_out1 as i64];
        let constant835_out1: [i64; 1] = [1024i64];
        let concat77_out1: [i64; 3usize] = [
            &unsqueeze156_out1[..],
            &unsqueeze157_out1[..],
            &constant835_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape77_out1 = transpose76_out1.reshape(concat77_out1);
        let linear112_out1 = self.linear112.forward(reshape77_out1);
        let add151_out1 = linear112_out1.add(add149_out1);
        add151_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule20 {
    constant836: burn::module::Param<Tensor<1>>,
    constant837: burn::module::Param<Tensor<1>>,
    constant190: burn::module::Param<Tensor<1>>,
    constant191: burn::module::Param<Tensor<1>>,
    linear113: Linear,
    constant838: burn::module::Param<Tensor<1>>,
    constant839: burn::module::Param<Tensor<1>>,
    constant840: burn::module::Param<Tensor<1>>,
    linear114: Linear,
    constant841: burn::module::Param<Tensor<1>>,
    constant842: burn::module::Param<Tensor<1>>,
    constant194: burn::module::Param<Tensor<1>>,
    constant195: burn::module::Param<Tensor<1>>,
    linear115: Linear,
    linear116: Linear,
    linear117: Linear,
    constant855: burn::module::Param<Tensor<1>>,
    linear118: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule20 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant836: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant837: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant190: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant191: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear113 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant838: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant839: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant840: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear114 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant841: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant842: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant194: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant195: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear115 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear116 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear117 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant855: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear118 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant836,
            constant837,
            constant190,
            constant191,
            linear113,
            constant838,
            constant839,
            constant840,
            linear114,
            constant841,
            constant842,
            constant194,
            constant195,
            linear115,
            linear116,
            linear117,
            constant855,
            linear118,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add151_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean75_out1 = { add151_out1.clone().mean_dim(2usize) };
        let sub39_out1 = add151_out1.sub(reducemean75_out1);
        let constant836_out1 = self.constant836.val();
        let pow38_out1 = sub39_out1
            .clone()
            .powf((constant836_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean76_out1 = { pow38_out1.mean_dim(2usize) };
        let constant837_out1 = self.constant837.val();
        let add152_out1 = reducemean76_out1
            .add((constant837_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt38_out1 = add152_out1.sqrt();
        let div75_out1 = sub39_out1.div(sqrt38_out1);
        let constant190_out1 = self.constant190.val();
        let mul77_out1 = div75_out1
            .mul((constant190_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant191_out1 = self.constant191.val();
        let add153_out1 = mul77_out1
            .add((constant191_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear113_out1 = self.linear113.forward(add153_out1.clone());
        let constant838_out1 = self.constant838.val();
        let div76_out1 = linear113_out1
            .clone()
            .div((constant838_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf19_out1 = div76_out1.erf();
        let constant839_out1 = self.constant839.val();
        let add154_out1 = erf19_out1
            .add((constant839_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul78_out1 = linear113_out1.mul(add154_out1);
        let constant840_out1 = self.constant840.val();
        let mul79_out1 = mul78_out1
            .mul((constant840_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear114_out1 = self.linear114.forward(mul79_out1);
        let add155_out1 = linear114_out1.add(add153_out1);
        let reducemean77_out1 = { add155_out1.clone().mean_dim(2usize) };
        let sub40_out1 = add155_out1.sub(reducemean77_out1);
        let constant841_out1 = self.constant841.val();
        let pow39_out1 = sub40_out1
            .clone()
            .powf((constant841_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean78_out1 = { pow39_out1.mean_dim(2usize) };
        let constant842_out1 = self.constant842.val();
        let add156_out1 = reducemean78_out1
            .add((constant842_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt39_out1 = add156_out1.sqrt();
        let div77_out1 = sub40_out1.div(sqrt39_out1);
        let constant194_out1 = self.constant194.val();
        let mul80_out1 = div77_out1
            .mul((constant194_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant195_out1 = self.constant195.val();
        let add157_out1 = mul80_out1
            .add((constant195_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear115_out1 = self.linear115.forward(add157_out1.clone());
        let linear116_out1 = self.linear116.forward(add157_out1.clone());
        let shape156_out1: [i64; 3] = {
            let axes = &linear116_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather158_out1 = shape156_out1[0] as i64;
        let gather159_out1 = shape156_out1[1] as i64;
        let unsqueeze158_out1 = [gather158_out1 as i64];
        let unsqueeze159_out1 = [gather159_out1 as i64];
        let constant846_out1: [i64; 1] = [64i64];
        let constant845_out1: [i64; 1] = [16i64];
        let concat78_out1: [i64; 4usize] = [
            &unsqueeze158_out1[..],
            &unsqueeze159_out1[..],
            &constant845_out1[..],
            &constant846_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape78_out1 = linear116_out1.reshape(concat78_out1);
        let linear117_out1 = self.linear117.forward(add157_out1.clone());
        let shape158_out1: [i64; 3] = {
            let axes = &linear117_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather160_out1 = shape158_out1[0] as i64;
        let gather161_out1 = shape158_out1[1] as i64;
        let unsqueeze160_out1 = [gather160_out1 as i64];
        let unsqueeze161_out1 = [gather161_out1 as i64];
        let constant850_out1: [i64; 1] = [64i64];
        let constant849_out1: [i64; 1] = [16i64];
        let concat79_out1: [i64; 4usize] = [
            &unsqueeze160_out1[..],
            &unsqueeze161_out1[..],
            &constant849_out1[..],
            &constant850_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape79_out1 = linear117_out1.reshape(concat79_out1);
        let transpose77_out1 = reshape79_out1.permute([0, 2, 1, 3]);
        let shape160_out1: [i64; 3] = {
            let axes = &linear115_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather162_out1 = shape160_out1[0] as i64;
        let gather163_out1 = shape160_out1[1] as i64;
        let unsqueeze162_out1 = [gather162_out1 as i64];
        let unsqueeze163_out1 = [gather163_out1 as i64];
        let constant854_out1: [i64; 1] = [64i64];
        let constant853_out1: [i64; 1] = [16i64];
        let concat80_out1: [i64; 4usize] = [
            &unsqueeze162_out1[..],
            &unsqueeze163_out1[..],
            &constant853_out1[..],
            &constant854_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape80_out1 = linear115_out1.reshape(concat80_out1);
        let transpose78_out1 = reshape80_out1.permute([0, 2, 1, 3]);
        let transpose79_out1 = reshape78_out1.permute([0, 2, 3, 1]);
        let matmul156_out1 = transpose78_out1.matmul(transpose79_out1);
        let constant855_out1 = self.constant855.val();
        let div78_out1 = matmul156_out1
            .div((constant855_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add158_out1 = div78_out1.add(mul2_out1);
        let softmax20_out1 = burn::tensor::activation::softmax(add158_out1, 3);
        let matmul157_out1 = softmax20_out1.matmul(transpose77_out1);
        let transpose80_out1 = matmul157_out1.permute([0, 2, 1, 3]);
        let shape162_out1: [i64; 4] = {
            let axes = &transpose80_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather164_out1 = shape162_out1[0] as i64;
        let gather165_out1 = shape162_out1[1] as i64;
        let unsqueeze164_out1 = [gather164_out1 as i64];
        let unsqueeze165_out1 = [gather165_out1 as i64];
        let constant858_out1: [i64; 1] = [1024i64];
        let concat81_out1: [i64; 3usize] = [
            &unsqueeze164_out1[..],
            &unsqueeze165_out1[..],
            &constant858_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape81_out1 = transpose80_out1.reshape(concat81_out1);
        let linear118_out1 = self.linear118.forward(reshape81_out1);
        let add159_out1 = linear118_out1.add(add157_out1);
        add159_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule21 {
    constant859: burn::module::Param<Tensor<1>>,
    constant860: burn::module::Param<Tensor<1>>,
    constant200: burn::module::Param<Tensor<1>>,
    constant201: burn::module::Param<Tensor<1>>,
    linear119: Linear,
    constant861: burn::module::Param<Tensor<1>>,
    constant862: burn::module::Param<Tensor<1>>,
    constant863: burn::module::Param<Tensor<1>>,
    linear120: Linear,
    constant864: burn::module::Param<Tensor<1>>,
    constant865: burn::module::Param<Tensor<1>>,
    constant204: burn::module::Param<Tensor<1>>,
    constant205: burn::module::Param<Tensor<1>>,
    linear121: Linear,
    linear122: Linear,
    linear123: Linear,
    constant878: burn::module::Param<Tensor<1>>,
    linear124: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule21 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant859: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant860: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant200: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant201: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear119 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant861: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant862: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant863: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear120 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant864: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant865: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant204: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant205: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear121 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear122 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear123 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant878: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear124 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant859,
            constant860,
            constant200,
            constant201,
            linear119,
            constant861,
            constant862,
            constant863,
            linear120,
            constant864,
            constant865,
            constant204,
            constant205,
            linear121,
            linear122,
            linear123,
            constant878,
            linear124,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add159_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean79_out1 = { add159_out1.clone().mean_dim(2usize) };
        let sub41_out1 = add159_out1.sub(reducemean79_out1);
        let constant859_out1 = self.constant859.val();
        let pow40_out1 = sub41_out1
            .clone()
            .powf((constant859_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean80_out1 = { pow40_out1.mean_dim(2usize) };
        let constant860_out1 = self.constant860.val();
        let add160_out1 = reducemean80_out1
            .add((constant860_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt40_out1 = add160_out1.sqrt();
        let div79_out1 = sub41_out1.div(sqrt40_out1);
        let constant200_out1 = self.constant200.val();
        let mul81_out1 = div79_out1
            .mul((constant200_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant201_out1 = self.constant201.val();
        let add161_out1 = mul81_out1
            .add((constant201_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear119_out1 = self.linear119.forward(add161_out1.clone());
        let constant861_out1 = self.constant861.val();
        let div80_out1 = linear119_out1
            .clone()
            .div((constant861_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf20_out1 = div80_out1.erf();
        let constant862_out1 = self.constant862.val();
        let add162_out1 = erf20_out1
            .add((constant862_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul82_out1 = linear119_out1.mul(add162_out1);
        let constant863_out1 = self.constant863.val();
        let mul83_out1 = mul82_out1
            .mul((constant863_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear120_out1 = self.linear120.forward(mul83_out1);
        let add163_out1 = linear120_out1.add(add161_out1);
        let reducemean81_out1 = { add163_out1.clone().mean_dim(2usize) };
        let sub42_out1 = add163_out1.sub(reducemean81_out1);
        let constant864_out1 = self.constant864.val();
        let pow41_out1 = sub42_out1
            .clone()
            .powf((constant864_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean82_out1 = { pow41_out1.mean_dim(2usize) };
        let constant865_out1 = self.constant865.val();
        let add164_out1 = reducemean82_out1
            .add((constant865_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt41_out1 = add164_out1.sqrt();
        let div81_out1 = sub42_out1.div(sqrt41_out1);
        let constant204_out1 = self.constant204.val();
        let mul84_out1 = div81_out1
            .mul((constant204_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant205_out1 = self.constant205.val();
        let add165_out1 = mul84_out1
            .add((constant205_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear121_out1 = self.linear121.forward(add165_out1.clone());
        let linear122_out1 = self.linear122.forward(add165_out1.clone());
        let shape164_out1: [i64; 3] = {
            let axes = &linear122_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather166_out1 = shape164_out1[0] as i64;
        let gather167_out1 = shape164_out1[1] as i64;
        let unsqueeze166_out1 = [gather166_out1 as i64];
        let unsqueeze167_out1 = [gather167_out1 as i64];
        let constant869_out1: [i64; 1] = [64i64];
        let constant868_out1: [i64; 1] = [16i64];
        let concat82_out1: [i64; 4usize] = [
            &unsqueeze166_out1[..],
            &unsqueeze167_out1[..],
            &constant868_out1[..],
            &constant869_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape82_out1 = linear122_out1.reshape(concat82_out1);
        let linear123_out1 = self.linear123.forward(add165_out1.clone());
        let shape166_out1: [i64; 3] = {
            let axes = &linear123_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather168_out1 = shape166_out1[0] as i64;
        let gather169_out1 = shape166_out1[1] as i64;
        let unsqueeze168_out1 = [gather168_out1 as i64];
        let unsqueeze169_out1 = [gather169_out1 as i64];
        let constant873_out1: [i64; 1] = [64i64];
        let constant872_out1: [i64; 1] = [16i64];
        let concat83_out1: [i64; 4usize] = [
            &unsqueeze168_out1[..],
            &unsqueeze169_out1[..],
            &constant872_out1[..],
            &constant873_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape83_out1 = linear123_out1.reshape(concat83_out1);
        let transpose81_out1 = reshape83_out1.permute([0, 2, 1, 3]);
        let shape168_out1: [i64; 3] = {
            let axes = &linear121_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather170_out1 = shape168_out1[0] as i64;
        let gather171_out1 = shape168_out1[1] as i64;
        let unsqueeze170_out1 = [gather170_out1 as i64];
        let unsqueeze171_out1 = [gather171_out1 as i64];
        let constant877_out1: [i64; 1] = [64i64];
        let constant876_out1: [i64; 1] = [16i64];
        let concat84_out1: [i64; 4usize] = [
            &unsqueeze170_out1[..],
            &unsqueeze171_out1[..],
            &constant876_out1[..],
            &constant877_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape84_out1 = linear121_out1.reshape(concat84_out1);
        let transpose82_out1 = reshape84_out1.permute([0, 2, 1, 3]);
        let transpose83_out1 = reshape82_out1.permute([0, 2, 3, 1]);
        let matmul164_out1 = transpose82_out1.matmul(transpose83_out1);
        let constant878_out1 = self.constant878.val();
        let div82_out1 = matmul164_out1
            .div((constant878_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add166_out1 = div82_out1.add(mul2_out1);
        let softmax21_out1 = burn::tensor::activation::softmax(add166_out1, 3);
        let matmul165_out1 = softmax21_out1.matmul(transpose81_out1);
        let transpose84_out1 = matmul165_out1.permute([0, 2, 1, 3]);
        let shape170_out1: [i64; 4] = {
            let axes = &transpose84_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather172_out1 = shape170_out1[0] as i64;
        let gather173_out1 = shape170_out1[1] as i64;
        let unsqueeze172_out1 = [gather172_out1 as i64];
        let unsqueeze173_out1 = [gather173_out1 as i64];
        let constant881_out1: [i64; 1] = [1024i64];
        let concat85_out1: [i64; 3usize] = [
            &unsqueeze172_out1[..],
            &unsqueeze173_out1[..],
            &constant881_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape85_out1 = transpose84_out1.reshape(concat85_out1);
        let linear124_out1 = self.linear124.forward(reshape85_out1);
        let add167_out1 = linear124_out1.add(add165_out1);
        add167_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule22 {
    constant882: burn::module::Param<Tensor<1>>,
    constant883: burn::module::Param<Tensor<1>>,
    constant210: burn::module::Param<Tensor<1>>,
    constant211: burn::module::Param<Tensor<1>>,
    linear125: Linear,
    constant884: burn::module::Param<Tensor<1>>,
    constant885: burn::module::Param<Tensor<1>>,
    constant886: burn::module::Param<Tensor<1>>,
    linear126: Linear,
    constant887: burn::module::Param<Tensor<1>>,
    constant888: burn::module::Param<Tensor<1>>,
    constant214: burn::module::Param<Tensor<1>>,
    constant215: burn::module::Param<Tensor<1>>,
    linear127: Linear,
    linear128: Linear,
    linear129: Linear,
    constant901: burn::module::Param<Tensor<1>>,
    linear130: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule22 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant882: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant883: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant210: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant211: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear125 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant884: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant885: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant886: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear126 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant887: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant888: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant214: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant215: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear127 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear128 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear129 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant901: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear130 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant882,
            constant883,
            constant210,
            constant211,
            linear125,
            constant884,
            constant885,
            constant886,
            linear126,
            constant887,
            constant888,
            constant214,
            constant215,
            linear127,
            linear128,
            linear129,
            constant901,
            linear130,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add167_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean83_out1 = { add167_out1.clone().mean_dim(2usize) };
        let sub43_out1 = add167_out1.sub(reducemean83_out1);
        let constant882_out1 = self.constant882.val();
        let pow42_out1 = sub43_out1
            .clone()
            .powf((constant882_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean84_out1 = { pow42_out1.mean_dim(2usize) };
        let constant883_out1 = self.constant883.val();
        let add168_out1 = reducemean84_out1
            .add((constant883_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt42_out1 = add168_out1.sqrt();
        let div83_out1 = sub43_out1.div(sqrt42_out1);
        let constant210_out1 = self.constant210.val();
        let mul85_out1 = div83_out1
            .mul((constant210_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant211_out1 = self.constant211.val();
        let add169_out1 = mul85_out1
            .add((constant211_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear125_out1 = self.linear125.forward(add169_out1.clone());
        let constant884_out1 = self.constant884.val();
        let div84_out1 = linear125_out1
            .clone()
            .div((constant884_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf21_out1 = div84_out1.erf();
        let constant885_out1 = self.constant885.val();
        let add170_out1 = erf21_out1
            .add((constant885_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul86_out1 = linear125_out1.mul(add170_out1);
        let constant886_out1 = self.constant886.val();
        let mul87_out1 = mul86_out1
            .mul((constant886_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear126_out1 = self.linear126.forward(mul87_out1);
        let add171_out1 = linear126_out1.add(add169_out1);
        let reducemean85_out1 = { add171_out1.clone().mean_dim(2usize) };
        let sub44_out1 = add171_out1.sub(reducemean85_out1);
        let constant887_out1 = self.constant887.val();
        let pow43_out1 = sub44_out1
            .clone()
            .powf((constant887_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean86_out1 = { pow43_out1.mean_dim(2usize) };
        let constant888_out1 = self.constant888.val();
        let add172_out1 = reducemean86_out1
            .add((constant888_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt43_out1 = add172_out1.sqrt();
        let div85_out1 = sub44_out1.div(sqrt43_out1);
        let constant214_out1 = self.constant214.val();
        let mul88_out1 = div85_out1
            .mul((constant214_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant215_out1 = self.constant215.val();
        let add173_out1 = mul88_out1
            .add((constant215_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear127_out1 = self.linear127.forward(add173_out1.clone());
        let linear128_out1 = self.linear128.forward(add173_out1.clone());
        let shape172_out1: [i64; 3] = {
            let axes = &linear128_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather174_out1 = shape172_out1[0] as i64;
        let gather175_out1 = shape172_out1[1] as i64;
        let unsqueeze174_out1 = [gather174_out1 as i64];
        let unsqueeze175_out1 = [gather175_out1 as i64];
        let constant892_out1: [i64; 1] = [64i64];
        let constant891_out1: [i64; 1] = [16i64];
        let concat86_out1: [i64; 4usize] = [
            &unsqueeze174_out1[..],
            &unsqueeze175_out1[..],
            &constant891_out1[..],
            &constant892_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape86_out1 = linear128_out1.reshape(concat86_out1);
        let linear129_out1 = self.linear129.forward(add173_out1.clone());
        let shape174_out1: [i64; 3] = {
            let axes = &linear129_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather176_out1 = shape174_out1[0] as i64;
        let gather177_out1 = shape174_out1[1] as i64;
        let unsqueeze176_out1 = [gather176_out1 as i64];
        let unsqueeze177_out1 = [gather177_out1 as i64];
        let constant896_out1: [i64; 1] = [64i64];
        let constant895_out1: [i64; 1] = [16i64];
        let concat87_out1: [i64; 4usize] = [
            &unsqueeze176_out1[..],
            &unsqueeze177_out1[..],
            &constant895_out1[..],
            &constant896_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape87_out1 = linear129_out1.reshape(concat87_out1);
        let transpose85_out1 = reshape87_out1.permute([0, 2, 1, 3]);
        let shape176_out1: [i64; 3] = {
            let axes = &linear127_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather178_out1 = shape176_out1[0] as i64;
        let gather179_out1 = shape176_out1[1] as i64;
        let unsqueeze178_out1 = [gather178_out1 as i64];
        let unsqueeze179_out1 = [gather179_out1 as i64];
        let constant900_out1: [i64; 1] = [64i64];
        let constant899_out1: [i64; 1] = [16i64];
        let concat88_out1: [i64; 4usize] = [
            &unsqueeze178_out1[..],
            &unsqueeze179_out1[..],
            &constant899_out1[..],
            &constant900_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape88_out1 = linear127_out1.reshape(concat88_out1);
        let transpose86_out1 = reshape88_out1.permute([0, 2, 1, 3]);
        let transpose87_out1 = reshape86_out1.permute([0, 2, 3, 1]);
        let matmul172_out1 = transpose86_out1.matmul(transpose87_out1);
        let constant901_out1 = self.constant901.val();
        let div86_out1 = matmul172_out1
            .div((constant901_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add174_out1 = div86_out1.add(mul2_out1);
        let softmax22_out1 = burn::tensor::activation::softmax(add174_out1, 3);
        let matmul173_out1 = softmax22_out1.matmul(transpose85_out1);
        let transpose88_out1 = matmul173_out1.permute([0, 2, 1, 3]);
        let shape178_out1: [i64; 4] = {
            let axes = &transpose88_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather180_out1 = shape178_out1[0] as i64;
        let gather181_out1 = shape178_out1[1] as i64;
        let unsqueeze180_out1 = [gather180_out1 as i64];
        let unsqueeze181_out1 = [gather181_out1 as i64];
        let constant904_out1: [i64; 1] = [1024i64];
        let concat89_out1: [i64; 3usize] = [
            &unsqueeze180_out1[..],
            &unsqueeze181_out1[..],
            &constant904_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape89_out1 = transpose88_out1.reshape(concat89_out1);
        let linear130_out1 = self.linear130.forward(reshape89_out1);
        let add175_out1 = linear130_out1.add(add173_out1);
        add175_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule23 {
    constant905: burn::module::Param<Tensor<1>>,
    constant906: burn::module::Param<Tensor<1>>,
    constant220: burn::module::Param<Tensor<1>>,
    constant221: burn::module::Param<Tensor<1>>,
    linear131: Linear,
    constant907: burn::module::Param<Tensor<1>>,
    constant908: burn::module::Param<Tensor<1>>,
    constant909: burn::module::Param<Tensor<1>>,
    linear132: Linear,
    constant910: burn::module::Param<Tensor<1>>,
    constant911: burn::module::Param<Tensor<1>>,
    constant224: burn::module::Param<Tensor<1>>,
    constant225: burn::module::Param<Tensor<1>>,
    linear133: Linear,
    linear134: Linear,
    linear135: Linear,
    constant924: burn::module::Param<Tensor<1>>,
    linear136: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule23 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant905: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant906: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant220: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant221: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear131 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant907: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant908: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant909: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear132 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant910: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant911: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant224: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant225: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear133 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear134 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear135 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant924: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear136 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant905,
            constant906,
            constant220,
            constant221,
            linear131,
            constant907,
            constant908,
            constant909,
            linear132,
            constant910,
            constant911,
            constant224,
            constant225,
            linear133,
            linear134,
            linear135,
            constant924,
            linear136,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add175_out1: Tensor<3>, mul2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean87_out1 = { add175_out1.clone().mean_dim(2usize) };
        let sub45_out1 = add175_out1.sub(reducemean87_out1);
        let constant905_out1 = self.constant905.val();
        let pow44_out1 = sub45_out1
            .clone()
            .powf((constant905_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean88_out1 = { pow44_out1.mean_dim(2usize) };
        let constant906_out1 = self.constant906.val();
        let add176_out1 = reducemean88_out1
            .add((constant906_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt44_out1 = add176_out1.sqrt();
        let div87_out1 = sub45_out1.div(sqrt44_out1);
        let constant220_out1 = self.constant220.val();
        let mul89_out1 = div87_out1
            .mul((constant220_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant221_out1 = self.constant221.val();
        let add177_out1 = mul89_out1
            .add((constant221_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear131_out1 = self.linear131.forward(add177_out1.clone());
        let constant907_out1 = self.constant907.val();
        let div88_out1 = linear131_out1
            .clone()
            .div((constant907_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf22_out1 = div88_out1.erf();
        let constant908_out1 = self.constant908.val();
        let add178_out1 = erf22_out1
            .add((constant908_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul90_out1 = linear131_out1.mul(add178_out1);
        let constant909_out1 = self.constant909.val();
        let mul91_out1 = mul90_out1
            .mul((constant909_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear132_out1 = self.linear132.forward(mul91_out1);
        let add179_out1 = linear132_out1.add(add177_out1);
        let reducemean89_out1 = { add179_out1.clone().mean_dim(2usize) };
        let sub46_out1 = add179_out1.sub(reducemean89_out1);
        let constant910_out1 = self.constant910.val();
        let pow45_out1 = sub46_out1
            .clone()
            .powf((constant910_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean90_out1 = { pow45_out1.mean_dim(2usize) };
        let constant911_out1 = self.constant911.val();
        let add180_out1 = reducemean90_out1
            .add((constant911_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt45_out1 = add180_out1.sqrt();
        let div89_out1 = sub46_out1.div(sqrt45_out1);
        let constant224_out1 = self.constant224.val();
        let mul92_out1 = div89_out1
            .mul((constant224_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant225_out1 = self.constant225.val();
        let add181_out1 = mul92_out1
            .add((constant225_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear133_out1 = self.linear133.forward(add181_out1.clone());
        let linear134_out1 = self.linear134.forward(add181_out1.clone());
        let shape180_out1: [i64; 3] = {
            let axes = &linear134_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather182_out1 = shape180_out1[0] as i64;
        let gather183_out1 = shape180_out1[1] as i64;
        let unsqueeze182_out1 = [gather182_out1 as i64];
        let unsqueeze183_out1 = [gather183_out1 as i64];
        let constant915_out1: [i64; 1] = [64i64];
        let constant914_out1: [i64; 1] = [16i64];
        let concat90_out1: [i64; 4usize] = [
            &unsqueeze182_out1[..],
            &unsqueeze183_out1[..],
            &constant914_out1[..],
            &constant915_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape90_out1 = linear134_out1.reshape(concat90_out1);
        let linear135_out1 = self.linear135.forward(add181_out1.clone());
        let shape182_out1: [i64; 3] = {
            let axes = &linear135_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather184_out1 = shape182_out1[0] as i64;
        let gather185_out1 = shape182_out1[1] as i64;
        let unsqueeze184_out1 = [gather184_out1 as i64];
        let unsqueeze185_out1 = [gather185_out1 as i64];
        let constant919_out1: [i64; 1] = [64i64];
        let constant918_out1: [i64; 1] = [16i64];
        let concat91_out1: [i64; 4usize] = [
            &unsqueeze184_out1[..],
            &unsqueeze185_out1[..],
            &constant918_out1[..],
            &constant919_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape91_out1 = linear135_out1.reshape(concat91_out1);
        let transpose89_out1 = reshape91_out1.permute([0, 2, 1, 3]);
        let shape184_out1: [i64; 3] = {
            let axes = &linear133_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather186_out1 = shape184_out1[0] as i64;
        let gather187_out1 = shape184_out1[1] as i64;
        let unsqueeze186_out1 = [gather186_out1 as i64];
        let unsqueeze187_out1 = [gather187_out1 as i64];
        let constant923_out1: [i64; 1] = [64i64];
        let constant922_out1: [i64; 1] = [16i64];
        let concat92_out1: [i64; 4usize] = [
            &unsqueeze186_out1[..],
            &unsqueeze187_out1[..],
            &constant922_out1[..],
            &constant923_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape92_out1 = linear133_out1.reshape(concat92_out1);
        let transpose90_out1 = reshape92_out1.permute([0, 2, 1, 3]);
        let transpose91_out1 = reshape90_out1.permute([0, 2, 3, 1]);
        let matmul180_out1 = transpose90_out1.matmul(transpose91_out1);
        let constant924_out1 = self.constant924.val();
        let div90_out1 = matmul180_out1
            .div((constant924_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add182_out1 = div90_out1.add(mul2_out1);
        let softmax23_out1 = burn::tensor::activation::softmax(add182_out1, 3);
        let matmul181_out1 = softmax23_out1.matmul(transpose89_out1);
        let transpose92_out1 = matmul181_out1.permute([0, 2, 1, 3]);
        let shape186_out1: [i64; 4] = {
            let axes = &transpose92_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather188_out1 = shape186_out1[0] as i64;
        let gather189_out1 = shape186_out1[1] as i64;
        let unsqueeze188_out1 = [gather188_out1 as i64];
        let unsqueeze189_out1 = [gather189_out1 as i64];
        let constant927_out1: [i64; 1] = [1024i64];
        let concat93_out1: [i64; 3usize] = [
            &unsqueeze188_out1[..],
            &unsqueeze189_out1[..],
            &constant927_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape93_out1 = transpose92_out1.reshape(concat93_out1);
        let linear136_out1 = self.linear136.forward(reshape93_out1);
        let add183_out1 = linear136_out1.add(add181_out1);
        add183_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule24 {
    constant928: burn::module::Param<Tensor<1>>,
    constant929: burn::module::Param<Tensor<1>>,
    constant230: burn::module::Param<Tensor<1>>,
    constant231: burn::module::Param<Tensor<1>>,
    linear137: Linear,
    constant930: burn::module::Param<Tensor<1>>,
    constant931: burn::module::Param<Tensor<1>>,
    constant932: burn::module::Param<Tensor<1>>,
    linear138: Linear,
    constant933: burn::module::Param<Tensor<1>>,
    constant934: burn::module::Param<Tensor<1>>,
    constant234: burn::module::Param<Tensor<1>>,
    constant235: burn::module::Param<Tensor<1>>,
    linear139: Linear,
    linear140: Linear,
    linear141: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule24 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant928: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant929: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant230: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant231: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear137 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant930: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant931: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant932: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear138 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant933: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant934: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant234: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant235: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear139 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear140 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let linear141 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant928,
            constant929,
            constant230,
            constant231,
            linear137,
            constant930,
            constant931,
            constant932,
            linear138,
            constant933,
            constant934,
            constant234,
            constant235,
            linear139,
            linear140,
            linear141,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add183_out1: Tensor<3>) -> (Tensor<4>, Tensor<4>, Tensor<3>) {
        let reducemean91_out1 = { add183_out1.clone().mean_dim(2usize) };
        let sub47_out1 = add183_out1.sub(reducemean91_out1);
        let constant928_out1 = self.constant928.val();
        let pow46_out1 = sub47_out1
            .clone()
            .powf((constant928_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean92_out1 = { pow46_out1.mean_dim(2usize) };
        let constant929_out1 = self.constant929.val();
        let add184_out1 = reducemean92_out1
            .add((constant929_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt46_out1 = add184_out1.sqrt();
        let div91_out1 = sub47_out1.div(sqrt46_out1);
        let constant230_out1 = self.constant230.val();
        let mul93_out1 = div91_out1
            .mul((constant230_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant231_out1 = self.constant231.val();
        let add185_out1 = mul93_out1
            .add((constant231_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear137_out1 = self.linear137.forward(add185_out1.clone());
        let constant930_out1 = self.constant930.val();
        let div92_out1 = linear137_out1
            .clone()
            .div((constant930_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf23_out1 = div92_out1.erf();
        let constant931_out1 = self.constant931.val();
        let add186_out1 = erf23_out1
            .add((constant931_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul94_out1 = linear137_out1.mul(add186_out1);
        let constant932_out1 = self.constant932.val();
        let mul95_out1 = mul94_out1
            .mul((constant932_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear138_out1 = self.linear138.forward(mul95_out1);
        let add187_out1 = linear138_out1.add(add185_out1);
        let reducemean93_out1 = { add187_out1.clone().mean_dim(2usize) };
        let sub48_out1 = add187_out1.sub(reducemean93_out1);
        let constant933_out1 = self.constant933.val();
        let pow47_out1 = sub48_out1
            .clone()
            .powf((constant933_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean94_out1 = { pow47_out1.mean_dim(2usize) };
        let constant934_out1 = self.constant934.val();
        let add188_out1 = reducemean94_out1
            .add((constant934_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt47_out1 = add188_out1.sqrt();
        let div93_out1 = sub48_out1.div(sqrt47_out1);
        let constant234_out1 = self.constant234.val();
        let mul96_out1 = div93_out1
            .mul((constant234_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant235_out1 = self.constant235.val();
        let add189_out1 = mul96_out1
            .add((constant235_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear139_out1 = self.linear139.forward(add189_out1.clone());
        let linear140_out1 = self.linear140.forward(add189_out1.clone());
        let shape188_out1: [i64; 3] = {
            let axes = &linear140_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather190_out1 = shape188_out1[0] as i64;
        let gather191_out1 = shape188_out1[1] as i64;
        let unsqueeze190_out1 = [gather190_out1 as i64];
        let unsqueeze191_out1 = [gather191_out1 as i64];
        let constant938_out1: [i64; 1] = [64i64];
        let constant937_out1: [i64; 1] = [16i64];
        let concat94_out1: [i64; 4usize] = [
            &unsqueeze190_out1[..],
            &unsqueeze191_out1[..],
            &constant937_out1[..],
            &constant938_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape94_out1 = linear140_out1.reshape(concat94_out1);
        let linear141_out1 = self.linear141.forward(add189_out1.clone());
        let shape190_out1: [i64; 3] = {
            let axes = &linear141_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather192_out1 = shape190_out1[0] as i64;
        let gather193_out1 = shape190_out1[1] as i64;
        let unsqueeze192_out1 = [gather192_out1 as i64];
        let unsqueeze193_out1 = [gather193_out1 as i64];
        let constant942_out1: [i64; 1] = [64i64];
        let constant941_out1: [i64; 1] = [16i64];
        let concat95_out1: [i64; 4usize] = [
            &unsqueeze192_out1[..],
            &unsqueeze193_out1[..],
            &constant941_out1[..],
            &constant942_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape95_out1 = linear141_out1.reshape(concat95_out1);
        let transpose93_out1 = reshape95_out1.permute([0, 2, 1, 3]);
        let shape192_out1: [i64; 3] = {
            let axes = &linear139_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather194_out1 = shape192_out1[0] as i64;
        let gather195_out1 = shape192_out1[1] as i64;
        let unsqueeze194_out1 = [gather194_out1 as i64];
        let unsqueeze195_out1 = [gather195_out1 as i64];
        let constant946_out1: [i64; 1] = [64i64];
        let constant945_out1: [i64; 1] = [16i64];
        let concat96_out1: [i64; 4usize] = [
            &unsqueeze194_out1[..],
            &unsqueeze195_out1[..],
            &constant945_out1[..],
            &constant946_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape96_out1 = linear139_out1.reshape(concat96_out1);
        let transpose94_out1 = reshape96_out1.permute([0, 2, 1, 3]);
        let transpose95_out1 = reshape94_out1.permute([0, 2, 3, 1]);
        let matmul188_out1 = transpose94_out1.matmul(transpose95_out1);
        (matmul188_out1, transpose93_out1, add189_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule25 {
    constant947: burn::module::Param<Tensor<1>>,
    linear142: Linear,
    constant951: burn::module::Param<Tensor<1>>,
    constant952: burn::module::Param<Tensor<1>>,
    constant240: burn::module::Param<Tensor<1>>,
    constant241: burn::module::Param<Tensor<1>>,
    linear143: Linear,
    constant953: burn::module::Param<Tensor<1>>,
    constant954: burn::module::Param<Tensor<1>>,
    constant955: burn::module::Param<Tensor<1>>,
    linear144: Linear,
    constant956: burn::module::Param<Tensor<1>>,
    constant957: burn::module::Param<Tensor<1>>,
    constant244: burn::module::Param<Tensor<1>>,
    constant245: burn::module::Param<Tensor<1>>,
    constant958: burn::module::Param<Tensor<1>>,
    constant959: burn::module::Param<Tensor<1>>,
    #[module(skip)]
    device: Device,
}
impl Submodule25 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant947: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([8f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear142 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant951: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant952: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant240: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant241: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear143 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant953: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1.4142135381698608f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant954: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant955: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let linear144 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant956: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant957: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.000009999999747378752f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant244: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant245: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant958: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([2f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant959: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.5f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        Self {
            constant947,
            linear142,
            constant951,
            constant952,
            constant240,
            constant241,
            linear143,
            constant953,
            constant954,
            constant955,
            linear144,
            constant956,
            constant957,
            constant244,
            constant245,
            constant958,
            constant959,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        matmul188_out1: Tensor<4>,
        mul2_out1: Tensor<4>,
        transpose93_out1: Tensor<4>,
        add189_out1: Tensor<3>,
    ) -> (Tensor<3>, Tensor<2>) {
        let constant947_out1 = self.constant947.val();
        let div94_out1 = matmul188_out1
            .div((constant947_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let add190_out1 = div94_out1.add(mul2_out1);
        let softmax24_out1 = burn::tensor::activation::softmax(add190_out1, 3);
        let matmul189_out1 = softmax24_out1.matmul(transpose93_out1);
        let transpose96_out1 = matmul189_out1.permute([0, 2, 1, 3]);
        let shape194_out1: [i64; 4] = {
            let axes = &transpose96_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather196_out1 = shape194_out1[0] as i64;
        let gather197_out1 = shape194_out1[1] as i64;
        let unsqueeze196_out1 = [gather196_out1 as i64];
        let unsqueeze197_out1 = [gather197_out1 as i64];
        let constant950_out1: [i64; 1] = [1024i64];
        let concat97_out1: [i64; 3usize] = [
            &unsqueeze196_out1[..],
            &unsqueeze197_out1[..],
            &constant950_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape97_out1 = transpose96_out1.reshape(concat97_out1);
        let linear142_out1 = self.linear142.forward(reshape97_out1);
        let add191_out1 = linear142_out1.add(add189_out1);
        let reducemean95_out1 = { add191_out1.clone().mean_dim(2usize) };
        let sub49_out1 = add191_out1.sub(reducemean95_out1);
        let constant951_out1 = self.constant951.val();
        let pow48_out1 = sub49_out1
            .clone()
            .powf((constant951_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean96_out1 = { pow48_out1.mean_dim(2usize) };
        let constant952_out1 = self.constant952.val();
        let add192_out1 = reducemean96_out1
            .add((constant952_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt48_out1 = add192_out1.sqrt();
        let div95_out1 = sub49_out1.div(sqrt48_out1);
        let constant240_out1 = self.constant240.val();
        let mul97_out1 = div95_out1
            .mul((constant240_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant241_out1 = self.constant241.val();
        let add193_out1 = mul97_out1
            .add((constant241_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear143_out1 = self.linear143.forward(add193_out1.clone());
        let constant953_out1 = self.constant953.val();
        let div96_out1 = linear143_out1
            .clone()
            .div((constant953_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf24_out1 = div96_out1.erf();
        let constant954_out1 = self.constant954.val();
        let add194_out1 = erf24_out1
            .add((constant954_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul98_out1 = linear143_out1.mul(add194_out1);
        let constant955_out1 = self.constant955.val();
        let mul99_out1 = mul98_out1
            .mul((constant955_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear144_out1 = self.linear144.forward(mul99_out1);
        let add195_out1 = linear144_out1.add(add193_out1);
        let reducemean97_out1 = { add195_out1.clone().mean_dim(2usize) };
        let sub50_out1 = add195_out1.sub(reducemean97_out1);
        let constant956_out1 = self.constant956.val();
        let pow49_out1 = sub50_out1
            .clone()
            .powf((constant956_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean98_out1 = { pow49_out1.mean_dim(2usize) };
        let constant957_out1 = self.constant957.val();
        let add196_out1 = reducemean98_out1
            .add((constant957_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt49_out1 = add196_out1.sqrt();
        let div97_out1 = sub50_out1.div(sqrt49_out1);
        let constant244_out1 = self.constant244.val();
        let mul100_out1 = div97_out1
            .mul((constant244_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant245_out1 = self.constant245.val();
        let add197_out1 = mul100_out1
            .add((constant245_out1).unsqueeze_dims(&[0isize, 1isize]));
        let gather198_out1 = {
            let sliced = add197_out1.clone().slice(s![.., 0, ..]);
            sliced.squeeze_dim::<2usize>(1)
        };
        let abs1_out1 = gather198_out1.clone().abs();
        let constant958_out1 = self.constant958.val();
        let pow50_out1 = abs1_out1.powf((constant958_out1).unsqueeze_dims(&[0isize]));
        let reducesum1_out1 = { pow50_out1.sum_dim(1usize) };
        let constant959_out1 = self.constant959.val();
        let pow51_out1 = reducesum1_out1
            .powf((constant959_out1).unsqueeze_dims(&[0isize]));
        let clip1_out1 = {
            let __clip_min = 0.0000000000009999999960041972f64;
            pow51_out1.clamp_min(__clip_min)
        };
        let shape196_out1: [i64; 2] = {
            let axes = &gather198_out1.clone().dims()[0..2];
            let mut output = [0i64; 2];
            for i in 0..2 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let expand2_out1 = {
            let onnx_shape: [i64; 2usize] = shape196_out1;
            let input_dims = clip1_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..2usize {
                let dim_offset = 2usize - 2usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            clip1_out1.expand(shape)
        };
        let div98_out1 = gather198_out1.div(expand2_out1);
        (add197_out1, div98_out1)
    }
}

#[derive(Module, Debug)]
pub struct Model {
    submodule1: Submodule1,
    submodule2: Submodule2,
    submodule3: Submodule3,
    submodule4: Submodule4,
    submodule5: Submodule5,
    submodule6: Submodule6,
    submodule7: Submodule7,
    submodule8: Submodule8,
    submodule9: Submodule9,
    submodule10: Submodule10,
    submodule11: Submodule11,
    submodule12: Submodule12,
    submodule13: Submodule13,
    submodule14: Submodule14,
    submodule15: Submodule15,
    submodule16: Submodule16,
    submodule17: Submodule17,
    submodule18: Submodule18,
    submodule19: Submodule19,
    submodule20: Submodule20,
    submodule21: Submodule21,
    submodule22: Submodule22,
    submodule23: Submodule23,
    submodule24: Submodule24,
    submodule25: Submodule25,
    #[module(skip)]
    device: Device,
}


impl Model {
    /// Load model weights from in-memory bytes.
    ///
    /// The bytes must be the contents of a `.bpk` file.
    pub fn from_bytes(bytes: Bytes, device: &Device) -> Self {
        let mut model = Self::new(device);
        let mut store = BurnpackStore::from_bytes(Some(bytes));
        model.load_from(&mut store).expect("Failed to load burnpack bytes");
        model
    }
}

impl Model {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let submodule1 = Submodule1::new(device);
        let submodule2 = Submodule2::new(device);
        let submodule3 = Submodule3::new(device);
        let submodule4 = Submodule4::new(device);
        let submodule5 = Submodule5::new(device);
        let submodule6 = Submodule6::new(device);
        let submodule7 = Submodule7::new(device);
        let submodule8 = Submodule8::new(device);
        let submodule9 = Submodule9::new(device);
        let submodule10 = Submodule10::new(device);
        let submodule11 = Submodule11::new(device);
        let submodule12 = Submodule12::new(device);
        let submodule13 = Submodule13::new(device);
        let submodule14 = Submodule14::new(device);
        let submodule15 = Submodule15::new(device);
        let submodule16 = Submodule16::new(device);
        let submodule17 = Submodule17::new(device);
        let submodule18 = Submodule18::new(device);
        let submodule19 = Submodule19::new(device);
        let submodule20 = Submodule20::new(device);
        let submodule21 = Submodule21::new(device);
        let submodule22 = Submodule22::new(device);
        let submodule23 = Submodule23::new(device);
        let submodule24 = Submodule24::new(device);
        let submodule25 = Submodule25::new(device);
        Self {
            submodule1,
            submodule2,
            submodule3,
            submodule4,
            submodule5,
            submodule6,
            submodule7,
            submodule8,
            submodule9,
            submodule10,
            submodule11,
            submodule12,
            submodule13,
            submodule14,
            submodule15,
            submodule16,
            submodule17,
            submodule18,
            submodule19,
            submodule20,
            submodule21,
            submodule22,
            submodule23,
            submodule24,
            submodule25,
            device: device.clone(),
        }
    }

    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        input_ids: Tensor<2, Int>,
        attention_mask: Tensor<2, Int>,
    ) -> (Tensor<3>, Tensor<2>) {
        let (add7_out1, mul2_out1) = self.submodule1.forward(input_ids, attention_mask);
        let add15_out1 = self.submodule2.forward(add7_out1, mul2_out1.clone());
        let add23_out1 = self.submodule3.forward(add15_out1, mul2_out1.clone());
        let add31_out1 = self.submodule4.forward(add23_out1, mul2_out1.clone());
        let add39_out1 = self.submodule5.forward(add31_out1, mul2_out1.clone());
        let add47_out1 = self.submodule6.forward(add39_out1, mul2_out1.clone());
        let add55_out1 = self.submodule7.forward(add47_out1, mul2_out1.clone());
        let add63_out1 = self.submodule8.forward(add55_out1, mul2_out1.clone());
        let add71_out1 = self.submodule9.forward(add63_out1, mul2_out1.clone());
        let add79_out1 = self.submodule10.forward(add71_out1, mul2_out1.clone());
        let add87_out1 = self.submodule11.forward(add79_out1, mul2_out1.clone());
        let add95_out1 = self.submodule12.forward(add87_out1, mul2_out1.clone());
        let add103_out1 = self.submodule13.forward(add95_out1, mul2_out1.clone());
        let add111_out1 = self.submodule14.forward(add103_out1, mul2_out1.clone());
        let add119_out1 = self.submodule15.forward(add111_out1, mul2_out1.clone());
        let add127_out1 = self.submodule16.forward(add119_out1, mul2_out1.clone());
        let add135_out1 = self.submodule17.forward(add127_out1, mul2_out1.clone());
        let add143_out1 = self.submodule18.forward(add135_out1, mul2_out1.clone());
        let add151_out1 = self.submodule19.forward(add143_out1, mul2_out1.clone());
        let add159_out1 = self.submodule20.forward(add151_out1, mul2_out1.clone());
        let add167_out1 = self.submodule21.forward(add159_out1, mul2_out1.clone());
        let add175_out1 = self.submodule22.forward(add167_out1, mul2_out1.clone());
        let add183_out1 = self.submodule23.forward(add175_out1, mul2_out1.clone());
        let (matmul188_out1, transpose93_out1, add189_out1) = self
            .submodule24
            .forward(add183_out1);
        let (add197_out1, div98_out1) = self
            .submodule25
            .forward(matmul188_out1, mul2_out1, transpose93_out1, add189_out1);
        (add197_out1, div98_out1)
    }
}
