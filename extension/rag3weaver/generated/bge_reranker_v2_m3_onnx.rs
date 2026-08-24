// Generated from ONNX "onnx-community/bge-reranker-v2-m3-ONNX onnx/model.onnx" by burn-onnx
use burn::prelude::*;
use burn::nn::Linear;
use burn::nn::LinearConfig;
use burn::nn::LinearLayout;
use burn::tensor::Bytes;
use burn_store::BurnpackStore;
use burn_store::ModuleSnapshot;


#[derive(Module, Debug)]
pub struct Submodule1 {
    constant1: burn::module::Param<Tensor<2>>,
    constant6: burn::module::Param<Tensor<2, Int>>,
    constant10: burn::module::Param<Tensor<1, Int>>,
    constant11: burn::module::Param<Tensor<1, Int>>,
    constant12: burn::module::Param<Tensor<1, Int>>,
    constant13: burn::module::Param<Tensor<1, Int>>,
    constant14: burn::module::Param<Tensor<1, Int>>,
    constant15: burn::module::Param<Tensor<2>>,
    constant16: burn::module::Param<Tensor<2>>,
    constant17: burn::module::Param<Tensor<1>>,
    constant19: burn::module::Param<Tensor<1>>,
    constant20: burn::module::Param<Tensor<1>>,
    constant21: burn::module::Param<Tensor<1>>,
    constant22: burn::module::Param<Tensor<1>>,
    linear1: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule1 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([250002, 1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [250002, 1024].into(),
        );
        let constant6: burn::module::Param<Tensor<2, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
                Int,
            >::zeros([1, 8194], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [1, 8194].into(),
        );
        let constant10: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([2], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [2].into(),
        );
        let constant11: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([4], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [4].into(),
        );
        let constant12: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([2], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [2].into(),
        );
        let constant13: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([4], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [4].into(),
        );
        let constant14: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
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
        let constant15: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([8194, 1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [8194, 1024].into(),
        );
        let constant16: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([1, 1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 1024].into(),
        );
        let constant17: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant19: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant20: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant21: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant22: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear1 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        Self {
            constant1,
            constant6,
            constant10,
            constant11,
            constant12,
            constant13,
            constant14,
            constant15,
            constant16,
            constant17,
            constant19,
            constant20,
            constant21,
            constant22,
            linear1,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        input_ids: Tensor<2, Int>,
        attention_mask: Tensor<2, Int>,
    ) -> (Tensor<3>, [i64; 3], Tensor<3>, Tensor<4>, Tensor<1>, Tensor<1>) {
        let shape1_out1: [i64; 2] = {
            let axes = &input_ids.clone().dims()[0..2];
            let mut output = [0i64; 2];
            for i in 0..2 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let constant1_out1 = self.constant1.val();
        let gather1_out1 = constant1_out1.take::<2, 3>(0, input_ids.clone());
        let shape2_out1: [i64; 2] = {
            let axes = &attention_mask.clone().dims()[0..2];
            let mut output = [0i64; 2];
            for i in 0..2 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let constant2_out1 = 1i64;
        let equal1_out1 = input_ids.equal_elem(constant2_out1);
        let unsqueeze1_out1: Tensor<4, Int> = attention_mask
            .unsqueeze_dims::<4>(&[1, 2]);
        let gather2_out1 = shape1_out1[0] as i64;
        let gather3_out1 = shape1_out1[1] as i64;
        let gather4_out1 = shape2_out1[0] as i64;
        let gather5_out1 = shape2_out1[1] as i64;
        let not1_out1 = equal1_out1.bool_not();
        let unsqueeze2_out1 = [gather3_out1 as i64];
        let unsqueeze3_out1 = [gather2_out1 as i64];
        let unsqueeze4_out1 = [gather4_out1 as i64];
        let unsqueeze5_out1 = [gather5_out1 as i64];
        let cast1_out1 = not1_out1.int().cast(burn::tensor::DType::I32);
        let constant6_out1 = self.constant6.val();
        let slice1_out1 = constant6_out1.slice(s![.., 0..unsqueeze2_out1[0]]);
        let concat1_out1: [i64; 2usize] = [&unsqueeze3_out1[..], &unsqueeze2_out1[..]]
            .concat()
            .try_into()
            .unwrap();
        let constant8_out1: [i64; 1] = [1i64];
        let concat2_out1: [i64; 4usize] = [
            &unsqueeze4_out1[..],
            &constant8_out1[..],
            &unsqueeze2_out1[..],
            &unsqueeze5_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let cumsum1_out1 = cast1_out1.clone().cumsum(1);
        let mul1_out1 = cumsum1_out1.mul(cast1_out1);
        let constant10_out1 = self.constant10.val();
        let equal2_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat1_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant10_out1)
        };
        let constant11_out1 = self.constant11.val();
        let equal3_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat2_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant11_out1)
        };
        let cast2_out1 = mul1_out1.cast(burn::tensor::DType::I64);
        let constant12_out1 = self.constant12.val();
        let where1_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat1_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal2_out1, constant12_out1);
        let constant13_out1 = self.constant13.val();
        let where2_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat2_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal3_out1, constant13_out1);
        let constant14_out1 = self.constant14.val();
        let add1_out1 = cast2_out1.add((constant14_out1).unsqueeze_dims(&[0isize]));
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
        let expand2_out1 = {
            let onnx_shape: [i64; 4usize] = TryInto::<
                [i64; 4usize],
            >::try_into(where2_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze1_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..4usize {
                let dim_offset = 4usize - 4usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze1_out1.expand(shape)
        };
        let constant15_out1 = self.constant15.val();
        let gather6_out1 = constant15_out1.take::<2, 3>(0, add1_out1);
        let constant16_out1 = self.constant16.val();
        let gather7_out1 = constant16_out1.take::<2, 3>(0, expand1_out1);
        let cast3_out1 = expand2_out1.float().cast(burn::tensor::DType::F32);
        let add2_out1 = gather1_out1.add(gather7_out1);
        let constant17_out1 = self.constant17.val();
        let sub1_out1 = (constant17_out1)
            .unsqueeze_dims(&[0isize, 1isize, 2isize])
            .sub(cast3_out1);
        let add3_out1 = add2_out1.add(gather6_out1);
        let cast4_out1 = sub1_out1.clone().bool();
        let reducemean1_out1 = { add3_out1.clone().mean_dim(2usize) };
        let constant18_out1 = -340282350000000000000000000000000000000f32;
        let where3_out1 = sub1_out1.mask_fill(cast4_out1, constant18_out1);
        let sub2_out1 = add3_out1.sub(reducemean1_out1);
        let constant19_out1 = self.constant19.val();
        let pow1_out1 = sub2_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean2_out1 = { pow1_out1.mean_dim(2usize) };
        let constant20_out1 = self.constant20.val();
        let add4_out1 = reducemean2_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt1_out1 = add4_out1.sqrt();
        let div1_out1 = sub2_out1.div(sqrt1_out1);
        let constant21_out1 = self.constant21.val();
        let mul2_out1 = div1_out1
            .mul((constant21_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant22_out1 = self.constant22.val();
        let add5_out1 = mul2_out1
            .add((constant22_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape3_out1: [i64; 3] = {
            let axes = &add5_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear1_out1 = self.linear1.forward(add5_out1.clone());
        (
            add5_out1,
            shape3_out1,
            linear1_out1,
            where3_out1,
            constant19_out1,
            constant20_out1,
        )
    }
}
#[derive(Module, Debug)]
pub struct Submodule2 {
    linear2: Linear,
    linear3: Linear,
    constant26: burn::module::Param<Tensor<1>>,
    constant27: burn::module::Param<Tensor<1>>,
    constant28: burn::module::Param<Tensor<1>>,
    linear4: Linear,
    constant35: burn::module::Param<Tensor<1>>,
    constant36: burn::module::Param<Tensor<1>>,
    linear5: Linear,
    constant39: burn::module::Param<Tensor<1>>,
    constant40: burn::module::Param<Tensor<1>>,
    #[module(skip)]
    device: Device,
}
impl Submodule2 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let linear2 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear3 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant26: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant27: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant28: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear4 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant35: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant36: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear5 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let constant39: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant40: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        Self {
            linear2,
            linear3,
            constant26,
            constant27,
            constant28,
            linear4,
            constant35,
            constant36,
            linear5,
            constant39,
            constant40,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add5_out1: Tensor<3>,
        shape3_out1: [i64; 3],
        linear1_out1: Tensor<3>,
        where3_out1: Tensor<4>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
    ) -> (Tensor<3>, Tensor<3>, [i64; 1], [i64; 1], [i64; 1], Tensor<1>, Tensor<1>) {
        let linear2_out1 = self.linear2.forward(add5_out1.clone());
        let linear3_out1 = self.linear3.forward(add5_out1.clone());
        let gather8_out1 = shape3_out1[0] as i64;
        let gather9_out1 = shape3_out1[1] as i64;
        let constant26_out1 = self.constant26.val();
        let add6_out1 = (constant26_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear1_out1);
        let constant27_out1 = self.constant27.val();
        let add7_out1 = (constant27_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear2_out1);
        let constant28_out1 = self.constant28.val();
        let add8_out1 = (constant28_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear3_out1);
        let shape4_out1: [i64; 3] = {
            let axes = &add6_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape5_out1: [i64; 3] = {
            let axes = &add7_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape6_out1: [i64; 3] = {
            let axes = &add8_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze6_out1 = [gather8_out1 as i64];
        let unsqueeze7_out1 = [gather9_out1 as i64];
        let gather10_out1 = shape4_out1[0] as i64;
        let gather11_out1 = shape4_out1[1] as i64;
        let gather12_out1 = shape5_out1[0] as i64;
        let gather13_out1 = shape5_out1[1] as i64;
        let gather14_out1 = shape6_out1[0] as i64;
        let gather15_out1 = shape6_out1[1] as i64;
        let constant29_out1: [i64; 1] = [1024i64];
        let concat3_out1: [i64; 3usize] = [
            &unsqueeze6_out1[..],
            &unsqueeze7_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze8_out1 = [gather10_out1 as i64];
        let unsqueeze9_out1 = [gather11_out1 as i64];
        let unsqueeze10_out1 = [gather12_out1 as i64];
        let unsqueeze11_out1 = [gather13_out1 as i64];
        let unsqueeze12_out1 = [gather14_out1 as i64];
        let unsqueeze13_out1 = [gather15_out1 as i64];
        let constant30_out1: [i64; 1] = [16i64];
        let constant31_out1: [i64; 1] = [64i64];
        let concat4_out1: [i64; 4usize] = [
            &unsqueeze8_out1[..],
            &unsqueeze9_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat5_out1: [i64; 4usize] = [
            &unsqueeze10_out1[..],
            &unsqueeze11_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat6_out1: [i64; 4usize] = [
            &unsqueeze12_out1[..],
            &unsqueeze13_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape1_out1 = add6_out1.reshape(concat4_out1);
        let reshape2_out1 = add7_out1.reshape(concat5_out1);
        let reshape3_out1 = add8_out1.reshape(concat6_out1);
        let transpose1_out1 = reshape1_out1.permute([0, 2, 1, 3]);
        let transpose2_out1 = reshape3_out1.permute([0, 2, 1, 3]);
        let transpose3_out1 = reshape2_out1.permute([0, 2, 3, 1]);
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
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul5_out1,)
        };
        let transpose4_out1 = matmul5_out1.permute([0, 2, 1, 3]);
        let reshape4_out1 = transpose4_out1.reshape(concat3_out1);
        let linear4_out1 = self.linear4.forward(reshape4_out1);
        let add10_out1 = linear4_out1.add(add5_out1);
        let reducemean3_out1 = { add10_out1.clone().mean_dim(2usize) };
        let sub3_out1 = add10_out1.sub(reducemean3_out1);
        let pow2_out1 = sub3_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean4_out1 = { pow2_out1.mean_dim(2usize) };
        let add11_out1 = reducemean4_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt2_out1 = add11_out1.sqrt();
        let div2_out1 = sub3_out1.div(sqrt2_out1);
        let constant35_out1 = self.constant35.val();
        let mul5_out1 = div2_out1
            .mul((constant35_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant36_out1 = self.constant36.val();
        let add12_out1 = mul5_out1
            .add((constant36_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear5_out1 = self.linear5.forward(add12_out1.clone());
        let constant39_out1 = self.constant39.val();
        let div3_out1 = linear5_out1
            .clone()
            .div((constant39_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let erf1_out1 = div3_out1.erf();
        let constant40_out1 = self.constant40.val();
        let add13_out1 = erf1_out1
            .add((constant40_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let mul6_out1 = linear5_out1.mul(add13_out1);
        (
            mul6_out1,
            add12_out1,
            constant29_out1,
            constant30_out1,
            constant31_out1,
            constant39_out1,
            constant40_out1,
        )
    }
}
#[derive(Module, Debug)]
pub struct Submodule3 {
    constant41: burn::module::Param<Tensor<1>>,
    linear6: Linear,
    constant44: burn::module::Param<Tensor<1>>,
    constant45: burn::module::Param<Tensor<1>>,
    linear7: Linear,
    linear8: Linear,
    linear9: Linear,
    constant49: burn::module::Param<Tensor<1>>,
    constant50: burn::module::Param<Tensor<1>>,
    constant51: burn::module::Param<Tensor<1>>,
    linear10: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule3 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant41: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear7 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear8 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear9 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant49: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
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
        let linear10 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant41,
            linear6,
            constant44,
            constant45,
            linear7,
            linear8,
            linear9,
            constant49,
            constant50,
            constant51,
            linear10,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        mul6_out1: Tensor<3>,
        add12_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<1>) {
        let constant41_out1 = self.constant41.val();
        let mul7_out1 = mul6_out1
            .mul((constant41_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let linear6_out1 = self.linear6.forward(mul7_out1);
        let add14_out1 = linear6_out1.add(add12_out1);
        let reducemean5_out1 = { add14_out1.clone().mean_dim(2usize) };
        let sub4_out1 = add14_out1.sub(reducemean5_out1);
        let pow3_out1 = sub4_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean6_out1 = { pow3_out1.mean_dim(2usize) };
        let add15_out1 = reducemean6_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt3_out1 = add15_out1.sqrt();
        let div4_out1 = sub4_out1.div(sqrt3_out1);
        let constant44_out1 = self.constant44.val();
        let mul8_out1 = div4_out1
            .mul((constant44_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant45_out1 = self.constant45.val();
        let add16_out1 = mul8_out1
            .add((constant45_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape7_out1: [i64; 3] = {
            let axes = &add16_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear7_out1 = self.linear7.forward(add16_out1.clone());
        let linear8_out1 = self.linear8.forward(add16_out1.clone());
        let linear9_out1 = self.linear9.forward(add16_out1.clone());
        let gather16_out1 = shape7_out1[0] as i64;
        let gather17_out1 = shape7_out1[1] as i64;
        let constant49_out1 = self.constant49.val();
        let add17_out1 = (constant49_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear7_out1);
        let constant50_out1 = self.constant50.val();
        let add18_out1 = (constant50_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear8_out1);
        let constant51_out1 = self.constant51.val();
        let add19_out1 = (constant51_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear9_out1);
        let shape8_out1: [i64; 3] = {
            let axes = &add17_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape9_out1: [i64; 3] = {
            let axes = &add18_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape10_out1: [i64; 3] = {
            let axes = &add19_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze14_out1 = [gather16_out1 as i64];
        let unsqueeze15_out1 = [gather17_out1 as i64];
        let gather18_out1 = shape8_out1[0] as i64;
        let gather19_out1 = shape8_out1[1] as i64;
        let gather20_out1 = shape9_out1[0] as i64;
        let gather21_out1 = shape9_out1[1] as i64;
        let gather22_out1 = shape10_out1[0] as i64;
        let gather23_out1 = shape10_out1[1] as i64;
        let concat7_out1: [i64; 3usize] = [
            &unsqueeze14_out1[..],
            &unsqueeze15_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze16_out1 = [gather18_out1 as i64];
        let unsqueeze17_out1 = [gather19_out1 as i64];
        let unsqueeze18_out1 = [gather20_out1 as i64];
        let unsqueeze19_out1 = [gather21_out1 as i64];
        let unsqueeze20_out1 = [gather22_out1 as i64];
        let unsqueeze21_out1 = [gather23_out1 as i64];
        let concat8_out1: [i64; 4usize] = [
            &unsqueeze16_out1[..],
            &unsqueeze17_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat9_out1: [i64; 4usize] = [
            &unsqueeze18_out1[..],
            &unsqueeze19_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat10_out1: [i64; 4usize] = [
            &unsqueeze20_out1[..],
            &unsqueeze21_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape5_out1 = add17_out1.reshape(concat8_out1);
        let reshape6_out1 = add18_out1.reshape(concat9_out1);
        let reshape7_out1 = add19_out1.reshape(concat10_out1);
        let transpose5_out1 = reshape5_out1.permute([0, 2, 1, 3]);
        let transpose6_out1 = reshape7_out1.permute([0, 2, 1, 3]);
        let transpose7_out1 = reshape6_out1.permute([0, 2, 3, 1]);
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
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul13_out1,)
        };
        let transpose8_out1 = matmul13_out1.permute([0, 2, 1, 3]);
        let reshape8_out1 = transpose8_out1.reshape(concat7_out1);
        let linear10_out1 = self.linear10.forward(reshape8_out1);
        let add21_out1 = linear10_out1.add(add16_out1);
        let reducemean7_out1 = { add21_out1.clone().mean_dim(2usize) };
        let sub5_out1 = add21_out1.sub(reducemean7_out1);
        let pow4_out1 = sub5_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean8_out1 = { pow4_out1.mean_dim(2usize) };
        let add22_out1 = reducemean8_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt4_out1 = add22_out1.sqrt();
        let div5_out1 = sub5_out1.div(sqrt4_out1);
        (div5_out1, constant41_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule4 {
    constant54: burn::module::Param<Tensor<1>>,
    constant55: burn::module::Param<Tensor<1>>,
    linear11: Linear,
    linear12: Linear,
    constant60: burn::module::Param<Tensor<1>>,
    constant61: burn::module::Param<Tensor<1>>,
    linear13: Linear,
    linear14: Linear,
    linear15: Linear,
    constant65: burn::module::Param<Tensor<1>>,
    constant66: burn::module::Param<Tensor<1>>,
    constant67: burn::module::Param<Tensor<1>>,
    linear16: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule4 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
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
        let linear11 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear12 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
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
        let linear13 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear14 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear15 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant65: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant66: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant67: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear16 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant54,
            constant55,
            linear11,
            linear12,
            constant60,
            constant61,
            linear13,
            linear14,
            linear15,
            constant65,
            constant66,
            constant67,
            linear16,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        div5_out1: Tensor<3>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let constant54_out1 = self.constant54.val();
        let mul11_out1 = div5_out1
            .mul((constant54_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant55_out1 = self.constant55.val();
        let add23_out1 = mul11_out1
            .add((constant55_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear11_out1 = self.linear11.forward(add23_out1.clone());
        let div6_out1 = linear11_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf2_out1 = div6_out1.erf();
        let add24_out1 = erf2_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul12_out1 = linear11_out1.mul(add24_out1);
        let mul13_out1 = mul12_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear12_out1 = self.linear12.forward(mul13_out1);
        let add25_out1 = linear12_out1.add(add23_out1);
        let reducemean9_out1 = { add25_out1.clone().mean_dim(2usize) };
        let sub6_out1 = add25_out1.sub(reducemean9_out1);
        let pow5_out1 = sub6_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean10_out1 = { pow5_out1.mean_dim(2usize) };
        let add26_out1 = reducemean10_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt5_out1 = add26_out1.sqrt();
        let div7_out1 = sub6_out1.div(sqrt5_out1);
        let constant60_out1 = self.constant60.val();
        let mul14_out1 = div7_out1
            .mul((constant60_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant61_out1 = self.constant61.val();
        let add27_out1 = mul14_out1
            .add((constant61_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape11_out1: [i64; 3] = {
            let axes = &add27_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear13_out1 = self.linear13.forward(add27_out1.clone());
        let linear14_out1 = self.linear14.forward(add27_out1.clone());
        let linear15_out1 = self.linear15.forward(add27_out1.clone());
        let gather24_out1 = shape11_out1[0] as i64;
        let gather25_out1 = shape11_out1[1] as i64;
        let constant65_out1 = self.constant65.val();
        let add28_out1 = (constant65_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear13_out1);
        let constant66_out1 = self.constant66.val();
        let add29_out1 = (constant66_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear14_out1);
        let constant67_out1 = self.constant67.val();
        let add30_out1 = (constant67_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear15_out1);
        let shape12_out1: [i64; 3] = {
            let axes = &add28_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape13_out1: [i64; 3] = {
            let axes = &add29_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape14_out1: [i64; 3] = {
            let axes = &add30_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze22_out1 = [gather24_out1 as i64];
        let unsqueeze23_out1 = [gather25_out1 as i64];
        let gather26_out1 = shape12_out1[0] as i64;
        let gather27_out1 = shape12_out1[1] as i64;
        let gather28_out1 = shape13_out1[0] as i64;
        let gather29_out1 = shape13_out1[1] as i64;
        let gather30_out1 = shape14_out1[0] as i64;
        let gather31_out1 = shape14_out1[1] as i64;
        let concat11_out1: [i64; 3usize] = [
            &unsqueeze22_out1[..],
            &unsqueeze23_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze24_out1 = [gather26_out1 as i64];
        let unsqueeze25_out1 = [gather27_out1 as i64];
        let unsqueeze26_out1 = [gather28_out1 as i64];
        let unsqueeze27_out1 = [gather29_out1 as i64];
        let unsqueeze28_out1 = [gather30_out1 as i64];
        let unsqueeze29_out1 = [gather31_out1 as i64];
        let concat12_out1: [i64; 4usize] = [
            &unsqueeze24_out1[..],
            &unsqueeze25_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat13_out1: [i64; 4usize] = [
            &unsqueeze26_out1[..],
            &unsqueeze27_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat14_out1: [i64; 4usize] = [
            &unsqueeze28_out1[..],
            &unsqueeze29_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape9_out1 = add28_out1.reshape(concat12_out1);
        let reshape10_out1 = add29_out1.reshape(concat13_out1);
        let reshape11_out1 = add30_out1.reshape(concat14_out1);
        let transpose9_out1 = reshape9_out1.permute([0, 2, 1, 3]);
        let transpose10_out1 = reshape11_out1.permute([0, 2, 1, 3]);
        let transpose11_out1 = reshape10_out1.permute([0, 2, 3, 1]);
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
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul21_out1,)
        };
        let transpose12_out1 = matmul21_out1.permute([0, 2, 1, 3]);
        let reshape12_out1 = transpose12_out1.reshape(concat11_out1);
        let linear16_out1 = self.linear16.forward(reshape12_out1);
        let add32_out1 = linear16_out1.add(add27_out1);
        add32_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule5 {
    constant70: burn::module::Param<Tensor<1>>,
    constant71: burn::module::Param<Tensor<1>>,
    linear17: Linear,
    linear18: Linear,
    constant76: burn::module::Param<Tensor<1>>,
    constant77: burn::module::Param<Tensor<1>>,
    linear19: Linear,
    linear20: Linear,
    linear21: Linear,
    constant81: burn::module::Param<Tensor<1>>,
    constant82: burn::module::Param<Tensor<1>>,
    constant83: burn::module::Param<Tensor<1>>,
    linear22: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule5 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
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
        let linear17 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear18 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant76: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant77: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear19 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear20 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear21 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant81: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant82: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant83: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear22 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant70,
            constant71,
            linear17,
            linear18,
            constant76,
            constant77,
            linear19,
            linear20,
            linear21,
            constant81,
            constant82,
            constant83,
            linear22,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add32_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean11_out1 = { add32_out1.clone().mean_dim(2usize) };
        let sub7_out1 = add32_out1.sub(reducemean11_out1);
        let pow6_out1 = sub7_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean12_out1 = { pow6_out1.mean_dim(2usize) };
        let add33_out1 = reducemean12_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt6_out1 = add33_out1.sqrt();
        let div8_out1 = sub7_out1.div(sqrt6_out1);
        let constant70_out1 = self.constant70.val();
        let mul17_out1 = div8_out1
            .mul((constant70_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant71_out1 = self.constant71.val();
        let add34_out1 = mul17_out1
            .add((constant71_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear17_out1 = self.linear17.forward(add34_out1.clone());
        let div9_out1 = linear17_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf3_out1 = div9_out1.erf();
        let add35_out1 = erf3_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul18_out1 = linear17_out1.mul(add35_out1);
        let mul19_out1 = mul18_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear18_out1 = self.linear18.forward(mul19_out1);
        let add36_out1 = linear18_out1.add(add34_out1);
        let reducemean13_out1 = { add36_out1.clone().mean_dim(2usize) };
        let sub8_out1 = add36_out1.sub(reducemean13_out1);
        let pow7_out1 = sub8_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean14_out1 = { pow7_out1.mean_dim(2usize) };
        let add37_out1 = reducemean14_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt7_out1 = add37_out1.sqrt();
        let div10_out1 = sub8_out1.div(sqrt7_out1);
        let constant76_out1 = self.constant76.val();
        let mul20_out1 = div10_out1
            .mul((constant76_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant77_out1 = self.constant77.val();
        let add38_out1 = mul20_out1
            .add((constant77_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape15_out1: [i64; 3] = {
            let axes = &add38_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear19_out1 = self.linear19.forward(add38_out1.clone());
        let linear20_out1 = self.linear20.forward(add38_out1.clone());
        let linear21_out1 = self.linear21.forward(add38_out1.clone());
        let gather32_out1 = shape15_out1[0] as i64;
        let gather33_out1 = shape15_out1[1] as i64;
        let constant81_out1 = self.constant81.val();
        let add39_out1 = (constant81_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear19_out1);
        let constant82_out1 = self.constant82.val();
        let add40_out1 = (constant82_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear20_out1);
        let constant83_out1 = self.constant83.val();
        let add41_out1 = (constant83_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear21_out1);
        let shape16_out1: [i64; 3] = {
            let axes = &add39_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape17_out1: [i64; 3] = {
            let axes = &add40_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape18_out1: [i64; 3] = {
            let axes = &add41_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze30_out1 = [gather32_out1 as i64];
        let unsqueeze31_out1 = [gather33_out1 as i64];
        let gather34_out1 = shape16_out1[0] as i64;
        let gather35_out1 = shape16_out1[1] as i64;
        let gather36_out1 = shape17_out1[0] as i64;
        let gather37_out1 = shape17_out1[1] as i64;
        let gather38_out1 = shape18_out1[0] as i64;
        let gather39_out1 = shape18_out1[1] as i64;
        let concat15_out1: [i64; 3usize] = [
            &unsqueeze30_out1[..],
            &unsqueeze31_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze32_out1 = [gather34_out1 as i64];
        let unsqueeze33_out1 = [gather35_out1 as i64];
        let unsqueeze34_out1 = [gather36_out1 as i64];
        let unsqueeze35_out1 = [gather37_out1 as i64];
        let unsqueeze36_out1 = [gather38_out1 as i64];
        let unsqueeze37_out1 = [gather39_out1 as i64];
        let concat16_out1: [i64; 4usize] = [
            &unsqueeze32_out1[..],
            &unsqueeze33_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat17_out1: [i64; 4usize] = [
            &unsqueeze34_out1[..],
            &unsqueeze35_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat18_out1: [i64; 4usize] = [
            &unsqueeze36_out1[..],
            &unsqueeze37_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape13_out1 = add39_out1.reshape(concat16_out1);
        let reshape14_out1 = add40_out1.reshape(concat17_out1);
        let reshape15_out1 = add41_out1.reshape(concat18_out1);
        let transpose13_out1 = reshape13_out1.permute([0, 2, 1, 3]);
        let transpose14_out1 = reshape15_out1.permute([0, 2, 1, 3]);
        let transpose15_out1 = reshape14_out1.permute([0, 2, 3, 1]);
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
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul29_out1,)
        };
        let transpose16_out1 = matmul29_out1.permute([0, 2, 1, 3]);
        let reshape16_out1 = transpose16_out1.reshape(concat15_out1);
        let linear22_out1 = self.linear22.forward(reshape16_out1);
        let add43_out1 = linear22_out1.add(add38_out1);
        add43_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule6 {
    constant86: burn::module::Param<Tensor<1>>,
    constant87: burn::module::Param<Tensor<1>>,
    linear23: Linear,
    linear24: Linear,
    constant92: burn::module::Param<Tensor<1>>,
    constant93: burn::module::Param<Tensor<1>>,
    linear25: Linear,
    linear26: Linear,
    linear27: Linear,
    constant97: burn::module::Param<Tensor<1>>,
    constant98: burn::module::Param<Tensor<1>>,
    constant99: burn::module::Param<Tensor<1>>,
    linear28: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule6 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant86: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant87: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear23 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear24 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant92: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant93: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear25 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear26 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear27 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant97: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant98: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant99: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear28 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant86,
            constant87,
            linear23,
            linear24,
            constant92,
            constant93,
            linear25,
            linear26,
            linear27,
            constant97,
            constant98,
            constant99,
            linear28,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add43_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean15_out1 = { add43_out1.clone().mean_dim(2usize) };
        let sub9_out1 = add43_out1.sub(reducemean15_out1);
        let pow8_out1 = sub9_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean16_out1 = { pow8_out1.mean_dim(2usize) };
        let add44_out1 = reducemean16_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt8_out1 = add44_out1.sqrt();
        let div11_out1 = sub9_out1.div(sqrt8_out1);
        let constant86_out1 = self.constant86.val();
        let mul23_out1 = div11_out1
            .mul((constant86_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant87_out1 = self.constant87.val();
        let add45_out1 = mul23_out1
            .add((constant87_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear23_out1 = self.linear23.forward(add45_out1.clone());
        let div12_out1 = linear23_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf4_out1 = div12_out1.erf();
        let add46_out1 = erf4_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul24_out1 = linear23_out1.mul(add46_out1);
        let mul25_out1 = mul24_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear24_out1 = self.linear24.forward(mul25_out1);
        let add47_out1 = linear24_out1.add(add45_out1);
        let reducemean17_out1 = { add47_out1.clone().mean_dim(2usize) };
        let sub10_out1 = add47_out1.sub(reducemean17_out1);
        let pow9_out1 = sub10_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean18_out1 = { pow9_out1.mean_dim(2usize) };
        let add48_out1 = reducemean18_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt9_out1 = add48_out1.sqrt();
        let div13_out1 = sub10_out1.div(sqrt9_out1);
        let constant92_out1 = self.constant92.val();
        let mul26_out1 = div13_out1
            .mul((constant92_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant93_out1 = self.constant93.val();
        let add49_out1 = mul26_out1
            .add((constant93_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape19_out1: [i64; 3] = {
            let axes = &add49_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear25_out1 = self.linear25.forward(add49_out1.clone());
        let linear26_out1 = self.linear26.forward(add49_out1.clone());
        let linear27_out1 = self.linear27.forward(add49_out1.clone());
        let gather40_out1 = shape19_out1[0] as i64;
        let gather41_out1 = shape19_out1[1] as i64;
        let constant97_out1 = self.constant97.val();
        let add50_out1 = (constant97_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear25_out1);
        let constant98_out1 = self.constant98.val();
        let add51_out1 = (constant98_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear26_out1);
        let constant99_out1 = self.constant99.val();
        let add52_out1 = (constant99_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear27_out1);
        let shape20_out1: [i64; 3] = {
            let axes = &add50_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape21_out1: [i64; 3] = {
            let axes = &add51_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape22_out1: [i64; 3] = {
            let axes = &add52_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze38_out1 = [gather40_out1 as i64];
        let unsqueeze39_out1 = [gather41_out1 as i64];
        let gather42_out1 = shape20_out1[0] as i64;
        let gather43_out1 = shape20_out1[1] as i64;
        let gather44_out1 = shape21_out1[0] as i64;
        let gather45_out1 = shape21_out1[1] as i64;
        let gather46_out1 = shape22_out1[0] as i64;
        let gather47_out1 = shape22_out1[1] as i64;
        let concat19_out1: [i64; 3usize] = [
            &unsqueeze38_out1[..],
            &unsqueeze39_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze40_out1 = [gather42_out1 as i64];
        let unsqueeze41_out1 = [gather43_out1 as i64];
        let unsqueeze42_out1 = [gather44_out1 as i64];
        let unsqueeze43_out1 = [gather45_out1 as i64];
        let unsqueeze44_out1 = [gather46_out1 as i64];
        let unsqueeze45_out1 = [gather47_out1 as i64];
        let concat20_out1: [i64; 4usize] = [
            &unsqueeze40_out1[..],
            &unsqueeze41_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat21_out1: [i64; 4usize] = [
            &unsqueeze42_out1[..],
            &unsqueeze43_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat22_out1: [i64; 4usize] = [
            &unsqueeze44_out1[..],
            &unsqueeze45_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape17_out1 = add50_out1.reshape(concat20_out1);
        let reshape18_out1 = add51_out1.reshape(concat21_out1);
        let reshape19_out1 = add52_out1.reshape(concat22_out1);
        let transpose17_out1 = reshape17_out1.permute([0, 2, 1, 3]);
        let transpose18_out1 = reshape19_out1.permute([0, 2, 1, 3]);
        let transpose19_out1 = reshape18_out1.permute([0, 2, 3, 1]);
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
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul37_out1,)
        };
        let transpose20_out1 = matmul37_out1.permute([0, 2, 1, 3]);
        let reshape20_out1 = transpose20_out1.reshape(concat19_out1);
        let linear28_out1 = self.linear28.forward(reshape20_out1);
        let add54_out1 = linear28_out1.add(add49_out1);
        add54_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule7 {
    constant102: burn::module::Param<Tensor<1>>,
    constant103: burn::module::Param<Tensor<1>>,
    linear29: Linear,
    linear30: Linear,
    constant108: burn::module::Param<Tensor<1>>,
    constant109: burn::module::Param<Tensor<1>>,
    linear31: Linear,
    linear32: Linear,
    linear33: Linear,
    constant113: burn::module::Param<Tensor<1>>,
    constant114: burn::module::Param<Tensor<1>>,
    constant115: burn::module::Param<Tensor<1>>,
    linear34: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule7 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant102: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant103: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear29 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear30 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant108: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant109: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear31 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear32 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear33 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant113: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
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
        let linear34 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant102,
            constant103,
            linear29,
            linear30,
            constant108,
            constant109,
            linear31,
            linear32,
            linear33,
            constant113,
            constant114,
            constant115,
            linear34,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add54_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean19_out1 = { add54_out1.clone().mean_dim(2usize) };
        let sub11_out1 = add54_out1.sub(reducemean19_out1);
        let pow10_out1 = sub11_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean20_out1 = { pow10_out1.mean_dim(2usize) };
        let add55_out1 = reducemean20_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt10_out1 = add55_out1.sqrt();
        let div14_out1 = sub11_out1.div(sqrt10_out1);
        let constant102_out1 = self.constant102.val();
        let mul29_out1 = div14_out1
            .mul((constant102_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant103_out1 = self.constant103.val();
        let add56_out1 = mul29_out1
            .add((constant103_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear29_out1 = self.linear29.forward(add56_out1.clone());
        let div15_out1 = linear29_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf5_out1 = div15_out1.erf();
        let add57_out1 = erf5_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul30_out1 = linear29_out1.mul(add57_out1);
        let mul31_out1 = mul30_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear30_out1 = self.linear30.forward(mul31_out1);
        let add58_out1 = linear30_out1.add(add56_out1);
        let reducemean21_out1 = { add58_out1.clone().mean_dim(2usize) };
        let sub12_out1 = add58_out1.sub(reducemean21_out1);
        let pow11_out1 = sub12_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean22_out1 = { pow11_out1.mean_dim(2usize) };
        let add59_out1 = reducemean22_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt11_out1 = add59_out1.sqrt();
        let div16_out1 = sub12_out1.div(sqrt11_out1);
        let constant108_out1 = self.constant108.val();
        let mul32_out1 = div16_out1
            .mul((constant108_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant109_out1 = self.constant109.val();
        let add60_out1 = mul32_out1
            .add((constant109_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape23_out1: [i64; 3] = {
            let axes = &add60_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear31_out1 = self.linear31.forward(add60_out1.clone());
        let linear32_out1 = self.linear32.forward(add60_out1.clone());
        let linear33_out1 = self.linear33.forward(add60_out1.clone());
        let gather48_out1 = shape23_out1[0] as i64;
        let gather49_out1 = shape23_out1[1] as i64;
        let constant113_out1 = self.constant113.val();
        let add61_out1 = (constant113_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear31_out1);
        let constant114_out1 = self.constant114.val();
        let add62_out1 = (constant114_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear32_out1);
        let constant115_out1 = self.constant115.val();
        let add63_out1 = (constant115_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear33_out1);
        let shape24_out1: [i64; 3] = {
            let axes = &add61_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape25_out1: [i64; 3] = {
            let axes = &add62_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape26_out1: [i64; 3] = {
            let axes = &add63_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze46_out1 = [gather48_out1 as i64];
        let unsqueeze47_out1 = [gather49_out1 as i64];
        let gather50_out1 = shape24_out1[0] as i64;
        let gather51_out1 = shape24_out1[1] as i64;
        let gather52_out1 = shape25_out1[0] as i64;
        let gather53_out1 = shape25_out1[1] as i64;
        let gather54_out1 = shape26_out1[0] as i64;
        let gather55_out1 = shape26_out1[1] as i64;
        let concat23_out1: [i64; 3usize] = [
            &unsqueeze46_out1[..],
            &unsqueeze47_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze48_out1 = [gather50_out1 as i64];
        let unsqueeze49_out1 = [gather51_out1 as i64];
        let unsqueeze50_out1 = [gather52_out1 as i64];
        let unsqueeze51_out1 = [gather53_out1 as i64];
        let unsqueeze52_out1 = [gather54_out1 as i64];
        let unsqueeze53_out1 = [gather55_out1 as i64];
        let concat24_out1: [i64; 4usize] = [
            &unsqueeze48_out1[..],
            &unsqueeze49_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat25_out1: [i64; 4usize] = [
            &unsqueeze50_out1[..],
            &unsqueeze51_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat26_out1: [i64; 4usize] = [
            &unsqueeze52_out1[..],
            &unsqueeze53_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape21_out1 = add61_out1.reshape(concat24_out1);
        let reshape22_out1 = add62_out1.reshape(concat25_out1);
        let reshape23_out1 = add63_out1.reshape(concat26_out1);
        let transpose21_out1 = reshape21_out1.permute([0, 2, 1, 3]);
        let transpose22_out1 = reshape23_out1.permute([0, 2, 1, 3]);
        let transpose23_out1 = reshape22_out1.permute([0, 2, 3, 1]);
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
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul45_out1,)
        };
        let transpose24_out1 = matmul45_out1.permute([0, 2, 1, 3]);
        let reshape24_out1 = transpose24_out1.reshape(concat23_out1);
        let linear34_out1 = self.linear34.forward(reshape24_out1);
        let add65_out1 = linear34_out1.add(add60_out1);
        add65_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule8 {
    constant118: burn::module::Param<Tensor<1>>,
    constant119: burn::module::Param<Tensor<1>>,
    linear35: Linear,
    linear36: Linear,
    constant124: burn::module::Param<Tensor<1>>,
    constant125: burn::module::Param<Tensor<1>>,
    linear37: Linear,
    linear38: Linear,
    linear39: Linear,
    constant129: burn::module::Param<Tensor<1>>,
    constant130: burn::module::Param<Tensor<1>>,
    constant131: burn::module::Param<Tensor<1>>,
    linear40: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule8 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant118: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant119: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear35 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear36 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
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
        let linear37 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear38 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear39 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant129: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
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
        let linear40 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant118,
            constant119,
            linear35,
            linear36,
            constant124,
            constant125,
            linear37,
            linear38,
            linear39,
            constant129,
            constant130,
            constant131,
            linear40,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add65_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean23_out1 = { add65_out1.clone().mean_dim(2usize) };
        let sub13_out1 = add65_out1.sub(reducemean23_out1);
        let pow12_out1 = sub13_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean24_out1 = { pow12_out1.mean_dim(2usize) };
        let add66_out1 = reducemean24_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt12_out1 = add66_out1.sqrt();
        let div17_out1 = sub13_out1.div(sqrt12_out1);
        let constant118_out1 = self.constant118.val();
        let mul35_out1 = div17_out1
            .mul((constant118_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant119_out1 = self.constant119.val();
        let add67_out1 = mul35_out1
            .add((constant119_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear35_out1 = self.linear35.forward(add67_out1.clone());
        let div18_out1 = linear35_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf6_out1 = div18_out1.erf();
        let add68_out1 = erf6_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul36_out1 = linear35_out1.mul(add68_out1);
        let mul37_out1 = mul36_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear36_out1 = self.linear36.forward(mul37_out1);
        let add69_out1 = linear36_out1.add(add67_out1);
        let reducemean25_out1 = { add69_out1.clone().mean_dim(2usize) };
        let sub14_out1 = add69_out1.sub(reducemean25_out1);
        let pow13_out1 = sub14_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean26_out1 = { pow13_out1.mean_dim(2usize) };
        let add70_out1 = reducemean26_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt13_out1 = add70_out1.sqrt();
        let div19_out1 = sub14_out1.div(sqrt13_out1);
        let constant124_out1 = self.constant124.val();
        let mul38_out1 = div19_out1
            .mul((constant124_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant125_out1 = self.constant125.val();
        let add71_out1 = mul38_out1
            .add((constant125_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape27_out1: [i64; 3] = {
            let axes = &add71_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear37_out1 = self.linear37.forward(add71_out1.clone());
        let linear38_out1 = self.linear38.forward(add71_out1.clone());
        let linear39_out1 = self.linear39.forward(add71_out1.clone());
        let gather56_out1 = shape27_out1[0] as i64;
        let gather57_out1 = shape27_out1[1] as i64;
        let constant129_out1 = self.constant129.val();
        let add72_out1 = (constant129_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear37_out1);
        let constant130_out1 = self.constant130.val();
        let add73_out1 = (constant130_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear38_out1);
        let constant131_out1 = self.constant131.val();
        let add74_out1 = (constant131_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear39_out1);
        let shape28_out1: [i64; 3] = {
            let axes = &add72_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape29_out1: [i64; 3] = {
            let axes = &add73_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape30_out1: [i64; 3] = {
            let axes = &add74_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze54_out1 = [gather56_out1 as i64];
        let unsqueeze55_out1 = [gather57_out1 as i64];
        let gather58_out1 = shape28_out1[0] as i64;
        let gather59_out1 = shape28_out1[1] as i64;
        let gather60_out1 = shape29_out1[0] as i64;
        let gather61_out1 = shape29_out1[1] as i64;
        let gather62_out1 = shape30_out1[0] as i64;
        let gather63_out1 = shape30_out1[1] as i64;
        let concat27_out1: [i64; 3usize] = [
            &unsqueeze54_out1[..],
            &unsqueeze55_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze56_out1 = [gather58_out1 as i64];
        let unsqueeze57_out1 = [gather59_out1 as i64];
        let unsqueeze58_out1 = [gather60_out1 as i64];
        let unsqueeze59_out1 = [gather61_out1 as i64];
        let unsqueeze60_out1 = [gather62_out1 as i64];
        let unsqueeze61_out1 = [gather63_out1 as i64];
        let concat28_out1: [i64; 4usize] = [
            &unsqueeze56_out1[..],
            &unsqueeze57_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat29_out1: [i64; 4usize] = [
            &unsqueeze58_out1[..],
            &unsqueeze59_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat30_out1: [i64; 4usize] = [
            &unsqueeze60_out1[..],
            &unsqueeze61_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape25_out1 = add72_out1.reshape(concat28_out1);
        let reshape26_out1 = add73_out1.reshape(concat29_out1);
        let reshape27_out1 = add74_out1.reshape(concat30_out1);
        let transpose25_out1 = reshape25_out1.permute([0, 2, 1, 3]);
        let transpose26_out1 = reshape27_out1.permute([0, 2, 1, 3]);
        let transpose27_out1 = reshape26_out1.permute([0, 2, 3, 1]);
        let matmul52_k_corrected = transpose27_out1.permute([0, 1, 3, 2]);
        let (matmul53_out1,) = {
            let q = transpose25_out1;
            let k = matmul52_k_corrected;
            let v = transpose26_out1;
            let matmul53_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul53_out1,)
        };
        let transpose28_out1 = matmul53_out1.permute([0, 2, 1, 3]);
        let reshape28_out1 = transpose28_out1.reshape(concat27_out1);
        let linear40_out1 = self.linear40.forward(reshape28_out1);
        let add76_out1 = linear40_out1.add(add71_out1);
        add76_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule9 {
    constant134: burn::module::Param<Tensor<1>>,
    constant135: burn::module::Param<Tensor<1>>,
    linear41: Linear,
    linear42: Linear,
    constant140: burn::module::Param<Tensor<1>>,
    constant141: burn::module::Param<Tensor<1>>,
    linear43: Linear,
    linear44: Linear,
    linear45: Linear,
    constant145: burn::module::Param<Tensor<1>>,
    constant146: burn::module::Param<Tensor<1>>,
    constant147: burn::module::Param<Tensor<1>>,
    linear46: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule9 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
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
        let linear41 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear42 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
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
        let linear43 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear44 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear45 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant145: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant146: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant147: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear46 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant134,
            constant135,
            linear41,
            linear42,
            constant140,
            constant141,
            linear43,
            linear44,
            linear45,
            constant145,
            constant146,
            constant147,
            linear46,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add76_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean27_out1 = { add76_out1.clone().mean_dim(2usize) };
        let sub15_out1 = add76_out1.sub(reducemean27_out1);
        let pow14_out1 = sub15_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean28_out1 = { pow14_out1.mean_dim(2usize) };
        let add77_out1 = reducemean28_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt14_out1 = add77_out1.sqrt();
        let div20_out1 = sub15_out1.div(sqrt14_out1);
        let constant134_out1 = self.constant134.val();
        let mul41_out1 = div20_out1
            .mul((constant134_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant135_out1 = self.constant135.val();
        let add78_out1 = mul41_out1
            .add((constant135_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear41_out1 = self.linear41.forward(add78_out1.clone());
        let div21_out1 = linear41_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf7_out1 = div21_out1.erf();
        let add79_out1 = erf7_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul42_out1 = linear41_out1.mul(add79_out1);
        let mul43_out1 = mul42_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear42_out1 = self.linear42.forward(mul43_out1);
        let add80_out1 = linear42_out1.add(add78_out1);
        let reducemean29_out1 = { add80_out1.clone().mean_dim(2usize) };
        let sub16_out1 = add80_out1.sub(reducemean29_out1);
        let pow15_out1 = sub16_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean30_out1 = { pow15_out1.mean_dim(2usize) };
        let add81_out1 = reducemean30_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt15_out1 = add81_out1.sqrt();
        let div22_out1 = sub16_out1.div(sqrt15_out1);
        let constant140_out1 = self.constant140.val();
        let mul44_out1 = div22_out1
            .mul((constant140_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant141_out1 = self.constant141.val();
        let add82_out1 = mul44_out1
            .add((constant141_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape31_out1: [i64; 3] = {
            let axes = &add82_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear43_out1 = self.linear43.forward(add82_out1.clone());
        let linear44_out1 = self.linear44.forward(add82_out1.clone());
        let linear45_out1 = self.linear45.forward(add82_out1.clone());
        let gather64_out1 = shape31_out1[0] as i64;
        let gather65_out1 = shape31_out1[1] as i64;
        let constant145_out1 = self.constant145.val();
        let add83_out1 = (constant145_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear43_out1);
        let constant146_out1 = self.constant146.val();
        let add84_out1 = (constant146_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear44_out1);
        let constant147_out1 = self.constant147.val();
        let add85_out1 = (constant147_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear45_out1);
        let shape32_out1: [i64; 3] = {
            let axes = &add83_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape33_out1: [i64; 3] = {
            let axes = &add84_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape34_out1: [i64; 3] = {
            let axes = &add85_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze62_out1 = [gather64_out1 as i64];
        let unsqueeze63_out1 = [gather65_out1 as i64];
        let gather66_out1 = shape32_out1[0] as i64;
        let gather67_out1 = shape32_out1[1] as i64;
        let gather68_out1 = shape33_out1[0] as i64;
        let gather69_out1 = shape33_out1[1] as i64;
        let gather70_out1 = shape34_out1[0] as i64;
        let gather71_out1 = shape34_out1[1] as i64;
        let concat31_out1: [i64; 3usize] = [
            &unsqueeze62_out1[..],
            &unsqueeze63_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze64_out1 = [gather66_out1 as i64];
        let unsqueeze65_out1 = [gather67_out1 as i64];
        let unsqueeze66_out1 = [gather68_out1 as i64];
        let unsqueeze67_out1 = [gather69_out1 as i64];
        let unsqueeze68_out1 = [gather70_out1 as i64];
        let unsqueeze69_out1 = [gather71_out1 as i64];
        let concat32_out1: [i64; 4usize] = [
            &unsqueeze64_out1[..],
            &unsqueeze65_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat33_out1: [i64; 4usize] = [
            &unsqueeze66_out1[..],
            &unsqueeze67_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat34_out1: [i64; 4usize] = [
            &unsqueeze68_out1[..],
            &unsqueeze69_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape29_out1 = add83_out1.reshape(concat32_out1);
        let reshape30_out1 = add84_out1.reshape(concat33_out1);
        let reshape31_out1 = add85_out1.reshape(concat34_out1);
        let transpose29_out1 = reshape29_out1.permute([0, 2, 1, 3]);
        let transpose30_out1 = reshape31_out1.permute([0, 2, 1, 3]);
        let transpose31_out1 = reshape30_out1.permute([0, 2, 3, 1]);
        let matmul60_k_corrected = transpose31_out1.permute([0, 1, 3, 2]);
        let (matmul61_out1,) = {
            let q = transpose29_out1;
            let k = matmul60_k_corrected;
            let v = transpose30_out1;
            let matmul61_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul61_out1,)
        };
        let transpose32_out1 = matmul61_out1.permute([0, 2, 1, 3]);
        let reshape32_out1 = transpose32_out1.reshape(concat31_out1);
        let linear46_out1 = self.linear46.forward(reshape32_out1);
        let add87_out1 = linear46_out1.add(add82_out1);
        add87_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule10 {
    constant150: burn::module::Param<Tensor<1>>,
    constant151: burn::module::Param<Tensor<1>>,
    linear47: Linear,
    linear48: Linear,
    constant156: burn::module::Param<Tensor<1>>,
    constant157: burn::module::Param<Tensor<1>>,
    linear49: Linear,
    linear50: Linear,
    linear51: Linear,
    constant161: burn::module::Param<Tensor<1>>,
    constant162: burn::module::Param<Tensor<1>>,
    constant163: burn::module::Param<Tensor<1>>,
    linear52: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule10 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
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
        let linear47 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear48 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant156: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant157: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear49 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear50 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear51 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant161: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant162: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant163: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear52 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant150,
            constant151,
            linear47,
            linear48,
            constant156,
            constant157,
            linear49,
            linear50,
            linear51,
            constant161,
            constant162,
            constant163,
            linear52,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add87_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean31_out1 = { add87_out1.clone().mean_dim(2usize) };
        let sub17_out1 = add87_out1.sub(reducemean31_out1);
        let pow16_out1 = sub17_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean32_out1 = { pow16_out1.mean_dim(2usize) };
        let add88_out1 = reducemean32_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt16_out1 = add88_out1.sqrt();
        let div23_out1 = sub17_out1.div(sqrt16_out1);
        let constant150_out1 = self.constant150.val();
        let mul47_out1 = div23_out1
            .mul((constant150_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant151_out1 = self.constant151.val();
        let add89_out1 = mul47_out1
            .add((constant151_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear47_out1 = self.linear47.forward(add89_out1.clone());
        let div24_out1 = linear47_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf8_out1 = div24_out1.erf();
        let add90_out1 = erf8_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul48_out1 = linear47_out1.mul(add90_out1);
        let mul49_out1 = mul48_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear48_out1 = self.linear48.forward(mul49_out1);
        let add91_out1 = linear48_out1.add(add89_out1);
        let reducemean33_out1 = { add91_out1.clone().mean_dim(2usize) };
        let sub18_out1 = add91_out1.sub(reducemean33_out1);
        let pow17_out1 = sub18_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean34_out1 = { pow17_out1.mean_dim(2usize) };
        let add92_out1 = reducemean34_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt17_out1 = add92_out1.sqrt();
        let div25_out1 = sub18_out1.div(sqrt17_out1);
        let constant156_out1 = self.constant156.val();
        let mul50_out1 = div25_out1
            .mul((constant156_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant157_out1 = self.constant157.val();
        let add93_out1 = mul50_out1
            .add((constant157_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape35_out1: [i64; 3] = {
            let axes = &add93_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear49_out1 = self.linear49.forward(add93_out1.clone());
        let linear50_out1 = self.linear50.forward(add93_out1.clone());
        let linear51_out1 = self.linear51.forward(add93_out1.clone());
        let gather72_out1 = shape35_out1[0] as i64;
        let gather73_out1 = shape35_out1[1] as i64;
        let constant161_out1 = self.constant161.val();
        let add94_out1 = (constant161_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear49_out1);
        let constant162_out1 = self.constant162.val();
        let add95_out1 = (constant162_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear50_out1);
        let constant163_out1 = self.constant163.val();
        let add96_out1 = (constant163_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear51_out1);
        let shape36_out1: [i64; 3] = {
            let axes = &add94_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape37_out1: [i64; 3] = {
            let axes = &add95_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape38_out1: [i64; 3] = {
            let axes = &add96_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze70_out1 = [gather72_out1 as i64];
        let unsqueeze71_out1 = [gather73_out1 as i64];
        let gather74_out1 = shape36_out1[0] as i64;
        let gather75_out1 = shape36_out1[1] as i64;
        let gather76_out1 = shape37_out1[0] as i64;
        let gather77_out1 = shape37_out1[1] as i64;
        let gather78_out1 = shape38_out1[0] as i64;
        let gather79_out1 = shape38_out1[1] as i64;
        let concat35_out1: [i64; 3usize] = [
            &unsqueeze70_out1[..],
            &unsqueeze71_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze72_out1 = [gather74_out1 as i64];
        let unsqueeze73_out1 = [gather75_out1 as i64];
        let unsqueeze74_out1 = [gather76_out1 as i64];
        let unsqueeze75_out1 = [gather77_out1 as i64];
        let unsqueeze76_out1 = [gather78_out1 as i64];
        let unsqueeze77_out1 = [gather79_out1 as i64];
        let concat36_out1: [i64; 4usize] = [
            &unsqueeze72_out1[..],
            &unsqueeze73_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat37_out1: [i64; 4usize] = [
            &unsqueeze74_out1[..],
            &unsqueeze75_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat38_out1: [i64; 4usize] = [
            &unsqueeze76_out1[..],
            &unsqueeze77_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape33_out1 = add94_out1.reshape(concat36_out1);
        let reshape34_out1 = add95_out1.reshape(concat37_out1);
        let reshape35_out1 = add96_out1.reshape(concat38_out1);
        let transpose33_out1 = reshape33_out1.permute([0, 2, 1, 3]);
        let transpose34_out1 = reshape35_out1.permute([0, 2, 1, 3]);
        let transpose35_out1 = reshape34_out1.permute([0, 2, 3, 1]);
        let matmul68_k_corrected = transpose35_out1.permute([0, 1, 3, 2]);
        let (matmul69_out1,) = {
            let q = transpose33_out1;
            let k = matmul68_k_corrected;
            let v = transpose34_out1;
            let matmul69_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul69_out1,)
        };
        let transpose36_out1 = matmul69_out1.permute([0, 2, 1, 3]);
        let reshape36_out1 = transpose36_out1.reshape(concat35_out1);
        let linear52_out1 = self.linear52.forward(reshape36_out1);
        let add98_out1 = linear52_out1.add(add93_out1);
        add98_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule11 {
    constant166: burn::module::Param<Tensor<1>>,
    constant167: burn::module::Param<Tensor<1>>,
    linear53: Linear,
    linear54: Linear,
    constant172: burn::module::Param<Tensor<1>>,
    constant173: burn::module::Param<Tensor<1>>,
    linear55: Linear,
    linear56: Linear,
    linear57: Linear,
    constant177: burn::module::Param<Tensor<1>>,
    constant178: burn::module::Param<Tensor<1>>,
    constant179: burn::module::Param<Tensor<1>>,
    linear58: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule11 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant166: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant167: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear53 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear54 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant172: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant173: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear55 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear56 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear57 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant177: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant178: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant179: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear58 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant166,
            constant167,
            linear53,
            linear54,
            constant172,
            constant173,
            linear55,
            linear56,
            linear57,
            constant177,
            constant178,
            constant179,
            linear58,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add98_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean35_out1 = { add98_out1.clone().mean_dim(2usize) };
        let sub19_out1 = add98_out1.sub(reducemean35_out1);
        let pow18_out1 = sub19_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean36_out1 = { pow18_out1.mean_dim(2usize) };
        let add99_out1 = reducemean36_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt18_out1 = add99_out1.sqrt();
        let div26_out1 = sub19_out1.div(sqrt18_out1);
        let constant166_out1 = self.constant166.val();
        let mul53_out1 = div26_out1
            .mul((constant166_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant167_out1 = self.constant167.val();
        let add100_out1 = mul53_out1
            .add((constant167_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear53_out1 = self.linear53.forward(add100_out1.clone());
        let div27_out1 = linear53_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf9_out1 = div27_out1.erf();
        let add101_out1 = erf9_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul54_out1 = linear53_out1.mul(add101_out1);
        let mul55_out1 = mul54_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear54_out1 = self.linear54.forward(mul55_out1);
        let add102_out1 = linear54_out1.add(add100_out1);
        let reducemean37_out1 = { add102_out1.clone().mean_dim(2usize) };
        let sub20_out1 = add102_out1.sub(reducemean37_out1);
        let pow19_out1 = sub20_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean38_out1 = { pow19_out1.mean_dim(2usize) };
        let add103_out1 = reducemean38_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt19_out1 = add103_out1.sqrt();
        let div28_out1 = sub20_out1.div(sqrt19_out1);
        let constant172_out1 = self.constant172.val();
        let mul56_out1 = div28_out1
            .mul((constant172_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant173_out1 = self.constant173.val();
        let add104_out1 = mul56_out1
            .add((constant173_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape39_out1: [i64; 3] = {
            let axes = &add104_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear55_out1 = self.linear55.forward(add104_out1.clone());
        let linear56_out1 = self.linear56.forward(add104_out1.clone());
        let linear57_out1 = self.linear57.forward(add104_out1.clone());
        let gather80_out1 = shape39_out1[0] as i64;
        let gather81_out1 = shape39_out1[1] as i64;
        let constant177_out1 = self.constant177.val();
        let add105_out1 = (constant177_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear55_out1);
        let constant178_out1 = self.constant178.val();
        let add106_out1 = (constant178_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear56_out1);
        let constant179_out1 = self.constant179.val();
        let add107_out1 = (constant179_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear57_out1);
        let shape40_out1: [i64; 3] = {
            let axes = &add105_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape41_out1: [i64; 3] = {
            let axes = &add106_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape42_out1: [i64; 3] = {
            let axes = &add107_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze78_out1 = [gather80_out1 as i64];
        let unsqueeze79_out1 = [gather81_out1 as i64];
        let gather82_out1 = shape40_out1[0] as i64;
        let gather83_out1 = shape40_out1[1] as i64;
        let gather84_out1 = shape41_out1[0] as i64;
        let gather85_out1 = shape41_out1[1] as i64;
        let gather86_out1 = shape42_out1[0] as i64;
        let gather87_out1 = shape42_out1[1] as i64;
        let concat39_out1: [i64; 3usize] = [
            &unsqueeze78_out1[..],
            &unsqueeze79_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze80_out1 = [gather82_out1 as i64];
        let unsqueeze81_out1 = [gather83_out1 as i64];
        let unsqueeze82_out1 = [gather84_out1 as i64];
        let unsqueeze83_out1 = [gather85_out1 as i64];
        let unsqueeze84_out1 = [gather86_out1 as i64];
        let unsqueeze85_out1 = [gather87_out1 as i64];
        let concat40_out1: [i64; 4usize] = [
            &unsqueeze80_out1[..],
            &unsqueeze81_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat41_out1: [i64; 4usize] = [
            &unsqueeze82_out1[..],
            &unsqueeze83_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat42_out1: [i64; 4usize] = [
            &unsqueeze84_out1[..],
            &unsqueeze85_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape37_out1 = add105_out1.reshape(concat40_out1);
        let reshape38_out1 = add106_out1.reshape(concat41_out1);
        let reshape39_out1 = add107_out1.reshape(concat42_out1);
        let transpose37_out1 = reshape37_out1.permute([0, 2, 1, 3]);
        let transpose38_out1 = reshape39_out1.permute([0, 2, 1, 3]);
        let transpose39_out1 = reshape38_out1.permute([0, 2, 3, 1]);
        let matmul76_k_corrected = transpose39_out1.permute([0, 1, 3, 2]);
        let (matmul77_out1,) = {
            let q = transpose37_out1;
            let k = matmul76_k_corrected;
            let v = transpose38_out1;
            let matmul77_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul77_out1,)
        };
        let transpose40_out1 = matmul77_out1.permute([0, 2, 1, 3]);
        let reshape40_out1 = transpose40_out1.reshape(concat39_out1);
        let linear58_out1 = self.linear58.forward(reshape40_out1);
        let add109_out1 = linear58_out1.add(add104_out1);
        add109_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule12 {
    constant182: burn::module::Param<Tensor<1>>,
    constant183: burn::module::Param<Tensor<1>>,
    linear59: Linear,
    linear60: Linear,
    constant188: burn::module::Param<Tensor<1>>,
    constant189: burn::module::Param<Tensor<1>>,
    linear61: Linear,
    linear62: Linear,
    linear63: Linear,
    constant193: burn::module::Param<Tensor<1>>,
    constant194: burn::module::Param<Tensor<1>>,
    constant195: burn::module::Param<Tensor<1>>,
    linear64: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule12 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant182: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant183: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear59 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear60 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant188: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant189: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear61 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear62 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear63 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant193: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
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
        let linear64 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant182,
            constant183,
            linear59,
            linear60,
            constant188,
            constant189,
            linear61,
            linear62,
            linear63,
            constant193,
            constant194,
            constant195,
            linear64,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add109_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean39_out1 = { add109_out1.clone().mean_dim(2usize) };
        let sub21_out1 = add109_out1.sub(reducemean39_out1);
        let pow20_out1 = sub21_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean40_out1 = { pow20_out1.mean_dim(2usize) };
        let add110_out1 = reducemean40_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt20_out1 = add110_out1.sqrt();
        let div29_out1 = sub21_out1.div(sqrt20_out1);
        let constant182_out1 = self.constant182.val();
        let mul59_out1 = div29_out1
            .mul((constant182_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant183_out1 = self.constant183.val();
        let add111_out1 = mul59_out1
            .add((constant183_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear59_out1 = self.linear59.forward(add111_out1.clone());
        let div30_out1 = linear59_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf10_out1 = div30_out1.erf();
        let add112_out1 = erf10_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul60_out1 = linear59_out1.mul(add112_out1);
        let mul61_out1 = mul60_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear60_out1 = self.linear60.forward(mul61_out1);
        let add113_out1 = linear60_out1.add(add111_out1);
        let reducemean41_out1 = { add113_out1.clone().mean_dim(2usize) };
        let sub22_out1 = add113_out1.sub(reducemean41_out1);
        let pow21_out1 = sub22_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean42_out1 = { pow21_out1.mean_dim(2usize) };
        let add114_out1 = reducemean42_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt21_out1 = add114_out1.sqrt();
        let div31_out1 = sub22_out1.div(sqrt21_out1);
        let constant188_out1 = self.constant188.val();
        let mul62_out1 = div31_out1
            .mul((constant188_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant189_out1 = self.constant189.val();
        let add115_out1 = mul62_out1
            .add((constant189_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape43_out1: [i64; 3] = {
            let axes = &add115_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear61_out1 = self.linear61.forward(add115_out1.clone());
        let linear62_out1 = self.linear62.forward(add115_out1.clone());
        let linear63_out1 = self.linear63.forward(add115_out1.clone());
        let gather88_out1 = shape43_out1[0] as i64;
        let gather89_out1 = shape43_out1[1] as i64;
        let constant193_out1 = self.constant193.val();
        let add116_out1 = (constant193_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear61_out1);
        let constant194_out1 = self.constant194.val();
        let add117_out1 = (constant194_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear62_out1);
        let constant195_out1 = self.constant195.val();
        let add118_out1 = (constant195_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear63_out1);
        let shape44_out1: [i64; 3] = {
            let axes = &add116_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape45_out1: [i64; 3] = {
            let axes = &add117_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape46_out1: [i64; 3] = {
            let axes = &add118_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze86_out1 = [gather88_out1 as i64];
        let unsqueeze87_out1 = [gather89_out1 as i64];
        let gather90_out1 = shape44_out1[0] as i64;
        let gather91_out1 = shape44_out1[1] as i64;
        let gather92_out1 = shape45_out1[0] as i64;
        let gather93_out1 = shape45_out1[1] as i64;
        let gather94_out1 = shape46_out1[0] as i64;
        let gather95_out1 = shape46_out1[1] as i64;
        let concat43_out1: [i64; 3usize] = [
            &unsqueeze86_out1[..],
            &unsqueeze87_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze88_out1 = [gather90_out1 as i64];
        let unsqueeze89_out1 = [gather91_out1 as i64];
        let unsqueeze90_out1 = [gather92_out1 as i64];
        let unsqueeze91_out1 = [gather93_out1 as i64];
        let unsqueeze92_out1 = [gather94_out1 as i64];
        let unsqueeze93_out1 = [gather95_out1 as i64];
        let concat44_out1: [i64; 4usize] = [
            &unsqueeze88_out1[..],
            &unsqueeze89_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat45_out1: [i64; 4usize] = [
            &unsqueeze90_out1[..],
            &unsqueeze91_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat46_out1: [i64; 4usize] = [
            &unsqueeze92_out1[..],
            &unsqueeze93_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape41_out1 = add116_out1.reshape(concat44_out1);
        let reshape42_out1 = add117_out1.reshape(concat45_out1);
        let reshape43_out1 = add118_out1.reshape(concat46_out1);
        let transpose41_out1 = reshape41_out1.permute([0, 2, 1, 3]);
        let transpose42_out1 = reshape43_out1.permute([0, 2, 1, 3]);
        let transpose43_out1 = reshape42_out1.permute([0, 2, 3, 1]);
        let matmul84_k_corrected = transpose43_out1.permute([0, 1, 3, 2]);
        let (matmul85_out1,) = {
            let q = transpose41_out1;
            let k = matmul84_k_corrected;
            let v = transpose42_out1;
            let matmul85_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul85_out1,)
        };
        let transpose44_out1 = matmul85_out1.permute([0, 2, 1, 3]);
        let reshape44_out1 = transpose44_out1.reshape(concat43_out1);
        let linear64_out1 = self.linear64.forward(reshape44_out1);
        let add120_out1 = linear64_out1.add(add115_out1);
        add120_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule13 {
    constant198: burn::module::Param<Tensor<1>>,
    constant199: burn::module::Param<Tensor<1>>,
    linear65: Linear,
    linear66: Linear,
    constant204: burn::module::Param<Tensor<1>>,
    constant205: burn::module::Param<Tensor<1>>,
    linear67: Linear,
    linear68: Linear,
    linear69: Linear,
    constant209: burn::module::Param<Tensor<1>>,
    constant210: burn::module::Param<Tensor<1>>,
    constant211: burn::module::Param<Tensor<1>>,
    linear70: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule13 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant198: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant199: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear65 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear66 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
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
        let linear67 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear68 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear69 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant209: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
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
        let linear70 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant198,
            constant199,
            linear65,
            linear66,
            constant204,
            constant205,
            linear67,
            linear68,
            linear69,
            constant209,
            constant210,
            constant211,
            linear70,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add120_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean43_out1 = { add120_out1.clone().mean_dim(2usize) };
        let sub23_out1 = add120_out1.sub(reducemean43_out1);
        let pow22_out1 = sub23_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean44_out1 = { pow22_out1.mean_dim(2usize) };
        let add121_out1 = reducemean44_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt22_out1 = add121_out1.sqrt();
        let div32_out1 = sub23_out1.div(sqrt22_out1);
        let constant198_out1 = self.constant198.val();
        let mul65_out1 = div32_out1
            .mul((constant198_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant199_out1 = self.constant199.val();
        let add122_out1 = mul65_out1
            .add((constant199_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear65_out1 = self.linear65.forward(add122_out1.clone());
        let div33_out1 = linear65_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf11_out1 = div33_out1.erf();
        let add123_out1 = erf11_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul66_out1 = linear65_out1.mul(add123_out1);
        let mul67_out1 = mul66_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear66_out1 = self.linear66.forward(mul67_out1);
        let add124_out1 = linear66_out1.add(add122_out1);
        let reducemean45_out1 = { add124_out1.clone().mean_dim(2usize) };
        let sub24_out1 = add124_out1.sub(reducemean45_out1);
        let pow23_out1 = sub24_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean46_out1 = { pow23_out1.mean_dim(2usize) };
        let add125_out1 = reducemean46_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt23_out1 = add125_out1.sqrt();
        let div34_out1 = sub24_out1.div(sqrt23_out1);
        let constant204_out1 = self.constant204.val();
        let mul68_out1 = div34_out1
            .mul((constant204_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant205_out1 = self.constant205.val();
        let add126_out1 = mul68_out1
            .add((constant205_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape47_out1: [i64; 3] = {
            let axes = &add126_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear67_out1 = self.linear67.forward(add126_out1.clone());
        let linear68_out1 = self.linear68.forward(add126_out1.clone());
        let linear69_out1 = self.linear69.forward(add126_out1.clone());
        let gather96_out1 = shape47_out1[0] as i64;
        let gather97_out1 = shape47_out1[1] as i64;
        let constant209_out1 = self.constant209.val();
        let add127_out1 = (constant209_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear67_out1);
        let constant210_out1 = self.constant210.val();
        let add128_out1 = (constant210_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear68_out1);
        let constant211_out1 = self.constant211.val();
        let add129_out1 = (constant211_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear69_out1);
        let shape48_out1: [i64; 3] = {
            let axes = &add127_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape49_out1: [i64; 3] = {
            let axes = &add128_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape50_out1: [i64; 3] = {
            let axes = &add129_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze94_out1 = [gather96_out1 as i64];
        let unsqueeze95_out1 = [gather97_out1 as i64];
        let gather98_out1 = shape48_out1[0] as i64;
        let gather99_out1 = shape48_out1[1] as i64;
        let gather100_out1 = shape49_out1[0] as i64;
        let gather101_out1 = shape49_out1[1] as i64;
        let gather102_out1 = shape50_out1[0] as i64;
        let gather103_out1 = shape50_out1[1] as i64;
        let concat47_out1: [i64; 3usize] = [
            &unsqueeze94_out1[..],
            &unsqueeze95_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze96_out1 = [gather98_out1 as i64];
        let unsqueeze97_out1 = [gather99_out1 as i64];
        let unsqueeze98_out1 = [gather100_out1 as i64];
        let unsqueeze99_out1 = [gather101_out1 as i64];
        let unsqueeze100_out1 = [gather102_out1 as i64];
        let unsqueeze101_out1 = [gather103_out1 as i64];
        let concat48_out1: [i64; 4usize] = [
            &unsqueeze96_out1[..],
            &unsqueeze97_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat49_out1: [i64; 4usize] = [
            &unsqueeze98_out1[..],
            &unsqueeze99_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat50_out1: [i64; 4usize] = [
            &unsqueeze100_out1[..],
            &unsqueeze101_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape45_out1 = add127_out1.reshape(concat48_out1);
        let reshape46_out1 = add128_out1.reshape(concat49_out1);
        let reshape47_out1 = add129_out1.reshape(concat50_out1);
        let transpose45_out1 = reshape45_out1.permute([0, 2, 1, 3]);
        let transpose46_out1 = reshape47_out1.permute([0, 2, 1, 3]);
        let transpose47_out1 = reshape46_out1.permute([0, 2, 3, 1]);
        let matmul92_k_corrected = transpose47_out1.permute([0, 1, 3, 2]);
        let (matmul93_out1,) = {
            let q = transpose45_out1;
            let k = matmul92_k_corrected;
            let v = transpose46_out1;
            let matmul93_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul93_out1,)
        };
        let transpose48_out1 = matmul93_out1.permute([0, 2, 1, 3]);
        let reshape48_out1 = transpose48_out1.reshape(concat47_out1);
        let linear70_out1 = self.linear70.forward(reshape48_out1);
        let add131_out1 = linear70_out1.add(add126_out1);
        add131_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule14 {
    constant214: burn::module::Param<Tensor<1>>,
    constant215: burn::module::Param<Tensor<1>>,
    linear71: Linear,
    linear72: Linear,
    constant220: burn::module::Param<Tensor<1>>,
    constant221: burn::module::Param<Tensor<1>>,
    linear73: Linear,
    linear74: Linear,
    linear75: Linear,
    constant225: burn::module::Param<Tensor<1>>,
    constant226: burn::module::Param<Tensor<1>>,
    constant227: burn::module::Param<Tensor<1>>,
    linear76: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule14 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
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
        let linear71 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear72 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
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
        let linear73 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear74 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear75 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant225: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant226: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant227: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear76 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant214,
            constant215,
            linear71,
            linear72,
            constant220,
            constant221,
            linear73,
            linear74,
            linear75,
            constant225,
            constant226,
            constant227,
            linear76,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add131_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean47_out1 = { add131_out1.clone().mean_dim(2usize) };
        let sub25_out1 = add131_out1.sub(reducemean47_out1);
        let pow24_out1 = sub25_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean48_out1 = { pow24_out1.mean_dim(2usize) };
        let add132_out1 = reducemean48_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt24_out1 = add132_out1.sqrt();
        let div35_out1 = sub25_out1.div(sqrt24_out1);
        let constant214_out1 = self.constant214.val();
        let mul71_out1 = div35_out1
            .mul((constant214_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant215_out1 = self.constant215.val();
        let add133_out1 = mul71_out1
            .add((constant215_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear71_out1 = self.linear71.forward(add133_out1.clone());
        let div36_out1 = linear71_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf12_out1 = div36_out1.erf();
        let add134_out1 = erf12_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul72_out1 = linear71_out1.mul(add134_out1);
        let mul73_out1 = mul72_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear72_out1 = self.linear72.forward(mul73_out1);
        let add135_out1 = linear72_out1.add(add133_out1);
        let reducemean49_out1 = { add135_out1.clone().mean_dim(2usize) };
        let sub26_out1 = add135_out1.sub(reducemean49_out1);
        let pow25_out1 = sub26_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean50_out1 = { pow25_out1.mean_dim(2usize) };
        let add136_out1 = reducemean50_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt25_out1 = add136_out1.sqrt();
        let div37_out1 = sub26_out1.div(sqrt25_out1);
        let constant220_out1 = self.constant220.val();
        let mul74_out1 = div37_out1
            .mul((constant220_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant221_out1 = self.constant221.val();
        let add137_out1 = mul74_out1
            .add((constant221_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape51_out1: [i64; 3] = {
            let axes = &add137_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear73_out1 = self.linear73.forward(add137_out1.clone());
        let linear74_out1 = self.linear74.forward(add137_out1.clone());
        let linear75_out1 = self.linear75.forward(add137_out1.clone());
        let gather104_out1 = shape51_out1[0] as i64;
        let gather105_out1 = shape51_out1[1] as i64;
        let constant225_out1 = self.constant225.val();
        let add138_out1 = (constant225_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear73_out1);
        let constant226_out1 = self.constant226.val();
        let add139_out1 = (constant226_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear74_out1);
        let constant227_out1 = self.constant227.val();
        let add140_out1 = (constant227_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear75_out1);
        let shape52_out1: [i64; 3] = {
            let axes = &add138_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape53_out1: [i64; 3] = {
            let axes = &add139_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape54_out1: [i64; 3] = {
            let axes = &add140_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze102_out1 = [gather104_out1 as i64];
        let unsqueeze103_out1 = [gather105_out1 as i64];
        let gather106_out1 = shape52_out1[0] as i64;
        let gather107_out1 = shape52_out1[1] as i64;
        let gather108_out1 = shape53_out1[0] as i64;
        let gather109_out1 = shape53_out1[1] as i64;
        let gather110_out1 = shape54_out1[0] as i64;
        let gather111_out1 = shape54_out1[1] as i64;
        let concat51_out1: [i64; 3usize] = [
            &unsqueeze102_out1[..],
            &unsqueeze103_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze104_out1 = [gather106_out1 as i64];
        let unsqueeze105_out1 = [gather107_out1 as i64];
        let unsqueeze106_out1 = [gather108_out1 as i64];
        let unsqueeze107_out1 = [gather109_out1 as i64];
        let unsqueeze108_out1 = [gather110_out1 as i64];
        let unsqueeze109_out1 = [gather111_out1 as i64];
        let concat52_out1: [i64; 4usize] = [
            &unsqueeze104_out1[..],
            &unsqueeze105_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat53_out1: [i64; 4usize] = [
            &unsqueeze106_out1[..],
            &unsqueeze107_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat54_out1: [i64; 4usize] = [
            &unsqueeze108_out1[..],
            &unsqueeze109_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape49_out1 = add138_out1.reshape(concat52_out1);
        let reshape50_out1 = add139_out1.reshape(concat53_out1);
        let reshape51_out1 = add140_out1.reshape(concat54_out1);
        let transpose49_out1 = reshape49_out1.permute([0, 2, 1, 3]);
        let transpose50_out1 = reshape51_out1.permute([0, 2, 1, 3]);
        let transpose51_out1 = reshape50_out1.permute([0, 2, 3, 1]);
        let matmul100_k_corrected = transpose51_out1.permute([0, 1, 3, 2]);
        let (matmul101_out1,) = {
            let q = transpose49_out1;
            let k = matmul100_k_corrected;
            let v = transpose50_out1;
            let matmul101_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul101_out1,)
        };
        let transpose52_out1 = matmul101_out1.permute([0, 2, 1, 3]);
        let reshape52_out1 = transpose52_out1.reshape(concat51_out1);
        let linear76_out1 = self.linear76.forward(reshape52_out1);
        let add142_out1 = linear76_out1.add(add137_out1);
        add142_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule15 {
    constant230: burn::module::Param<Tensor<1>>,
    constant231: burn::module::Param<Tensor<1>>,
    linear77: Linear,
    linear78: Linear,
    constant236: burn::module::Param<Tensor<1>>,
    constant237: burn::module::Param<Tensor<1>>,
    linear79: Linear,
    linear80: Linear,
    linear81: Linear,
    constant241: burn::module::Param<Tensor<1>>,
    constant242: burn::module::Param<Tensor<1>>,
    constant243: burn::module::Param<Tensor<1>>,
    linear82: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule15 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
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
        let linear77 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear78 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant236: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant237: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear79 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear80 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear81 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant241: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant242: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant243: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear82 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant230,
            constant231,
            linear77,
            linear78,
            constant236,
            constant237,
            linear79,
            linear80,
            linear81,
            constant241,
            constant242,
            constant243,
            linear82,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add142_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean51_out1 = { add142_out1.clone().mean_dim(2usize) };
        let sub27_out1 = add142_out1.sub(reducemean51_out1);
        let pow26_out1 = sub27_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean52_out1 = { pow26_out1.mean_dim(2usize) };
        let add143_out1 = reducemean52_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt26_out1 = add143_out1.sqrt();
        let div38_out1 = sub27_out1.div(sqrt26_out1);
        let constant230_out1 = self.constant230.val();
        let mul77_out1 = div38_out1
            .mul((constant230_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant231_out1 = self.constant231.val();
        let add144_out1 = mul77_out1
            .add((constant231_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear77_out1 = self.linear77.forward(add144_out1.clone());
        let div39_out1 = linear77_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf13_out1 = div39_out1.erf();
        let add145_out1 = erf13_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul78_out1 = linear77_out1.mul(add145_out1);
        let mul79_out1 = mul78_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear78_out1 = self.linear78.forward(mul79_out1);
        let add146_out1 = linear78_out1.add(add144_out1);
        let reducemean53_out1 = { add146_out1.clone().mean_dim(2usize) };
        let sub28_out1 = add146_out1.sub(reducemean53_out1);
        let pow27_out1 = sub28_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean54_out1 = { pow27_out1.mean_dim(2usize) };
        let add147_out1 = reducemean54_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt27_out1 = add147_out1.sqrt();
        let div40_out1 = sub28_out1.div(sqrt27_out1);
        let constant236_out1 = self.constant236.val();
        let mul80_out1 = div40_out1
            .mul((constant236_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant237_out1 = self.constant237.val();
        let add148_out1 = mul80_out1
            .add((constant237_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape55_out1: [i64; 3] = {
            let axes = &add148_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear79_out1 = self.linear79.forward(add148_out1.clone());
        let linear80_out1 = self.linear80.forward(add148_out1.clone());
        let linear81_out1 = self.linear81.forward(add148_out1.clone());
        let gather112_out1 = shape55_out1[0] as i64;
        let gather113_out1 = shape55_out1[1] as i64;
        let constant241_out1 = self.constant241.val();
        let add149_out1 = (constant241_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear79_out1);
        let constant242_out1 = self.constant242.val();
        let add150_out1 = (constant242_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear80_out1);
        let constant243_out1 = self.constant243.val();
        let add151_out1 = (constant243_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear81_out1);
        let shape56_out1: [i64; 3] = {
            let axes = &add149_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape57_out1: [i64; 3] = {
            let axes = &add150_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape58_out1: [i64; 3] = {
            let axes = &add151_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze110_out1 = [gather112_out1 as i64];
        let unsqueeze111_out1 = [gather113_out1 as i64];
        let gather114_out1 = shape56_out1[0] as i64;
        let gather115_out1 = shape56_out1[1] as i64;
        let gather116_out1 = shape57_out1[0] as i64;
        let gather117_out1 = shape57_out1[1] as i64;
        let gather118_out1 = shape58_out1[0] as i64;
        let gather119_out1 = shape58_out1[1] as i64;
        let concat55_out1: [i64; 3usize] = [
            &unsqueeze110_out1[..],
            &unsqueeze111_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze112_out1 = [gather114_out1 as i64];
        let unsqueeze113_out1 = [gather115_out1 as i64];
        let unsqueeze114_out1 = [gather116_out1 as i64];
        let unsqueeze115_out1 = [gather117_out1 as i64];
        let unsqueeze116_out1 = [gather118_out1 as i64];
        let unsqueeze117_out1 = [gather119_out1 as i64];
        let concat56_out1: [i64; 4usize] = [
            &unsqueeze112_out1[..],
            &unsqueeze113_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat57_out1: [i64; 4usize] = [
            &unsqueeze114_out1[..],
            &unsqueeze115_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat58_out1: [i64; 4usize] = [
            &unsqueeze116_out1[..],
            &unsqueeze117_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape53_out1 = add149_out1.reshape(concat56_out1);
        let reshape54_out1 = add150_out1.reshape(concat57_out1);
        let reshape55_out1 = add151_out1.reshape(concat58_out1);
        let transpose53_out1 = reshape53_out1.permute([0, 2, 1, 3]);
        let transpose54_out1 = reshape55_out1.permute([0, 2, 1, 3]);
        let transpose55_out1 = reshape54_out1.permute([0, 2, 3, 1]);
        let matmul108_k_corrected = transpose55_out1.permute([0, 1, 3, 2]);
        let (matmul109_out1,) = {
            let q = transpose53_out1;
            let k = matmul108_k_corrected;
            let v = transpose54_out1;
            let matmul109_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul109_out1,)
        };
        let transpose56_out1 = matmul109_out1.permute([0, 2, 1, 3]);
        let reshape56_out1 = transpose56_out1.reshape(concat55_out1);
        let linear82_out1 = self.linear82.forward(reshape56_out1);
        let add153_out1 = linear82_out1.add(add148_out1);
        add153_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule16 {
    constant246: burn::module::Param<Tensor<1>>,
    constant247: burn::module::Param<Tensor<1>>,
    linear83: Linear,
    linear84: Linear,
    constant252: burn::module::Param<Tensor<1>>,
    constant253: burn::module::Param<Tensor<1>>,
    linear85: Linear,
    linear86: Linear,
    linear87: Linear,
    constant257: burn::module::Param<Tensor<1>>,
    constant258: burn::module::Param<Tensor<1>>,
    constant259: burn::module::Param<Tensor<1>>,
    linear88: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule16 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant246: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant247: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear83 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear84 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant252: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant253: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear85 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear86 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear87 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant257: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant258: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant259: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear88 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant246,
            constant247,
            linear83,
            linear84,
            constant252,
            constant253,
            linear85,
            linear86,
            linear87,
            constant257,
            constant258,
            constant259,
            linear88,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add153_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean55_out1 = { add153_out1.clone().mean_dim(2usize) };
        let sub29_out1 = add153_out1.sub(reducemean55_out1);
        let pow28_out1 = sub29_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean56_out1 = { pow28_out1.mean_dim(2usize) };
        let add154_out1 = reducemean56_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt28_out1 = add154_out1.sqrt();
        let div41_out1 = sub29_out1.div(sqrt28_out1);
        let constant246_out1 = self.constant246.val();
        let mul83_out1 = div41_out1
            .mul((constant246_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant247_out1 = self.constant247.val();
        let add155_out1 = mul83_out1
            .add((constant247_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear83_out1 = self.linear83.forward(add155_out1.clone());
        let div42_out1 = linear83_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf14_out1 = div42_out1.erf();
        let add156_out1 = erf14_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul84_out1 = linear83_out1.mul(add156_out1);
        let mul85_out1 = mul84_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear84_out1 = self.linear84.forward(mul85_out1);
        let add157_out1 = linear84_out1.add(add155_out1);
        let reducemean57_out1 = { add157_out1.clone().mean_dim(2usize) };
        let sub30_out1 = add157_out1.sub(reducemean57_out1);
        let pow29_out1 = sub30_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean58_out1 = { pow29_out1.mean_dim(2usize) };
        let add158_out1 = reducemean58_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt29_out1 = add158_out1.sqrt();
        let div43_out1 = sub30_out1.div(sqrt29_out1);
        let constant252_out1 = self.constant252.val();
        let mul86_out1 = div43_out1
            .mul((constant252_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant253_out1 = self.constant253.val();
        let add159_out1 = mul86_out1
            .add((constant253_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape59_out1: [i64; 3] = {
            let axes = &add159_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear85_out1 = self.linear85.forward(add159_out1.clone());
        let linear86_out1 = self.linear86.forward(add159_out1.clone());
        let linear87_out1 = self.linear87.forward(add159_out1.clone());
        let gather120_out1 = shape59_out1[0] as i64;
        let gather121_out1 = shape59_out1[1] as i64;
        let constant257_out1 = self.constant257.val();
        let add160_out1 = (constant257_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear85_out1);
        let constant258_out1 = self.constant258.val();
        let add161_out1 = (constant258_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear86_out1);
        let constant259_out1 = self.constant259.val();
        let add162_out1 = (constant259_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear87_out1);
        let shape60_out1: [i64; 3] = {
            let axes = &add160_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape61_out1: [i64; 3] = {
            let axes = &add161_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape62_out1: [i64; 3] = {
            let axes = &add162_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze118_out1 = [gather120_out1 as i64];
        let unsqueeze119_out1 = [gather121_out1 as i64];
        let gather122_out1 = shape60_out1[0] as i64;
        let gather123_out1 = shape60_out1[1] as i64;
        let gather124_out1 = shape61_out1[0] as i64;
        let gather125_out1 = shape61_out1[1] as i64;
        let gather126_out1 = shape62_out1[0] as i64;
        let gather127_out1 = shape62_out1[1] as i64;
        let concat59_out1: [i64; 3usize] = [
            &unsqueeze118_out1[..],
            &unsqueeze119_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze120_out1 = [gather122_out1 as i64];
        let unsqueeze121_out1 = [gather123_out1 as i64];
        let unsqueeze122_out1 = [gather124_out1 as i64];
        let unsqueeze123_out1 = [gather125_out1 as i64];
        let unsqueeze124_out1 = [gather126_out1 as i64];
        let unsqueeze125_out1 = [gather127_out1 as i64];
        let concat60_out1: [i64; 4usize] = [
            &unsqueeze120_out1[..],
            &unsqueeze121_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat61_out1: [i64; 4usize] = [
            &unsqueeze122_out1[..],
            &unsqueeze123_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat62_out1: [i64; 4usize] = [
            &unsqueeze124_out1[..],
            &unsqueeze125_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape57_out1 = add160_out1.reshape(concat60_out1);
        let reshape58_out1 = add161_out1.reshape(concat61_out1);
        let reshape59_out1 = add162_out1.reshape(concat62_out1);
        let transpose57_out1 = reshape57_out1.permute([0, 2, 1, 3]);
        let transpose58_out1 = reshape59_out1.permute([0, 2, 1, 3]);
        let transpose59_out1 = reshape58_out1.permute([0, 2, 3, 1]);
        let matmul116_k_corrected = transpose59_out1.permute([0, 1, 3, 2]);
        let (matmul117_out1,) = {
            let q = transpose57_out1;
            let k = matmul116_k_corrected;
            let v = transpose58_out1;
            let matmul117_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul117_out1,)
        };
        let transpose60_out1 = matmul117_out1.permute([0, 2, 1, 3]);
        let reshape60_out1 = transpose60_out1.reshape(concat59_out1);
        let linear88_out1 = self.linear88.forward(reshape60_out1);
        let add164_out1 = linear88_out1.add(add159_out1);
        add164_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule17 {
    constant262: burn::module::Param<Tensor<1>>,
    constant263: burn::module::Param<Tensor<1>>,
    linear89: Linear,
    linear90: Linear,
    constant268: burn::module::Param<Tensor<1>>,
    constant269: burn::module::Param<Tensor<1>>,
    linear91: Linear,
    linear92: Linear,
    linear93: Linear,
    constant273: burn::module::Param<Tensor<1>>,
    constant274: burn::module::Param<Tensor<1>>,
    constant275: burn::module::Param<Tensor<1>>,
    linear94: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule17 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant262: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant263: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear89 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear90 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant268: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant269: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear91 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear92 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear93 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant273: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant274: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant275: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear94 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant262,
            constant263,
            linear89,
            linear90,
            constant268,
            constant269,
            linear91,
            linear92,
            linear93,
            constant273,
            constant274,
            constant275,
            linear94,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add164_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean59_out1 = { add164_out1.clone().mean_dim(2usize) };
        let sub31_out1 = add164_out1.sub(reducemean59_out1);
        let pow30_out1 = sub31_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean60_out1 = { pow30_out1.mean_dim(2usize) };
        let add165_out1 = reducemean60_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt30_out1 = add165_out1.sqrt();
        let div44_out1 = sub31_out1.div(sqrt30_out1);
        let constant262_out1 = self.constant262.val();
        let mul89_out1 = div44_out1
            .mul((constant262_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant263_out1 = self.constant263.val();
        let add166_out1 = mul89_out1
            .add((constant263_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear89_out1 = self.linear89.forward(add166_out1.clone());
        let div45_out1 = linear89_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf15_out1 = div45_out1.erf();
        let add167_out1 = erf15_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul90_out1 = linear89_out1.mul(add167_out1);
        let mul91_out1 = mul90_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear90_out1 = self.linear90.forward(mul91_out1);
        let add168_out1 = linear90_out1.add(add166_out1);
        let reducemean61_out1 = { add168_out1.clone().mean_dim(2usize) };
        let sub32_out1 = add168_out1.sub(reducemean61_out1);
        let pow31_out1 = sub32_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean62_out1 = { pow31_out1.mean_dim(2usize) };
        let add169_out1 = reducemean62_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt31_out1 = add169_out1.sqrt();
        let div46_out1 = sub32_out1.div(sqrt31_out1);
        let constant268_out1 = self.constant268.val();
        let mul92_out1 = div46_out1
            .mul((constant268_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant269_out1 = self.constant269.val();
        let add170_out1 = mul92_out1
            .add((constant269_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape63_out1: [i64; 3] = {
            let axes = &add170_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear91_out1 = self.linear91.forward(add170_out1.clone());
        let linear92_out1 = self.linear92.forward(add170_out1.clone());
        let linear93_out1 = self.linear93.forward(add170_out1.clone());
        let gather128_out1 = shape63_out1[0] as i64;
        let gather129_out1 = shape63_out1[1] as i64;
        let constant273_out1 = self.constant273.val();
        let add171_out1 = (constant273_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear91_out1);
        let constant274_out1 = self.constant274.val();
        let add172_out1 = (constant274_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear92_out1);
        let constant275_out1 = self.constant275.val();
        let add173_out1 = (constant275_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear93_out1);
        let shape64_out1: [i64; 3] = {
            let axes = &add171_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape65_out1: [i64; 3] = {
            let axes = &add172_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape66_out1: [i64; 3] = {
            let axes = &add173_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze126_out1 = [gather128_out1 as i64];
        let unsqueeze127_out1 = [gather129_out1 as i64];
        let gather130_out1 = shape64_out1[0] as i64;
        let gather131_out1 = shape64_out1[1] as i64;
        let gather132_out1 = shape65_out1[0] as i64;
        let gather133_out1 = shape65_out1[1] as i64;
        let gather134_out1 = shape66_out1[0] as i64;
        let gather135_out1 = shape66_out1[1] as i64;
        let concat63_out1: [i64; 3usize] = [
            &unsqueeze126_out1[..],
            &unsqueeze127_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze128_out1 = [gather130_out1 as i64];
        let unsqueeze129_out1 = [gather131_out1 as i64];
        let unsqueeze130_out1 = [gather132_out1 as i64];
        let unsqueeze131_out1 = [gather133_out1 as i64];
        let unsqueeze132_out1 = [gather134_out1 as i64];
        let unsqueeze133_out1 = [gather135_out1 as i64];
        let concat64_out1: [i64; 4usize] = [
            &unsqueeze128_out1[..],
            &unsqueeze129_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat65_out1: [i64; 4usize] = [
            &unsqueeze130_out1[..],
            &unsqueeze131_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat66_out1: [i64; 4usize] = [
            &unsqueeze132_out1[..],
            &unsqueeze133_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape61_out1 = add171_out1.reshape(concat64_out1);
        let reshape62_out1 = add172_out1.reshape(concat65_out1);
        let reshape63_out1 = add173_out1.reshape(concat66_out1);
        let transpose61_out1 = reshape61_out1.permute([0, 2, 1, 3]);
        let transpose62_out1 = reshape63_out1.permute([0, 2, 1, 3]);
        let transpose63_out1 = reshape62_out1.permute([0, 2, 3, 1]);
        let matmul124_k_corrected = transpose63_out1.permute([0, 1, 3, 2]);
        let (matmul125_out1,) = {
            let q = transpose61_out1;
            let k = matmul124_k_corrected;
            let v = transpose62_out1;
            let matmul125_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul125_out1,)
        };
        let transpose64_out1 = matmul125_out1.permute([0, 2, 1, 3]);
        let reshape64_out1 = transpose64_out1.reshape(concat63_out1);
        let linear94_out1 = self.linear94.forward(reshape64_out1);
        let add175_out1 = linear94_out1.add(add170_out1);
        add175_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule18 {
    constant278: burn::module::Param<Tensor<1>>,
    constant279: burn::module::Param<Tensor<1>>,
    linear95: Linear,
    linear96: Linear,
    constant284: burn::module::Param<Tensor<1>>,
    constant285: burn::module::Param<Tensor<1>>,
    linear97: Linear,
    linear98: Linear,
    linear99: Linear,
    constant289: burn::module::Param<Tensor<1>>,
    constant290: burn::module::Param<Tensor<1>>,
    constant291: burn::module::Param<Tensor<1>>,
    linear100: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule18 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant278: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant279: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear95 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear96 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant284: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant285: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear97 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear98 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear99 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant289: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant290: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant291: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear100 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant278,
            constant279,
            linear95,
            linear96,
            constant284,
            constant285,
            linear97,
            linear98,
            linear99,
            constant289,
            constant290,
            constant291,
            linear100,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add175_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean63_out1 = { add175_out1.clone().mean_dim(2usize) };
        let sub33_out1 = add175_out1.sub(reducemean63_out1);
        let pow32_out1 = sub33_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean64_out1 = { pow32_out1.mean_dim(2usize) };
        let add176_out1 = reducemean64_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt32_out1 = add176_out1.sqrt();
        let div47_out1 = sub33_out1.div(sqrt32_out1);
        let constant278_out1 = self.constant278.val();
        let mul95_out1 = div47_out1
            .mul((constant278_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant279_out1 = self.constant279.val();
        let add177_out1 = mul95_out1
            .add((constant279_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear95_out1 = self.linear95.forward(add177_out1.clone());
        let div48_out1 = linear95_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf16_out1 = div48_out1.erf();
        let add178_out1 = erf16_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul96_out1 = linear95_out1.mul(add178_out1);
        let mul97_out1 = mul96_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear96_out1 = self.linear96.forward(mul97_out1);
        let add179_out1 = linear96_out1.add(add177_out1);
        let reducemean65_out1 = { add179_out1.clone().mean_dim(2usize) };
        let sub34_out1 = add179_out1.sub(reducemean65_out1);
        let pow33_out1 = sub34_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean66_out1 = { pow33_out1.mean_dim(2usize) };
        let add180_out1 = reducemean66_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt33_out1 = add180_out1.sqrt();
        let div49_out1 = sub34_out1.div(sqrt33_out1);
        let constant284_out1 = self.constant284.val();
        let mul98_out1 = div49_out1
            .mul((constant284_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant285_out1 = self.constant285.val();
        let add181_out1 = mul98_out1
            .add((constant285_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape67_out1: [i64; 3] = {
            let axes = &add181_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear97_out1 = self.linear97.forward(add181_out1.clone());
        let linear98_out1 = self.linear98.forward(add181_out1.clone());
        let linear99_out1 = self.linear99.forward(add181_out1.clone());
        let gather136_out1 = shape67_out1[0] as i64;
        let gather137_out1 = shape67_out1[1] as i64;
        let constant289_out1 = self.constant289.val();
        let add182_out1 = (constant289_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear97_out1);
        let constant290_out1 = self.constant290.val();
        let add183_out1 = (constant290_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear98_out1);
        let constant291_out1 = self.constant291.val();
        let add184_out1 = (constant291_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear99_out1);
        let shape68_out1: [i64; 3] = {
            let axes = &add182_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape69_out1: [i64; 3] = {
            let axes = &add183_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape70_out1: [i64; 3] = {
            let axes = &add184_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze134_out1 = [gather136_out1 as i64];
        let unsqueeze135_out1 = [gather137_out1 as i64];
        let gather138_out1 = shape68_out1[0] as i64;
        let gather139_out1 = shape68_out1[1] as i64;
        let gather140_out1 = shape69_out1[0] as i64;
        let gather141_out1 = shape69_out1[1] as i64;
        let gather142_out1 = shape70_out1[0] as i64;
        let gather143_out1 = shape70_out1[1] as i64;
        let concat67_out1: [i64; 3usize] = [
            &unsqueeze134_out1[..],
            &unsqueeze135_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze136_out1 = [gather138_out1 as i64];
        let unsqueeze137_out1 = [gather139_out1 as i64];
        let unsqueeze138_out1 = [gather140_out1 as i64];
        let unsqueeze139_out1 = [gather141_out1 as i64];
        let unsqueeze140_out1 = [gather142_out1 as i64];
        let unsqueeze141_out1 = [gather143_out1 as i64];
        let concat68_out1: [i64; 4usize] = [
            &unsqueeze136_out1[..],
            &unsqueeze137_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat69_out1: [i64; 4usize] = [
            &unsqueeze138_out1[..],
            &unsqueeze139_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat70_out1: [i64; 4usize] = [
            &unsqueeze140_out1[..],
            &unsqueeze141_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape65_out1 = add182_out1.reshape(concat68_out1);
        let reshape66_out1 = add183_out1.reshape(concat69_out1);
        let reshape67_out1 = add184_out1.reshape(concat70_out1);
        let transpose65_out1 = reshape65_out1.permute([0, 2, 1, 3]);
        let transpose66_out1 = reshape67_out1.permute([0, 2, 1, 3]);
        let transpose67_out1 = reshape66_out1.permute([0, 2, 3, 1]);
        let matmul132_k_corrected = transpose67_out1.permute([0, 1, 3, 2]);
        let (matmul133_out1,) = {
            let q = transpose65_out1;
            let k = matmul132_k_corrected;
            let v = transpose66_out1;
            let matmul133_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul133_out1,)
        };
        let transpose68_out1 = matmul133_out1.permute([0, 2, 1, 3]);
        let reshape68_out1 = transpose68_out1.reshape(concat67_out1);
        let linear100_out1 = self.linear100.forward(reshape68_out1);
        let add186_out1 = linear100_out1.add(add181_out1);
        add186_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule19 {
    constant294: burn::module::Param<Tensor<1>>,
    constant295: burn::module::Param<Tensor<1>>,
    linear101: Linear,
    linear102: Linear,
    constant300: burn::module::Param<Tensor<1>>,
    constant301: burn::module::Param<Tensor<1>>,
    linear103: Linear,
    linear104: Linear,
    linear105: Linear,
    constant305: burn::module::Param<Tensor<1>>,
    constant306: burn::module::Param<Tensor<1>>,
    constant307: burn::module::Param<Tensor<1>>,
    linear106: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule19 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant294: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant295: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear101 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear102 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant300: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant301: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear103 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear104 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear105 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant305: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant306: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant307: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear106 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant294,
            constant295,
            linear101,
            linear102,
            constant300,
            constant301,
            linear103,
            linear104,
            linear105,
            constant305,
            constant306,
            constant307,
            linear106,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add186_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean67_out1 = { add186_out1.clone().mean_dim(2usize) };
        let sub35_out1 = add186_out1.sub(reducemean67_out1);
        let pow34_out1 = sub35_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean68_out1 = { pow34_out1.mean_dim(2usize) };
        let add187_out1 = reducemean68_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt34_out1 = add187_out1.sqrt();
        let div50_out1 = sub35_out1.div(sqrt34_out1);
        let constant294_out1 = self.constant294.val();
        let mul101_out1 = div50_out1
            .mul((constant294_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant295_out1 = self.constant295.val();
        let add188_out1 = mul101_out1
            .add((constant295_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear101_out1 = self.linear101.forward(add188_out1.clone());
        let div51_out1 = linear101_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf17_out1 = div51_out1.erf();
        let add189_out1 = erf17_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul102_out1 = linear101_out1.mul(add189_out1);
        let mul103_out1 = mul102_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear102_out1 = self.linear102.forward(mul103_out1);
        let add190_out1 = linear102_out1.add(add188_out1);
        let reducemean69_out1 = { add190_out1.clone().mean_dim(2usize) };
        let sub36_out1 = add190_out1.sub(reducemean69_out1);
        let pow35_out1 = sub36_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean70_out1 = { pow35_out1.mean_dim(2usize) };
        let add191_out1 = reducemean70_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt35_out1 = add191_out1.sqrt();
        let div52_out1 = sub36_out1.div(sqrt35_out1);
        let constant300_out1 = self.constant300.val();
        let mul104_out1 = div52_out1
            .mul((constant300_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant301_out1 = self.constant301.val();
        let add192_out1 = mul104_out1
            .add((constant301_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape71_out1: [i64; 3] = {
            let axes = &add192_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear103_out1 = self.linear103.forward(add192_out1.clone());
        let linear104_out1 = self.linear104.forward(add192_out1.clone());
        let linear105_out1 = self.linear105.forward(add192_out1.clone());
        let gather144_out1 = shape71_out1[0] as i64;
        let gather145_out1 = shape71_out1[1] as i64;
        let constant305_out1 = self.constant305.val();
        let add193_out1 = (constant305_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear103_out1);
        let constant306_out1 = self.constant306.val();
        let add194_out1 = (constant306_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear104_out1);
        let constant307_out1 = self.constant307.val();
        let add195_out1 = (constant307_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear105_out1);
        let shape72_out1: [i64; 3] = {
            let axes = &add193_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape73_out1: [i64; 3] = {
            let axes = &add194_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape74_out1: [i64; 3] = {
            let axes = &add195_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze142_out1 = [gather144_out1 as i64];
        let unsqueeze143_out1 = [gather145_out1 as i64];
        let gather146_out1 = shape72_out1[0] as i64;
        let gather147_out1 = shape72_out1[1] as i64;
        let gather148_out1 = shape73_out1[0] as i64;
        let gather149_out1 = shape73_out1[1] as i64;
        let gather150_out1 = shape74_out1[0] as i64;
        let gather151_out1 = shape74_out1[1] as i64;
        let concat71_out1: [i64; 3usize] = [
            &unsqueeze142_out1[..],
            &unsqueeze143_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze144_out1 = [gather146_out1 as i64];
        let unsqueeze145_out1 = [gather147_out1 as i64];
        let unsqueeze146_out1 = [gather148_out1 as i64];
        let unsqueeze147_out1 = [gather149_out1 as i64];
        let unsqueeze148_out1 = [gather150_out1 as i64];
        let unsqueeze149_out1 = [gather151_out1 as i64];
        let concat72_out1: [i64; 4usize] = [
            &unsqueeze144_out1[..],
            &unsqueeze145_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat73_out1: [i64; 4usize] = [
            &unsqueeze146_out1[..],
            &unsqueeze147_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat74_out1: [i64; 4usize] = [
            &unsqueeze148_out1[..],
            &unsqueeze149_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape69_out1 = add193_out1.reshape(concat72_out1);
        let reshape70_out1 = add194_out1.reshape(concat73_out1);
        let reshape71_out1 = add195_out1.reshape(concat74_out1);
        let transpose69_out1 = reshape69_out1.permute([0, 2, 1, 3]);
        let transpose70_out1 = reshape71_out1.permute([0, 2, 1, 3]);
        let transpose71_out1 = reshape70_out1.permute([0, 2, 3, 1]);
        let matmul140_k_corrected = transpose71_out1.permute([0, 1, 3, 2]);
        let (matmul141_out1,) = {
            let q = transpose69_out1;
            let k = matmul140_k_corrected;
            let v = transpose70_out1;
            let matmul141_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul141_out1,)
        };
        let transpose72_out1 = matmul141_out1.permute([0, 2, 1, 3]);
        let reshape72_out1 = transpose72_out1.reshape(concat71_out1);
        let linear106_out1 = self.linear106.forward(reshape72_out1);
        let add197_out1 = linear106_out1.add(add192_out1);
        add197_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule20 {
    constant310: burn::module::Param<Tensor<1>>,
    constant311: burn::module::Param<Tensor<1>>,
    linear107: Linear,
    linear108: Linear,
    constant316: burn::module::Param<Tensor<1>>,
    constant317: burn::module::Param<Tensor<1>>,
    linear109: Linear,
    linear110: Linear,
    linear111: Linear,
    constant321: burn::module::Param<Tensor<1>>,
    constant322: burn::module::Param<Tensor<1>>,
    constant323: burn::module::Param<Tensor<1>>,
    linear112: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule20 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant310: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant311: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear107 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear108 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant316: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant317: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear109 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear110 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear111 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant321: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant322: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant323: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear112 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant310,
            constant311,
            linear107,
            linear108,
            constant316,
            constant317,
            linear109,
            linear110,
            linear111,
            constant321,
            constant322,
            constant323,
            linear112,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add197_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean71_out1 = { add197_out1.clone().mean_dim(2usize) };
        let sub37_out1 = add197_out1.sub(reducemean71_out1);
        let pow36_out1 = sub37_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean72_out1 = { pow36_out1.mean_dim(2usize) };
        let add198_out1 = reducemean72_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt36_out1 = add198_out1.sqrt();
        let div53_out1 = sub37_out1.div(sqrt36_out1);
        let constant310_out1 = self.constant310.val();
        let mul107_out1 = div53_out1
            .mul((constant310_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant311_out1 = self.constant311.val();
        let add199_out1 = mul107_out1
            .add((constant311_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear107_out1 = self.linear107.forward(add199_out1.clone());
        let div54_out1 = linear107_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf18_out1 = div54_out1.erf();
        let add200_out1 = erf18_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul108_out1 = linear107_out1.mul(add200_out1);
        let mul109_out1 = mul108_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear108_out1 = self.linear108.forward(mul109_out1);
        let add201_out1 = linear108_out1.add(add199_out1);
        let reducemean73_out1 = { add201_out1.clone().mean_dim(2usize) };
        let sub38_out1 = add201_out1.sub(reducemean73_out1);
        let pow37_out1 = sub38_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean74_out1 = { pow37_out1.mean_dim(2usize) };
        let add202_out1 = reducemean74_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt37_out1 = add202_out1.sqrt();
        let div55_out1 = sub38_out1.div(sqrt37_out1);
        let constant316_out1 = self.constant316.val();
        let mul110_out1 = div55_out1
            .mul((constant316_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant317_out1 = self.constant317.val();
        let add203_out1 = mul110_out1
            .add((constant317_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape75_out1: [i64; 3] = {
            let axes = &add203_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear109_out1 = self.linear109.forward(add203_out1.clone());
        let linear110_out1 = self.linear110.forward(add203_out1.clone());
        let linear111_out1 = self.linear111.forward(add203_out1.clone());
        let gather152_out1 = shape75_out1[0] as i64;
        let gather153_out1 = shape75_out1[1] as i64;
        let constant321_out1 = self.constant321.val();
        let add204_out1 = (constant321_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear109_out1);
        let constant322_out1 = self.constant322.val();
        let add205_out1 = (constant322_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear110_out1);
        let constant323_out1 = self.constant323.val();
        let add206_out1 = (constant323_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear111_out1);
        let shape76_out1: [i64; 3] = {
            let axes = &add204_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape77_out1: [i64; 3] = {
            let axes = &add205_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape78_out1: [i64; 3] = {
            let axes = &add206_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze150_out1 = [gather152_out1 as i64];
        let unsqueeze151_out1 = [gather153_out1 as i64];
        let gather154_out1 = shape76_out1[0] as i64;
        let gather155_out1 = shape76_out1[1] as i64;
        let gather156_out1 = shape77_out1[0] as i64;
        let gather157_out1 = shape77_out1[1] as i64;
        let gather158_out1 = shape78_out1[0] as i64;
        let gather159_out1 = shape78_out1[1] as i64;
        let concat75_out1: [i64; 3usize] = [
            &unsqueeze150_out1[..],
            &unsqueeze151_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze152_out1 = [gather154_out1 as i64];
        let unsqueeze153_out1 = [gather155_out1 as i64];
        let unsqueeze154_out1 = [gather156_out1 as i64];
        let unsqueeze155_out1 = [gather157_out1 as i64];
        let unsqueeze156_out1 = [gather158_out1 as i64];
        let unsqueeze157_out1 = [gather159_out1 as i64];
        let concat76_out1: [i64; 4usize] = [
            &unsqueeze152_out1[..],
            &unsqueeze153_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat77_out1: [i64; 4usize] = [
            &unsqueeze154_out1[..],
            &unsqueeze155_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat78_out1: [i64; 4usize] = [
            &unsqueeze156_out1[..],
            &unsqueeze157_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape73_out1 = add204_out1.reshape(concat76_out1);
        let reshape74_out1 = add205_out1.reshape(concat77_out1);
        let reshape75_out1 = add206_out1.reshape(concat78_out1);
        let transpose73_out1 = reshape73_out1.permute([0, 2, 1, 3]);
        let transpose74_out1 = reshape75_out1.permute([0, 2, 1, 3]);
        let transpose75_out1 = reshape74_out1.permute([0, 2, 3, 1]);
        let matmul148_k_corrected = transpose75_out1.permute([0, 1, 3, 2]);
        let (matmul149_out1,) = {
            let q = transpose73_out1;
            let k = matmul148_k_corrected;
            let v = transpose74_out1;
            let matmul149_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul149_out1,)
        };
        let transpose76_out1 = matmul149_out1.permute([0, 2, 1, 3]);
        let reshape76_out1 = transpose76_out1.reshape(concat75_out1);
        let linear112_out1 = self.linear112.forward(reshape76_out1);
        let add208_out1 = linear112_out1.add(add203_out1);
        add208_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule21 {
    constant326: burn::module::Param<Tensor<1>>,
    constant327: burn::module::Param<Tensor<1>>,
    linear113: Linear,
    linear114: Linear,
    constant332: burn::module::Param<Tensor<1>>,
    constant333: burn::module::Param<Tensor<1>>,
    linear115: Linear,
    linear116: Linear,
    linear117: Linear,
    constant337: burn::module::Param<Tensor<1>>,
    constant338: burn::module::Param<Tensor<1>>,
    constant339: burn::module::Param<Tensor<1>>,
    linear118: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule21 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant326: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant327: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear113 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear114 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant332: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant333: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear115 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear116 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear117 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant337: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant338: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant339: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear118 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant326,
            constant327,
            linear113,
            linear114,
            constant332,
            constant333,
            linear115,
            linear116,
            linear117,
            constant337,
            constant338,
            constant339,
            linear118,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add208_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean75_out1 = { add208_out1.clone().mean_dim(2usize) };
        let sub39_out1 = add208_out1.sub(reducemean75_out1);
        let pow38_out1 = sub39_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean76_out1 = { pow38_out1.mean_dim(2usize) };
        let add209_out1 = reducemean76_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt38_out1 = add209_out1.sqrt();
        let div56_out1 = sub39_out1.div(sqrt38_out1);
        let constant326_out1 = self.constant326.val();
        let mul113_out1 = div56_out1
            .mul((constant326_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant327_out1 = self.constant327.val();
        let add210_out1 = mul113_out1
            .add((constant327_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear113_out1 = self.linear113.forward(add210_out1.clone());
        let div57_out1 = linear113_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf19_out1 = div57_out1.erf();
        let add211_out1 = erf19_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul114_out1 = linear113_out1.mul(add211_out1);
        let mul115_out1 = mul114_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear114_out1 = self.linear114.forward(mul115_out1);
        let add212_out1 = linear114_out1.add(add210_out1);
        let reducemean77_out1 = { add212_out1.clone().mean_dim(2usize) };
        let sub40_out1 = add212_out1.sub(reducemean77_out1);
        let pow39_out1 = sub40_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean78_out1 = { pow39_out1.mean_dim(2usize) };
        let add213_out1 = reducemean78_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt39_out1 = add213_out1.sqrt();
        let div58_out1 = sub40_out1.div(sqrt39_out1);
        let constant332_out1 = self.constant332.val();
        let mul116_out1 = div58_out1
            .mul((constant332_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant333_out1 = self.constant333.val();
        let add214_out1 = mul116_out1
            .add((constant333_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape79_out1: [i64; 3] = {
            let axes = &add214_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear115_out1 = self.linear115.forward(add214_out1.clone());
        let linear116_out1 = self.linear116.forward(add214_out1.clone());
        let linear117_out1 = self.linear117.forward(add214_out1.clone());
        let gather160_out1 = shape79_out1[0] as i64;
        let gather161_out1 = shape79_out1[1] as i64;
        let constant337_out1 = self.constant337.val();
        let add215_out1 = (constant337_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear115_out1);
        let constant338_out1 = self.constant338.val();
        let add216_out1 = (constant338_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear116_out1);
        let constant339_out1 = self.constant339.val();
        let add217_out1 = (constant339_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear117_out1);
        let shape80_out1: [i64; 3] = {
            let axes = &add215_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape81_out1: [i64; 3] = {
            let axes = &add216_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape82_out1: [i64; 3] = {
            let axes = &add217_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze158_out1 = [gather160_out1 as i64];
        let unsqueeze159_out1 = [gather161_out1 as i64];
        let gather162_out1 = shape80_out1[0] as i64;
        let gather163_out1 = shape80_out1[1] as i64;
        let gather164_out1 = shape81_out1[0] as i64;
        let gather165_out1 = shape81_out1[1] as i64;
        let gather166_out1 = shape82_out1[0] as i64;
        let gather167_out1 = shape82_out1[1] as i64;
        let concat79_out1: [i64; 3usize] = [
            &unsqueeze158_out1[..],
            &unsqueeze159_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze160_out1 = [gather162_out1 as i64];
        let unsqueeze161_out1 = [gather163_out1 as i64];
        let unsqueeze162_out1 = [gather164_out1 as i64];
        let unsqueeze163_out1 = [gather165_out1 as i64];
        let unsqueeze164_out1 = [gather166_out1 as i64];
        let unsqueeze165_out1 = [gather167_out1 as i64];
        let concat80_out1: [i64; 4usize] = [
            &unsqueeze160_out1[..],
            &unsqueeze161_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat81_out1: [i64; 4usize] = [
            &unsqueeze162_out1[..],
            &unsqueeze163_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat82_out1: [i64; 4usize] = [
            &unsqueeze164_out1[..],
            &unsqueeze165_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape77_out1 = add215_out1.reshape(concat80_out1);
        let reshape78_out1 = add216_out1.reshape(concat81_out1);
        let reshape79_out1 = add217_out1.reshape(concat82_out1);
        let transpose77_out1 = reshape77_out1.permute([0, 2, 1, 3]);
        let transpose78_out1 = reshape79_out1.permute([0, 2, 1, 3]);
        let transpose79_out1 = reshape78_out1.permute([0, 2, 3, 1]);
        let matmul156_k_corrected = transpose79_out1.permute([0, 1, 3, 2]);
        let (matmul157_out1,) = {
            let q = transpose77_out1;
            let k = matmul156_k_corrected;
            let v = transpose78_out1;
            let matmul157_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul157_out1,)
        };
        let transpose80_out1 = matmul157_out1.permute([0, 2, 1, 3]);
        let reshape80_out1 = transpose80_out1.reshape(concat79_out1);
        let linear118_out1 = self.linear118.forward(reshape80_out1);
        let add219_out1 = linear118_out1.add(add214_out1);
        add219_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule22 {
    constant342: burn::module::Param<Tensor<1>>,
    constant343: burn::module::Param<Tensor<1>>,
    linear119: Linear,
    linear120: Linear,
    constant348: burn::module::Param<Tensor<1>>,
    constant349: burn::module::Param<Tensor<1>>,
    linear121: Linear,
    linear122: Linear,
    linear123: Linear,
    constant353: burn::module::Param<Tensor<1>>,
    constant354: burn::module::Param<Tensor<1>>,
    constant355: burn::module::Param<Tensor<1>>,
    linear124: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule22 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant342: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant343: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear119 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear120 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant348: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant349: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear121 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear122 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear123 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant353: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant354: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant355: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear124 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant342,
            constant343,
            linear119,
            linear120,
            constant348,
            constant349,
            linear121,
            linear122,
            linear123,
            constant353,
            constant354,
            constant355,
            linear124,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add219_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean79_out1 = { add219_out1.clone().mean_dim(2usize) };
        let sub41_out1 = add219_out1.sub(reducemean79_out1);
        let pow40_out1 = sub41_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean80_out1 = { pow40_out1.mean_dim(2usize) };
        let add220_out1 = reducemean80_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt40_out1 = add220_out1.sqrt();
        let div59_out1 = sub41_out1.div(sqrt40_out1);
        let constant342_out1 = self.constant342.val();
        let mul119_out1 = div59_out1
            .mul((constant342_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant343_out1 = self.constant343.val();
        let add221_out1 = mul119_out1
            .add((constant343_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear119_out1 = self.linear119.forward(add221_out1.clone());
        let div60_out1 = linear119_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf20_out1 = div60_out1.erf();
        let add222_out1 = erf20_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul120_out1 = linear119_out1.mul(add222_out1);
        let mul121_out1 = mul120_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear120_out1 = self.linear120.forward(mul121_out1);
        let add223_out1 = linear120_out1.add(add221_out1);
        let reducemean81_out1 = { add223_out1.clone().mean_dim(2usize) };
        let sub42_out1 = add223_out1.sub(reducemean81_out1);
        let pow41_out1 = sub42_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean82_out1 = { pow41_out1.mean_dim(2usize) };
        let add224_out1 = reducemean82_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt41_out1 = add224_out1.sqrt();
        let div61_out1 = sub42_out1.div(sqrt41_out1);
        let constant348_out1 = self.constant348.val();
        let mul122_out1 = div61_out1
            .mul((constant348_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant349_out1 = self.constant349.val();
        let add225_out1 = mul122_out1
            .add((constant349_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape83_out1: [i64; 3] = {
            let axes = &add225_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear121_out1 = self.linear121.forward(add225_out1.clone());
        let linear122_out1 = self.linear122.forward(add225_out1.clone());
        let linear123_out1 = self.linear123.forward(add225_out1.clone());
        let gather168_out1 = shape83_out1[0] as i64;
        let gather169_out1 = shape83_out1[1] as i64;
        let constant353_out1 = self.constant353.val();
        let add226_out1 = (constant353_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear121_out1);
        let constant354_out1 = self.constant354.val();
        let add227_out1 = (constant354_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear122_out1);
        let constant355_out1 = self.constant355.val();
        let add228_out1 = (constant355_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear123_out1);
        let shape84_out1: [i64; 3] = {
            let axes = &add226_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape85_out1: [i64; 3] = {
            let axes = &add227_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape86_out1: [i64; 3] = {
            let axes = &add228_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze166_out1 = [gather168_out1 as i64];
        let unsqueeze167_out1 = [gather169_out1 as i64];
        let gather170_out1 = shape84_out1[0] as i64;
        let gather171_out1 = shape84_out1[1] as i64;
        let gather172_out1 = shape85_out1[0] as i64;
        let gather173_out1 = shape85_out1[1] as i64;
        let gather174_out1 = shape86_out1[0] as i64;
        let gather175_out1 = shape86_out1[1] as i64;
        let concat83_out1: [i64; 3usize] = [
            &unsqueeze166_out1[..],
            &unsqueeze167_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze168_out1 = [gather170_out1 as i64];
        let unsqueeze169_out1 = [gather171_out1 as i64];
        let unsqueeze170_out1 = [gather172_out1 as i64];
        let unsqueeze171_out1 = [gather173_out1 as i64];
        let unsqueeze172_out1 = [gather174_out1 as i64];
        let unsqueeze173_out1 = [gather175_out1 as i64];
        let concat84_out1: [i64; 4usize] = [
            &unsqueeze168_out1[..],
            &unsqueeze169_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat85_out1: [i64; 4usize] = [
            &unsqueeze170_out1[..],
            &unsqueeze171_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat86_out1: [i64; 4usize] = [
            &unsqueeze172_out1[..],
            &unsqueeze173_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape81_out1 = add226_out1.reshape(concat84_out1);
        let reshape82_out1 = add227_out1.reshape(concat85_out1);
        let reshape83_out1 = add228_out1.reshape(concat86_out1);
        let transpose81_out1 = reshape81_out1.permute([0, 2, 1, 3]);
        let transpose82_out1 = reshape83_out1.permute([0, 2, 1, 3]);
        let transpose83_out1 = reshape82_out1.permute([0, 2, 3, 1]);
        let matmul164_k_corrected = transpose83_out1.permute([0, 1, 3, 2]);
        let (matmul165_out1,) = {
            let q = transpose81_out1;
            let k = matmul164_k_corrected;
            let v = transpose82_out1;
            let matmul165_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul165_out1,)
        };
        let transpose84_out1 = matmul165_out1.permute([0, 2, 1, 3]);
        let reshape84_out1 = transpose84_out1.reshape(concat83_out1);
        let linear124_out1 = self.linear124.forward(reshape84_out1);
        let add230_out1 = linear124_out1.add(add225_out1);
        add230_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule23 {
    constant358: burn::module::Param<Tensor<1>>,
    constant359: burn::module::Param<Tensor<1>>,
    linear125: Linear,
    linear126: Linear,
    constant364: burn::module::Param<Tensor<1>>,
    constant365: burn::module::Param<Tensor<1>>,
    linear127: Linear,
    linear128: Linear,
    linear129: Linear,
    constant369: burn::module::Param<Tensor<1>>,
    constant370: burn::module::Param<Tensor<1>>,
    constant371: burn::module::Param<Tensor<1>>,
    linear130: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule23 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant358: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant359: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear125 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear126 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant364: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant365: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear127 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear128 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear129 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant369: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant370: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant371: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear130 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant358,
            constant359,
            linear125,
            linear126,
            constant364,
            constant365,
            linear127,
            linear128,
            linear129,
            constant369,
            constant370,
            constant371,
            linear130,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add230_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean83_out1 = { add230_out1.clone().mean_dim(2usize) };
        let sub43_out1 = add230_out1.sub(reducemean83_out1);
        let pow42_out1 = sub43_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean84_out1 = { pow42_out1.mean_dim(2usize) };
        let add231_out1 = reducemean84_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt42_out1 = add231_out1.sqrt();
        let div62_out1 = sub43_out1.div(sqrt42_out1);
        let constant358_out1 = self.constant358.val();
        let mul125_out1 = div62_out1
            .mul((constant358_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant359_out1 = self.constant359.val();
        let add232_out1 = mul125_out1
            .add((constant359_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear125_out1 = self.linear125.forward(add232_out1.clone());
        let div63_out1 = linear125_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf21_out1 = div63_out1.erf();
        let add233_out1 = erf21_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul126_out1 = linear125_out1.mul(add233_out1);
        let mul127_out1 = mul126_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear126_out1 = self.linear126.forward(mul127_out1);
        let add234_out1 = linear126_out1.add(add232_out1);
        let reducemean85_out1 = { add234_out1.clone().mean_dim(2usize) };
        let sub44_out1 = add234_out1.sub(reducemean85_out1);
        let pow43_out1 = sub44_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean86_out1 = { pow43_out1.mean_dim(2usize) };
        let add235_out1 = reducemean86_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt43_out1 = add235_out1.sqrt();
        let div64_out1 = sub44_out1.div(sqrt43_out1);
        let constant364_out1 = self.constant364.val();
        let mul128_out1 = div64_out1
            .mul((constant364_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant365_out1 = self.constant365.val();
        let add236_out1 = mul128_out1
            .add((constant365_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape87_out1: [i64; 3] = {
            let axes = &add236_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear127_out1 = self.linear127.forward(add236_out1.clone());
        let linear128_out1 = self.linear128.forward(add236_out1.clone());
        let linear129_out1 = self.linear129.forward(add236_out1.clone());
        let gather176_out1 = shape87_out1[0] as i64;
        let gather177_out1 = shape87_out1[1] as i64;
        let constant369_out1 = self.constant369.val();
        let add237_out1 = (constant369_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear127_out1);
        let constant370_out1 = self.constant370.val();
        let add238_out1 = (constant370_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear128_out1);
        let constant371_out1 = self.constant371.val();
        let add239_out1 = (constant371_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear129_out1);
        let shape88_out1: [i64; 3] = {
            let axes = &add237_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape89_out1: [i64; 3] = {
            let axes = &add238_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape90_out1: [i64; 3] = {
            let axes = &add239_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze174_out1 = [gather176_out1 as i64];
        let unsqueeze175_out1 = [gather177_out1 as i64];
        let gather178_out1 = shape88_out1[0] as i64;
        let gather179_out1 = shape88_out1[1] as i64;
        let gather180_out1 = shape89_out1[0] as i64;
        let gather181_out1 = shape89_out1[1] as i64;
        let gather182_out1 = shape90_out1[0] as i64;
        let gather183_out1 = shape90_out1[1] as i64;
        let concat87_out1: [i64; 3usize] = [
            &unsqueeze174_out1[..],
            &unsqueeze175_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze176_out1 = [gather178_out1 as i64];
        let unsqueeze177_out1 = [gather179_out1 as i64];
        let unsqueeze178_out1 = [gather180_out1 as i64];
        let unsqueeze179_out1 = [gather181_out1 as i64];
        let unsqueeze180_out1 = [gather182_out1 as i64];
        let unsqueeze181_out1 = [gather183_out1 as i64];
        let concat88_out1: [i64; 4usize] = [
            &unsqueeze176_out1[..],
            &unsqueeze177_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat89_out1: [i64; 4usize] = [
            &unsqueeze178_out1[..],
            &unsqueeze179_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat90_out1: [i64; 4usize] = [
            &unsqueeze180_out1[..],
            &unsqueeze181_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape85_out1 = add237_out1.reshape(concat88_out1);
        let reshape86_out1 = add238_out1.reshape(concat89_out1);
        let reshape87_out1 = add239_out1.reshape(concat90_out1);
        let transpose85_out1 = reshape85_out1.permute([0, 2, 1, 3]);
        let transpose86_out1 = reshape87_out1.permute([0, 2, 1, 3]);
        let transpose87_out1 = reshape86_out1.permute([0, 2, 3, 1]);
        let matmul172_k_corrected = transpose87_out1.permute([0, 1, 3, 2]);
        let (matmul173_out1,) = {
            let q = transpose85_out1;
            let k = matmul172_k_corrected;
            let v = transpose86_out1;
            let matmul173_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul173_out1,)
        };
        let transpose88_out1 = matmul173_out1.permute([0, 2, 1, 3]);
        let reshape88_out1 = transpose88_out1.reshape(concat87_out1);
        let linear130_out1 = self.linear130.forward(reshape88_out1);
        let add241_out1 = linear130_out1.add(add236_out1);
        add241_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule24 {
    constant374: burn::module::Param<Tensor<1>>,
    constant375: burn::module::Param<Tensor<1>>,
    linear131: Linear,
    linear132: Linear,
    constant380: burn::module::Param<Tensor<1>>,
    constant381: burn::module::Param<Tensor<1>>,
    linear133: Linear,
    linear134: Linear,
    linear135: Linear,
    constant385: burn::module::Param<Tensor<1>>,
    constant386: burn::module::Param<Tensor<1>>,
    constant387: burn::module::Param<Tensor<1>>,
    linear136: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule24 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant374: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant375: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear131 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear132 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant380: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant381: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear133 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear134 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear135 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant385: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant386: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant387: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear136 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        Self {
            constant374,
            constant375,
            linear131,
            linear132,
            constant380,
            constant381,
            linear133,
            linear134,
            linear135,
            constant385,
            constant386,
            constant387,
            linear136,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add241_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<3> {
        let reducemean87_out1 = { add241_out1.clone().mean_dim(2usize) };
        let sub45_out1 = add241_out1.sub(reducemean87_out1);
        let pow44_out1 = sub45_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean88_out1 = { pow44_out1.mean_dim(2usize) };
        let add242_out1 = reducemean88_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt44_out1 = add242_out1.sqrt();
        let div65_out1 = sub45_out1.div(sqrt44_out1);
        let constant374_out1 = self.constant374.val();
        let mul131_out1 = div65_out1
            .mul((constant374_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant375_out1 = self.constant375.val();
        let add243_out1 = mul131_out1
            .add((constant375_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear131_out1 = self.linear131.forward(add243_out1.clone());
        let div66_out1 = linear131_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf22_out1 = div66_out1.erf();
        let add244_out1 = erf22_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul132_out1 = linear131_out1.mul(add244_out1);
        let mul133_out1 = mul132_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear132_out1 = self.linear132.forward(mul133_out1);
        let add245_out1 = linear132_out1.add(add243_out1);
        let reducemean89_out1 = { add245_out1.clone().mean_dim(2usize) };
        let sub46_out1 = add245_out1.sub(reducemean89_out1);
        let pow45_out1 = sub46_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean90_out1 = { pow45_out1.mean_dim(2usize) };
        let add246_out1 = reducemean90_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt45_out1 = add246_out1.sqrt();
        let div67_out1 = sub46_out1.div(sqrt45_out1);
        let constant380_out1 = self.constant380.val();
        let mul134_out1 = div67_out1
            .mul((constant380_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant381_out1 = self.constant381.val();
        let add247_out1 = mul134_out1
            .add((constant381_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape91_out1: [i64; 3] = {
            let axes = &add247_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear133_out1 = self.linear133.forward(add247_out1.clone());
        let linear134_out1 = self.linear134.forward(add247_out1.clone());
        let linear135_out1 = self.linear135.forward(add247_out1.clone());
        let gather184_out1 = shape91_out1[0] as i64;
        let gather185_out1 = shape91_out1[1] as i64;
        let constant385_out1 = self.constant385.val();
        let add248_out1 = (constant385_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear133_out1);
        let constant386_out1 = self.constant386.val();
        let add249_out1 = (constant386_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear134_out1);
        let constant387_out1 = self.constant387.val();
        let add250_out1 = (constant387_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear135_out1);
        let shape92_out1: [i64; 3] = {
            let axes = &add248_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape93_out1: [i64; 3] = {
            let axes = &add249_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape94_out1: [i64; 3] = {
            let axes = &add250_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze182_out1 = [gather184_out1 as i64];
        let unsqueeze183_out1 = [gather185_out1 as i64];
        let gather186_out1 = shape92_out1[0] as i64;
        let gather187_out1 = shape92_out1[1] as i64;
        let gather188_out1 = shape93_out1[0] as i64;
        let gather189_out1 = shape93_out1[1] as i64;
        let gather190_out1 = shape94_out1[0] as i64;
        let gather191_out1 = shape94_out1[1] as i64;
        let concat91_out1: [i64; 3usize] = [
            &unsqueeze182_out1[..],
            &unsqueeze183_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze184_out1 = [gather186_out1 as i64];
        let unsqueeze185_out1 = [gather187_out1 as i64];
        let unsqueeze186_out1 = [gather188_out1 as i64];
        let unsqueeze187_out1 = [gather189_out1 as i64];
        let unsqueeze188_out1 = [gather190_out1 as i64];
        let unsqueeze189_out1 = [gather191_out1 as i64];
        let concat92_out1: [i64; 4usize] = [
            &unsqueeze184_out1[..],
            &unsqueeze185_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat93_out1: [i64; 4usize] = [
            &unsqueeze186_out1[..],
            &unsqueeze187_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat94_out1: [i64; 4usize] = [
            &unsqueeze188_out1[..],
            &unsqueeze189_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape89_out1 = add248_out1.reshape(concat92_out1);
        let reshape90_out1 = add249_out1.reshape(concat93_out1);
        let reshape91_out1 = add250_out1.reshape(concat94_out1);
        let transpose89_out1 = reshape89_out1.permute([0, 2, 1, 3]);
        let transpose90_out1 = reshape91_out1.permute([0, 2, 1, 3]);
        let transpose91_out1 = reshape90_out1.permute([0, 2, 3, 1]);
        let matmul180_k_corrected = transpose91_out1.permute([0, 1, 3, 2]);
        let (matmul181_out1,) = {
            let q = transpose89_out1;
            let k = matmul180_k_corrected;
            let v = transpose90_out1;
            let matmul181_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul181_out1,)
        };
        let transpose92_out1 = matmul181_out1.permute([0, 2, 1, 3]);
        let reshape92_out1 = transpose92_out1.reshape(concat91_out1);
        let linear136_out1 = self.linear136.forward(reshape92_out1);
        let add252_out1 = linear136_out1.add(add247_out1);
        add252_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule25 {
    constant390: burn::module::Param<Tensor<1>>,
    constant391: burn::module::Param<Tensor<1>>,
    linear137: Linear,
    linear138: Linear,
    constant396: burn::module::Param<Tensor<1>>,
    constant397: burn::module::Param<Tensor<1>>,
    linear139: Linear,
    linear140: Linear,
    linear141: Linear,
    constant401: burn::module::Param<Tensor<1>>,
    constant402: burn::module::Param<Tensor<1>>,
    constant403: burn::module::Param<Tensor<1>>,
    linear142: Linear,
    constant406: burn::module::Param<Tensor<1>>,
    constant407: burn::module::Param<Tensor<1>>,
    linear143: Linear,
    linear144: Linear,
    constant412: burn::module::Param<Tensor<1>>,
    constant413: burn::module::Param<Tensor<1>>,
    linear145: Linear,
    linear146: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule25 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant390: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant391: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear137 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear138 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant396: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant397: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear139 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear140 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let linear141 = LinearConfig::new(1024, 1024).with_bias(false).init(device);
        let constant401: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant402: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant403: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear142 = LinearConfig::new(1024, 1024).with_bias(true).init(device);
        let constant406: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant407: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear143 = LinearConfig::new(1024, 4096).with_bias(true).init(device);
        let linear144 = LinearConfig::new(4096, 1024).with_bias(true).init(device);
        let constant412: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let constant413: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([1024], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1024].into(),
        );
        let linear145 = LinearConfig::new(1024, 1024)
            .with_bias(true)
            .with_layout(LinearLayout::Col)
            .init(device);
        let linear146 = LinearConfig::new(1024, 1)
            .with_bias(true)
            .with_layout(LinearLayout::Col)
            .init(device);
        Self {
            constant390,
            constant391,
            linear137,
            linear138,
            constant396,
            constant397,
            linear139,
            linear140,
            linear141,
            constant401,
            constant402,
            constant403,
            linear142,
            constant406,
            constant407,
            linear143,
            linear144,
            constant412,
            constant413,
            linear145,
            linear146,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add252_out1: Tensor<3>,
        constant19_out1: Tensor<1>,
        constant20_out1: Tensor<1>,
        constant39_out1: Tensor<1>,
        constant40_out1: Tensor<1>,
        constant41_out1: Tensor<1>,
        constant29_out1: [i64; 1],
        constant30_out1: [i64; 1],
        constant31_out1: [i64; 1],
        where3_out1: Tensor<4>,
    ) -> Tensor<2> {
        let reducemean91_out1 = { add252_out1.clone().mean_dim(2usize) };
        let sub47_out1 = add252_out1.sub(reducemean91_out1);
        let pow46_out1 = sub47_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean92_out1 = { pow46_out1.mean_dim(2usize) };
        let add253_out1 = reducemean92_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt46_out1 = add253_out1.sqrt();
        let div68_out1 = sub47_out1.div(sqrt46_out1);
        let constant390_out1 = self.constant390.val();
        let mul137_out1 = div68_out1
            .mul((constant390_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant391_out1 = self.constant391.val();
        let add254_out1 = mul137_out1
            .add((constant391_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear137_out1 = self.linear137.forward(add254_out1.clone());
        let div69_out1 = linear137_out1
            .clone()
            .div((constant39_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let erf23_out1 = div69_out1.erf();
        let add255_out1 = erf23_out1
            .add((constant40_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let mul138_out1 = linear137_out1.mul(add255_out1);
        let mul139_out1 = mul138_out1
            .mul((constant41_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let linear138_out1 = self.linear138.forward(mul139_out1);
        let add256_out1 = linear138_out1.add(add254_out1);
        let reducemean93_out1 = { add256_out1.clone().mean_dim(2usize) };
        let sub48_out1 = add256_out1.sub(reducemean93_out1);
        let pow47_out1 = sub48_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean94_out1 = { pow47_out1.mean_dim(2usize) };
        let add257_out1 = reducemean94_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt47_out1 = add257_out1.sqrt();
        let div70_out1 = sub48_out1.div(sqrt47_out1);
        let constant396_out1 = self.constant396.val();
        let mul140_out1 = div70_out1
            .mul((constant396_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant397_out1 = self.constant397.val();
        let add258_out1 = mul140_out1
            .add((constant397_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape95_out1: [i64; 3] = {
            let axes = &add258_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear139_out1 = self.linear139.forward(add258_out1.clone());
        let linear140_out1 = self.linear140.forward(add258_out1.clone());
        let linear141_out1 = self.linear141.forward(add258_out1.clone());
        let gather192_out1 = shape95_out1[0] as i64;
        let gather193_out1 = shape95_out1[1] as i64;
        let constant401_out1 = self.constant401.val();
        let add259_out1 = (constant401_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear139_out1);
        let constant402_out1 = self.constant402.val();
        let add260_out1 = (constant402_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear140_out1);
        let constant403_out1 = self.constant403.val();
        let add261_out1 = (constant403_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear141_out1);
        let shape96_out1: [i64; 3] = {
            let axes = &add259_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape97_out1: [i64; 3] = {
            let axes = &add260_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape98_out1: [i64; 3] = {
            let axes = &add261_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze190_out1 = [gather192_out1 as i64];
        let unsqueeze191_out1 = [gather193_out1 as i64];
        let gather194_out1 = shape96_out1[0] as i64;
        let gather195_out1 = shape96_out1[1] as i64;
        let gather196_out1 = shape97_out1[0] as i64;
        let gather197_out1 = shape97_out1[1] as i64;
        let gather198_out1 = shape98_out1[0] as i64;
        let gather199_out1 = shape98_out1[1] as i64;
        let concat95_out1: [i64; 3usize] = [
            &unsqueeze190_out1[..],
            &unsqueeze191_out1[..],
            &constant29_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let unsqueeze192_out1 = [gather194_out1 as i64];
        let unsqueeze193_out1 = [gather195_out1 as i64];
        let unsqueeze194_out1 = [gather196_out1 as i64];
        let unsqueeze195_out1 = [gather197_out1 as i64];
        let unsqueeze196_out1 = [gather198_out1 as i64];
        let unsqueeze197_out1 = [gather199_out1 as i64];
        let concat96_out1: [i64; 4usize] = [
            &unsqueeze192_out1[..],
            &unsqueeze193_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat97_out1: [i64; 4usize] = [
            &unsqueeze194_out1[..],
            &unsqueeze195_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let concat98_out1: [i64; 4usize] = [
            &unsqueeze196_out1[..],
            &unsqueeze197_out1[..],
            &constant30_out1[..],
            &constant31_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape93_out1 = add259_out1.reshape(concat96_out1);
        let reshape94_out1 = add260_out1.reshape(concat97_out1);
        let reshape95_out1 = add261_out1.reshape(concat98_out1);
        let transpose93_out1 = reshape93_out1.permute([0, 2, 1, 3]);
        let transpose94_out1 = reshape95_out1.permute([0, 2, 1, 3]);
        let transpose95_out1 = reshape94_out1.permute([0, 2, 3, 1]);
        let matmul188_k_corrected = transpose95_out1.permute([0, 1, 3, 2]);
        let (matmul189_out1,) = {
            let q = transpose93_out1;
            let k = matmul188_k_corrected;
            let v = transpose94_out1;
            let matmul189_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(where3_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul189_out1,)
        };
        let transpose96_out1 = matmul189_out1.permute([0, 2, 1, 3]);
        let reshape96_out1 = transpose96_out1.reshape(concat95_out1);
        let linear142_out1 = self.linear142.forward(reshape96_out1);
        let add263_out1 = linear142_out1.add(add258_out1);
        let reducemean95_out1 = { add263_out1.clone().mean_dim(2usize) };
        let sub49_out1 = add263_out1.sub(reducemean95_out1);
        let pow48_out1 = sub49_out1
            .clone()
            .powf((constant19_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean96_out1 = { pow48_out1.mean_dim(2usize) };
        let add264_out1 = reducemean96_out1
            .add((constant20_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt48_out1 = add264_out1.sqrt();
        let div71_out1 = sub49_out1.div(sqrt48_out1);
        let constant406_out1 = self.constant406.val();
        let mul143_out1 = div71_out1
            .mul((constant406_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant407_out1 = self.constant407.val();
        let add265_out1 = mul143_out1
            .add((constant407_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear143_out1 = self.linear143.forward(add265_out1.clone());
        let div72_out1 = linear143_out1
            .clone()
            .div((constant39_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf24_out1 = div72_out1.erf();
        let add266_out1 = erf24_out1
            .add((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul144_out1 = linear143_out1.mul(add266_out1);
        let mul145_out1 = mul144_out1
            .mul((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear144_out1 = self.linear144.forward(mul145_out1);
        let add267_out1 = linear144_out1.add(add265_out1);
        let reducemean97_out1 = { add267_out1.clone().mean_dim(2usize) };
        let sub50_out1 = add267_out1.sub(reducemean97_out1);
        let pow49_out1 = sub50_out1
            .clone()
            .powf((constant19_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean98_out1 = { pow49_out1.mean_dim(2usize) };
        let add268_out1 = reducemean98_out1
            .add((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt49_out1 = add268_out1.sqrt();
        let div73_out1 = sub50_out1.div(sqrt49_out1);
        let constant412_out1 = self.constant412.val();
        let mul146_out1 = div73_out1
            .mul((constant412_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant413_out1 = self.constant413.val();
        let add269_out1 = mul146_out1
            .add((constant413_out1).unsqueeze_dims(&[0isize, 1isize]));
        let gather200_out1 = {
            let sliced = add269_out1.slice(s![.., 0, ..]);
            sliced.squeeze_dim::<2usize>(1)
        };
        let linear145_out1 = self.linear145.forward(gather200_out1);
        let tanh1_out1 = linear145_out1.tanh();
        let linear146_out1 = self.linear146.forward(tanh1_out1);
        linear146_out1
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
    ) -> Tensor<2> {
        let (
            add5_out1,
            shape3_out1,
            linear1_out1,
            where3_out1,
            constant19_out1,
            constant20_out1,
        ) = self.submodule1.forward(input_ids, attention_mask);
        let (
            mul6_out1,
            add12_out1,
            constant29_out1,
            constant30_out1,
            constant31_out1,
            constant39_out1,
            constant40_out1,
        ) = self
            .submodule2
            .forward(
                add5_out1,
                shape3_out1,
                linear1_out1,
                where3_out1.clone(),
                constant19_out1.clone(),
                constant20_out1.clone(),
            );
        let (div5_out1, constant41_out1) = self
            .submodule3
            .forward(
                mul6_out1,
                add12_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add32_out1 = self
            .submodule4
            .forward(
                div5_out1,
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add43_out1 = self
            .submodule5
            .forward(
                add32_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add54_out1 = self
            .submodule6
            .forward(
                add43_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add65_out1 = self
            .submodule7
            .forward(
                add54_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add76_out1 = self
            .submodule8
            .forward(
                add65_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add87_out1 = self
            .submodule9
            .forward(
                add76_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add98_out1 = self
            .submodule10
            .forward(
                add87_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add109_out1 = self
            .submodule11
            .forward(
                add98_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add120_out1 = self
            .submodule12
            .forward(
                add109_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add131_out1 = self
            .submodule13
            .forward(
                add120_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add142_out1 = self
            .submodule14
            .forward(
                add131_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add153_out1 = self
            .submodule15
            .forward(
                add142_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add164_out1 = self
            .submodule16
            .forward(
                add153_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add175_out1 = self
            .submodule17
            .forward(
                add164_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add186_out1 = self
            .submodule18
            .forward(
                add175_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add197_out1 = self
            .submodule19
            .forward(
                add186_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add208_out1 = self
            .submodule20
            .forward(
                add197_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add219_out1 = self
            .submodule21
            .forward(
                add208_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add230_out1 = self
            .submodule22
            .forward(
                add219_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add241_out1 = self
            .submodule23
            .forward(
                add230_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let add252_out1 = self
            .submodule24
            .forward(
                add241_out1,
                constant19_out1.clone(),
                constant20_out1.clone(),
                constant39_out1.clone(),
                constant40_out1.clone(),
                constant41_out1.clone(),
                constant29_out1.clone(),
                constant30_out1.clone(),
                constant31_out1.clone(),
                where3_out1.clone(),
            );
        let linear146_out1 = self
            .submodule25
            .forward(
                add252_out1,
                constant19_out1,
                constant20_out1,
                constant39_out1,
                constant40_out1,
                constant41_out1,
                constant29_out1,
                constant30_out1,
                constant31_out1,
                where3_out1,
            );
        linear146_out1
    }
}
