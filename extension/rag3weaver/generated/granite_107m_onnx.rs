// rag3weaver : casts « float » neutres (patch_attention.py --casts-neutres, 6 septembre 2026).
// Generated from ONNX "/tmp/claude-1000/-home-lucied-git-workspaces-rag3db/13068ada-ca9b-4752-8b48-bf8f40ed08a2/scratchpad/granite/hf-107m/model.onnx" by burn-onnx
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
    constant107: burn::module::Param<Tensor<2, Int>>,
    constant115: burn::module::Param<Tensor<1, Int>>,
    constant118: burn::module::Param<Tensor<1, Int>>,
    constant1: burn::module::Param<Tensor<2>>,
    constant3: burn::module::Param<Tensor<2>>,
    constant2: burn::module::Param<Tensor<2>>,
    constant119: burn::module::Param<Tensor<1>>,
    constant120: burn::module::Param<Tensor<1>>,
    constant4: burn::module::Param<Tensor<1>>,
    constant5: burn::module::Param<Tensor<1>>,
    constant130: burn::module::Param<Tensor<1, Int>>,
    #[module(skip)]
    device: Device,
}
impl Submodule1 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant107: burn::module::Param<Tensor<2, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
                Int,
            >::zeros([1, 514], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [1, 514].into(),
        );
        let constant115: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
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
        let constant118: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
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
            >::zeros([250002, 384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [250002, 384].into(),
        );
        let constant3: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([2, 384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [2, 384].into(),
        );
        let constant2: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([514, 384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [514, 384].into(),
        );
        let constant119: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant120: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant5: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant130: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
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
            constant107,
            constant115,
            constant118,
            constant1,
            constant3,
            constant2,
            constant119,
            constant120,
            constant4,
            constant5,
            constant130,
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
        let constant107_out1 = self.constant107.val();
        let slice1_out1 = constant107_out1.slice(s![.., 0..unsqueeze1_out1[0]]);
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
        let constant115_out1 = self.constant115.val();
        let mul1_out1 = constantofshape1_out1.clone().mul(constant115_out1);
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
        let constant116_out1 = 1i64;
        let equal2_out1 = input_ids.clone().equal_elem(constant116_out1);
        let not1_out1 = equal2_out1.bool_not();
        let cast1_out1 = not1_out1.int().cast(burn::tensor::DType::I32);
        let cumsum1_out1 = cast1_out1.clone().cumsum(1);
        let mul2_out1 = cumsum1_out1.mul(cast1_out1);
        let cast2_out1 = mul2_out1.cast(burn::tensor::DType::I64);
        let constant118_out1 = self.constant118.val();
        let add1_out1 = cast2_out1.add((constant118_out1).unsqueeze_dims(&[0isize]));
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
        let constant119_out1 = self.constant119.val();
        let pow1_out1 = sub1_out1
            .clone()
            .powf((constant119_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean2_out1 = { pow1_out1.mean_dim(2usize) };
        let constant120_out1 = self.constant120.val();
        let add4_out1 = reducemean2_out1
            .add((constant120_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant126_out1: [i64; 1] = [1i64];
        let concat2_out1: [i64; 4usize] = [
            &unsqueeze6_out1[..],
            &constant126_out1[..],
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
        let constant130_out1 = self.constant130.val();
        let mul4_out1 = constantofshape2_out1.clone().mul(constant130_out1);
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
    constant131: burn::module::Param<Tensor<1>>,
    linear1: Linear,
    linear2: Linear,
    linear3: Linear,
    linear4: Linear,
    constant153: burn::module::Param<Tensor<1>>,
    constant154: burn::module::Param<Tensor<1>>,
    constant10: burn::module::Param<Tensor<1>>,
    constant11: burn::module::Param<Tensor<1>>,
    linear5: Linear,
    constant155: burn::module::Param<Tensor<1>>,
    constant156: burn::module::Param<Tensor<1>>,
    constant157: burn::module::Param<Tensor<1>>,
    linear6: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule2 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant131: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear1 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear2 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear3 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear4 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let constant153: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant154: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant11: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear5 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant155: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant156: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant157: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear6 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        Self {
            constant131,
            linear1,
            linear2,
            linear3,
            linear4,
            constant153,
            constant154,
            constant10,
            constant11,
            linear5,
            constant155,
            constant156,
            constant157,
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
        let constant131_out1 = self.constant131.val();
        let sub2_out1 = (constant131_out1)
            .unsqueeze_dims(&[0isize, 1isize, 2isize])
            .sub(cast3_out1);
        let cast4_out1 = sub2_out1.clone().bool();
        let constant132_out1 = -340282350000000000000000000000000000000f32;
        let where3_out1 = sub2_out1.mask_fill(cast4_out1, constant132_out1);
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
        let constant138_out1: [i64; 1] = [32i64];
        let constant136_out1: [i64; 1] = [-1i64];
        let constant137_out1: [i64; 1] = [12i64];
        let concat3_out1: [i64; 4usize] = [
            &unsqueeze9_out1[..],
            &constant136_out1[..],
            &constant137_out1[..],
            &constant138_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze10_out1 = [gather8_out1 as i64];
        let constant142_out1: [i64; 1] = [32i64];
        let constant140_out1: [i64; 1] = [-1i64];
        let constant141_out1: [i64; 1] = [12i64];
        let concat4_out1: [i64; 4usize] = [
            &unsqueeze10_out1[..],
            &constant140_out1[..],
            &constant141_out1[..],
            &constant142_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze11_out1 = [gather8_out1 as i64];
        let constant146_out1: [i64; 1] = [32i64];
        let constant144_out1: [i64; 1] = [-1i64];
        let constant145_out1: [i64; 1] = [12i64];
        let concat5_out1: [i64; 4usize] = [
            &unsqueeze11_out1[..],
            &constant144_out1[..],
            &constant145_out1[..],
            &constant146_out1[..],
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
            let matmul5_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1.clone()),
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
        let constant152_out1: [i64; 1] = [384i64];
        let concat6_out1: [i64; 3usize] = [
            &unsqueeze12_out1[..],
            &unsqueeze13_out1[..],
            &constant152_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape6_out1 = transpose4_out1.reshape(concat6_out1);
        let linear4_out1 = self.linear4.forward(reshape6_out1);
        let add7_out1 = linear4_out1.add(add5_out1);
        let reducemean3_out1 = { add7_out1.clone().mean_dim(2usize) };
        let sub3_out1 = add7_out1.sub(reducemean3_out1);
        let constant153_out1 = self.constant153.val();
        let pow2_out1 = sub3_out1
            .clone()
            .powf((constant153_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean4_out1 = { pow2_out1.mean_dim(2usize) };
        let constant154_out1 = self.constant154.val();
        let add8_out1 = reducemean4_out1
            .add((constant154_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt5_out1 = add8_out1.sqrt();
        let div3_out1 = sub3_out1.div(sqrt5_out1);
        let constant10_out1 = self.constant10.val();
        let mul7_out1 = div3_out1
            .mul((constant10_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant11_out1 = self.constant11.val();
        let add9_out1 = mul7_out1
            .add((constant11_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear5_out1 = self.linear5.forward(add9_out1.clone());
        let constant155_out1 = self.constant155.val();
        let div4_out1 = linear5_out1
            .clone()
            .div((constant155_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf1_out1 = div4_out1.erf();
        let constant156_out1 = self.constant156.val();
        let add10_out1 = erf1_out1
            .add((constant156_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul8_out1 = linear5_out1.mul(add10_out1);
        let constant157_out1 = self.constant157.val();
        let mul9_out1 = mul8_out1
            .mul((constant157_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear6_out1 = self.linear6.forward(mul9_out1);
        let add11_out1 = linear6_out1.add(add9_out1);
        (add11_out1, where3_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule3 {
    constant158: burn::module::Param<Tensor<1>>,
    constant159: burn::module::Param<Tensor<1>>,
    constant14: burn::module::Param<Tensor<1>>,
    constant15: burn::module::Param<Tensor<1>>,
    linear7: Linear,
    linear8: Linear,
    linear9: Linear,
    linear10: Linear,
    constant180: burn::module::Param<Tensor<1>>,
    constant181: burn::module::Param<Tensor<1>>,
    constant20: burn::module::Param<Tensor<1>>,
    constant21: burn::module::Param<Tensor<1>>,
    linear11: Linear,
    constant182: burn::module::Param<Tensor<1>>,
    constant183: burn::module::Param<Tensor<1>>,
    constant184: burn::module::Param<Tensor<1>>,
    linear12: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule3 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant158: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant159: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant15: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear7 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear8 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear9 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear10 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let constant180: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant181: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant21: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear11 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant182: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant183: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant184: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear12 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        Self {
            constant158,
            constant159,
            constant14,
            constant15,
            linear7,
            linear8,
            linear9,
            linear10,
            constant180,
            constant181,
            constant20,
            constant21,
            linear11,
            constant182,
            constant183,
            constant184,
            linear12,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add11_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean5_out1 = { add11_out1.clone().mean_dim(2usize) };
        let sub4_out1 = add11_out1.sub(reducemean5_out1);
        let constant158_out1 = self.constant158.val();
        let pow3_out1 = sub4_out1
            .clone()
            .powf((constant158_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean6_out1 = { pow3_out1.mean_dim(2usize) };
        let constant159_out1 = self.constant159.val();
        let add12_out1 = reducemean6_out1
            .add((constant159_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant165_out1: [i64; 1] = [32i64];
        let constant163_out1: [i64; 1] = [-1i64];
        let constant164_out1: [i64; 1] = [12i64];
        let concat7_out1: [i64; 4usize] = [
            &unsqueeze14_out1[..],
            &constant163_out1[..],
            &constant164_out1[..],
            &constant165_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze15_out1 = [gather10_out1 as i64];
        let constant169_out1: [i64; 1] = [32i64];
        let constant167_out1: [i64; 1] = [-1i64];
        let constant168_out1: [i64; 1] = [12i64];
        let concat8_out1: [i64; 4usize] = [
            &unsqueeze15_out1[..],
            &constant167_out1[..],
            &constant168_out1[..],
            &constant169_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze16_out1 = [gather10_out1 as i64];
        let constant173_out1: [i64; 1] = [32i64];
        let constant171_out1: [i64; 1] = [-1i64];
        let constant172_out1: [i64; 1] = [12i64];
        let concat9_out1: [i64; 4usize] = [
            &unsqueeze16_out1[..],
            &constant171_out1[..],
            &constant172_out1[..],
            &constant173_out1[..],
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
            let matmul13_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
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
        let constant179_out1: [i64; 1] = [384i64];
        let concat10_out1: [i64; 3usize] = [
            &unsqueeze17_out1[..],
            &unsqueeze18_out1[..],
            &constant179_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape10_out1 = transpose8_out1.reshape(concat10_out1);
        let linear10_out1 = self.linear10.forward(reshape10_out1);
        let add15_out1 = linear10_out1.add(add13_out1);
        let reducemean7_out1 = { add15_out1.clone().mean_dim(2usize) };
        let sub5_out1 = add15_out1.sub(reducemean7_out1);
        let constant180_out1 = self.constant180.val();
        let pow4_out1 = sub5_out1
            .clone()
            .powf((constant180_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean8_out1 = { pow4_out1.mean_dim(2usize) };
        let constant181_out1 = self.constant181.val();
        let add16_out1 = reducemean8_out1
            .add((constant181_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt10_out1 = add16_out1.sqrt();
        let div7_out1 = sub5_out1.div(sqrt10_out1);
        let constant20_out1 = self.constant20.val();
        let mul13_out1 = div7_out1
            .mul((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant21_out1 = self.constant21.val();
        let add17_out1 = mul13_out1
            .add((constant21_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear11_out1 = self.linear11.forward(add17_out1.clone());
        let constant182_out1 = self.constant182.val();
        let div8_out1 = linear11_out1
            .clone()
            .div((constant182_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf2_out1 = div8_out1.erf();
        let constant183_out1 = self.constant183.val();
        let add18_out1 = erf2_out1
            .add((constant183_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul14_out1 = linear11_out1.mul(add18_out1);
        let constant184_out1 = self.constant184.val();
        let mul15_out1 = mul14_out1
            .mul((constant184_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear12_out1 = self.linear12.forward(mul15_out1);
        let add19_out1 = linear12_out1.add(add17_out1);
        add19_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule4 {
    constant185: burn::module::Param<Tensor<1>>,
    constant186: burn::module::Param<Tensor<1>>,
    constant24: burn::module::Param<Tensor<1>>,
    constant25: burn::module::Param<Tensor<1>>,
    linear13: Linear,
    linear14: Linear,
    linear15: Linear,
    linear16: Linear,
    constant207: burn::module::Param<Tensor<1>>,
    constant208: burn::module::Param<Tensor<1>>,
    constant30: burn::module::Param<Tensor<1>>,
    constant31: burn::module::Param<Tensor<1>>,
    linear17: Linear,
    constant209: burn::module::Param<Tensor<1>>,
    constant210: burn::module::Param<Tensor<1>>,
    constant211: burn::module::Param<Tensor<1>>,
    linear18: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule4 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant185: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant186: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant25: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear13 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear14 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear15 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear16 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let constant207: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant208: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant31: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear17 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant209: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant210: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant211: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear18 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        Self {
            constant185,
            constant186,
            constant24,
            constant25,
            linear13,
            linear14,
            linear15,
            linear16,
            constant207,
            constant208,
            constant30,
            constant31,
            linear17,
            constant209,
            constant210,
            constant211,
            linear18,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add19_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean9_out1 = { add19_out1.clone().mean_dim(2usize) };
        let sub6_out1 = add19_out1.sub(reducemean9_out1);
        let constant185_out1 = self.constant185.val();
        let pow5_out1 = sub6_out1
            .clone()
            .powf((constant185_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean10_out1 = { pow5_out1.mean_dim(2usize) };
        let constant186_out1 = self.constant186.val();
        let add20_out1 = reducemean10_out1
            .add((constant186_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant192_out1: [i64; 1] = [32i64];
        let constant190_out1: [i64; 1] = [-1i64];
        let constant191_out1: [i64; 1] = [12i64];
        let concat11_out1: [i64; 4usize] = [
            &unsqueeze19_out1[..],
            &constant190_out1[..],
            &constant191_out1[..],
            &constant192_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze20_out1 = [gather12_out1 as i64];
        let constant196_out1: [i64; 1] = [32i64];
        let constant194_out1: [i64; 1] = [-1i64];
        let constant195_out1: [i64; 1] = [12i64];
        let concat12_out1: [i64; 4usize] = [
            &unsqueeze20_out1[..],
            &constant194_out1[..],
            &constant195_out1[..],
            &constant196_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze21_out1 = [gather12_out1 as i64];
        let constant200_out1: [i64; 1] = [32i64];
        let constant198_out1: [i64; 1] = [-1i64];
        let constant199_out1: [i64; 1] = [12i64];
        let concat13_out1: [i64; 4usize] = [
            &unsqueeze21_out1[..],
            &constant198_out1[..],
            &constant199_out1[..],
            &constant200_out1[..],
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
            let matmul21_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
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
        let constant206_out1: [i64; 1] = [384i64];
        let concat14_out1: [i64; 3usize] = [
            &unsqueeze22_out1[..],
            &unsqueeze23_out1[..],
            &constant206_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape14_out1 = transpose12_out1.reshape(concat14_out1);
        let linear16_out1 = self.linear16.forward(reshape14_out1);
        let add23_out1 = linear16_out1.add(add21_out1);
        let reducemean11_out1 = { add23_out1.clone().mean_dim(2usize) };
        let sub7_out1 = add23_out1.sub(reducemean11_out1);
        let constant207_out1 = self.constant207.val();
        let pow6_out1 = sub7_out1
            .clone()
            .powf((constant207_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean12_out1 = { pow6_out1.mean_dim(2usize) };
        let constant208_out1 = self.constant208.val();
        let add24_out1 = reducemean12_out1
            .add((constant208_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt15_out1 = add24_out1.sqrt();
        let div11_out1 = sub7_out1.div(sqrt15_out1);
        let constant30_out1 = self.constant30.val();
        let mul19_out1 = div11_out1
            .mul((constant30_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant31_out1 = self.constant31.val();
        let add25_out1 = mul19_out1
            .add((constant31_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear17_out1 = self.linear17.forward(add25_out1.clone());
        let constant209_out1 = self.constant209.val();
        let div12_out1 = linear17_out1
            .clone()
            .div((constant209_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf3_out1 = div12_out1.erf();
        let constant210_out1 = self.constant210.val();
        let add26_out1 = erf3_out1
            .add((constant210_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul20_out1 = linear17_out1.mul(add26_out1);
        let constant211_out1 = self.constant211.val();
        let mul21_out1 = mul20_out1
            .mul((constant211_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear18_out1 = self.linear18.forward(mul21_out1);
        let add27_out1 = linear18_out1.add(add25_out1);
        add27_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule5 {
    constant212: burn::module::Param<Tensor<1>>,
    constant213: burn::module::Param<Tensor<1>>,
    constant34: burn::module::Param<Tensor<1>>,
    constant35: burn::module::Param<Tensor<1>>,
    linear19: Linear,
    linear20: Linear,
    linear21: Linear,
    linear22: Linear,
    constant234: burn::module::Param<Tensor<1>>,
    constant235: burn::module::Param<Tensor<1>>,
    constant40: burn::module::Param<Tensor<1>>,
    constant41: burn::module::Param<Tensor<1>>,
    linear23: Linear,
    constant236: burn::module::Param<Tensor<1>>,
    constant237: burn::module::Param<Tensor<1>>,
    constant238: burn::module::Param<Tensor<1>>,
    linear24: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule5 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant212: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant213: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant35: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear19 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear20 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear21 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear22 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let constant234: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant235: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant41: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear23 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant236: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant237: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant238: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear24 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        Self {
            constant212,
            constant213,
            constant34,
            constant35,
            linear19,
            linear20,
            linear21,
            linear22,
            constant234,
            constant235,
            constant40,
            constant41,
            linear23,
            constant236,
            constant237,
            constant238,
            linear24,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add27_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean13_out1 = { add27_out1.clone().mean_dim(2usize) };
        let sub8_out1 = add27_out1.sub(reducemean13_out1);
        let constant212_out1 = self.constant212.val();
        let pow7_out1 = sub8_out1
            .clone()
            .powf((constant212_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean14_out1 = { pow7_out1.mean_dim(2usize) };
        let constant213_out1 = self.constant213.val();
        let add28_out1 = reducemean14_out1
            .add((constant213_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant219_out1: [i64; 1] = [32i64];
        let constant217_out1: [i64; 1] = [-1i64];
        let constant218_out1: [i64; 1] = [12i64];
        let concat15_out1: [i64; 4usize] = [
            &unsqueeze24_out1[..],
            &constant217_out1[..],
            &constant218_out1[..],
            &constant219_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze25_out1 = [gather14_out1 as i64];
        let constant223_out1: [i64; 1] = [32i64];
        let constant221_out1: [i64; 1] = [-1i64];
        let constant222_out1: [i64; 1] = [12i64];
        let concat16_out1: [i64; 4usize] = [
            &unsqueeze25_out1[..],
            &constant221_out1[..],
            &constant222_out1[..],
            &constant223_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze26_out1 = [gather14_out1 as i64];
        let constant227_out1: [i64; 1] = [32i64];
        let constant225_out1: [i64; 1] = [-1i64];
        let constant226_out1: [i64; 1] = [12i64];
        let concat17_out1: [i64; 4usize] = [
            &unsqueeze26_out1[..],
            &constant225_out1[..],
            &constant226_out1[..],
            &constant227_out1[..],
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
            let matmul29_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
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
        let constant233_out1: [i64; 1] = [384i64];
        let concat18_out1: [i64; 3usize] = [
            &unsqueeze27_out1[..],
            &unsqueeze28_out1[..],
            &constant233_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape18_out1 = transpose16_out1.reshape(concat18_out1);
        let linear22_out1 = self.linear22.forward(reshape18_out1);
        let add31_out1 = linear22_out1.add(add29_out1);
        let reducemean15_out1 = { add31_out1.clone().mean_dim(2usize) };
        let sub9_out1 = add31_out1.sub(reducemean15_out1);
        let constant234_out1 = self.constant234.val();
        let pow8_out1 = sub9_out1
            .clone()
            .powf((constant234_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean16_out1 = { pow8_out1.mean_dim(2usize) };
        let constant235_out1 = self.constant235.val();
        let add32_out1 = reducemean16_out1
            .add((constant235_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt20_out1 = add32_out1.sqrt();
        let div15_out1 = sub9_out1.div(sqrt20_out1);
        let constant40_out1 = self.constant40.val();
        let mul25_out1 = div15_out1
            .mul((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant41_out1 = self.constant41.val();
        let add33_out1 = mul25_out1
            .add((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear23_out1 = self.linear23.forward(add33_out1.clone());
        let constant236_out1 = self.constant236.val();
        let div16_out1 = linear23_out1
            .clone()
            .div((constant236_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf4_out1 = div16_out1.erf();
        let constant237_out1 = self.constant237.val();
        let add34_out1 = erf4_out1
            .add((constant237_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul26_out1 = linear23_out1.mul(add34_out1);
        let constant238_out1 = self.constant238.val();
        let mul27_out1 = mul26_out1
            .mul((constant238_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear24_out1 = self.linear24.forward(mul27_out1);
        let add35_out1 = linear24_out1.add(add33_out1);
        add35_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule6 {
    constant239: burn::module::Param<Tensor<1>>,
    constant240: burn::module::Param<Tensor<1>>,
    constant44: burn::module::Param<Tensor<1>>,
    constant45: burn::module::Param<Tensor<1>>,
    linear25: Linear,
    linear26: Linear,
    linear27: Linear,
    linear28: Linear,
    constant261: burn::module::Param<Tensor<1>>,
    constant262: burn::module::Param<Tensor<1>>,
    constant50: burn::module::Param<Tensor<1>>,
    constant51: burn::module::Param<Tensor<1>>,
    linear29: Linear,
    constant263: burn::module::Param<Tensor<1>>,
    constant264: burn::module::Param<Tensor<1>>,
    constant265: burn::module::Param<Tensor<1>>,
    linear30: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule6 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant239: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant240: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant45: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear25 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear26 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear27 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear28 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let constant261: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant262: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant51: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear29 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant263: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant264: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant265: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear30 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        Self {
            constant239,
            constant240,
            constant44,
            constant45,
            linear25,
            linear26,
            linear27,
            linear28,
            constant261,
            constant262,
            constant50,
            constant51,
            linear29,
            constant263,
            constant264,
            constant265,
            linear30,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add35_out1: Tensor<3>, where3_out1: Tensor<4>) -> Tensor<3> {
        let reducemean17_out1 = { add35_out1.clone().mean_dim(2usize) };
        let sub10_out1 = add35_out1.sub(reducemean17_out1);
        let constant239_out1 = self.constant239.val();
        let pow9_out1 = sub10_out1
            .clone()
            .powf((constant239_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean18_out1 = { pow9_out1.mean_dim(2usize) };
        let constant240_out1 = self.constant240.val();
        let add36_out1 = reducemean18_out1
            .add((constant240_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant246_out1: [i64; 1] = [32i64];
        let constant244_out1: [i64; 1] = [-1i64];
        let constant245_out1: [i64; 1] = [12i64];
        let concat19_out1: [i64; 4usize] = [
            &unsqueeze29_out1[..],
            &constant244_out1[..],
            &constant245_out1[..],
            &constant246_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze30_out1 = [gather16_out1 as i64];
        let constant250_out1: [i64; 1] = [32i64];
        let constant248_out1: [i64; 1] = [-1i64];
        let constant249_out1: [i64; 1] = [12i64];
        let concat20_out1: [i64; 4usize] = [
            &unsqueeze30_out1[..],
            &constant248_out1[..],
            &constant249_out1[..],
            &constant250_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze31_out1 = [gather16_out1 as i64];
        let constant254_out1: [i64; 1] = [32i64];
        let constant252_out1: [i64; 1] = [-1i64];
        let constant253_out1: [i64; 1] = [12i64];
        let concat21_out1: [i64; 4usize] = [
            &unsqueeze31_out1[..],
            &constant252_out1[..],
            &constant253_out1[..],
            &constant254_out1[..],
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
            let matmul37_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
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
        let constant260_out1: [i64; 1] = [384i64];
        let concat22_out1: [i64; 3usize] = [
            &unsqueeze32_out1[..],
            &unsqueeze33_out1[..],
            &constant260_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape22_out1 = transpose20_out1.reshape(concat22_out1);
        let linear28_out1 = self.linear28.forward(reshape22_out1);
        let add39_out1 = linear28_out1.add(add37_out1);
        let reducemean19_out1 = { add39_out1.clone().mean_dim(2usize) };
        let sub11_out1 = add39_out1.sub(reducemean19_out1);
        let constant261_out1 = self.constant261.val();
        let pow10_out1 = sub11_out1
            .clone()
            .powf((constant261_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean20_out1 = { pow10_out1.mean_dim(2usize) };
        let constant262_out1 = self.constant262.val();
        let add40_out1 = reducemean20_out1
            .add((constant262_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt25_out1 = add40_out1.sqrt();
        let div19_out1 = sub11_out1.div(sqrt25_out1);
        let constant50_out1 = self.constant50.val();
        let mul31_out1 = div19_out1
            .mul((constant50_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant51_out1 = self.constant51.val();
        let add41_out1 = mul31_out1
            .add((constant51_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear29_out1 = self.linear29.forward(add41_out1.clone());
        let constant263_out1 = self.constant263.val();
        let div20_out1 = linear29_out1
            .clone()
            .div((constant263_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf5_out1 = div20_out1.erf();
        let constant264_out1 = self.constant264.val();
        let add42_out1 = erf5_out1
            .add((constant264_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul32_out1 = linear29_out1.mul(add42_out1);
        let constant265_out1 = self.constant265.val();
        let mul33_out1 = mul32_out1
            .mul((constant265_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear30_out1 = self.linear30.forward(mul33_out1);
        let add43_out1 = linear30_out1.add(add41_out1);
        add43_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule7 {
    constant266: burn::module::Param<Tensor<1>>,
    constant267: burn::module::Param<Tensor<1>>,
    constant54: burn::module::Param<Tensor<1>>,
    constant55: burn::module::Param<Tensor<1>>,
    linear31: Linear,
    linear32: Linear,
    linear33: Linear,
    linear34: Linear,
    constant288: burn::module::Param<Tensor<1>>,
    constant289: burn::module::Param<Tensor<1>>,
    constant60: burn::module::Param<Tensor<1>>,
    constant61: burn::module::Param<Tensor<1>>,
    linear35: Linear,
    constant290: burn::module::Param<Tensor<1>>,
    constant291: burn::module::Param<Tensor<1>>,
    constant292: burn::module::Param<Tensor<1>>,
    linear36: Linear,
    constant293: burn::module::Param<Tensor<1>>,
    constant294: burn::module::Param<Tensor<1>>,
    constant64: burn::module::Param<Tensor<1>>,
    constant65: burn::module::Param<Tensor<1>>,
    linear37: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule7 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant266: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant267: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant55: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear31 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear32 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear33 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear34 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let constant288: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant289: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant61: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear35 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant290: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant291: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant292: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear36 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        let constant293: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant294: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant65: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear37 = LinearConfig::new(384, 384)
            .with_bias(true)
            .with_layout(LinearLayout::Col)
            .init(device);
        Self {
            constant266,
            constant267,
            constant54,
            constant55,
            linear31,
            linear32,
            linear33,
            linear34,
            constant288,
            constant289,
            constant60,
            constant61,
            linear35,
            constant290,
            constant291,
            constant292,
            linear36,
            constant293,
            constant294,
            constant64,
            constant65,
            linear37,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add43_out1: Tensor<3>,
        where3_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<2>) {
        let reducemean21_out1 = { add43_out1.clone().mean_dim(2usize) };
        let sub12_out1 = add43_out1.sub(reducemean21_out1);
        let constant266_out1 = self.constant266.val();
        let pow11_out1 = sub12_out1
            .clone()
            .powf((constant266_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean22_out1 = { pow11_out1.mean_dim(2usize) };
        let constant267_out1 = self.constant267.val();
        let add44_out1 = reducemean22_out1
            .add((constant267_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant273_out1: [i64; 1] = [32i64];
        let constant271_out1: [i64; 1] = [-1i64];
        let constant272_out1: [i64; 1] = [12i64];
        let concat23_out1: [i64; 4usize] = [
            &unsqueeze34_out1[..],
            &constant271_out1[..],
            &constant272_out1[..],
            &constant273_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze35_out1 = [gather18_out1 as i64];
        let constant277_out1: [i64; 1] = [32i64];
        let constant275_out1: [i64; 1] = [-1i64];
        let constant276_out1: [i64; 1] = [12i64];
        let concat24_out1: [i64; 4usize] = [
            &unsqueeze35_out1[..],
            &constant275_out1[..],
            &constant276_out1[..],
            &constant277_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze36_out1 = [gather18_out1 as i64];
        let constant281_out1: [i64; 1] = [32i64];
        let constant279_out1: [i64; 1] = [-1i64];
        let constant280_out1: [i64; 1] = [12i64];
        let concat25_out1: [i64; 4usize] = [
            &unsqueeze36_out1[..],
            &constant279_out1[..],
            &constant280_out1[..],
            &constant281_out1[..],
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
            let matmul45_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
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
        let constant287_out1: [i64; 1] = [384i64];
        let concat26_out1: [i64; 3usize] = [
            &unsqueeze37_out1[..],
            &unsqueeze38_out1[..],
            &constant287_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape26_out1 = transpose24_out1.reshape(concat26_out1);
        let linear34_out1 = self.linear34.forward(reshape26_out1);
        let add47_out1 = linear34_out1.add(add45_out1);
        let reducemean23_out1 = { add47_out1.clone().mean_dim(2usize) };
        let sub13_out1 = add47_out1.sub(reducemean23_out1);
        let constant288_out1 = self.constant288.val();
        let pow12_out1 = sub13_out1
            .clone()
            .powf((constant288_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean24_out1 = { pow12_out1.mean_dim(2usize) };
        let constant289_out1 = self.constant289.val();
        let add48_out1 = reducemean24_out1
            .add((constant289_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt30_out1 = add48_out1.sqrt();
        let div23_out1 = sub13_out1.div(sqrt30_out1);
        let constant60_out1 = self.constant60.val();
        let mul37_out1 = div23_out1
            .mul((constant60_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant61_out1 = self.constant61.val();
        let add49_out1 = mul37_out1
            .add((constant61_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear35_out1 = self.linear35.forward(add49_out1.clone());
        let constant290_out1 = self.constant290.val();
        let div24_out1 = linear35_out1
            .clone()
            .div((constant290_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf6_out1 = div24_out1.erf();
        let constant291_out1 = self.constant291.val();
        let add50_out1 = erf6_out1
            .add((constant291_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul38_out1 = linear35_out1.mul(add50_out1);
        let constant292_out1 = self.constant292.val();
        let mul39_out1 = mul38_out1
            .mul((constant292_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear36_out1 = self.linear36.forward(mul39_out1);
        let add51_out1 = linear36_out1.add(add49_out1);
        let reducemean25_out1 = { add51_out1.clone().mean_dim(2usize) };
        let sub14_out1 = add51_out1.sub(reducemean25_out1);
        let constant293_out1 = self.constant293.val();
        let pow13_out1 = sub14_out1
            .clone()
            .powf((constant293_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean26_out1 = { pow13_out1.mean_dim(2usize) };
        let constant294_out1 = self.constant294.val();
        let add52_out1 = reducemean26_out1
            .add((constant294_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt31_out1 = add52_out1.sqrt();
        let div25_out1 = sub14_out1.div(sqrt31_out1);
        let constant64_out1 = self.constant64.val();
        let mul40_out1 = div25_out1
            .mul((constant64_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant65_out1 = self.constant65.val();
        let add53_out1 = mul40_out1
            .add((constant65_out1).unsqueeze_dims(&[0isize, 1isize]));
        let gather20_out1 = {
            let sliced = add53_out1.clone().slice(s![.., 0, ..]);
            sliced.squeeze_dim::<2usize>(1)
        };
        let linear37_out1 = self.linear37.forward(gather20_out1);
        let tanh1_out1 = linear37_out1.tanh();
        (add53_out1, tanh1_out1)
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
        Self {
            submodule1,
            submodule2,
            submodule3,
            submodule4,
            submodule5,
            submodule6,
            submodule7,
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
        let (add53_out1, tanh1_out1) = self.submodule7.forward(add43_out1, where3_out1);
        (add53_out1, tanh1_out1)
    }
}
