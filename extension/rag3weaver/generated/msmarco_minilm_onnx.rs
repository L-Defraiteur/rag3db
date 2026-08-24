// Generated from ONNX "cross-encoder/ms-marco-MiniLM-L-6-v2 onnx/model.onnx" by burn-onnx
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
    constant1: burn::module::Param<Tensor<2>>,
    constant3: burn::module::Param<Tensor<2>>,
    constant2: burn::module::Param<Tensor<2>>,
    constant113: burn::module::Param<Tensor<1>>,
    constant114: burn::module::Param<Tensor<1>>,
    constant4: burn::module::Param<Tensor<1>>,
    constant5: burn::module::Param<Tensor<1>>,
    constant124: burn::module::Param<Tensor<1, Int>>,
    constant125: burn::module::Param<Tensor<1>>,
    linear1: Linear,
    linear2: Linear,
    linear3: Linear,
    linear4: Linear,
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
            >::zeros([1, 512], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [1, 512].into(),
        );
        let constant1: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([30522, 384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [30522, 384].into(),
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
            >::zeros([512, 384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [512, 384].into(),
        );
        let constant113: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant114: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant124: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
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
        let constant125: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        Self {
            constant107,
            constant1,
            constant3,
            constant2,
            constant113,
            constant114,
            constant4,
            constant5,
            constant124,
            constant125,
            linear1,
            linear2,
            linear3,
            linear4,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        input_ids: Tensor<2, Int>,
        token_type_ids: Tensor<2, Int>,
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
        let gather1_out1 = shape1_out1[1] as i64;
        let unsqueeze1_out1 = [gather1_out1 as i64];
        let constant107_out1 = self.constant107.val();
        let slice1_out1 = constant107_out1.slice(s![.., 0..unsqueeze1_out1[0]]);
        let constant1_out1 = self.constant1.val();
        let gather2_out1 = constant1_out1.take::<2, 3>(0, input_ids);
        let constant3_out1 = self.constant3.val();
        let gather3_out1 = constant3_out1.take::<2, 3>(0, token_type_ids);
        let add1_out1 = gather2_out1.add(gather3_out1);
        let constant2_out1 = self.constant2.val();
        let gather4_out1 = constant2_out1.take::<2, 3>(0, slice1_out1);
        let add2_out1 = add1_out1.add(gather4_out1);
        let reducemean1_out1 = { add2_out1.clone().mean_dim(2usize) };
        let sub1_out1 = add2_out1.sub(reducemean1_out1);
        let constant113_out1 = self.constant113.val();
        let pow1_out1 = sub1_out1
            .clone()
            .powf((constant113_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean2_out1 = { pow1_out1.mean_dim(2usize) };
        let constant114_out1 = self.constant114.val();
        let add3_out1 = reducemean2_out1
            .add((constant114_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt1_out1 = add3_out1.sqrt();
        let div1_out1 = sub1_out1.div(sqrt1_out1);
        let constant4_out1 = self.constant4.val();
        let mul1_out1 = div1_out1
            .mul((constant4_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant5_out1 = self.constant5.val();
        let add4_out1 = mul1_out1
            .add((constant5_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape2_out1: [i64; 2] = {
            let axes = &attention_mask.clone().dims()[0..2];
            let mut output = [0i64; 2];
            for i in 0..2 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather5_out1 = shape2_out1[0] as i64;
        let gather6_out1 = shape2_out1[1] as i64;
        let unsqueeze2_out1: Tensor<3, Int> = attention_mask.unsqueeze_dims::<3>(&[1]);
        let unsqueeze3_out1: Tensor<4, Int> = unsqueeze2_out1.unsqueeze_dims::<4>(&[2]);
        let unsqueeze4_out1 = [gather5_out1 as i64];
        let unsqueeze5_out1 = [gather1_out1 as i64];
        let unsqueeze6_out1 = [gather6_out1 as i64];
        let constant120_out1: [i64; 1] = [1i64];
        let concat1_out1: [i64; 4usize] = [
            &unsqueeze4_out1[..],
            &constant120_out1[..],
            &unsqueeze5_out1[..],
            &unsqueeze6_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape1_out1 = concat1_out1;
        let shape4_out1: [i64; 1] = [4i64];
        let constantofshape1_out1 = Tensor::<
            1,
            Int,
        >::from_data(
                burn::tensor::TensorData::from([1i64 as i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .reshape([1])
            .expand(shape4_out1);
        let constant124_out1 = self.constant124.val();
        let mul2_out1 = constantofshape1_out1.clone().mul(constant124_out1);
        let equal1_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(reshape1_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(mul2_out1)
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
            let onnx_shape: [i64; 4usize] = TryInto::<
                [i64; 4usize],
            >::try_into(where1_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze3_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..4usize {
                let dim_offset = 4usize - 4usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze3_out1.expand(shape)
        };
        let cast1_out1 = expand1_out1.float().cast(burn::tensor::DType::F32);
        let constant125_out1 = self.constant125.val();
        let sub2_out1 = (constant125_out1)
            .unsqueeze_dims(&[0isize, 1isize, 2isize])
            .sub(cast1_out1);
        let cast2_out1 = sub2_out1.clone().bool();
        let constant126_out1 = -340282350000000000000000000000000000000f32;
        let where2_out1 = sub2_out1.mask_fill(cast2_out1, constant126_out1);
        let shape5_out1: [i64; 3] = {
            let axes = &add4_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather7_out1 = shape5_out1[0] as i64;
        let gather8_out1 = shape5_out1[1] as i64;
        let linear1_out1 = self.linear1.forward(add4_out1.clone());
        let shape7_out1: [i64; 3] = {
            let axes = &linear1_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather9_out1 = shape7_out1[0] as i64;
        let gather10_out1 = shape7_out1[1] as i64;
        let unsqueeze7_out1 = [gather9_out1 as i64];
        let unsqueeze8_out1 = [gather10_out1 as i64];
        let constant134_out1: [i64; 1] = [32i64];
        let constant133_out1: [i64; 1] = [12i64];
        let concat2_out1: [i64; 4usize] = [
            &unsqueeze7_out1[..],
            &unsqueeze8_out1[..],
            &constant133_out1[..],
            &constant134_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape2_out1 = linear1_out1.reshape(concat2_out1);
        let transpose1_out1 = reshape2_out1.permute([0, 2, 1, 3]);
        let linear2_out1 = self.linear2.forward(add4_out1.clone());
        let shape9_out1: [i64; 3] = {
            let axes = &linear2_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather11_out1 = shape9_out1[0] as i64;
        let gather12_out1 = shape9_out1[1] as i64;
        let unsqueeze9_out1 = [gather11_out1 as i64];
        let unsqueeze10_out1 = [gather12_out1 as i64];
        let constant140_out1: [i64; 1] = [32i64];
        let constant139_out1: [i64; 1] = [12i64];
        let concat3_out1: [i64; 4usize] = [
            &unsqueeze9_out1[..],
            &unsqueeze10_out1[..],
            &constant139_out1[..],
            &constant140_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape3_out1 = linear2_out1.reshape(concat3_out1);
        let linear3_out1 = self.linear3.forward(add4_out1.clone());
        let shape11_out1: [i64; 3] = {
            let axes = &linear3_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather13_out1 = shape11_out1[0] as i64;
        let gather14_out1 = shape11_out1[1] as i64;
        let unsqueeze11_out1 = [gather13_out1 as i64];
        let unsqueeze12_out1 = [gather14_out1 as i64];
        let constant146_out1: [i64; 1] = [32i64];
        let constant145_out1: [i64; 1] = [12i64];
        let concat4_out1: [i64; 4usize] = [
            &unsqueeze11_out1[..],
            &unsqueeze12_out1[..],
            &constant145_out1[..],
            &constant146_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape4_out1 = linear3_out1.reshape(concat4_out1);
        let transpose2_out1 = reshape4_out1.permute([0, 2, 1, 3]);
        let transpose3_out1 = reshape3_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1.clone()),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul5_out1,)
        };
        let transpose4_out1 = matmul5_out1.permute([0, 2, 1, 3]);
        let unsqueeze13_out1 = [gather7_out1 as i64];
        let unsqueeze14_out1 = [gather8_out1 as i64];
        let constant152_out1: [i64; 1] = [384i64];
        let concat5_out1: [i64; 3usize] = [
            &unsqueeze13_out1[..],
            &unsqueeze14_out1[..],
            &constant152_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape5_out1 = transpose4_out1.reshape(concat5_out1);
        let linear4_out1 = self.linear4.forward(reshape5_out1);
        let add6_out1 = linear4_out1.add(add4_out1);
        (add6_out1, where2_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule2 {
    constant153: burn::module::Param<Tensor<1>>,
    constant154: burn::module::Param<Tensor<1>>,
    constant10: burn::module::Param<Tensor<1>>,
    constant11: burn::module::Param<Tensor<1>>,
    linear5: Linear,
    constant155: burn::module::Param<Tensor<1>>,
    constant156: burn::module::Param<Tensor<1>>,
    constant157: burn::module::Param<Tensor<1>>,
    linear6: Linear,
    constant158: burn::module::Param<Tensor<1>>,
    constant159: burn::module::Param<Tensor<1>>,
    constant14: burn::module::Param<Tensor<1>>,
    constant15: burn::module::Param<Tensor<1>>,
    linear7: Linear,
    linear8: Linear,
    linear9: Linear,
    linear10: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule2 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
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
        Self {
            constant153,
            constant154,
            constant10,
            constant11,
            linear5,
            constant155,
            constant156,
            constant157,
            linear6,
            constant158,
            constant159,
            constant14,
            constant15,
            linear7,
            linear8,
            linear9,
            linear10,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add6_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean3_out1 = { add6_out1.clone().mean_dim(2usize) };
        let sub3_out1 = add6_out1.sub(reducemean3_out1);
        let constant153_out1 = self.constant153.val();
        let pow2_out1 = sub3_out1
            .clone()
            .powf((constant153_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean4_out1 = { pow2_out1.mean_dim(2usize) };
        let constant154_out1 = self.constant154.val();
        let add7_out1 = reducemean4_out1
            .add((constant154_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt5_out1 = add7_out1.sqrt();
        let div3_out1 = sub3_out1.div(sqrt5_out1);
        let constant10_out1 = self.constant10.val();
        let mul5_out1 = div3_out1
            .mul((constant10_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant11_out1 = self.constant11.val();
        let add8_out1 = mul5_out1
            .add((constant11_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear5_out1 = self.linear5.forward(add8_out1.clone());
        let constant155_out1 = self.constant155.val();
        let div4_out1 = linear5_out1
            .clone()
            .div((constant155_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf1_out1 = div4_out1.erf();
        let constant156_out1 = self.constant156.val();
        let add9_out1 = erf1_out1
            .add((constant156_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul6_out1 = linear5_out1.mul(add9_out1);
        let constant157_out1 = self.constant157.val();
        let mul7_out1 = mul6_out1
            .mul((constant157_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear6_out1 = self.linear6.forward(mul7_out1);
        let add10_out1 = linear6_out1.add(add8_out1);
        let reducemean5_out1 = { add10_out1.clone().mean_dim(2usize) };
        let sub4_out1 = add10_out1.sub(reducemean5_out1);
        let constant158_out1 = self.constant158.val();
        let pow3_out1 = sub4_out1
            .clone()
            .powf((constant158_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean6_out1 = { pow3_out1.mean_dim(2usize) };
        let constant159_out1 = self.constant159.val();
        let add11_out1 = reducemean6_out1
            .add((constant159_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt6_out1 = add11_out1.sqrt();
        let div5_out1 = sub4_out1.div(sqrt6_out1);
        let constant14_out1 = self.constant14.val();
        let mul8_out1 = div5_out1
            .mul((constant14_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant15_out1 = self.constant15.val();
        let add12_out1 = mul8_out1
            .add((constant15_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape14_out1: [i64; 3] = {
            let axes = &add12_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather15_out1 = shape14_out1[0] as i64;
        let gather16_out1 = shape14_out1[1] as i64;
        let linear7_out1 = self.linear7.forward(add12_out1.clone());
        let shape16_out1: [i64; 3] = {
            let axes = &linear7_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather17_out1 = shape16_out1[0] as i64;
        let gather18_out1 = shape16_out1[1] as i64;
        let unsqueeze15_out1 = [gather17_out1 as i64];
        let unsqueeze16_out1 = [gather18_out1 as i64];
        let constant167_out1: [i64; 1] = [32i64];
        let constant166_out1: [i64; 1] = [12i64];
        let concat6_out1: [i64; 4usize] = [
            &unsqueeze15_out1[..],
            &unsqueeze16_out1[..],
            &constant166_out1[..],
            &constant167_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape6_out1 = linear7_out1.reshape(concat6_out1);
        let transpose5_out1 = reshape6_out1.permute([0, 2, 1, 3]);
        let linear8_out1 = self.linear8.forward(add12_out1.clone());
        let shape18_out1: [i64; 3] = {
            let axes = &linear8_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather19_out1 = shape18_out1[0] as i64;
        let gather20_out1 = shape18_out1[1] as i64;
        let unsqueeze17_out1 = [gather19_out1 as i64];
        let unsqueeze18_out1 = [gather20_out1 as i64];
        let constant173_out1: [i64; 1] = [32i64];
        let constant172_out1: [i64; 1] = [12i64];
        let concat7_out1: [i64; 4usize] = [
            &unsqueeze17_out1[..],
            &unsqueeze18_out1[..],
            &constant172_out1[..],
            &constant173_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape7_out1 = linear8_out1.reshape(concat7_out1);
        let linear9_out1 = self.linear9.forward(add12_out1.clone());
        let shape20_out1: [i64; 3] = {
            let axes = &linear9_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather21_out1 = shape20_out1[0] as i64;
        let gather22_out1 = shape20_out1[1] as i64;
        let unsqueeze19_out1 = [gather21_out1 as i64];
        let unsqueeze20_out1 = [gather22_out1 as i64];
        let constant179_out1: [i64; 1] = [32i64];
        let constant178_out1: [i64; 1] = [12i64];
        let concat8_out1: [i64; 4usize] = [
            &unsqueeze19_out1[..],
            &unsqueeze20_out1[..],
            &constant178_out1[..],
            &constant179_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape8_out1 = linear9_out1.reshape(concat8_out1);
        let transpose6_out1 = reshape8_out1.permute([0, 2, 1, 3]);
        let transpose7_out1 = reshape7_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul13_out1,)
        };
        let transpose8_out1 = matmul13_out1.permute([0, 2, 1, 3]);
        let unsqueeze21_out1 = [gather15_out1 as i64];
        let unsqueeze22_out1 = [gather16_out1 as i64];
        let constant185_out1: [i64; 1] = [384i64];
        let concat9_out1: [i64; 3usize] = [
            &unsqueeze21_out1[..],
            &unsqueeze22_out1[..],
            &constant185_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape9_out1 = transpose8_out1.reshape(concat9_out1);
        let linear10_out1 = self.linear10.forward(reshape9_out1);
        let add14_out1 = linear10_out1.add(add12_out1);
        add14_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule3 {
    constant186: burn::module::Param<Tensor<1>>,
    constant187: burn::module::Param<Tensor<1>>,
    constant20: burn::module::Param<Tensor<1>>,
    constant21: burn::module::Param<Tensor<1>>,
    linear11: Linear,
    constant188: burn::module::Param<Tensor<1>>,
    constant189: burn::module::Param<Tensor<1>>,
    constant190: burn::module::Param<Tensor<1>>,
    linear12: Linear,
    constant191: burn::module::Param<Tensor<1>>,
    constant192: burn::module::Param<Tensor<1>>,
    constant24: burn::module::Param<Tensor<1>>,
    constant25: burn::module::Param<Tensor<1>>,
    linear13: Linear,
    linear14: Linear,
    linear15: Linear,
    linear16: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule3 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant186: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant187: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant188: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant189: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant190: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant191: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant192: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        Self {
            constant186,
            constant187,
            constant20,
            constant21,
            linear11,
            constant188,
            constant189,
            constant190,
            linear12,
            constant191,
            constant192,
            constant24,
            constant25,
            linear13,
            linear14,
            linear15,
            linear16,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add14_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean7_out1 = { add14_out1.clone().mean_dim(2usize) };
        let sub5_out1 = add14_out1.sub(reducemean7_out1);
        let constant186_out1 = self.constant186.val();
        let pow4_out1 = sub5_out1
            .clone()
            .powf((constant186_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean8_out1 = { pow4_out1.mean_dim(2usize) };
        let constant187_out1 = self.constant187.val();
        let add15_out1 = reducemean8_out1
            .add((constant187_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt10_out1 = add15_out1.sqrt();
        let div7_out1 = sub5_out1.div(sqrt10_out1);
        let constant20_out1 = self.constant20.val();
        let mul11_out1 = div7_out1
            .mul((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant21_out1 = self.constant21.val();
        let add16_out1 = mul11_out1
            .add((constant21_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear11_out1 = self.linear11.forward(add16_out1.clone());
        let constant188_out1 = self.constant188.val();
        let div8_out1 = linear11_out1
            .clone()
            .div((constant188_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf2_out1 = div8_out1.erf();
        let constant189_out1 = self.constant189.val();
        let add17_out1 = erf2_out1
            .add((constant189_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul12_out1 = linear11_out1.mul(add17_out1);
        let constant190_out1 = self.constant190.val();
        let mul13_out1 = mul12_out1
            .mul((constant190_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear12_out1 = self.linear12.forward(mul13_out1);
        let add18_out1 = linear12_out1.add(add16_out1);
        let reducemean9_out1 = { add18_out1.clone().mean_dim(2usize) };
        let sub6_out1 = add18_out1.sub(reducemean9_out1);
        let constant191_out1 = self.constant191.val();
        let pow5_out1 = sub6_out1
            .clone()
            .powf((constant191_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean10_out1 = { pow5_out1.mean_dim(2usize) };
        let constant192_out1 = self.constant192.val();
        let add19_out1 = reducemean10_out1
            .add((constant192_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt11_out1 = add19_out1.sqrt();
        let div9_out1 = sub6_out1.div(sqrt11_out1);
        let constant24_out1 = self.constant24.val();
        let mul14_out1 = div9_out1
            .mul((constant24_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant25_out1 = self.constant25.val();
        let add20_out1 = mul14_out1
            .add((constant25_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape23_out1: [i64; 3] = {
            let axes = &add20_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather23_out1 = shape23_out1[0] as i64;
        let gather24_out1 = shape23_out1[1] as i64;
        let linear13_out1 = self.linear13.forward(add20_out1.clone());
        let shape25_out1: [i64; 3] = {
            let axes = &linear13_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather25_out1 = shape25_out1[0] as i64;
        let gather26_out1 = shape25_out1[1] as i64;
        let unsqueeze23_out1 = [gather25_out1 as i64];
        let unsqueeze24_out1 = [gather26_out1 as i64];
        let constant200_out1: [i64; 1] = [32i64];
        let constant199_out1: [i64; 1] = [12i64];
        let concat10_out1: [i64; 4usize] = [
            &unsqueeze23_out1[..],
            &unsqueeze24_out1[..],
            &constant199_out1[..],
            &constant200_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape10_out1 = linear13_out1.reshape(concat10_out1);
        let transpose9_out1 = reshape10_out1.permute([0, 2, 1, 3]);
        let linear14_out1 = self.linear14.forward(add20_out1.clone());
        let shape27_out1: [i64; 3] = {
            let axes = &linear14_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather27_out1 = shape27_out1[0] as i64;
        let gather28_out1 = shape27_out1[1] as i64;
        let unsqueeze25_out1 = [gather27_out1 as i64];
        let unsqueeze26_out1 = [gather28_out1 as i64];
        let constant206_out1: [i64; 1] = [32i64];
        let constant205_out1: [i64; 1] = [12i64];
        let concat11_out1: [i64; 4usize] = [
            &unsqueeze25_out1[..],
            &unsqueeze26_out1[..],
            &constant205_out1[..],
            &constant206_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape11_out1 = linear14_out1.reshape(concat11_out1);
        let linear15_out1 = self.linear15.forward(add20_out1.clone());
        let shape29_out1: [i64; 3] = {
            let axes = &linear15_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather29_out1 = shape29_out1[0] as i64;
        let gather30_out1 = shape29_out1[1] as i64;
        let unsqueeze27_out1 = [gather29_out1 as i64];
        let unsqueeze28_out1 = [gather30_out1 as i64];
        let constant212_out1: [i64; 1] = [32i64];
        let constant211_out1: [i64; 1] = [12i64];
        let concat12_out1: [i64; 4usize] = [
            &unsqueeze27_out1[..],
            &unsqueeze28_out1[..],
            &constant211_out1[..],
            &constant212_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape12_out1 = linear15_out1.reshape(concat12_out1);
        let transpose10_out1 = reshape12_out1.permute([0, 2, 1, 3]);
        let transpose11_out1 = reshape11_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul21_out1,)
        };
        let transpose12_out1 = matmul21_out1.permute([0, 2, 1, 3]);
        let unsqueeze29_out1 = [gather23_out1 as i64];
        let unsqueeze30_out1 = [gather24_out1 as i64];
        let constant218_out1: [i64; 1] = [384i64];
        let concat13_out1: [i64; 3usize] = [
            &unsqueeze29_out1[..],
            &unsqueeze30_out1[..],
            &constant218_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape13_out1 = transpose12_out1.reshape(concat13_out1);
        let linear16_out1 = self.linear16.forward(reshape13_out1);
        let add22_out1 = linear16_out1.add(add20_out1);
        add22_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule4 {
    constant219: burn::module::Param<Tensor<1>>,
    constant220: burn::module::Param<Tensor<1>>,
    constant30: burn::module::Param<Tensor<1>>,
    constant31: burn::module::Param<Tensor<1>>,
    linear17: Linear,
    constant221: burn::module::Param<Tensor<1>>,
    constant222: burn::module::Param<Tensor<1>>,
    constant223: burn::module::Param<Tensor<1>>,
    linear18: Linear,
    constant224: burn::module::Param<Tensor<1>>,
    constant225: burn::module::Param<Tensor<1>>,
    constant34: burn::module::Param<Tensor<1>>,
    constant35: burn::module::Param<Tensor<1>>,
    linear19: Linear,
    linear20: Linear,
    linear21: Linear,
    linear22: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule4 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant219: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant220: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant221: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant222: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant223: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant224: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant225: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        Self {
            constant219,
            constant220,
            constant30,
            constant31,
            linear17,
            constant221,
            constant222,
            constant223,
            linear18,
            constant224,
            constant225,
            constant34,
            constant35,
            linear19,
            linear20,
            linear21,
            linear22,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add22_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean11_out1 = { add22_out1.clone().mean_dim(2usize) };
        let sub7_out1 = add22_out1.sub(reducemean11_out1);
        let constant219_out1 = self.constant219.val();
        let pow6_out1 = sub7_out1
            .clone()
            .powf((constant219_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean12_out1 = { pow6_out1.mean_dim(2usize) };
        let constant220_out1 = self.constant220.val();
        let add23_out1 = reducemean12_out1
            .add((constant220_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt15_out1 = add23_out1.sqrt();
        let div11_out1 = sub7_out1.div(sqrt15_out1);
        let constant30_out1 = self.constant30.val();
        let mul17_out1 = div11_out1
            .mul((constant30_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant31_out1 = self.constant31.val();
        let add24_out1 = mul17_out1
            .add((constant31_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear17_out1 = self.linear17.forward(add24_out1.clone());
        let constant221_out1 = self.constant221.val();
        let div12_out1 = linear17_out1
            .clone()
            .div((constant221_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf3_out1 = div12_out1.erf();
        let constant222_out1 = self.constant222.val();
        let add25_out1 = erf3_out1
            .add((constant222_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul18_out1 = linear17_out1.mul(add25_out1);
        let constant223_out1 = self.constant223.val();
        let mul19_out1 = mul18_out1
            .mul((constant223_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear18_out1 = self.linear18.forward(mul19_out1);
        let add26_out1 = linear18_out1.add(add24_out1);
        let reducemean13_out1 = { add26_out1.clone().mean_dim(2usize) };
        let sub8_out1 = add26_out1.sub(reducemean13_out1);
        let constant224_out1 = self.constant224.val();
        let pow7_out1 = sub8_out1
            .clone()
            .powf((constant224_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean14_out1 = { pow7_out1.mean_dim(2usize) };
        let constant225_out1 = self.constant225.val();
        let add27_out1 = reducemean14_out1
            .add((constant225_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt16_out1 = add27_out1.sqrt();
        let div13_out1 = sub8_out1.div(sqrt16_out1);
        let constant34_out1 = self.constant34.val();
        let mul20_out1 = div13_out1
            .mul((constant34_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant35_out1 = self.constant35.val();
        let add28_out1 = mul20_out1
            .add((constant35_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape32_out1: [i64; 3] = {
            let axes = &add28_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather31_out1 = shape32_out1[0] as i64;
        let gather32_out1 = shape32_out1[1] as i64;
        let linear19_out1 = self.linear19.forward(add28_out1.clone());
        let shape34_out1: [i64; 3] = {
            let axes = &linear19_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather33_out1 = shape34_out1[0] as i64;
        let gather34_out1 = shape34_out1[1] as i64;
        let unsqueeze31_out1 = [gather33_out1 as i64];
        let unsqueeze32_out1 = [gather34_out1 as i64];
        let constant233_out1: [i64; 1] = [32i64];
        let constant232_out1: [i64; 1] = [12i64];
        let concat14_out1: [i64; 4usize] = [
            &unsqueeze31_out1[..],
            &unsqueeze32_out1[..],
            &constant232_out1[..],
            &constant233_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape14_out1 = linear19_out1.reshape(concat14_out1);
        let transpose13_out1 = reshape14_out1.permute([0, 2, 1, 3]);
        let linear20_out1 = self.linear20.forward(add28_out1.clone());
        let shape36_out1: [i64; 3] = {
            let axes = &linear20_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather35_out1 = shape36_out1[0] as i64;
        let gather36_out1 = shape36_out1[1] as i64;
        let unsqueeze33_out1 = [gather35_out1 as i64];
        let unsqueeze34_out1 = [gather36_out1 as i64];
        let constant239_out1: [i64; 1] = [32i64];
        let constant238_out1: [i64; 1] = [12i64];
        let concat15_out1: [i64; 4usize] = [
            &unsqueeze33_out1[..],
            &unsqueeze34_out1[..],
            &constant238_out1[..],
            &constant239_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape15_out1 = linear20_out1.reshape(concat15_out1);
        let linear21_out1 = self.linear21.forward(add28_out1.clone());
        let shape38_out1: [i64; 3] = {
            let axes = &linear21_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather37_out1 = shape38_out1[0] as i64;
        let gather38_out1 = shape38_out1[1] as i64;
        let unsqueeze35_out1 = [gather37_out1 as i64];
        let unsqueeze36_out1 = [gather38_out1 as i64];
        let constant245_out1: [i64; 1] = [32i64];
        let constant244_out1: [i64; 1] = [12i64];
        let concat16_out1: [i64; 4usize] = [
            &unsqueeze35_out1[..],
            &unsqueeze36_out1[..],
            &constant244_out1[..],
            &constant245_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape16_out1 = linear21_out1.reshape(concat16_out1);
        let transpose14_out1 = reshape16_out1.permute([0, 2, 1, 3]);
        let transpose15_out1 = reshape15_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul29_out1,)
        };
        let transpose16_out1 = matmul29_out1.permute([0, 2, 1, 3]);
        let unsqueeze37_out1 = [gather31_out1 as i64];
        let unsqueeze38_out1 = [gather32_out1 as i64];
        let constant251_out1: [i64; 1] = [384i64];
        let concat17_out1: [i64; 3usize] = [
            &unsqueeze37_out1[..],
            &unsqueeze38_out1[..],
            &constant251_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape17_out1 = transpose16_out1.reshape(concat17_out1);
        let linear22_out1 = self.linear22.forward(reshape17_out1);
        let add30_out1 = linear22_out1.add(add28_out1);
        add30_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule5 {
    constant252: burn::module::Param<Tensor<1>>,
    constant253: burn::module::Param<Tensor<1>>,
    constant40: burn::module::Param<Tensor<1>>,
    constant41: burn::module::Param<Tensor<1>>,
    linear23: Linear,
    constant254: burn::module::Param<Tensor<1>>,
    constant255: burn::module::Param<Tensor<1>>,
    constant256: burn::module::Param<Tensor<1>>,
    linear24: Linear,
    constant257: burn::module::Param<Tensor<1>>,
    constant258: burn::module::Param<Tensor<1>>,
    constant44: burn::module::Param<Tensor<1>>,
    constant45: burn::module::Param<Tensor<1>>,
    linear25: Linear,
    linear26: Linear,
    linear27: Linear,
    linear28: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule5 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant252: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant253: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant254: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant255: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant256: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant257: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant258: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        Self {
            constant252,
            constant253,
            constant40,
            constant41,
            linear23,
            constant254,
            constant255,
            constant256,
            linear24,
            constant257,
            constant258,
            constant44,
            constant45,
            linear25,
            linear26,
            linear27,
            linear28,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add30_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean15_out1 = { add30_out1.clone().mean_dim(2usize) };
        let sub9_out1 = add30_out1.sub(reducemean15_out1);
        let constant252_out1 = self.constant252.val();
        let pow8_out1 = sub9_out1
            .clone()
            .powf((constant252_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean16_out1 = { pow8_out1.mean_dim(2usize) };
        let constant253_out1 = self.constant253.val();
        let add31_out1 = reducemean16_out1
            .add((constant253_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt20_out1 = add31_out1.sqrt();
        let div15_out1 = sub9_out1.div(sqrt20_out1);
        let constant40_out1 = self.constant40.val();
        let mul23_out1 = div15_out1
            .mul((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant41_out1 = self.constant41.val();
        let add32_out1 = mul23_out1
            .add((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear23_out1 = self.linear23.forward(add32_out1.clone());
        let constant254_out1 = self.constant254.val();
        let div16_out1 = linear23_out1
            .clone()
            .div((constant254_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf4_out1 = div16_out1.erf();
        let constant255_out1 = self.constant255.val();
        let add33_out1 = erf4_out1
            .add((constant255_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul24_out1 = linear23_out1.mul(add33_out1);
        let constant256_out1 = self.constant256.val();
        let mul25_out1 = mul24_out1
            .mul((constant256_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear24_out1 = self.linear24.forward(mul25_out1);
        let add34_out1 = linear24_out1.add(add32_out1);
        let reducemean17_out1 = { add34_out1.clone().mean_dim(2usize) };
        let sub10_out1 = add34_out1.sub(reducemean17_out1);
        let constant257_out1 = self.constant257.val();
        let pow9_out1 = sub10_out1
            .clone()
            .powf((constant257_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean18_out1 = { pow9_out1.mean_dim(2usize) };
        let constant258_out1 = self.constant258.val();
        let add35_out1 = reducemean18_out1
            .add((constant258_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt21_out1 = add35_out1.sqrt();
        let div17_out1 = sub10_out1.div(sqrt21_out1);
        let constant44_out1 = self.constant44.val();
        let mul26_out1 = div17_out1
            .mul((constant44_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant45_out1 = self.constant45.val();
        let add36_out1 = mul26_out1
            .add((constant45_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape41_out1: [i64; 3] = {
            let axes = &add36_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather39_out1 = shape41_out1[0] as i64;
        let gather40_out1 = shape41_out1[1] as i64;
        let linear25_out1 = self.linear25.forward(add36_out1.clone());
        let shape43_out1: [i64; 3] = {
            let axes = &linear25_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather41_out1 = shape43_out1[0] as i64;
        let gather42_out1 = shape43_out1[1] as i64;
        let unsqueeze39_out1 = [gather41_out1 as i64];
        let unsqueeze40_out1 = [gather42_out1 as i64];
        let constant266_out1: [i64; 1] = [32i64];
        let constant265_out1: [i64; 1] = [12i64];
        let concat18_out1: [i64; 4usize] = [
            &unsqueeze39_out1[..],
            &unsqueeze40_out1[..],
            &constant265_out1[..],
            &constant266_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape18_out1 = linear25_out1.reshape(concat18_out1);
        let transpose17_out1 = reshape18_out1.permute([0, 2, 1, 3]);
        let linear26_out1 = self.linear26.forward(add36_out1.clone());
        let shape45_out1: [i64; 3] = {
            let axes = &linear26_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather43_out1 = shape45_out1[0] as i64;
        let gather44_out1 = shape45_out1[1] as i64;
        let unsqueeze41_out1 = [gather43_out1 as i64];
        let unsqueeze42_out1 = [gather44_out1 as i64];
        let constant272_out1: [i64; 1] = [32i64];
        let constant271_out1: [i64; 1] = [12i64];
        let concat19_out1: [i64; 4usize] = [
            &unsqueeze41_out1[..],
            &unsqueeze42_out1[..],
            &constant271_out1[..],
            &constant272_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape19_out1 = linear26_out1.reshape(concat19_out1);
        let linear27_out1 = self.linear27.forward(add36_out1.clone());
        let shape47_out1: [i64; 3] = {
            let axes = &linear27_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather45_out1 = shape47_out1[0] as i64;
        let gather46_out1 = shape47_out1[1] as i64;
        let unsqueeze43_out1 = [gather45_out1 as i64];
        let unsqueeze44_out1 = [gather46_out1 as i64];
        let constant278_out1: [i64; 1] = [32i64];
        let constant277_out1: [i64; 1] = [12i64];
        let concat20_out1: [i64; 4usize] = [
            &unsqueeze43_out1[..],
            &unsqueeze44_out1[..],
            &constant277_out1[..],
            &constant278_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape20_out1 = linear27_out1.reshape(concat20_out1);
        let transpose18_out1 = reshape20_out1.permute([0, 2, 1, 3]);
        let transpose19_out1 = reshape19_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul37_out1,)
        };
        let transpose20_out1 = matmul37_out1.permute([0, 2, 1, 3]);
        let unsqueeze45_out1 = [gather39_out1 as i64];
        let unsqueeze46_out1 = [gather40_out1 as i64];
        let constant284_out1: [i64; 1] = [384i64];
        let concat21_out1: [i64; 3usize] = [
            &unsqueeze45_out1[..],
            &unsqueeze46_out1[..],
            &constant284_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape21_out1 = transpose20_out1.reshape(concat21_out1);
        let linear28_out1 = self.linear28.forward(reshape21_out1);
        let add38_out1 = linear28_out1.add(add36_out1);
        add38_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule6 {
    constant285: burn::module::Param<Tensor<1>>,
    constant286: burn::module::Param<Tensor<1>>,
    constant50: burn::module::Param<Tensor<1>>,
    constant51: burn::module::Param<Tensor<1>>,
    linear29: Linear,
    constant287: burn::module::Param<Tensor<1>>,
    constant288: burn::module::Param<Tensor<1>>,
    constant289: burn::module::Param<Tensor<1>>,
    linear30: Linear,
    constant290: burn::module::Param<Tensor<1>>,
    constant291: burn::module::Param<Tensor<1>>,
    constant54: burn::module::Param<Tensor<1>>,
    constant55: burn::module::Param<Tensor<1>>,
    linear31: Linear,
    linear32: Linear,
    linear33: Linear,
    linear34: Linear,
    constant318: burn::module::Param<Tensor<1>>,
    constant319: burn::module::Param<Tensor<1>>,
    constant60: burn::module::Param<Tensor<1>>,
    constant61: burn::module::Param<Tensor<1>>,
    linear35: Linear,
    constant320: burn::module::Param<Tensor<1>>,
    constant321: burn::module::Param<Tensor<1>>,
    constant322: burn::module::Param<Tensor<1>>,
    linear36: Linear,
    constant323: burn::module::Param<Tensor<1>>,
    constant324: burn::module::Param<Tensor<1>>,
    constant64: burn::module::Param<Tensor<1>>,
    constant65: burn::module::Param<Tensor<1>>,
    linear37: Linear,
    linear38: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule6 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant285: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant286: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant287: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant288: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant289: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant290: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant291: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant318: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant319: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant320: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant321: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant322: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant323: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant324: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear38 = LinearConfig::new(384, 1)
            .with_bias(true)
            .with_layout(LinearLayout::Col)
            .init(device);
        Self {
            constant285,
            constant286,
            constant50,
            constant51,
            linear29,
            constant287,
            constant288,
            constant289,
            linear30,
            constant290,
            constant291,
            constant54,
            constant55,
            linear31,
            linear32,
            linear33,
            linear34,
            constant318,
            constant319,
            constant60,
            constant61,
            linear35,
            constant320,
            constant321,
            constant322,
            linear36,
            constant323,
            constant324,
            constant64,
            constant65,
            linear37,
            linear38,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add38_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<2> {
        let reducemean19_out1 = { add38_out1.clone().mean_dim(2usize) };
        let sub11_out1 = add38_out1.sub(reducemean19_out1);
        let constant285_out1 = self.constant285.val();
        let pow10_out1 = sub11_out1
            .clone()
            .powf((constant285_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean20_out1 = { pow10_out1.mean_dim(2usize) };
        let constant286_out1 = self.constant286.val();
        let add39_out1 = reducemean20_out1
            .add((constant286_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt25_out1 = add39_out1.sqrt();
        let div19_out1 = sub11_out1.div(sqrt25_out1);
        let constant50_out1 = self.constant50.val();
        let mul29_out1 = div19_out1
            .mul((constant50_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant51_out1 = self.constant51.val();
        let add40_out1 = mul29_out1
            .add((constant51_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear29_out1 = self.linear29.forward(add40_out1.clone());
        let constant287_out1 = self.constant287.val();
        let div20_out1 = linear29_out1
            .clone()
            .div((constant287_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf5_out1 = div20_out1.erf();
        let constant288_out1 = self.constant288.val();
        let add41_out1 = erf5_out1
            .add((constant288_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul30_out1 = linear29_out1.mul(add41_out1);
        let constant289_out1 = self.constant289.val();
        let mul31_out1 = mul30_out1
            .mul((constant289_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear30_out1 = self.linear30.forward(mul31_out1);
        let add42_out1 = linear30_out1.add(add40_out1);
        let reducemean21_out1 = { add42_out1.clone().mean_dim(2usize) };
        let sub12_out1 = add42_out1.sub(reducemean21_out1);
        let constant290_out1 = self.constant290.val();
        let pow11_out1 = sub12_out1
            .clone()
            .powf((constant290_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean22_out1 = { pow11_out1.mean_dim(2usize) };
        let constant291_out1 = self.constant291.val();
        let add43_out1 = reducemean22_out1
            .add((constant291_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt26_out1 = add43_out1.sqrt();
        let div21_out1 = sub12_out1.div(sqrt26_out1);
        let constant54_out1 = self.constant54.val();
        let mul32_out1 = div21_out1
            .mul((constant54_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant55_out1 = self.constant55.val();
        let add44_out1 = mul32_out1
            .add((constant55_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape50_out1: [i64; 3] = {
            let axes = &add44_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather47_out1 = shape50_out1[0] as i64;
        let gather48_out1 = shape50_out1[1] as i64;
        let linear31_out1 = self.linear31.forward(add44_out1.clone());
        let shape52_out1: [i64; 3] = {
            let axes = &linear31_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather49_out1 = shape52_out1[0] as i64;
        let gather50_out1 = shape52_out1[1] as i64;
        let unsqueeze47_out1 = [gather49_out1 as i64];
        let unsqueeze48_out1 = [gather50_out1 as i64];
        let constant299_out1: [i64; 1] = [32i64];
        let constant298_out1: [i64; 1] = [12i64];
        let concat22_out1: [i64; 4usize] = [
            &unsqueeze47_out1[..],
            &unsqueeze48_out1[..],
            &constant298_out1[..],
            &constant299_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape22_out1 = linear31_out1.reshape(concat22_out1);
        let transpose21_out1 = reshape22_out1.permute([0, 2, 1, 3]);
        let linear32_out1 = self.linear32.forward(add44_out1.clone());
        let shape54_out1: [i64; 3] = {
            let axes = &linear32_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather51_out1 = shape54_out1[0] as i64;
        let gather52_out1 = shape54_out1[1] as i64;
        let unsqueeze49_out1 = [gather51_out1 as i64];
        let unsqueeze50_out1 = [gather52_out1 as i64];
        let constant305_out1: [i64; 1] = [32i64];
        let constant304_out1: [i64; 1] = [12i64];
        let concat23_out1: [i64; 4usize] = [
            &unsqueeze49_out1[..],
            &unsqueeze50_out1[..],
            &constant304_out1[..],
            &constant305_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape23_out1 = linear32_out1.reshape(concat23_out1);
        let linear33_out1 = self.linear33.forward(add44_out1.clone());
        let shape56_out1: [i64; 3] = {
            let axes = &linear33_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather53_out1 = shape56_out1[0] as i64;
        let gather54_out1 = shape56_out1[1] as i64;
        let unsqueeze51_out1 = [gather53_out1 as i64];
        let unsqueeze52_out1 = [gather54_out1 as i64];
        let constant311_out1: [i64; 1] = [32i64];
        let constant310_out1: [i64; 1] = [12i64];
        let concat24_out1: [i64; 4usize] = [
            &unsqueeze51_out1[..],
            &unsqueeze52_out1[..],
            &constant310_out1[..],
            &constant311_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape24_out1 = linear33_out1.reshape(concat24_out1);
        let transpose22_out1 = reshape24_out1.permute([0, 2, 1, 3]);
        let transpose23_out1 = reshape23_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul45_out1,)
        };
        let transpose24_out1 = matmul45_out1.permute([0, 2, 1, 3]);
        let unsqueeze53_out1 = [gather47_out1 as i64];
        let unsqueeze54_out1 = [gather48_out1 as i64];
        let constant317_out1: [i64; 1] = [384i64];
        let concat25_out1: [i64; 3usize] = [
            &unsqueeze53_out1[..],
            &unsqueeze54_out1[..],
            &constant317_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape25_out1 = transpose24_out1.reshape(concat25_out1);
        let linear34_out1 = self.linear34.forward(reshape25_out1);
        let add46_out1 = linear34_out1.add(add44_out1);
        let reducemean23_out1 = { add46_out1.clone().mean_dim(2usize) };
        let sub13_out1 = add46_out1.sub(reducemean23_out1);
        let constant318_out1 = self.constant318.val();
        let pow12_out1 = sub13_out1
            .clone()
            .powf((constant318_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean24_out1 = { pow12_out1.mean_dim(2usize) };
        let constant319_out1 = self.constant319.val();
        let add47_out1 = reducemean24_out1
            .add((constant319_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt30_out1 = add47_out1.sqrt();
        let div23_out1 = sub13_out1.div(sqrt30_out1);
        let constant60_out1 = self.constant60.val();
        let mul35_out1 = div23_out1
            .mul((constant60_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant61_out1 = self.constant61.val();
        let add48_out1 = mul35_out1
            .add((constant61_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear35_out1 = self.linear35.forward(add48_out1.clone());
        let constant320_out1 = self.constant320.val();
        let div24_out1 = linear35_out1
            .clone()
            .div((constant320_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf6_out1 = div24_out1.erf();
        let constant321_out1 = self.constant321.val();
        let add49_out1 = erf6_out1
            .add((constant321_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul36_out1 = linear35_out1.mul(add49_out1);
        let constant322_out1 = self.constant322.val();
        let mul37_out1 = mul36_out1
            .mul((constant322_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear36_out1 = self.linear36.forward(mul37_out1);
        let add50_out1 = linear36_out1.add(add48_out1);
        let reducemean25_out1 = { add50_out1.clone().mean_dim(2usize) };
        let sub14_out1 = add50_out1.sub(reducemean25_out1);
        let constant323_out1 = self.constant323.val();
        let pow13_out1 = sub14_out1
            .clone()
            .powf((constant323_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean26_out1 = { pow13_out1.mean_dim(2usize) };
        let constant324_out1 = self.constant324.val();
        let add51_out1 = reducemean26_out1
            .add((constant324_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt31_out1 = add51_out1.sqrt();
        let div25_out1 = sub14_out1.div(sqrt31_out1);
        let constant64_out1 = self.constant64.val();
        let mul38_out1 = div25_out1
            .mul((constant64_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant65_out1 = self.constant65.val();
        let add52_out1 = mul38_out1
            .add((constant65_out1).unsqueeze_dims(&[0isize, 1isize]));
        let gather55_out1 = {
            let sliced = add52_out1.slice(s![.., 0, ..]);
            sliced.squeeze_dim::<2usize>(1)
        };
        let linear37_out1 = self.linear37.forward(gather55_out1);
        let tanh1_out1 = linear37_out1.tanh();
        let linear38_out1 = self.linear38.forward(tanh1_out1);
        linear38_out1
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
        Self {
            submodule1,
            submodule2,
            submodule3,
            submodule4,
            submodule5,
            submodule6,
            device: device.clone(),
        }
    }

    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        input_ids: Tensor<2, Int>,
        attention_mask: Tensor<2, Int>,
        token_type_ids: Tensor<2, Int>,
    ) -> Tensor<2> {
        let (add6_out1, where2_out1) = self
            .submodule1
            .forward(input_ids, token_type_ids, attention_mask);
        let add14_out1 = self.submodule2.forward(add6_out1, where2_out1.clone());
        let add22_out1 = self.submodule3.forward(add14_out1, where2_out1.clone());
        let add30_out1 = self.submodule4.forward(add22_out1, where2_out1.clone());
        let add38_out1 = self.submodule5.forward(add30_out1, where2_out1.clone());
        let linear38_out1 = self.submodule6.forward(add38_out1, where2_out1);
        linear38_out1
    }
}
