// rag3weaver : patch_attention.py — attention fusionnée / masque booléen / casts « float » neutres, 6 septembre 2026.
// rag3weaver : casts « float » neutres (patch_attention.py --casts-neutres, 6 septembre 2026).
// Generated from ONNX "/tmp/claude-1000/-home-lucied-git-workspaces-rag3db/13068ada-ca9b-4752-8b48-bf8f40ed08a2/scratchpad/granite/hf-278m/model.onnx" by burn-onnx
extern crate alloc;
use burn::prelude::*;
use burn::nn::Linear;
use burn::nn::LinearConfig;
use burn::nn::LinearLayout;
use burn::tensor::Bytes;
use burn_store::BurnpackStore;
use burn_store::ModuleSnapshot;


#[derive(Module, Debug)]
pub struct Submodule1 {
    constant203: burn::module::Param<Tensor<2, Int>>,
    constant211: burn::module::Param<Tensor<1, Int>>,
    constant214: burn::module::Param<Tensor<1, Int>>,
    constant1: burn::module::Param<Tensor<2>>,
    constant3: burn::module::Param<Tensor<2>>,
    constant2: burn::module::Param<Tensor<2>>,
    constant215: burn::module::Param<Tensor<1>>,
    constant216: burn::module::Param<Tensor<1>>,
    constant4: burn::module::Param<Tensor<1>>,
    constant5: burn::module::Param<Tensor<1>>,
    constant226: burn::module::Param<Tensor<1, Int>>,
    #[module(skip)]
    device: Device,
}
impl Submodule1 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant203: burn::module::Param<Tensor<2, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
                Int,
            >::zeros([1, 514], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [1, 514].into(),
        );
        let constant211: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
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
        let constant214: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
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
            >::zeros([250002, 768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [250002, 768].into(),
        );
        let constant3: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([1, 768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 768].into(),
        );
        let constant2: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([514, 768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [514, 768].into(),
        );
        let constant215: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant216: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant5: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant226: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
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
        Self {
            constant203,
            constant211,
            constant214,
            constant1,
            constant3,
            constant2,
            constant215,
            constant216,
            constant4,
            constant5,
            constant226,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        input_ids: Tensor<2, Int>,
        attention_mask: Tensor<2, Int>,
    ) -> (Tensor<4, Int>, Tensor<3>) {
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
        let constant203_out1 = self.constant203.val();
        let slice1_out1 = constant203_out1.slice(s![.., 0..unsqueeze1_out1[0]]);
        let unsqueeze2_out1 = [gather1_out1 as i64];
        let unsqueeze3_out1 = [gather2_out1 as i64];
        let concat1_out1: [i64; 2usize] = [&unsqueeze2_out1[..], &unsqueeze3_out1[..]]
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
        let constant211_out1 = self.constant211.val();
        let mul1_out1 = constantofshape1_out1.clone().mul(constant211_out1);
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
        let constant212_out1 = 1i64;
        let equal2_out1 = input_ids.clone().equal_elem(constant212_out1);
        let not1_out1 = equal2_out1.bool_not();
        let cast1_out1 = not1_out1.int().cast(burn::tensor::DType::I32);
        let cumsum1_out1 = cast1_out1.clone().cumsum(1);
        let mul2_out1 = cumsum1_out1.mul(cast1_out1);
        let cast2_out1 = mul2_out1.cast(burn::tensor::DType::I64);
        let constant214_out1 = self.constant214.val();
        let add1_out1 = cast2_out1.add((constant214_out1).unsqueeze_dims(&[0isize]));
        let constant1_out1 = self.constant1.val();
        let gather3_out1 = constant1_out1.take::<2, 3>(0, input_ids);
        let constant3_out1 = self.constant3.val();
        let gather4_out1 = constant3_out1.take::<2, 3>(0, expand1_out1);
        let add2_out1 = gather3_out1.add(gather4_out1);
        let constant2_out1 = self.constant2.val();
        let gather5_out1 = constant2_out1.take::<2, 3>(0, add1_out1);
        let add3_out1 = add2_out1.add(gather5_out1);
        let reducemean1_out1 = { add3_out1.clone().mean_dim(2usize) };
        let sub1_out1 = add3_out1.sub(reducemean1_out1);
        let constant215_out1 = self.constant215.val();
        let pow1_out1 = sub1_out1
            .clone()
            .powf((constant215_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean2_out1 = { pow1_out1.mean_dim(2usize) };
        let constant216_out1 = self.constant216.val();
        let add4_out1 = reducemean2_out1
            .add((constant216_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt1_out1 = add4_out1.sqrt();
        let div1_out1 = sub1_out1.div(sqrt1_out1);
        let constant4_out1 = self.constant4.val();
        let mul3_out1 = div1_out1
            .mul((constant4_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant5_out1 = self.constant5.val();
        let add5_out1 = mul3_out1
            .add((constant5_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape4_out1: [i64; 2] = {
            let axes = &attention_mask.clone().dims()[0..2];
            let mut output = [0i64; 2];
            for i in 0..2 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather6_out1 = shape4_out1[0] as i64;
        let gather7_out1 = shape4_out1[1] as i64;
        let unsqueeze4_out1: Tensor<3, Int> = attention_mask.unsqueeze_dims::<3>(&[1]);
        let unsqueeze5_out1: Tensor<4, Int> = unsqueeze4_out1.unsqueeze_dims::<4>(&[2]);
        let unsqueeze6_out1 = [gather6_out1 as i64];
        let unsqueeze7_out1 = [gather2_out1 as i64];
        let unsqueeze8_out1 = [gather7_out1 as i64];
        let constant222_out1: [i64; 1] = [1i64];
        let concat2_out1: [i64; 4usize] = [
            &unsqueeze6_out1[..],
            &constant222_out1[..],
            &unsqueeze7_out1[..],
            &unsqueeze8_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape2_out1 = concat2_out1;
        let shape6_out1: [i64; 1] = [4i64];
        let constantofshape2_out1 = Tensor::<
            1,
            Int,
        >::from_data(
                burn::tensor::TensorData::from([1i64 as i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .reshape([1])
            .expand(shape6_out1);
        let constant226_out1 = self.constant226.val();
        let mul4_out1 = constantofshape2_out1.clone().mul(constant226_out1);
        let equal3_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(reshape2_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(mul4_out1)
        };
        let where2_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&reshape2_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal3_out1, constantofshape2_out1);
        let expand2_out1 = {
            let onnx_shape: [i64; 4usize] = TryInto::<
                [i64; 4usize],
            >::try_into(where2_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze5_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..4usize {
                let dim_offset = 4usize - 4usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze5_out1.expand(shape)
        };
        (expand2_out1, add5_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule2 {
    constant227: burn::module::Param<Tensor<1>>,
    linear1: Linear,
    linear2: Linear,
    linear3: Linear,
    linear4: Linear,
    constant249: burn::module::Param<Tensor<1>>,
    constant250: burn::module::Param<Tensor<1>>,
    constant10: burn::module::Param<Tensor<1>>,
    constant11: burn::module::Param<Tensor<1>>,
    linear5: Linear,
    constant251: burn::module::Param<Tensor<1>>,
    constant252: burn::module::Param<Tensor<1>>,
    constant253: burn::module::Param<Tensor<1>>,
    linear6: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule2 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant227: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear1 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear2 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear3 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear4 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant249: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant250: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant11: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear5 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant251: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant252: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant253: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear6 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant227,
            linear1,
            linear2,
            linear3,
            linear4,
            constant249,
            constant250,
            constant10,
            constant11,
            linear5,
            constant251,
            constant252,
            constant253,
            linear6,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        expand2_out1: Tensor<4, Int>,
        add5_out1: Tensor<3>,
    ) -> (Tensor<3>, Tensor<4>) {
        let cast3_out1 = expand2_out1.float();
        let constant227_out1 = self.constant227.val();
        let sub2_out1 = (constant227_out1)
            .unsqueeze_dims(&[0isize, 1isize, 2isize])
            .sub(cast3_out1);
        let cast4_out1 = sub2_out1.clone().bool();
        let constant228_out1 = -340282350000000000000000000000000000000f32;
        let where3_out1 = sub2_out1.mask_fill(cast4_out1, constant228_out1);
        let shape7_out1: [i64; 3] = {
            let axes = &add5_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather8_out1 = shape7_out1[0] as i64;
        let gather9_out1 = shape7_out1[1] as i64;
        let linear1_out1 = self.linear1.forward(add5_out1.clone());
        let unsqueeze9_out1 = [gather8_out1 as i64];
        let constant234_out1: [i64; 1] = [64i64];
        let constant232_out1: [i64; 1] = [-1i64];
        let constant233_out1: [i64; 1] = [12i64];
        let concat3_out1: [i64; 4usize] = [
            &unsqueeze9_out1[..],
            &constant232_out1[..],
            &constant233_out1[..],
            &constant234_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze10_out1 = [gather8_out1 as i64];
        let constant238_out1: [i64; 1] = [64i64];
        let constant236_out1: [i64; 1] = [-1i64];
        let constant237_out1: [i64; 1] = [12i64];
        let concat4_out1: [i64; 4usize] = [
            &unsqueeze10_out1[..],
            &constant236_out1[..],
            &constant237_out1[..],
            &constant238_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze11_out1 = [gather8_out1 as i64];
        let constant242_out1: [i64; 1] = [64i64];
        let constant240_out1: [i64; 1] = [-1i64];
        let constant241_out1: [i64; 1] = [12i64];
        let concat5_out1: [i64; 4usize] = [
            &unsqueeze11_out1[..],
            &constant240_out1[..],
            &constant241_out1[..],
            &constant242_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape3_out1 = linear1_out1.reshape(concat3_out1);
        let transpose1_out1 = reshape3_out1.permute([0, 2, 1, 3]);
        let linear2_out1 = self.linear2.forward(add5_out1.clone());
        let reshape4_out1 = linear2_out1.reshape(concat4_out1);
        let linear3_out1 = self.linear3.forward(add5_out1.clone());
        let reshape5_out1 = linear3_out1.reshape(concat5_out1);
        let transpose2_out1 = reshape5_out1.permute([0, 2, 1, 3]);
        let transpose3_out1 = reshape4_out1.permute([0, 2, 3, 1]);
        let matmul4_k_corrected = transpose3_out1.permute([0, 1, 3, 2]);
        let (matmul5_out1,) = {
            let q = transpose1_out1;
            let k = matmul4_k_corrected;
            let v = transpose2_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul5_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul5_out1,)
        };
        let transpose4_out1 = matmul5_out1.permute([0, 2, 1, 3]);
        let unsqueeze12_out1 = [gather8_out1 as i64];
        let unsqueeze13_out1 = [gather9_out1 as i64];
        let constant248_out1: [i64; 1] = [768i64];
        let concat6_out1: [i64; 3usize] = [
            &unsqueeze12_out1[..],
            &unsqueeze13_out1[..],
            &constant248_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape6_out1 = transpose4_out1.reshape(concat6_out1);
        let linear4_out1 = self.linear4.forward(reshape6_out1);
        let add7_out1 = linear4_out1.add(add5_out1);
        let reducemean3_out1 = { add7_out1.clone().mean_dim(2usize) };
        let sub3_out1 = add7_out1.sub(reducemean3_out1);
        let constant249_out1 = self.constant249.val();
        let pow2_out1 = sub3_out1
            .clone()
            .powf((constant249_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean4_out1 = { pow2_out1.mean_dim(2usize) };
        let constant250_out1 = self.constant250.val();
        let add8_out1 = reducemean4_out1
            .add((constant250_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt5_out1 = add8_out1.sqrt();
        let div3_out1 = sub3_out1.div(sqrt5_out1);
        let constant10_out1 = self.constant10.val();
        let mul7_out1 = div3_out1
            .mul((constant10_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant11_out1 = self.constant11.val();
        let add9_out1 = mul7_out1
            .add((constant11_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear5_out1 = self.linear5.forward(add9_out1.clone());
        let constant251_out1 = self.constant251.val();
        let div4_out1 = linear5_out1
            .clone()
            .div((constant251_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf1_out1 = div4_out1.erf();
        let constant252_out1 = self.constant252.val();
        let add10_out1 = erf1_out1
            .add((constant252_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul8_out1 = linear5_out1.mul(add10_out1);
        let constant253_out1 = self.constant253.val();
        let mul9_out1 = mul8_out1
            .mul((constant253_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear6_out1 = self.linear6.forward(mul9_out1);
        let add11_out1 = linear6_out1.add(add9_out1);
        (add11_out1, where3_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule3 {
    constant254: burn::module::Param<Tensor<1>>,
    constant255: burn::module::Param<Tensor<1>>,
    constant14: burn::module::Param<Tensor<1>>,
    constant15: burn::module::Param<Tensor<1>>,
    linear7: Linear,
    linear8: Linear,
    linear9: Linear,
    linear10: Linear,
    constant276: burn::module::Param<Tensor<1>>,
    constant277: burn::module::Param<Tensor<1>>,
    constant20: burn::module::Param<Tensor<1>>,
    constant21: burn::module::Param<Tensor<1>>,
    linear11: Linear,
    constant278: burn::module::Param<Tensor<1>>,
    constant279: burn::module::Param<Tensor<1>>,
    constant280: burn::module::Param<Tensor<1>>,
    linear12: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule3 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant254: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant255: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant15: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear7 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear8 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear9 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear10 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant276: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant277: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant21: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear11 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant278: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant279: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant280: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear12 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant254,
            constant255,
            constant14,
            constant15,
            linear7,
            linear8,
            linear9,
            linear10,
            constant276,
            constant277,
            constant20,
            constant21,
            linear11,
            constant278,
            constant279,
            constant280,
            linear12,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add11_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean5_out1 = { add11_out1.clone().mean_dim(2usize) };
        let sub4_out1 = add11_out1.sub(reducemean5_out1);
        let constant254_out1 = self.constant254.val();
        let pow3_out1 = sub4_out1
            .clone()
            .powf((constant254_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean6_out1 = { pow3_out1.mean_dim(2usize) };
        let constant255_out1 = self.constant255.val();
        let add12_out1 = reducemean6_out1
            .add((constant255_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt6_out1 = add12_out1.sqrt();
        let div5_out1 = sub4_out1.div(sqrt6_out1);
        let constant14_out1 = self.constant14.val();
        let mul10_out1 = div5_out1
            .mul((constant14_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant15_out1 = self.constant15.val();
        let add13_out1 = mul10_out1
            .add((constant15_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape10_out1: [i64; 3] = {
            let axes = &add13_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather10_out1 = shape10_out1[0] as i64;
        let gather11_out1 = shape10_out1[1] as i64;
        let linear7_out1 = self.linear7.forward(add13_out1.clone());
        let unsqueeze14_out1 = [gather10_out1 as i64];
        let constant261_out1: [i64; 1] = [64i64];
        let constant259_out1: [i64; 1] = [-1i64];
        let constant260_out1: [i64; 1] = [12i64];
        let concat7_out1: [i64; 4usize] = [
            &unsqueeze14_out1[..],
            &constant259_out1[..],
            &constant260_out1[..],
            &constant261_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze15_out1 = [gather10_out1 as i64];
        let constant265_out1: [i64; 1] = [64i64];
        let constant263_out1: [i64; 1] = [-1i64];
        let constant264_out1: [i64; 1] = [12i64];
        let concat8_out1: [i64; 4usize] = [
            &unsqueeze15_out1[..],
            &constant263_out1[..],
            &constant264_out1[..],
            &constant265_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze16_out1 = [gather10_out1 as i64];
        let constant269_out1: [i64; 1] = [64i64];
        let constant267_out1: [i64; 1] = [-1i64];
        let constant268_out1: [i64; 1] = [12i64];
        let concat9_out1: [i64; 4usize] = [
            &unsqueeze16_out1[..],
            &constant267_out1[..],
            &constant268_out1[..],
            &constant269_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape7_out1 = linear7_out1.reshape(concat7_out1);
        let transpose5_out1 = reshape7_out1.permute([0, 2, 1, 3]);
        let linear8_out1 = self.linear8.forward(add13_out1.clone());
        let reshape8_out1 = linear8_out1.reshape(concat8_out1);
        let linear9_out1 = self.linear9.forward(add13_out1.clone());
        let reshape9_out1 = linear9_out1.reshape(concat9_out1);
        let transpose6_out1 = reshape9_out1.permute([0, 2, 1, 3]);
        let transpose7_out1 = reshape8_out1.permute([0, 2, 3, 1]);
        let matmul12_k_corrected = transpose7_out1.permute([0, 1, 3, 2]);
        let (matmul13_out1,) = {
            let q = transpose5_out1;
            let k = matmul12_k_corrected;
            let v = transpose6_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul13_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul13_out1,)
        };
        let transpose8_out1 = matmul13_out1.permute([0, 2, 1, 3]);
        let unsqueeze17_out1 = [gather10_out1 as i64];
        let unsqueeze18_out1 = [gather11_out1 as i64];
        let constant275_out1: [i64; 1] = [768i64];
        let concat10_out1: [i64; 3usize] = [
            &unsqueeze17_out1[..],
            &unsqueeze18_out1[..],
            &constant275_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape10_out1 = transpose8_out1.reshape(concat10_out1);
        let linear10_out1 = self.linear10.forward(reshape10_out1);
        let add15_out1 = linear10_out1.add(add13_out1);
        let reducemean7_out1 = { add15_out1.clone().mean_dim(2usize) };
        let sub5_out1 = add15_out1.sub(reducemean7_out1);
        let constant276_out1 = self.constant276.val();
        let pow4_out1 = sub5_out1
            .clone()
            .powf((constant276_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean8_out1 = { pow4_out1.mean_dim(2usize) };
        let constant277_out1 = self.constant277.val();
        let add16_out1 = reducemean8_out1
            .add((constant277_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt10_out1 = add16_out1.sqrt();
        let div7_out1 = sub5_out1.div(sqrt10_out1);
        let constant20_out1 = self.constant20.val();
        let mul13_out1 = div7_out1
            .mul((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant21_out1 = self.constant21.val();
        let add17_out1 = mul13_out1
            .add((constant21_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear11_out1 = self.linear11.forward(add17_out1.clone());
        let constant278_out1 = self.constant278.val();
        let div8_out1 = linear11_out1
            .clone()
            .div((constant278_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf2_out1 = div8_out1.erf();
        let constant279_out1 = self.constant279.val();
        let add18_out1 = erf2_out1
            .add((constant279_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul14_out1 = linear11_out1.mul(add18_out1);
        let constant280_out1 = self.constant280.val();
        let mul15_out1 = mul14_out1
            .mul((constant280_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear12_out1 = self.linear12.forward(mul15_out1);
        let add19_out1 = linear12_out1.add(add17_out1);
        add19_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule4 {
    constant281: burn::module::Param<Tensor<1>>,
    constant282: burn::module::Param<Tensor<1>>,
    constant24: burn::module::Param<Tensor<1>>,
    constant25: burn::module::Param<Tensor<1>>,
    linear13: Linear,
    linear14: Linear,
    linear15: Linear,
    linear16: Linear,
    constant303: burn::module::Param<Tensor<1>>,
    constant304: burn::module::Param<Tensor<1>>,
    constant30: burn::module::Param<Tensor<1>>,
    constant31: burn::module::Param<Tensor<1>>,
    linear17: Linear,
    constant305: burn::module::Param<Tensor<1>>,
    constant306: burn::module::Param<Tensor<1>>,
    constant307: burn::module::Param<Tensor<1>>,
    linear18: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule4 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant281: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant282: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant25: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear13 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear14 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear15 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear16 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant303: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant304: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant31: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear17 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant305: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant306: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant307: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear18 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant281,
            constant282,
            constant24,
            constant25,
            linear13,
            linear14,
            linear15,
            linear16,
            constant303,
            constant304,
            constant30,
            constant31,
            linear17,
            constant305,
            constant306,
            constant307,
            linear18,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add19_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean9_out1 = { add19_out1.clone().mean_dim(2usize) };
        let sub6_out1 = add19_out1.sub(reducemean9_out1);
        let constant281_out1 = self.constant281.val();
        let pow5_out1 = sub6_out1
            .clone()
            .powf((constant281_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean10_out1 = { pow5_out1.mean_dim(2usize) };
        let constant282_out1 = self.constant282.val();
        let add20_out1 = reducemean10_out1
            .add((constant282_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt11_out1 = add20_out1.sqrt();
        let div9_out1 = sub6_out1.div(sqrt11_out1);
        let constant24_out1 = self.constant24.val();
        let mul16_out1 = div9_out1
            .mul((constant24_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant25_out1 = self.constant25.val();
        let add21_out1 = mul16_out1
            .add((constant25_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape13_out1: [i64; 3] = {
            let axes = &add21_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather12_out1 = shape13_out1[0] as i64;
        let gather13_out1 = shape13_out1[1] as i64;
        let linear13_out1 = self.linear13.forward(add21_out1.clone());
        let unsqueeze19_out1 = [gather12_out1 as i64];
        let constant288_out1: [i64; 1] = [64i64];
        let constant286_out1: [i64; 1] = [-1i64];
        let constant287_out1: [i64; 1] = [12i64];
        let concat11_out1: [i64; 4usize] = [
            &unsqueeze19_out1[..],
            &constant286_out1[..],
            &constant287_out1[..],
            &constant288_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze20_out1 = [gather12_out1 as i64];
        let constant292_out1: [i64; 1] = [64i64];
        let constant290_out1: [i64; 1] = [-1i64];
        let constant291_out1: [i64; 1] = [12i64];
        let concat12_out1: [i64; 4usize] = [
            &unsqueeze20_out1[..],
            &constant290_out1[..],
            &constant291_out1[..],
            &constant292_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze21_out1 = [gather12_out1 as i64];
        let constant296_out1: [i64; 1] = [64i64];
        let constant294_out1: [i64; 1] = [-1i64];
        let constant295_out1: [i64; 1] = [12i64];
        let concat13_out1: [i64; 4usize] = [
            &unsqueeze21_out1[..],
            &constant294_out1[..],
            &constant295_out1[..],
            &constant296_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape11_out1 = linear13_out1.reshape(concat11_out1);
        let transpose9_out1 = reshape11_out1.permute([0, 2, 1, 3]);
        let linear14_out1 = self.linear14.forward(add21_out1.clone());
        let reshape12_out1 = linear14_out1.reshape(concat12_out1);
        let linear15_out1 = self.linear15.forward(add21_out1.clone());
        let reshape13_out1 = linear15_out1.reshape(concat13_out1);
        let transpose10_out1 = reshape13_out1.permute([0, 2, 1, 3]);
        let transpose11_out1 = reshape12_out1.permute([0, 2, 3, 1]);
        let matmul20_k_corrected = transpose11_out1.permute([0, 1, 3, 2]);
        let (matmul21_out1,) = {
            let q = transpose9_out1;
            let k = matmul20_k_corrected;
            let v = transpose10_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul21_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul21_out1,)
        };
        let transpose12_out1 = matmul21_out1.permute([0, 2, 1, 3]);
        let unsqueeze22_out1 = [gather12_out1 as i64];
        let unsqueeze23_out1 = [gather13_out1 as i64];
        let constant302_out1: [i64; 1] = [768i64];
        let concat14_out1: [i64; 3usize] = [
            &unsqueeze22_out1[..],
            &unsqueeze23_out1[..],
            &constant302_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape14_out1 = transpose12_out1.reshape(concat14_out1);
        let linear16_out1 = self.linear16.forward(reshape14_out1);
        let add23_out1 = linear16_out1.add(add21_out1);
        let reducemean11_out1 = { add23_out1.clone().mean_dim(2usize) };
        let sub7_out1 = add23_out1.sub(reducemean11_out1);
        let constant303_out1 = self.constant303.val();
        let pow6_out1 = sub7_out1
            .clone()
            .powf((constant303_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean12_out1 = { pow6_out1.mean_dim(2usize) };
        let constant304_out1 = self.constant304.val();
        let add24_out1 = reducemean12_out1
            .add((constant304_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt15_out1 = add24_out1.sqrt();
        let div11_out1 = sub7_out1.div(sqrt15_out1);
        let constant30_out1 = self.constant30.val();
        let mul19_out1 = div11_out1
            .mul((constant30_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant31_out1 = self.constant31.val();
        let add25_out1 = mul19_out1
            .add((constant31_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear17_out1 = self.linear17.forward(add25_out1.clone());
        let constant305_out1 = self.constant305.val();
        let div12_out1 = linear17_out1
            .clone()
            .div((constant305_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf3_out1 = div12_out1.erf();
        let constant306_out1 = self.constant306.val();
        let add26_out1 = erf3_out1
            .add((constant306_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul20_out1 = linear17_out1.mul(add26_out1);
        let constant307_out1 = self.constant307.val();
        let mul21_out1 = mul20_out1
            .mul((constant307_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear18_out1 = self.linear18.forward(mul21_out1);
        let add27_out1 = linear18_out1.add(add25_out1);
        add27_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule5 {
    constant308: burn::module::Param<Tensor<1>>,
    constant309: burn::module::Param<Tensor<1>>,
    constant34: burn::module::Param<Tensor<1>>,
    constant35: burn::module::Param<Tensor<1>>,
    linear19: Linear,
    linear20: Linear,
    linear21: Linear,
    linear22: Linear,
    constant330: burn::module::Param<Tensor<1>>,
    constant331: burn::module::Param<Tensor<1>>,
    constant40: burn::module::Param<Tensor<1>>,
    constant41: burn::module::Param<Tensor<1>>,
    linear23: Linear,
    constant332: burn::module::Param<Tensor<1>>,
    constant333: burn::module::Param<Tensor<1>>,
    constant334: burn::module::Param<Tensor<1>>,
    linear24: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule5 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant308: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant309: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant35: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear19 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear20 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear21 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear22 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant330: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant331: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant41: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear23 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant332: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant333: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant334: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear24 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant308,
            constant309,
            constant34,
            constant35,
            linear19,
            linear20,
            linear21,
            linear22,
            constant330,
            constant331,
            constant40,
            constant41,
            linear23,
            constant332,
            constant333,
            constant334,
            linear24,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add27_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean13_out1 = { add27_out1.clone().mean_dim(2usize) };
        let sub8_out1 = add27_out1.sub(reducemean13_out1);
        let constant308_out1 = self.constant308.val();
        let pow7_out1 = sub8_out1
            .clone()
            .powf((constant308_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean14_out1 = { pow7_out1.mean_dim(2usize) };
        let constant309_out1 = self.constant309.val();
        let add28_out1 = reducemean14_out1
            .add((constant309_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt16_out1 = add28_out1.sqrt();
        let div13_out1 = sub8_out1.div(sqrt16_out1);
        let constant34_out1 = self.constant34.val();
        let mul22_out1 = div13_out1
            .mul((constant34_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant35_out1 = self.constant35.val();
        let add29_out1 = mul22_out1
            .add((constant35_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape16_out1: [i64; 3] = {
            let axes = &add29_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather14_out1 = shape16_out1[0] as i64;
        let gather15_out1 = shape16_out1[1] as i64;
        let linear19_out1 = self.linear19.forward(add29_out1.clone());
        let unsqueeze24_out1 = [gather14_out1 as i64];
        let constant315_out1: [i64; 1] = [64i64];
        let constant313_out1: [i64; 1] = [-1i64];
        let constant314_out1: [i64; 1] = [12i64];
        let concat15_out1: [i64; 4usize] = [
            &unsqueeze24_out1[..],
            &constant313_out1[..],
            &constant314_out1[..],
            &constant315_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze25_out1 = [gather14_out1 as i64];
        let constant319_out1: [i64; 1] = [64i64];
        let constant317_out1: [i64; 1] = [-1i64];
        let constant318_out1: [i64; 1] = [12i64];
        let concat16_out1: [i64; 4usize] = [
            &unsqueeze25_out1[..],
            &constant317_out1[..],
            &constant318_out1[..],
            &constant319_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze26_out1 = [gather14_out1 as i64];
        let constant323_out1: [i64; 1] = [64i64];
        let constant321_out1: [i64; 1] = [-1i64];
        let constant322_out1: [i64; 1] = [12i64];
        let concat17_out1: [i64; 4usize] = [
            &unsqueeze26_out1[..],
            &constant321_out1[..],
            &constant322_out1[..],
            &constant323_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape15_out1 = linear19_out1.reshape(concat15_out1);
        let transpose13_out1 = reshape15_out1.permute([0, 2, 1, 3]);
        let linear20_out1 = self.linear20.forward(add29_out1.clone());
        let reshape16_out1 = linear20_out1.reshape(concat16_out1);
        let linear21_out1 = self.linear21.forward(add29_out1.clone());
        let reshape17_out1 = linear21_out1.reshape(concat17_out1);
        let transpose14_out1 = reshape17_out1.permute([0, 2, 1, 3]);
        let transpose15_out1 = reshape16_out1.permute([0, 2, 3, 1]);
        let matmul28_k_corrected = transpose15_out1.permute([0, 1, 3, 2]);
        let (matmul29_out1,) = {
            let q = transpose13_out1;
            let k = matmul28_k_corrected;
            let v = transpose14_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul29_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul29_out1,)
        };
        let transpose16_out1 = matmul29_out1.permute([0, 2, 1, 3]);
        let unsqueeze27_out1 = [gather14_out1 as i64];
        let unsqueeze28_out1 = [gather15_out1 as i64];
        let constant329_out1: [i64; 1] = [768i64];
        let concat18_out1: [i64; 3usize] = [
            &unsqueeze27_out1[..],
            &unsqueeze28_out1[..],
            &constant329_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape18_out1 = transpose16_out1.reshape(concat18_out1);
        let linear22_out1 = self.linear22.forward(reshape18_out1);
        let add31_out1 = linear22_out1.add(add29_out1);
        let reducemean15_out1 = { add31_out1.clone().mean_dim(2usize) };
        let sub9_out1 = add31_out1.sub(reducemean15_out1);
        let constant330_out1 = self.constant330.val();
        let pow8_out1 = sub9_out1
            .clone()
            .powf((constant330_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean16_out1 = { pow8_out1.mean_dim(2usize) };
        let constant331_out1 = self.constant331.val();
        let add32_out1 = reducemean16_out1
            .add((constant331_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt20_out1 = add32_out1.sqrt();
        let div15_out1 = sub9_out1.div(sqrt20_out1);
        let constant40_out1 = self.constant40.val();
        let mul25_out1 = div15_out1
            .mul((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant41_out1 = self.constant41.val();
        let add33_out1 = mul25_out1
            .add((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear23_out1 = self.linear23.forward(add33_out1.clone());
        let constant332_out1 = self.constant332.val();
        let div16_out1 = linear23_out1
            .clone()
            .div((constant332_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf4_out1 = div16_out1.erf();
        let constant333_out1 = self.constant333.val();
        let add34_out1 = erf4_out1
            .add((constant333_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul26_out1 = linear23_out1.mul(add34_out1);
        let constant334_out1 = self.constant334.val();
        let mul27_out1 = mul26_out1
            .mul((constant334_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear24_out1 = self.linear24.forward(mul27_out1);
        let add35_out1 = linear24_out1.add(add33_out1);
        add35_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule6 {
    constant335: burn::module::Param<Tensor<1>>,
    constant336: burn::module::Param<Tensor<1>>,
    constant44: burn::module::Param<Tensor<1>>,
    constant45: burn::module::Param<Tensor<1>>,
    linear25: Linear,
    linear26: Linear,
    linear27: Linear,
    linear28: Linear,
    constant357: burn::module::Param<Tensor<1>>,
    constant358: burn::module::Param<Tensor<1>>,
    constant50: burn::module::Param<Tensor<1>>,
    constant51: burn::module::Param<Tensor<1>>,
    linear29: Linear,
    constant359: burn::module::Param<Tensor<1>>,
    constant360: burn::module::Param<Tensor<1>>,
    constant361: burn::module::Param<Tensor<1>>,
    linear30: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule6 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant335: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant336: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant45: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear25 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear26 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear27 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear28 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant357: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant358: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant51: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear29 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant359: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant360: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant361: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear30 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant335,
            constant336,
            constant44,
            constant45,
            linear25,
            linear26,
            linear27,
            linear28,
            constant357,
            constant358,
            constant50,
            constant51,
            linear29,
            constant359,
            constant360,
            constant361,
            linear30,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add35_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean17_out1 = { add35_out1.clone().mean_dim(2usize) };
        let sub10_out1 = add35_out1.sub(reducemean17_out1);
        let constant335_out1 = self.constant335.val();
        let pow9_out1 = sub10_out1
            .clone()
            .powf((constant335_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean18_out1 = { pow9_out1.mean_dim(2usize) };
        let constant336_out1 = self.constant336.val();
        let add36_out1 = reducemean18_out1
            .add((constant336_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt21_out1 = add36_out1.sqrt();
        let div17_out1 = sub10_out1.div(sqrt21_out1);
        let constant44_out1 = self.constant44.val();
        let mul28_out1 = div17_out1
            .mul((constant44_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant45_out1 = self.constant45.val();
        let add37_out1 = mul28_out1
            .add((constant45_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape19_out1: [i64; 3] = {
            let axes = &add37_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather16_out1 = shape19_out1[0] as i64;
        let gather17_out1 = shape19_out1[1] as i64;
        let linear25_out1 = self.linear25.forward(add37_out1.clone());
        let unsqueeze29_out1 = [gather16_out1 as i64];
        let constant342_out1: [i64; 1] = [64i64];
        let constant340_out1: [i64; 1] = [-1i64];
        let constant341_out1: [i64; 1] = [12i64];
        let concat19_out1: [i64; 4usize] = [
            &unsqueeze29_out1[..],
            &constant340_out1[..],
            &constant341_out1[..],
            &constant342_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze30_out1 = [gather16_out1 as i64];
        let constant346_out1: [i64; 1] = [64i64];
        let constant344_out1: [i64; 1] = [-1i64];
        let constant345_out1: [i64; 1] = [12i64];
        let concat20_out1: [i64; 4usize] = [
            &unsqueeze30_out1[..],
            &constant344_out1[..],
            &constant345_out1[..],
            &constant346_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze31_out1 = [gather16_out1 as i64];
        let constant350_out1: [i64; 1] = [64i64];
        let constant348_out1: [i64; 1] = [-1i64];
        let constant349_out1: [i64; 1] = [12i64];
        let concat21_out1: [i64; 4usize] = [
            &unsqueeze31_out1[..],
            &constant348_out1[..],
            &constant349_out1[..],
            &constant350_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape19_out1 = linear25_out1.reshape(concat19_out1);
        let transpose17_out1 = reshape19_out1.permute([0, 2, 1, 3]);
        let linear26_out1 = self.linear26.forward(add37_out1.clone());
        let reshape20_out1 = linear26_out1.reshape(concat20_out1);
        let linear27_out1 = self.linear27.forward(add37_out1.clone());
        let reshape21_out1 = linear27_out1.reshape(concat21_out1);
        let transpose18_out1 = reshape21_out1.permute([0, 2, 1, 3]);
        let transpose19_out1 = reshape20_out1.permute([0, 2, 3, 1]);
        let matmul36_k_corrected = transpose19_out1.permute([0, 1, 3, 2]);
        let (matmul37_out1,) = {
            let q = transpose17_out1;
            let k = matmul36_k_corrected;
            let v = transpose18_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul37_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul37_out1,)
        };
        let transpose20_out1 = matmul37_out1.permute([0, 2, 1, 3]);
        let unsqueeze32_out1 = [gather16_out1 as i64];
        let unsqueeze33_out1 = [gather17_out1 as i64];
        let constant356_out1: [i64; 1] = [768i64];
        let concat22_out1: [i64; 3usize] = [
            &unsqueeze32_out1[..],
            &unsqueeze33_out1[..],
            &constant356_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape22_out1 = transpose20_out1.reshape(concat22_out1);
        let linear28_out1 = self.linear28.forward(reshape22_out1);
        let add39_out1 = linear28_out1.add(add37_out1);
        let reducemean19_out1 = { add39_out1.clone().mean_dim(2usize) };
        let sub11_out1 = add39_out1.sub(reducemean19_out1);
        let constant357_out1 = self.constant357.val();
        let pow10_out1 = sub11_out1
            .clone()
            .powf((constant357_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean20_out1 = { pow10_out1.mean_dim(2usize) };
        let constant358_out1 = self.constant358.val();
        let add40_out1 = reducemean20_out1
            .add((constant358_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt25_out1 = add40_out1.sqrt();
        let div19_out1 = sub11_out1.div(sqrt25_out1);
        let constant50_out1 = self.constant50.val();
        let mul31_out1 = div19_out1
            .mul((constant50_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant51_out1 = self.constant51.val();
        let add41_out1 = mul31_out1
            .add((constant51_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear29_out1 = self.linear29.forward(add41_out1.clone());
        let constant359_out1 = self.constant359.val();
        let div20_out1 = linear29_out1
            .clone()
            .div((constant359_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf5_out1 = div20_out1.erf();
        let constant360_out1 = self.constant360.val();
        let add42_out1 = erf5_out1
            .add((constant360_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul32_out1 = linear29_out1.mul(add42_out1);
        let constant361_out1 = self.constant361.val();
        let mul33_out1 = mul32_out1
            .mul((constant361_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear30_out1 = self.linear30.forward(mul33_out1);
        let add43_out1 = linear30_out1.add(add41_out1);
        add43_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule7 {
    constant362: burn::module::Param<Tensor<1>>,
    constant363: burn::module::Param<Tensor<1>>,
    constant54: burn::module::Param<Tensor<1>>,
    constant55: burn::module::Param<Tensor<1>>,
    linear31: Linear,
    linear32: Linear,
    linear33: Linear,
    linear34: Linear,
    constant384: burn::module::Param<Tensor<1>>,
    constant385: burn::module::Param<Tensor<1>>,
    constant60: burn::module::Param<Tensor<1>>,
    constant61: burn::module::Param<Tensor<1>>,
    linear35: Linear,
    constant386: burn::module::Param<Tensor<1>>,
    constant387: burn::module::Param<Tensor<1>>,
    constant388: burn::module::Param<Tensor<1>>,
    linear36: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule7 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant362: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant363: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant55: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear31 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear32 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear33 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear34 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant384: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant385: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant61: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear35 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant386: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant387: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant388: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear36 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant362,
            constant363,
            constant54,
            constant55,
            linear31,
            linear32,
            linear33,
            linear34,
            constant384,
            constant385,
            constant60,
            constant61,
            linear35,
            constant386,
            constant387,
            constant388,
            linear36,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add43_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean21_out1 = { add43_out1.clone().mean_dim(2usize) };
        let sub12_out1 = add43_out1.sub(reducemean21_out1);
        let constant362_out1 = self.constant362.val();
        let pow11_out1 = sub12_out1
            .clone()
            .powf((constant362_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean22_out1 = { pow11_out1.mean_dim(2usize) };
        let constant363_out1 = self.constant363.val();
        let add44_out1 = reducemean22_out1
            .add((constant363_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt26_out1 = add44_out1.sqrt();
        let div21_out1 = sub12_out1.div(sqrt26_out1);
        let constant54_out1 = self.constant54.val();
        let mul34_out1 = div21_out1
            .mul((constant54_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant55_out1 = self.constant55.val();
        let add45_out1 = mul34_out1
            .add((constant55_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape22_out1: [i64; 3] = {
            let axes = &add45_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather18_out1 = shape22_out1[0] as i64;
        let gather19_out1 = shape22_out1[1] as i64;
        let linear31_out1 = self.linear31.forward(add45_out1.clone());
        let unsqueeze34_out1 = [gather18_out1 as i64];
        let constant369_out1: [i64; 1] = [64i64];
        let constant367_out1: [i64; 1] = [-1i64];
        let constant368_out1: [i64; 1] = [12i64];
        let concat23_out1: [i64; 4usize] = [
            &unsqueeze34_out1[..],
            &constant367_out1[..],
            &constant368_out1[..],
            &constant369_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze35_out1 = [gather18_out1 as i64];
        let constant373_out1: [i64; 1] = [64i64];
        let constant371_out1: [i64; 1] = [-1i64];
        let constant372_out1: [i64; 1] = [12i64];
        let concat24_out1: [i64; 4usize] = [
            &unsqueeze35_out1[..],
            &constant371_out1[..],
            &constant372_out1[..],
            &constant373_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze36_out1 = [gather18_out1 as i64];
        let constant377_out1: [i64; 1] = [64i64];
        let constant375_out1: [i64; 1] = [-1i64];
        let constant376_out1: [i64; 1] = [12i64];
        let concat25_out1: [i64; 4usize] = [
            &unsqueeze36_out1[..],
            &constant375_out1[..],
            &constant376_out1[..],
            &constant377_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape23_out1 = linear31_out1.reshape(concat23_out1);
        let transpose21_out1 = reshape23_out1.permute([0, 2, 1, 3]);
        let linear32_out1 = self.linear32.forward(add45_out1.clone());
        let reshape24_out1 = linear32_out1.reshape(concat24_out1);
        let linear33_out1 = self.linear33.forward(add45_out1.clone());
        let reshape25_out1 = linear33_out1.reshape(concat25_out1);
        let transpose22_out1 = reshape25_out1.permute([0, 2, 1, 3]);
        let transpose23_out1 = reshape24_out1.permute([0, 2, 3, 1]);
        let matmul44_k_corrected = transpose23_out1.permute([0, 1, 3, 2]);
        let (matmul45_out1,) = {
            let q = transpose21_out1;
            let k = matmul44_k_corrected;
            let v = transpose22_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul45_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul45_out1,)
        };
        let transpose24_out1 = matmul45_out1.permute([0, 2, 1, 3]);
        let unsqueeze37_out1 = [gather18_out1 as i64];
        let unsqueeze38_out1 = [gather19_out1 as i64];
        let constant383_out1: [i64; 1] = [768i64];
        let concat26_out1: [i64; 3usize] = [
            &unsqueeze37_out1[..],
            &unsqueeze38_out1[..],
            &constant383_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape26_out1 = transpose24_out1.reshape(concat26_out1);
        let linear34_out1 = self.linear34.forward(reshape26_out1);
        let add47_out1 = linear34_out1.add(add45_out1);
        let reducemean23_out1 = { add47_out1.clone().mean_dim(2usize) };
        let sub13_out1 = add47_out1.sub(reducemean23_out1);
        let constant384_out1 = self.constant384.val();
        let pow12_out1 = sub13_out1
            .clone()
            .powf((constant384_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean24_out1 = { pow12_out1.mean_dim(2usize) };
        let constant385_out1 = self.constant385.val();
        let add48_out1 = reducemean24_out1
            .add((constant385_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt30_out1 = add48_out1.sqrt();
        let div23_out1 = sub13_out1.div(sqrt30_out1);
        let constant60_out1 = self.constant60.val();
        let mul37_out1 = div23_out1
            .mul((constant60_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant61_out1 = self.constant61.val();
        let add49_out1 = mul37_out1
            .add((constant61_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear35_out1 = self.linear35.forward(add49_out1.clone());
        let constant386_out1 = self.constant386.val();
        let div24_out1 = linear35_out1
            .clone()
            .div((constant386_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf6_out1 = div24_out1.erf();
        let constant387_out1 = self.constant387.val();
        let add50_out1 = erf6_out1
            .add((constant387_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul38_out1 = linear35_out1.mul(add50_out1);
        let constant388_out1 = self.constant388.val();
        let mul39_out1 = mul38_out1
            .mul((constant388_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear36_out1 = self.linear36.forward(mul39_out1);
        let add51_out1 = linear36_out1.add(add49_out1);
        add51_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule8 {
    constant389: burn::module::Param<Tensor<1>>,
    constant390: burn::module::Param<Tensor<1>>,
    constant64: burn::module::Param<Tensor<1>>,
    constant65: burn::module::Param<Tensor<1>>,
    linear37: Linear,
    linear38: Linear,
    linear39: Linear,
    linear40: Linear,
    constant411: burn::module::Param<Tensor<1>>,
    constant412: burn::module::Param<Tensor<1>>,
    constant70: burn::module::Param<Tensor<1>>,
    constant71: burn::module::Param<Tensor<1>>,
    linear41: Linear,
    constant413: burn::module::Param<Tensor<1>>,
    constant414: burn::module::Param<Tensor<1>>,
    constant415: burn::module::Param<Tensor<1>>,
    linear42: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule8 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant389: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant390: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant65: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear37 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear38 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear39 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear40 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant411: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant412: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant71: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear41 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant413: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant414: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant415: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear42 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant389,
            constant390,
            constant64,
            constant65,
            linear37,
            linear38,
            linear39,
            linear40,
            constant411,
            constant412,
            constant70,
            constant71,
            linear41,
            constant413,
            constant414,
            constant415,
            linear42,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add51_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean25_out1 = { add51_out1.clone().mean_dim(2usize) };
        let sub14_out1 = add51_out1.sub(reducemean25_out1);
        let constant389_out1 = self.constant389.val();
        let pow13_out1 = sub14_out1
            .clone()
            .powf((constant389_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean26_out1 = { pow13_out1.mean_dim(2usize) };
        let constant390_out1 = self.constant390.val();
        let add52_out1 = reducemean26_out1
            .add((constant390_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt31_out1 = add52_out1.sqrt();
        let div25_out1 = sub14_out1.div(sqrt31_out1);
        let constant64_out1 = self.constant64.val();
        let mul40_out1 = div25_out1
            .mul((constant64_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant65_out1 = self.constant65.val();
        let add53_out1 = mul40_out1
            .add((constant65_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape25_out1: [i64; 3] = {
            let axes = &add53_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather20_out1 = shape25_out1[0] as i64;
        let gather21_out1 = shape25_out1[1] as i64;
        let linear37_out1 = self.linear37.forward(add53_out1.clone());
        let unsqueeze39_out1 = [gather20_out1 as i64];
        let constant396_out1: [i64; 1] = [64i64];
        let constant394_out1: [i64; 1] = [-1i64];
        let constant395_out1: [i64; 1] = [12i64];
        let concat27_out1: [i64; 4usize] = [
            &unsqueeze39_out1[..],
            &constant394_out1[..],
            &constant395_out1[..],
            &constant396_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze40_out1 = [gather20_out1 as i64];
        let constant400_out1: [i64; 1] = [64i64];
        let constant398_out1: [i64; 1] = [-1i64];
        let constant399_out1: [i64; 1] = [12i64];
        let concat28_out1: [i64; 4usize] = [
            &unsqueeze40_out1[..],
            &constant398_out1[..],
            &constant399_out1[..],
            &constant400_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze41_out1 = [gather20_out1 as i64];
        let constant404_out1: [i64; 1] = [64i64];
        let constant402_out1: [i64; 1] = [-1i64];
        let constant403_out1: [i64; 1] = [12i64];
        let concat29_out1: [i64; 4usize] = [
            &unsqueeze41_out1[..],
            &constant402_out1[..],
            &constant403_out1[..],
            &constant404_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape27_out1 = linear37_out1.reshape(concat27_out1);
        let transpose25_out1 = reshape27_out1.permute([0, 2, 1, 3]);
        let linear38_out1 = self.linear38.forward(add53_out1.clone());
        let reshape28_out1 = linear38_out1.reshape(concat28_out1);
        let linear39_out1 = self.linear39.forward(add53_out1.clone());
        let reshape29_out1 = linear39_out1.reshape(concat29_out1);
        let transpose26_out1 = reshape29_out1.permute([0, 2, 1, 3]);
        let transpose27_out1 = reshape28_out1.permute([0, 2, 3, 1]);
        let matmul52_k_corrected = transpose27_out1.permute([0, 1, 3, 2]);
        let (matmul53_out1,) = {
            let q = transpose25_out1;
            let k = matmul52_k_corrected;
            let v = transpose26_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul53_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul53_out1,)
        };
        let transpose28_out1 = matmul53_out1.permute([0, 2, 1, 3]);
        let unsqueeze42_out1 = [gather20_out1 as i64];
        let unsqueeze43_out1 = [gather21_out1 as i64];
        let constant410_out1: [i64; 1] = [768i64];
        let concat30_out1: [i64; 3usize] = [
            &unsqueeze42_out1[..],
            &unsqueeze43_out1[..],
            &constant410_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape30_out1 = transpose28_out1.reshape(concat30_out1);
        let linear40_out1 = self.linear40.forward(reshape30_out1);
        let add55_out1 = linear40_out1.add(add53_out1);
        let reducemean27_out1 = { add55_out1.clone().mean_dim(2usize) };
        let sub15_out1 = add55_out1.sub(reducemean27_out1);
        let constant411_out1 = self.constant411.val();
        let pow14_out1 = sub15_out1
            .clone()
            .powf((constant411_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean28_out1 = { pow14_out1.mean_dim(2usize) };
        let constant412_out1 = self.constant412.val();
        let add56_out1 = reducemean28_out1
            .add((constant412_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt35_out1 = add56_out1.sqrt();
        let div27_out1 = sub15_out1.div(sqrt35_out1);
        let constant70_out1 = self.constant70.val();
        let mul43_out1 = div27_out1
            .mul((constant70_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant71_out1 = self.constant71.val();
        let add57_out1 = mul43_out1
            .add((constant71_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear41_out1 = self.linear41.forward(add57_out1.clone());
        let constant413_out1 = self.constant413.val();
        let div28_out1 = linear41_out1
            .clone()
            .div((constant413_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf7_out1 = div28_out1.erf();
        let constant414_out1 = self.constant414.val();
        let add58_out1 = erf7_out1
            .add((constant414_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul44_out1 = linear41_out1.mul(add58_out1);
        let constant415_out1 = self.constant415.val();
        let mul45_out1 = mul44_out1
            .mul((constant415_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear42_out1 = self.linear42.forward(mul45_out1);
        let add59_out1 = linear42_out1.add(add57_out1);
        add59_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule9 {
    constant416: burn::module::Param<Tensor<1>>,
    constant417: burn::module::Param<Tensor<1>>,
    constant74: burn::module::Param<Tensor<1>>,
    constant75: burn::module::Param<Tensor<1>>,
    linear43: Linear,
    linear44: Linear,
    linear45: Linear,
    linear46: Linear,
    constant438: burn::module::Param<Tensor<1>>,
    constant439: burn::module::Param<Tensor<1>>,
    constant80: burn::module::Param<Tensor<1>>,
    constant81: burn::module::Param<Tensor<1>>,
    linear47: Linear,
    constant440: burn::module::Param<Tensor<1>>,
    constant441: burn::module::Param<Tensor<1>>,
    constant442: burn::module::Param<Tensor<1>>,
    linear48: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule9 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant416: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant417: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant75: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear43 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear44 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear45 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear46 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant438: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant439: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant81: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear47 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant440: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant441: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant442: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear48 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant416,
            constant417,
            constant74,
            constant75,
            linear43,
            linear44,
            linear45,
            linear46,
            constant438,
            constant439,
            constant80,
            constant81,
            linear47,
            constant440,
            constant441,
            constant442,
            linear48,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add59_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean29_out1 = { add59_out1.clone().mean_dim(2usize) };
        let sub16_out1 = add59_out1.sub(reducemean29_out1);
        let constant416_out1 = self.constant416.val();
        let pow15_out1 = sub16_out1
            .clone()
            .powf((constant416_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean30_out1 = { pow15_out1.mean_dim(2usize) };
        let constant417_out1 = self.constant417.val();
        let add60_out1 = reducemean30_out1
            .add((constant417_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt36_out1 = add60_out1.sqrt();
        let div29_out1 = sub16_out1.div(sqrt36_out1);
        let constant74_out1 = self.constant74.val();
        let mul46_out1 = div29_out1
            .mul((constant74_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant75_out1 = self.constant75.val();
        let add61_out1 = mul46_out1
            .add((constant75_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape28_out1: [i64; 3] = {
            let axes = &add61_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather22_out1 = shape28_out1[0] as i64;
        let gather23_out1 = shape28_out1[1] as i64;
        let linear43_out1 = self.linear43.forward(add61_out1.clone());
        let unsqueeze44_out1 = [gather22_out1 as i64];
        let constant423_out1: [i64; 1] = [64i64];
        let constant421_out1: [i64; 1] = [-1i64];
        let constant422_out1: [i64; 1] = [12i64];
        let concat31_out1: [i64; 4usize] = [
            &unsqueeze44_out1[..],
            &constant421_out1[..],
            &constant422_out1[..],
            &constant423_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze45_out1 = [gather22_out1 as i64];
        let constant427_out1: [i64; 1] = [64i64];
        let constant425_out1: [i64; 1] = [-1i64];
        let constant426_out1: [i64; 1] = [12i64];
        let concat32_out1: [i64; 4usize] = [
            &unsqueeze45_out1[..],
            &constant425_out1[..],
            &constant426_out1[..],
            &constant427_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze46_out1 = [gather22_out1 as i64];
        let constant431_out1: [i64; 1] = [64i64];
        let constant429_out1: [i64; 1] = [-1i64];
        let constant430_out1: [i64; 1] = [12i64];
        let concat33_out1: [i64; 4usize] = [
            &unsqueeze46_out1[..],
            &constant429_out1[..],
            &constant430_out1[..],
            &constant431_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape31_out1 = linear43_out1.reshape(concat31_out1);
        let transpose29_out1 = reshape31_out1.permute([0, 2, 1, 3]);
        let linear44_out1 = self.linear44.forward(add61_out1.clone());
        let reshape32_out1 = linear44_out1.reshape(concat32_out1);
        let linear45_out1 = self.linear45.forward(add61_out1.clone());
        let reshape33_out1 = linear45_out1.reshape(concat33_out1);
        let transpose30_out1 = reshape33_out1.permute([0, 2, 1, 3]);
        let transpose31_out1 = reshape32_out1.permute([0, 2, 3, 1]);
        let matmul60_k_corrected = transpose31_out1.permute([0, 1, 3, 2]);
        let (matmul61_out1,) = {
            let q = transpose29_out1;
            let k = matmul60_k_corrected;
            let v = transpose30_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul61_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul61_out1,)
        };
        let transpose32_out1 = matmul61_out1.permute([0, 2, 1, 3]);
        let unsqueeze47_out1 = [gather22_out1 as i64];
        let unsqueeze48_out1 = [gather23_out1 as i64];
        let constant437_out1: [i64; 1] = [768i64];
        let concat34_out1: [i64; 3usize] = [
            &unsqueeze47_out1[..],
            &unsqueeze48_out1[..],
            &constant437_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape34_out1 = transpose32_out1.reshape(concat34_out1);
        let linear46_out1 = self.linear46.forward(reshape34_out1);
        let add63_out1 = linear46_out1.add(add61_out1);
        let reducemean31_out1 = { add63_out1.clone().mean_dim(2usize) };
        let sub17_out1 = add63_out1.sub(reducemean31_out1);
        let constant438_out1 = self.constant438.val();
        let pow16_out1 = sub17_out1
            .clone()
            .powf((constant438_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean32_out1 = { pow16_out1.mean_dim(2usize) };
        let constant439_out1 = self.constant439.val();
        let add64_out1 = reducemean32_out1
            .add((constant439_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt40_out1 = add64_out1.sqrt();
        let div31_out1 = sub17_out1.div(sqrt40_out1);
        let constant80_out1 = self.constant80.val();
        let mul49_out1 = div31_out1
            .mul((constant80_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant81_out1 = self.constant81.val();
        let add65_out1 = mul49_out1
            .add((constant81_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear47_out1 = self.linear47.forward(add65_out1.clone());
        let constant440_out1 = self.constant440.val();
        let div32_out1 = linear47_out1
            .clone()
            .div((constant440_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf8_out1 = div32_out1.erf();
        let constant441_out1 = self.constant441.val();
        let add66_out1 = erf8_out1
            .add((constant441_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul50_out1 = linear47_out1.mul(add66_out1);
        let constant442_out1 = self.constant442.val();
        let mul51_out1 = mul50_out1
            .mul((constant442_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear48_out1 = self.linear48.forward(mul51_out1);
        let add67_out1 = linear48_out1.add(add65_out1);
        add67_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule10 {
    constant443: burn::module::Param<Tensor<1>>,
    constant444: burn::module::Param<Tensor<1>>,
    constant84: burn::module::Param<Tensor<1>>,
    constant85: burn::module::Param<Tensor<1>>,
    linear49: Linear,
    linear50: Linear,
    linear51: Linear,
    linear52: Linear,
    constant465: burn::module::Param<Tensor<1>>,
    constant466: burn::module::Param<Tensor<1>>,
    constant90: burn::module::Param<Tensor<1>>,
    constant91: burn::module::Param<Tensor<1>>,
    linear53: Linear,
    constant467: burn::module::Param<Tensor<1>>,
    constant468: burn::module::Param<Tensor<1>>,
    constant469: burn::module::Param<Tensor<1>>,
    linear54: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule10 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant443: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant444: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant85: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear49 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear50 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear51 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear52 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant465: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant466: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant91: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear53 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant467: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant468: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant469: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear54 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant443,
            constant444,
            constant84,
            constant85,
            linear49,
            linear50,
            linear51,
            linear52,
            constant465,
            constant466,
            constant90,
            constant91,
            linear53,
            constant467,
            constant468,
            constant469,
            linear54,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add67_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean33_out1 = { add67_out1.clone().mean_dim(2usize) };
        let sub18_out1 = add67_out1.sub(reducemean33_out1);
        let constant443_out1 = self.constant443.val();
        let pow17_out1 = sub18_out1
            .clone()
            .powf((constant443_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean34_out1 = { pow17_out1.mean_dim(2usize) };
        let constant444_out1 = self.constant444.val();
        let add68_out1 = reducemean34_out1
            .add((constant444_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt41_out1 = add68_out1.sqrt();
        let div33_out1 = sub18_out1.div(sqrt41_out1);
        let constant84_out1 = self.constant84.val();
        let mul52_out1 = div33_out1
            .mul((constant84_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant85_out1 = self.constant85.val();
        let add69_out1 = mul52_out1
            .add((constant85_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape31_out1: [i64; 3] = {
            let axes = &add69_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather24_out1 = shape31_out1[0] as i64;
        let gather25_out1 = shape31_out1[1] as i64;
        let linear49_out1 = self.linear49.forward(add69_out1.clone());
        let unsqueeze49_out1 = [gather24_out1 as i64];
        let constant450_out1: [i64; 1] = [64i64];
        let constant448_out1: [i64; 1] = [-1i64];
        let constant449_out1: [i64; 1] = [12i64];
        let concat35_out1: [i64; 4usize] = [
            &unsqueeze49_out1[..],
            &constant448_out1[..],
            &constant449_out1[..],
            &constant450_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze50_out1 = [gather24_out1 as i64];
        let constant454_out1: [i64; 1] = [64i64];
        let constant452_out1: [i64; 1] = [-1i64];
        let constant453_out1: [i64; 1] = [12i64];
        let concat36_out1: [i64; 4usize] = [
            &unsqueeze50_out1[..],
            &constant452_out1[..],
            &constant453_out1[..],
            &constant454_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze51_out1 = [gather24_out1 as i64];
        let constant458_out1: [i64; 1] = [64i64];
        let constant456_out1: [i64; 1] = [-1i64];
        let constant457_out1: [i64; 1] = [12i64];
        let concat37_out1: [i64; 4usize] = [
            &unsqueeze51_out1[..],
            &constant456_out1[..],
            &constant457_out1[..],
            &constant458_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape35_out1 = linear49_out1.reshape(concat35_out1);
        let transpose33_out1 = reshape35_out1.permute([0, 2, 1, 3]);
        let linear50_out1 = self.linear50.forward(add69_out1.clone());
        let reshape36_out1 = linear50_out1.reshape(concat36_out1);
        let linear51_out1 = self.linear51.forward(add69_out1.clone());
        let reshape37_out1 = linear51_out1.reshape(concat37_out1);
        let transpose34_out1 = reshape37_out1.permute([0, 2, 1, 3]);
        let transpose35_out1 = reshape36_out1.permute([0, 2, 3, 1]);
        let matmul68_k_corrected = transpose35_out1.permute([0, 1, 3, 2]);
        let (matmul69_out1,) = {
            let q = transpose33_out1;
            let k = matmul68_k_corrected;
            let v = transpose34_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul69_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul69_out1,)
        };
        let transpose36_out1 = matmul69_out1.permute([0, 2, 1, 3]);
        let unsqueeze52_out1 = [gather24_out1 as i64];
        let unsqueeze53_out1 = [gather25_out1 as i64];
        let constant464_out1: [i64; 1] = [768i64];
        let concat38_out1: [i64; 3usize] = [
            &unsqueeze52_out1[..],
            &unsqueeze53_out1[..],
            &constant464_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape38_out1 = transpose36_out1.reshape(concat38_out1);
        let linear52_out1 = self.linear52.forward(reshape38_out1);
        let add71_out1 = linear52_out1.add(add69_out1);
        let reducemean35_out1 = { add71_out1.clone().mean_dim(2usize) };
        let sub19_out1 = add71_out1.sub(reducemean35_out1);
        let constant465_out1 = self.constant465.val();
        let pow18_out1 = sub19_out1
            .clone()
            .powf((constant465_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean36_out1 = { pow18_out1.mean_dim(2usize) };
        let constant466_out1 = self.constant466.val();
        let add72_out1 = reducemean36_out1
            .add((constant466_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt45_out1 = add72_out1.sqrt();
        let div35_out1 = sub19_out1.div(sqrt45_out1);
        let constant90_out1 = self.constant90.val();
        let mul55_out1 = div35_out1
            .mul((constant90_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant91_out1 = self.constant91.val();
        let add73_out1 = mul55_out1
            .add((constant91_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear53_out1 = self.linear53.forward(add73_out1.clone());
        let constant467_out1 = self.constant467.val();
        let div36_out1 = linear53_out1
            .clone()
            .div((constant467_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf9_out1 = div36_out1.erf();
        let constant468_out1 = self.constant468.val();
        let add74_out1 = erf9_out1
            .add((constant468_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul56_out1 = linear53_out1.mul(add74_out1);
        let constant469_out1 = self.constant469.val();
        let mul57_out1 = mul56_out1
            .mul((constant469_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear54_out1 = self.linear54.forward(mul57_out1);
        let add75_out1 = linear54_out1.add(add73_out1);
        add75_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule11 {
    constant470: burn::module::Param<Tensor<1>>,
    constant471: burn::module::Param<Tensor<1>>,
    constant94: burn::module::Param<Tensor<1>>,
    constant95: burn::module::Param<Tensor<1>>,
    linear55: Linear,
    linear56: Linear,
    linear57: Linear,
    linear58: Linear,
    constant492: burn::module::Param<Tensor<1>>,
    constant493: burn::module::Param<Tensor<1>>,
    constant100: burn::module::Param<Tensor<1>>,
    constant101: burn::module::Param<Tensor<1>>,
    linear59: Linear,
    constant494: burn::module::Param<Tensor<1>>,
    constant495: burn::module::Param<Tensor<1>>,
    constant496: burn::module::Param<Tensor<1>>,
    linear60: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule11 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant470: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant471: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant95: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear55 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear56 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear57 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear58 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant492: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant493: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant101: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear59 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant494: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant495: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant496: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear60 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant470,
            constant471,
            constant94,
            constant95,
            linear55,
            linear56,
            linear57,
            linear58,
            constant492,
            constant493,
            constant100,
            constant101,
            linear59,
            constant494,
            constant495,
            constant496,
            linear60,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add75_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean37_out1 = { add75_out1.clone().mean_dim(2usize) };
        let sub20_out1 = add75_out1.sub(reducemean37_out1);
        let constant470_out1 = self.constant470.val();
        let pow19_out1 = sub20_out1
            .clone()
            .powf((constant470_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean38_out1 = { pow19_out1.mean_dim(2usize) };
        let constant471_out1 = self.constant471.val();
        let add76_out1 = reducemean38_out1
            .add((constant471_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt46_out1 = add76_out1.sqrt();
        let div37_out1 = sub20_out1.div(sqrt46_out1);
        let constant94_out1 = self.constant94.val();
        let mul58_out1 = div37_out1
            .mul((constant94_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant95_out1 = self.constant95.val();
        let add77_out1 = mul58_out1
            .add((constant95_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape34_out1: [i64; 3] = {
            let axes = &add77_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather26_out1 = shape34_out1[0] as i64;
        let gather27_out1 = shape34_out1[1] as i64;
        let linear55_out1 = self.linear55.forward(add77_out1.clone());
        let unsqueeze54_out1 = [gather26_out1 as i64];
        let constant477_out1: [i64; 1] = [64i64];
        let constant475_out1: [i64; 1] = [-1i64];
        let constant476_out1: [i64; 1] = [12i64];
        let concat39_out1: [i64; 4usize] = [
            &unsqueeze54_out1[..],
            &constant475_out1[..],
            &constant476_out1[..],
            &constant477_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze55_out1 = [gather26_out1 as i64];
        let constant481_out1: [i64; 1] = [64i64];
        let constant479_out1: [i64; 1] = [-1i64];
        let constant480_out1: [i64; 1] = [12i64];
        let concat40_out1: [i64; 4usize] = [
            &unsqueeze55_out1[..],
            &constant479_out1[..],
            &constant480_out1[..],
            &constant481_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze56_out1 = [gather26_out1 as i64];
        let constant485_out1: [i64; 1] = [64i64];
        let constant483_out1: [i64; 1] = [-1i64];
        let constant484_out1: [i64; 1] = [12i64];
        let concat41_out1: [i64; 4usize] = [
            &unsqueeze56_out1[..],
            &constant483_out1[..],
            &constant484_out1[..],
            &constant485_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape39_out1 = linear55_out1.reshape(concat39_out1);
        let transpose37_out1 = reshape39_out1.permute([0, 2, 1, 3]);
        let linear56_out1 = self.linear56.forward(add77_out1.clone());
        let reshape40_out1 = linear56_out1.reshape(concat40_out1);
        let linear57_out1 = self.linear57.forward(add77_out1.clone());
        let reshape41_out1 = linear57_out1.reshape(concat41_out1);
        let transpose38_out1 = reshape41_out1.permute([0, 2, 1, 3]);
        let transpose39_out1 = reshape40_out1.permute([0, 2, 3, 1]);
        let matmul76_k_corrected = transpose39_out1.permute([0, 1, 3, 2]);
        let (matmul77_out1,) = {
            let q = transpose37_out1;
            let k = matmul76_k_corrected;
            let v = transpose38_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul77_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul77_out1,)
        };
        let transpose40_out1 = matmul77_out1.permute([0, 2, 1, 3]);
        let unsqueeze57_out1 = [gather26_out1 as i64];
        let unsqueeze58_out1 = [gather27_out1 as i64];
        let constant491_out1: [i64; 1] = [768i64];
        let concat42_out1: [i64; 3usize] = [
            &unsqueeze57_out1[..],
            &unsqueeze58_out1[..],
            &constant491_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape42_out1 = transpose40_out1.reshape(concat42_out1);
        let linear58_out1 = self.linear58.forward(reshape42_out1);
        let add79_out1 = linear58_out1.add(add77_out1);
        let reducemean39_out1 = { add79_out1.clone().mean_dim(2usize) };
        let sub21_out1 = add79_out1.sub(reducemean39_out1);
        let constant492_out1 = self.constant492.val();
        let pow20_out1 = sub21_out1
            .clone()
            .powf((constant492_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean40_out1 = { pow20_out1.mean_dim(2usize) };
        let constant493_out1 = self.constant493.val();
        let add80_out1 = reducemean40_out1
            .add((constant493_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt50_out1 = add80_out1.sqrt();
        let div39_out1 = sub21_out1.div(sqrt50_out1);
        let constant100_out1 = self.constant100.val();
        let mul61_out1 = div39_out1
            .mul((constant100_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant101_out1 = self.constant101.val();
        let add81_out1 = mul61_out1
            .add((constant101_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear59_out1 = self.linear59.forward(add81_out1.clone());
        let constant494_out1 = self.constant494.val();
        let div40_out1 = linear59_out1
            .clone()
            .div((constant494_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf10_out1 = div40_out1.erf();
        let constant495_out1 = self.constant495.val();
        let add82_out1 = erf10_out1
            .add((constant495_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul62_out1 = linear59_out1.mul(add82_out1);
        let constant496_out1 = self.constant496.val();
        let mul63_out1 = mul62_out1
            .mul((constant496_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear60_out1 = self.linear60.forward(mul63_out1);
        let add83_out1 = linear60_out1.add(add81_out1);
        add83_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule12 {
    constant497: burn::module::Param<Tensor<1>>,
    constant498: burn::module::Param<Tensor<1>>,
    constant104: burn::module::Param<Tensor<1>>,
    constant105: burn::module::Param<Tensor<1>>,
    linear61: Linear,
    linear62: Linear,
    linear63: Linear,
    linear64: Linear,
    constant519: burn::module::Param<Tensor<1>>,
    constant520: burn::module::Param<Tensor<1>>,
    constant110: burn::module::Param<Tensor<1>>,
    constant111: burn::module::Param<Tensor<1>>,
    linear65: Linear,
    constant521: burn::module::Param<Tensor<1>>,
    constant522: burn::module::Param<Tensor<1>>,
    constant523: burn::module::Param<Tensor<1>>,
    linear66: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule12 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant497: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant498: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant105: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear61 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear62 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear63 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear64 = LinearConfig::new(768, 768).with_bias(true).init(device);
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
        let constant110: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant111: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear65 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant521: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant522: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant523: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear66 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        Self {
            constant497,
            constant498,
            constant104,
            constant105,
            linear61,
            linear62,
            linear63,
            linear64,
            constant519,
            constant520,
            constant110,
            constant111,
            linear65,
            constant521,
            constant522,
            constant523,
            linear66,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add83_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean41_out1 = { add83_out1.clone().mean_dim(2usize) };
        let sub22_out1 = add83_out1.sub(reducemean41_out1);
        let constant497_out1 = self.constant497.val();
        let pow21_out1 = sub22_out1
            .clone()
            .powf((constant497_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean42_out1 = { pow21_out1.mean_dim(2usize) };
        let constant498_out1 = self.constant498.val();
        let add84_out1 = reducemean42_out1
            .add((constant498_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt51_out1 = add84_out1.sqrt();
        let div41_out1 = sub22_out1.div(sqrt51_out1);
        let constant104_out1 = self.constant104.val();
        let mul64_out1 = div41_out1
            .mul((constant104_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant105_out1 = self.constant105.val();
        let add85_out1 = mul64_out1
            .add((constant105_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape37_out1: [i64; 3] = {
            let axes = &add85_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather28_out1 = shape37_out1[0] as i64;
        let gather29_out1 = shape37_out1[1] as i64;
        let linear61_out1 = self.linear61.forward(add85_out1.clone());
        let unsqueeze59_out1 = [gather28_out1 as i64];
        let constant504_out1: [i64; 1] = [64i64];
        let constant502_out1: [i64; 1] = [-1i64];
        let constant503_out1: [i64; 1] = [12i64];
        let concat43_out1: [i64; 4usize] = [
            &unsqueeze59_out1[..],
            &constant502_out1[..],
            &constant503_out1[..],
            &constant504_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze60_out1 = [gather28_out1 as i64];
        let constant508_out1: [i64; 1] = [64i64];
        let constant506_out1: [i64; 1] = [-1i64];
        let constant507_out1: [i64; 1] = [12i64];
        let concat44_out1: [i64; 4usize] = [
            &unsqueeze60_out1[..],
            &constant506_out1[..],
            &constant507_out1[..],
            &constant508_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze61_out1 = [gather28_out1 as i64];
        let constant512_out1: [i64; 1] = [64i64];
        let constant510_out1: [i64; 1] = [-1i64];
        let constant511_out1: [i64; 1] = [12i64];
        let concat45_out1: [i64; 4usize] = [
            &unsqueeze61_out1[..],
            &constant510_out1[..],
            &constant511_out1[..],
            &constant512_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape43_out1 = linear61_out1.reshape(concat43_out1);
        let transpose41_out1 = reshape43_out1.permute([0, 2, 1, 3]);
        let linear62_out1 = self.linear62.forward(add85_out1.clone());
        let reshape44_out1 = linear62_out1.reshape(concat44_out1);
        let linear63_out1 = self.linear63.forward(add85_out1.clone());
        let reshape45_out1 = linear63_out1.reshape(concat45_out1);
        let transpose42_out1 = reshape45_out1.permute([0, 2, 1, 3]);
        let transpose43_out1 = reshape44_out1.permute([0, 2, 3, 1]);
        let matmul84_k_corrected = transpose43_out1.permute([0, 1, 3, 2]);
        let (matmul85_out1,) = {
            let q = transpose41_out1;
            let k = matmul84_k_corrected;
            let v = transpose42_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul85_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul85_out1,)
        };
        let transpose44_out1 = matmul85_out1.permute([0, 2, 1, 3]);
        let unsqueeze62_out1 = [gather28_out1 as i64];
        let unsqueeze63_out1 = [gather29_out1 as i64];
        let constant518_out1: [i64; 1] = [768i64];
        let concat46_out1: [i64; 3usize] = [
            &unsqueeze62_out1[..],
            &unsqueeze63_out1[..],
            &constant518_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape46_out1 = transpose44_out1.reshape(concat46_out1);
        let linear64_out1 = self.linear64.forward(reshape46_out1);
        let add87_out1 = linear64_out1.add(add85_out1);
        let reducemean43_out1 = { add87_out1.clone().mean_dim(2usize) };
        let sub23_out1 = add87_out1.sub(reducemean43_out1);
        let constant519_out1 = self.constant519.val();
        let pow22_out1 = sub23_out1
            .clone()
            .powf((constant519_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean44_out1 = { pow22_out1.mean_dim(2usize) };
        let constant520_out1 = self.constant520.val();
        let add88_out1 = reducemean44_out1
            .add((constant520_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt55_out1 = add88_out1.sqrt();
        let div43_out1 = sub23_out1.div(sqrt55_out1);
        let constant110_out1 = self.constant110.val();
        let mul67_out1 = div43_out1
            .mul((constant110_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant111_out1 = self.constant111.val();
        let add89_out1 = mul67_out1
            .add((constant111_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear65_out1 = self.linear65.forward(add89_out1.clone());
        let constant521_out1 = self.constant521.val();
        let div44_out1 = linear65_out1
            .clone()
            .div((constant521_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf11_out1 = div44_out1.erf();
        let constant522_out1 = self.constant522.val();
        let add90_out1 = erf11_out1
            .add((constant522_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul68_out1 = linear65_out1.mul(add90_out1);
        let constant523_out1 = self.constant523.val();
        let mul69_out1 = mul68_out1
            .mul((constant523_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear66_out1 = self.linear66.forward(mul69_out1);
        let add91_out1 = linear66_out1.add(add89_out1);
        add91_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule13 {
    constant524: burn::module::Param<Tensor<1>>,
    constant525: burn::module::Param<Tensor<1>>,
    constant114: burn::module::Param<Tensor<1>>,
    constant115: burn::module::Param<Tensor<1>>,
    linear67: Linear,
    linear68: Linear,
    linear69: Linear,
    linear70: Linear,
    constant546: burn::module::Param<Tensor<1>>,
    constant547: burn::module::Param<Tensor<1>>,
    constant120: burn::module::Param<Tensor<1>>,
    constant121: burn::module::Param<Tensor<1>>,
    linear71: Linear,
    constant548: burn::module::Param<Tensor<1>>,
    constant549: burn::module::Param<Tensor<1>>,
    constant550: burn::module::Param<Tensor<1>>,
    linear72: Linear,
    constant551: burn::module::Param<Tensor<1>>,
    constant552: burn::module::Param<Tensor<1>>,
    constant124: burn::module::Param<Tensor<1>>,
    constant125: burn::module::Param<Tensor<1>>,
    linear73: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule13 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant524: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant525: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant115: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear67 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear68 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear69 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let linear70 = LinearConfig::new(768, 768).with_bias(true).init(device);
        let constant546: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant547: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant121: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear71 = LinearConfig::new(768, 3072).with_bias(true).init(device);
        let constant548: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant549: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant550: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear72 = LinearConfig::new(3072, 768).with_bias(true).init(device);
        let constant551: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant552: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let constant125: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([768], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [768].into(),
        );
        let linear73 = LinearConfig::new(768, 768)
            .with_bias(true)
            .with_layout(LinearLayout::Col)
            .init(device);
        Self {
            constant524,
            constant525,
            constant114,
            constant115,
            linear67,
            linear68,
            linear69,
            linear70,
            constant546,
            constant547,
            constant120,
            constant121,
            linear71,
            constant548,
            constant549,
            constant550,
            linear72,
            constant551,
            constant552,
            constant124,
            constant125,
            linear73,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add91_out1: Tensor<3>,
        where3_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<2>) {
        let reducemean45_out1 = { add91_out1.clone().mean_dim(2usize) };
        let sub24_out1 = add91_out1.sub(reducemean45_out1);
        let constant524_out1 = self.constant524.val();
        let pow23_out1 = sub24_out1
            .clone()
            .powf((constant524_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean46_out1 = { pow23_out1.mean_dim(2usize) };
        let constant525_out1 = self.constant525.val();
        let add92_out1 = reducemean46_out1
            .add((constant525_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt56_out1 = add92_out1.sqrt();
        let div45_out1 = sub24_out1.div(sqrt56_out1);
        let constant114_out1 = self.constant114.val();
        let mul70_out1 = div45_out1
            .mul((constant114_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant115_out1 = self.constant115.val();
        let add93_out1 = mul70_out1
            .add((constant115_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape40_out1: [i64; 3] = {
            let axes = &add93_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather30_out1 = shape40_out1[0] as i64;
        let gather31_out1 = shape40_out1[1] as i64;
        let linear67_out1 = self.linear67.forward(add93_out1.clone());
        let unsqueeze64_out1 = [gather30_out1 as i64];
        let constant531_out1: [i64; 1] = [64i64];
        let constant529_out1: [i64; 1] = [-1i64];
        let constant530_out1: [i64; 1] = [12i64];
        let concat47_out1: [i64; 4usize] = [
            &unsqueeze64_out1[..],
            &constant529_out1[..],
            &constant530_out1[..],
            &constant531_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze65_out1 = [gather30_out1 as i64];
        let constant535_out1: [i64; 1] = [64i64];
        let constant533_out1: [i64; 1] = [-1i64];
        let constant534_out1: [i64; 1] = [12i64];
        let concat48_out1: [i64; 4usize] = [
            &unsqueeze65_out1[..],
            &constant533_out1[..],
            &constant534_out1[..],
            &constant535_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze66_out1 = [gather30_out1 as i64];
        let constant539_out1: [i64; 1] = [64i64];
        let constant537_out1: [i64; 1] = [-1i64];
        let constant538_out1: [i64; 1] = [12i64];
        let concat49_out1: [i64; 4usize] = [
            &unsqueeze66_out1[..],
            &constant537_out1[..],
            &constant538_out1[..],
            &constant539_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape47_out1 = linear67_out1.reshape(concat47_out1);
        let transpose45_out1 = reshape47_out1.permute([0, 2, 1, 3]);
        let linear68_out1 = self.linear68.forward(add93_out1.clone());
        let reshape48_out1 = linear68_out1.reshape(concat48_out1);
        let linear69_out1 = self.linear69.forward(add93_out1.clone());
        let reshape49_out1 = linear69_out1.reshape(concat49_out1);
        let transpose46_out1 = reshape49_out1.permute([0, 2, 1, 3]);
        let transpose47_out1 = reshape48_out1.permute([0, 2, 3, 1]);
        let matmul92_k_corrected = transpose47_out1.permute([0, 1, 3, 2]);
        let (matmul93_out1,) = {
            let q = transpose45_out1;
            let k = matmul92_k_corrected;
            let v = transpose46_out1;
            // rag3weaver : le masque en booléen, pas en biais — avec un biais,
            // burn-cubecl retombe sur l'attention naïve (patch_attention.py, 6 sept. 2026).
            let masque = {
                let [b, h, sq, _] = q.dims();
                let sk = k.dims()[2];
                where3_out1.clone().lower_elem(0.0).expand([b, h, sq, sk])
            };
            let matmul93_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                Some(masque),
                None,
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul93_out1,)
        };
        let transpose48_out1 = matmul93_out1.permute([0, 2, 1, 3]);
        let unsqueeze67_out1 = [gather30_out1 as i64];
        let unsqueeze68_out1 = [gather31_out1 as i64];
        let constant545_out1: [i64; 1] = [768i64];
        let concat50_out1: [i64; 3usize] = [
            &unsqueeze67_out1[..],
            &unsqueeze68_out1[..],
            &constant545_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape50_out1 = transpose48_out1.reshape(concat50_out1);
        let linear70_out1 = self.linear70.forward(reshape50_out1);
        let add95_out1 = linear70_out1.add(add93_out1);
        let reducemean47_out1 = { add95_out1.clone().mean_dim(2usize) };
        let sub25_out1 = add95_out1.sub(reducemean47_out1);
        let constant546_out1 = self.constant546.val();
        let pow24_out1 = sub25_out1
            .clone()
            .powf((constant546_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean48_out1 = { pow24_out1.mean_dim(2usize) };
        let constant547_out1 = self.constant547.val();
        let add96_out1 = reducemean48_out1
            .add((constant547_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt60_out1 = add96_out1.sqrt();
        let div47_out1 = sub25_out1.div(sqrt60_out1);
        let constant120_out1 = self.constant120.val();
        let mul73_out1 = div47_out1
            .mul((constant120_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant121_out1 = self.constant121.val();
        let add97_out1 = mul73_out1
            .add((constant121_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear71_out1 = self.linear71.forward(add97_out1.clone());
        let constant548_out1 = self.constant548.val();
        let div48_out1 = linear71_out1
            .clone()
            .div((constant548_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf12_out1 = div48_out1.erf();
        let constant549_out1 = self.constant549.val();
        let add98_out1 = erf12_out1
            .add((constant549_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul74_out1 = linear71_out1.mul(add98_out1);
        let constant550_out1 = self.constant550.val();
        let mul75_out1 = mul74_out1
            .mul((constant550_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear72_out1 = self.linear72.forward(mul75_out1);
        let add99_out1 = linear72_out1.add(add97_out1);
        let reducemean49_out1 = { add99_out1.clone().mean_dim(2usize) };
        let sub26_out1 = add99_out1.sub(reducemean49_out1);
        let constant551_out1 = self.constant551.val();
        let pow25_out1 = sub26_out1
            .clone()
            .powf((constant551_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean50_out1 = { pow25_out1.mean_dim(2usize) };
        let constant552_out1 = self.constant552.val();
        let add100_out1 = reducemean50_out1
            .add((constant552_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt61_out1 = add100_out1.sqrt();
        let div49_out1 = sub26_out1.div(sqrt61_out1);
        let constant124_out1 = self.constant124.val();
        let mul76_out1 = div49_out1
            .mul((constant124_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant125_out1 = self.constant125.val();
        let add101_out1 = mul76_out1
            .add((constant125_out1).unsqueeze_dims(&[0isize, 1isize]));
        let gather32_out1 = {
            let sliced = add101_out1.clone().slice(s![.., 0, ..]);
            sliced.squeeze_dim::<2usize>(1)
        };
        let linear73_out1 = self.linear73.forward(gather32_out1);
        let tanh1_out1 = linear73_out1.tanh();
        (add101_out1, tanh1_out1)
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
        model
            .load_from(&mut store)
            .unwrap_or_else(|e| panic!("Failed to load burnpack bytes: {e}"));
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
            device: device.clone(),
        }
    }

    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        input_ids: Tensor<2, Int>,
        attention_mask: Tensor<2, Int>,
    ) -> (Tensor<3>, Tensor<2>) {
        let (expand2_out1, add5_out1) = self
            .submodule1
            .forward(input_ids, attention_mask);
        let (add11_out1, where3_out1) = self.submodule2.forward(expand2_out1, add5_out1);
        let add19_out1 = self.submodule3.forward(add11_out1, where3_out1.clone());
        let add27_out1 = self.submodule4.forward(add19_out1, where3_out1.clone());
        let add35_out1 = self.submodule5.forward(add27_out1, where3_out1.clone());
        let add43_out1 = self.submodule6.forward(add35_out1, where3_out1.clone());
        let add51_out1 = self.submodule7.forward(add43_out1, where3_out1.clone());
        let add59_out1 = self.submodule8.forward(add51_out1, where3_out1.clone());
        let add67_out1 = self.submodule9.forward(add59_out1, where3_out1.clone());
        let add75_out1 = self.submodule10.forward(add67_out1, where3_out1.clone());
        let add83_out1 = self.submodule11.forward(add75_out1, where3_out1.clone());
        let add91_out1 = self.submodule12.forward(add83_out1, where3_out1.clone());
        let (add101_out1, tanh1_out1) = self
            .submodule13
            .forward(add91_out1, where3_out1);
        (add101_out1, tanh1_out1)
    }
}
