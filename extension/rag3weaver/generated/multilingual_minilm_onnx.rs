// Generated from ONNX "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2 onnx/model.onnx" by burn-onnx
use burn::prelude::*;
use burn::nn::Linear;
use burn::nn::LinearConfig;
use burn::tensor::Bytes;
use burn_store::BurnpackStore;
use burn_store::ModuleSnapshot;


#[derive(Module, Debug)]
pub struct Submodule1 {
    constant199: burn::module::Param<Tensor<2, Int>>,
    constant1: burn::module::Param<Tensor<2>>,
    constant3: burn::module::Param<Tensor<2>>,
    constant2: burn::module::Param<Tensor<2>>,
    constant204: burn::module::Param<Tensor<1>>,
    constant205: burn::module::Param<Tensor<1>>,
    constant4: burn::module::Param<Tensor<1>>,
    constant5: burn::module::Param<Tensor<1>>,
    constant215: burn::module::Param<Tensor<1, Int>>,
    constant216: burn::module::Param<Tensor<1>>,
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
        let constant199: burn::module::Param<Tensor<2, Int>> = burn::module::Param::uninitialized(
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
            >::zeros([250037, 384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [250037, 384].into(),
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
        let constant204: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant205: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant215: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
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
        let constant216: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            constant199,
            constant1,
            constant3,
            constant2,
            constant204,
            constant205,
            constant4,
            constant5,
            constant215,
            constant216,
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
        let constant199_out1 = self.constant199.val();
        let slice1_out1 = constant199_out1.slice(s![.., 0..unsqueeze1_out1[0]]);
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
        let constant204_out1 = self.constant204.val();
        let pow1_out1 = sub1_out1
            .clone()
            .powf((constant204_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean2_out1 = { pow1_out1.mean_dim(2usize) };
        let constant205_out1 = self.constant205.val();
        let add3_out1 = reducemean2_out1
            .add((constant205_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant211_out1: [i64; 1] = [1i64];
        let concat1_out1: [i64; 4usize] = [
            &unsqueeze4_out1[..],
            &constant211_out1[..],
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
        let constant215_out1 = self.constant215.val();
        let mul2_out1 = constantofshape1_out1.clone().mul(constant215_out1);
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
        let constant216_out1 = self.constant216.val();
        let sub2_out1 = (constant216_out1)
            .unsqueeze_dims(&[0isize, 1isize, 2isize])
            .sub(cast1_out1);
        let cast2_out1 = sub2_out1.clone().bool();
        let constant217_out1 = -340282350000000000000000000000000000000f32;
        let where2_out1 = sub2_out1.mask_fill(cast2_out1, constant217_out1);
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
        let constant225_out1: [i64; 1] = [32i64];
        let constant224_out1: [i64; 1] = [12i64];
        let concat2_out1: [i64; 4usize] = [
            &unsqueeze7_out1[..],
            &unsqueeze8_out1[..],
            &constant224_out1[..],
            &constant225_out1[..],
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
        let constant231_out1: [i64; 1] = [32i64];
        let constant230_out1: [i64; 1] = [12i64];
        let concat3_out1: [i64; 4usize] = [
            &unsqueeze9_out1[..],
            &unsqueeze10_out1[..],
            &constant230_out1[..],
            &constant231_out1[..],
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
        let constant237_out1: [i64; 1] = [32i64];
        let constant236_out1: [i64; 1] = [12i64];
        let concat4_out1: [i64; 4usize] = [
            &unsqueeze11_out1[..],
            &unsqueeze12_out1[..],
            &constant236_out1[..],
            &constant237_out1[..],
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
        let constant243_out1: [i64; 1] = [384i64];
        let concat5_out1: [i64; 3usize] = [
            &unsqueeze13_out1[..],
            &unsqueeze14_out1[..],
            &constant243_out1[..],
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
    constant244: burn::module::Param<Tensor<1>>,
    constant245: burn::module::Param<Tensor<1>>,
    constant10: burn::module::Param<Tensor<1>>,
    constant11: burn::module::Param<Tensor<1>>,
    linear5: Linear,
    constant246: burn::module::Param<Tensor<1>>,
    constant247: burn::module::Param<Tensor<1>>,
    constant248: burn::module::Param<Tensor<1>>,
    linear6: Linear,
    constant249: burn::module::Param<Tensor<1>>,
    constant250: burn::module::Param<Tensor<1>>,
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
        let constant244: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant245: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant246: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant247: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant248: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            constant244,
            constant245,
            constant10,
            constant11,
            linear5,
            constant246,
            constant247,
            constant248,
            linear6,
            constant249,
            constant250,
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
        let constant244_out1 = self.constant244.val();
        let pow2_out1 = sub3_out1
            .clone()
            .powf((constant244_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean4_out1 = { pow2_out1.mean_dim(2usize) };
        let constant245_out1 = self.constant245.val();
        let add7_out1 = reducemean4_out1
            .add((constant245_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt5_out1 = add7_out1.sqrt();
        let div3_out1 = sub3_out1.div(sqrt5_out1);
        let constant10_out1 = self.constant10.val();
        let mul5_out1 = div3_out1
            .mul((constant10_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant11_out1 = self.constant11.val();
        let add8_out1 = mul5_out1
            .add((constant11_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear5_out1 = self.linear5.forward(add8_out1.clone());
        let constant246_out1 = self.constant246.val();
        let div4_out1 = linear5_out1
            .clone()
            .div((constant246_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf1_out1 = div4_out1.erf();
        let constant247_out1 = self.constant247.val();
        let add9_out1 = erf1_out1
            .add((constant247_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul6_out1 = linear5_out1.mul(add9_out1);
        let constant248_out1 = self.constant248.val();
        let mul7_out1 = mul6_out1
            .mul((constant248_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear6_out1 = self.linear6.forward(mul7_out1);
        let add10_out1 = linear6_out1.add(add8_out1);
        let reducemean5_out1 = { add10_out1.clone().mean_dim(2usize) };
        let sub4_out1 = add10_out1.sub(reducemean5_out1);
        let constant249_out1 = self.constant249.val();
        let pow3_out1 = sub4_out1
            .clone()
            .powf((constant249_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean6_out1 = { pow3_out1.mean_dim(2usize) };
        let constant250_out1 = self.constant250.val();
        let add11_out1 = reducemean6_out1
            .add((constant250_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant258_out1: [i64; 1] = [32i64];
        let constant257_out1: [i64; 1] = [12i64];
        let concat6_out1: [i64; 4usize] = [
            &unsqueeze15_out1[..],
            &unsqueeze16_out1[..],
            &constant257_out1[..],
            &constant258_out1[..],
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
        let constant264_out1: [i64; 1] = [32i64];
        let constant263_out1: [i64; 1] = [12i64];
        let concat7_out1: [i64; 4usize] = [
            &unsqueeze17_out1[..],
            &unsqueeze18_out1[..],
            &constant263_out1[..],
            &constant264_out1[..],
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
        let constant270_out1: [i64; 1] = [32i64];
        let constant269_out1: [i64; 1] = [12i64];
        let concat8_out1: [i64; 4usize] = [
            &unsqueeze19_out1[..],
            &unsqueeze20_out1[..],
            &constant269_out1[..],
            &constant270_out1[..],
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
        let constant276_out1: [i64; 1] = [384i64];
        let concat9_out1: [i64; 3usize] = [
            &unsqueeze21_out1[..],
            &unsqueeze22_out1[..],
            &constant276_out1[..],
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
    constant277: burn::module::Param<Tensor<1>>,
    constant278: burn::module::Param<Tensor<1>>,
    constant20: burn::module::Param<Tensor<1>>,
    constant21: burn::module::Param<Tensor<1>>,
    linear11: Linear,
    constant279: burn::module::Param<Tensor<1>>,
    constant280: burn::module::Param<Tensor<1>>,
    constant281: burn::module::Param<Tensor<1>>,
    linear12: Linear,
    constant282: burn::module::Param<Tensor<1>>,
    constant283: burn::module::Param<Tensor<1>>,
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
        let constant277: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant278: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant279: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant280: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant281: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant282: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant283: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            constant277,
            constant278,
            constant20,
            constant21,
            linear11,
            constant279,
            constant280,
            constant281,
            linear12,
            constant282,
            constant283,
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
        let constant277_out1 = self.constant277.val();
        let pow4_out1 = sub5_out1
            .clone()
            .powf((constant277_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean8_out1 = { pow4_out1.mean_dim(2usize) };
        let constant278_out1 = self.constant278.val();
        let add15_out1 = reducemean8_out1
            .add((constant278_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt10_out1 = add15_out1.sqrt();
        let div7_out1 = sub5_out1.div(sqrt10_out1);
        let constant20_out1 = self.constant20.val();
        let mul11_out1 = div7_out1
            .mul((constant20_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant21_out1 = self.constant21.val();
        let add16_out1 = mul11_out1
            .add((constant21_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear11_out1 = self.linear11.forward(add16_out1.clone());
        let constant279_out1 = self.constant279.val();
        let div8_out1 = linear11_out1
            .clone()
            .div((constant279_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf2_out1 = div8_out1.erf();
        let constant280_out1 = self.constant280.val();
        let add17_out1 = erf2_out1
            .add((constant280_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul12_out1 = linear11_out1.mul(add17_out1);
        let constant281_out1 = self.constant281.val();
        let mul13_out1 = mul12_out1
            .mul((constant281_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear12_out1 = self.linear12.forward(mul13_out1);
        let add18_out1 = linear12_out1.add(add16_out1);
        let reducemean9_out1 = { add18_out1.clone().mean_dim(2usize) };
        let sub6_out1 = add18_out1.sub(reducemean9_out1);
        let constant282_out1 = self.constant282.val();
        let pow5_out1 = sub6_out1
            .clone()
            .powf((constant282_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean10_out1 = { pow5_out1.mean_dim(2usize) };
        let constant283_out1 = self.constant283.val();
        let add19_out1 = reducemean10_out1
            .add((constant283_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant291_out1: [i64; 1] = [32i64];
        let constant290_out1: [i64; 1] = [12i64];
        let concat10_out1: [i64; 4usize] = [
            &unsqueeze23_out1[..],
            &unsqueeze24_out1[..],
            &constant290_out1[..],
            &constant291_out1[..],
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
        let constant297_out1: [i64; 1] = [32i64];
        let constant296_out1: [i64; 1] = [12i64];
        let concat11_out1: [i64; 4usize] = [
            &unsqueeze25_out1[..],
            &unsqueeze26_out1[..],
            &constant296_out1[..],
            &constant297_out1[..],
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
        let constant303_out1: [i64; 1] = [32i64];
        let constant302_out1: [i64; 1] = [12i64];
        let concat12_out1: [i64; 4usize] = [
            &unsqueeze27_out1[..],
            &unsqueeze28_out1[..],
            &constant302_out1[..],
            &constant303_out1[..],
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
        let constant309_out1: [i64; 1] = [384i64];
        let concat13_out1: [i64; 3usize] = [
            &unsqueeze29_out1[..],
            &unsqueeze30_out1[..],
            &constant309_out1[..],
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
    constant310: burn::module::Param<Tensor<1>>,
    constant311: burn::module::Param<Tensor<1>>,
    constant30: burn::module::Param<Tensor<1>>,
    constant31: burn::module::Param<Tensor<1>>,
    linear17: Linear,
    constant312: burn::module::Param<Tensor<1>>,
    constant313: burn::module::Param<Tensor<1>>,
    constant314: burn::module::Param<Tensor<1>>,
    linear18: Linear,
    constant315: burn::module::Param<Tensor<1>>,
    constant316: burn::module::Param<Tensor<1>>,
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
        let constant310: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant311: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant312: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant313: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant314: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant315: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant316: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            constant310,
            constant311,
            constant30,
            constant31,
            linear17,
            constant312,
            constant313,
            constant314,
            linear18,
            constant315,
            constant316,
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
        let constant310_out1 = self.constant310.val();
        let pow6_out1 = sub7_out1
            .clone()
            .powf((constant310_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean12_out1 = { pow6_out1.mean_dim(2usize) };
        let constant311_out1 = self.constant311.val();
        let add23_out1 = reducemean12_out1
            .add((constant311_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt15_out1 = add23_out1.sqrt();
        let div11_out1 = sub7_out1.div(sqrt15_out1);
        let constant30_out1 = self.constant30.val();
        let mul17_out1 = div11_out1
            .mul((constant30_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant31_out1 = self.constant31.val();
        let add24_out1 = mul17_out1
            .add((constant31_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear17_out1 = self.linear17.forward(add24_out1.clone());
        let constant312_out1 = self.constant312.val();
        let div12_out1 = linear17_out1
            .clone()
            .div((constant312_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf3_out1 = div12_out1.erf();
        let constant313_out1 = self.constant313.val();
        let add25_out1 = erf3_out1
            .add((constant313_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul18_out1 = linear17_out1.mul(add25_out1);
        let constant314_out1 = self.constant314.val();
        let mul19_out1 = mul18_out1
            .mul((constant314_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear18_out1 = self.linear18.forward(mul19_out1);
        let add26_out1 = linear18_out1.add(add24_out1);
        let reducemean13_out1 = { add26_out1.clone().mean_dim(2usize) };
        let sub8_out1 = add26_out1.sub(reducemean13_out1);
        let constant315_out1 = self.constant315.val();
        let pow7_out1 = sub8_out1
            .clone()
            .powf((constant315_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean14_out1 = { pow7_out1.mean_dim(2usize) };
        let constant316_out1 = self.constant316.val();
        let add27_out1 = reducemean14_out1
            .add((constant316_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant324_out1: [i64; 1] = [32i64];
        let constant323_out1: [i64; 1] = [12i64];
        let concat14_out1: [i64; 4usize] = [
            &unsqueeze31_out1[..],
            &unsqueeze32_out1[..],
            &constant323_out1[..],
            &constant324_out1[..],
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
        let constant330_out1: [i64; 1] = [32i64];
        let constant329_out1: [i64; 1] = [12i64];
        let concat15_out1: [i64; 4usize] = [
            &unsqueeze33_out1[..],
            &unsqueeze34_out1[..],
            &constant329_out1[..],
            &constant330_out1[..],
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
        let constant336_out1: [i64; 1] = [32i64];
        let constant335_out1: [i64; 1] = [12i64];
        let concat16_out1: [i64; 4usize] = [
            &unsqueeze35_out1[..],
            &unsqueeze36_out1[..],
            &constant335_out1[..],
            &constant336_out1[..],
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
        let constant342_out1: [i64; 1] = [384i64];
        let concat17_out1: [i64; 3usize] = [
            &unsqueeze37_out1[..],
            &unsqueeze38_out1[..],
            &constant342_out1[..],
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
    constant343: burn::module::Param<Tensor<1>>,
    constant344: burn::module::Param<Tensor<1>>,
    constant40: burn::module::Param<Tensor<1>>,
    constant41: burn::module::Param<Tensor<1>>,
    linear23: Linear,
    constant345: burn::module::Param<Tensor<1>>,
    constant346: burn::module::Param<Tensor<1>>,
    constant347: burn::module::Param<Tensor<1>>,
    linear24: Linear,
    constant348: burn::module::Param<Tensor<1>>,
    constant349: burn::module::Param<Tensor<1>>,
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
        let constant343: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant344: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant345: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant346: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant347: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant348: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant349: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            constant343,
            constant344,
            constant40,
            constant41,
            linear23,
            constant345,
            constant346,
            constant347,
            linear24,
            constant348,
            constant349,
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
        let constant343_out1 = self.constant343.val();
        let pow8_out1 = sub9_out1
            .clone()
            .powf((constant343_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean16_out1 = { pow8_out1.mean_dim(2usize) };
        let constant344_out1 = self.constant344.val();
        let add31_out1 = reducemean16_out1
            .add((constant344_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt20_out1 = add31_out1.sqrt();
        let div15_out1 = sub9_out1.div(sqrt20_out1);
        let constant40_out1 = self.constant40.val();
        let mul23_out1 = div15_out1
            .mul((constant40_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant41_out1 = self.constant41.val();
        let add32_out1 = mul23_out1
            .add((constant41_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear23_out1 = self.linear23.forward(add32_out1.clone());
        let constant345_out1 = self.constant345.val();
        let div16_out1 = linear23_out1
            .clone()
            .div((constant345_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf4_out1 = div16_out1.erf();
        let constant346_out1 = self.constant346.val();
        let add33_out1 = erf4_out1
            .add((constant346_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul24_out1 = linear23_out1.mul(add33_out1);
        let constant347_out1 = self.constant347.val();
        let mul25_out1 = mul24_out1
            .mul((constant347_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear24_out1 = self.linear24.forward(mul25_out1);
        let add34_out1 = linear24_out1.add(add32_out1);
        let reducemean17_out1 = { add34_out1.clone().mean_dim(2usize) };
        let sub10_out1 = add34_out1.sub(reducemean17_out1);
        let constant348_out1 = self.constant348.val();
        let pow9_out1 = sub10_out1
            .clone()
            .powf((constant348_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean18_out1 = { pow9_out1.mean_dim(2usize) };
        let constant349_out1 = self.constant349.val();
        let add35_out1 = reducemean18_out1
            .add((constant349_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant357_out1: [i64; 1] = [32i64];
        let constant356_out1: [i64; 1] = [12i64];
        let concat18_out1: [i64; 4usize] = [
            &unsqueeze39_out1[..],
            &unsqueeze40_out1[..],
            &constant356_out1[..],
            &constant357_out1[..],
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
        let constant363_out1: [i64; 1] = [32i64];
        let constant362_out1: [i64; 1] = [12i64];
        let concat19_out1: [i64; 4usize] = [
            &unsqueeze41_out1[..],
            &unsqueeze42_out1[..],
            &constant362_out1[..],
            &constant363_out1[..],
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
        let constant369_out1: [i64; 1] = [32i64];
        let constant368_out1: [i64; 1] = [12i64];
        let concat20_out1: [i64; 4usize] = [
            &unsqueeze43_out1[..],
            &unsqueeze44_out1[..],
            &constant368_out1[..],
            &constant369_out1[..],
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
        let constant375_out1: [i64; 1] = [384i64];
        let concat21_out1: [i64; 3usize] = [
            &unsqueeze45_out1[..],
            &unsqueeze46_out1[..],
            &constant375_out1[..],
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
    constant376: burn::module::Param<Tensor<1>>,
    constant377: burn::module::Param<Tensor<1>>,
    constant50: burn::module::Param<Tensor<1>>,
    constant51: burn::module::Param<Tensor<1>>,
    linear29: Linear,
    constant378: burn::module::Param<Tensor<1>>,
    constant379: burn::module::Param<Tensor<1>>,
    constant380: burn::module::Param<Tensor<1>>,
    linear30: Linear,
    constant381: burn::module::Param<Tensor<1>>,
    constant382: burn::module::Param<Tensor<1>>,
    constant54: burn::module::Param<Tensor<1>>,
    constant55: burn::module::Param<Tensor<1>>,
    linear31: Linear,
    linear32: Linear,
    linear33: Linear,
    linear34: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule6 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant376: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant377: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant378: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant379: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant380: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant381: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant382: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        Self {
            constant376,
            constant377,
            constant50,
            constant51,
            linear29,
            constant378,
            constant379,
            constant380,
            linear30,
            constant381,
            constant382,
            constant54,
            constant55,
            linear31,
            linear32,
            linear33,
            linear34,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add38_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean19_out1 = { add38_out1.clone().mean_dim(2usize) };
        let sub11_out1 = add38_out1.sub(reducemean19_out1);
        let constant376_out1 = self.constant376.val();
        let pow10_out1 = sub11_out1
            .clone()
            .powf((constant376_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean20_out1 = { pow10_out1.mean_dim(2usize) };
        let constant377_out1 = self.constant377.val();
        let add39_out1 = reducemean20_out1
            .add((constant377_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt25_out1 = add39_out1.sqrt();
        let div19_out1 = sub11_out1.div(sqrt25_out1);
        let constant50_out1 = self.constant50.val();
        let mul29_out1 = div19_out1
            .mul((constant50_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant51_out1 = self.constant51.val();
        let add40_out1 = mul29_out1
            .add((constant51_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear29_out1 = self.linear29.forward(add40_out1.clone());
        let constant378_out1 = self.constant378.val();
        let div20_out1 = linear29_out1
            .clone()
            .div((constant378_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf5_out1 = div20_out1.erf();
        let constant379_out1 = self.constant379.val();
        let add41_out1 = erf5_out1
            .add((constant379_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul30_out1 = linear29_out1.mul(add41_out1);
        let constant380_out1 = self.constant380.val();
        let mul31_out1 = mul30_out1
            .mul((constant380_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear30_out1 = self.linear30.forward(mul31_out1);
        let add42_out1 = linear30_out1.add(add40_out1);
        let reducemean21_out1 = { add42_out1.clone().mean_dim(2usize) };
        let sub12_out1 = add42_out1.sub(reducemean21_out1);
        let constant381_out1 = self.constant381.val();
        let pow11_out1 = sub12_out1
            .clone()
            .powf((constant381_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean22_out1 = { pow11_out1.mean_dim(2usize) };
        let constant382_out1 = self.constant382.val();
        let add43_out1 = reducemean22_out1
            .add((constant382_out1).unsqueeze_dims(&[0isize, 1isize]));
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
        let constant390_out1: [i64; 1] = [32i64];
        let constant389_out1: [i64; 1] = [12i64];
        let concat22_out1: [i64; 4usize] = [
            &unsqueeze47_out1[..],
            &unsqueeze48_out1[..],
            &constant389_out1[..],
            &constant390_out1[..],
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
        let constant396_out1: [i64; 1] = [32i64];
        let constant395_out1: [i64; 1] = [12i64];
        let concat23_out1: [i64; 4usize] = [
            &unsqueeze49_out1[..],
            &unsqueeze50_out1[..],
            &constant395_out1[..],
            &constant396_out1[..],
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
        let constant402_out1: [i64; 1] = [32i64];
        let constant401_out1: [i64; 1] = [12i64];
        let concat24_out1: [i64; 4usize] = [
            &unsqueeze51_out1[..],
            &unsqueeze52_out1[..],
            &constant401_out1[..],
            &constant402_out1[..],
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
        let constant408_out1: [i64; 1] = [384i64];
        let concat25_out1: [i64; 3usize] = [
            &unsqueeze53_out1[..],
            &unsqueeze54_out1[..],
            &constant408_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape25_out1 = transpose24_out1.reshape(concat25_out1);
        let linear34_out1 = self.linear34.forward(reshape25_out1);
        let add46_out1 = linear34_out1.add(add44_out1);
        add46_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule7 {
    constant409: burn::module::Param<Tensor<1>>,
    constant410: burn::module::Param<Tensor<1>>,
    constant60: burn::module::Param<Tensor<1>>,
    constant61: burn::module::Param<Tensor<1>>,
    linear35: Linear,
    constant411: burn::module::Param<Tensor<1>>,
    constant412: burn::module::Param<Tensor<1>>,
    constant413: burn::module::Param<Tensor<1>>,
    linear36: Linear,
    constant414: burn::module::Param<Tensor<1>>,
    constant415: burn::module::Param<Tensor<1>>,
    constant64: burn::module::Param<Tensor<1>>,
    constant65: burn::module::Param<Tensor<1>>,
    linear37: Linear,
    linear38: Linear,
    linear39: Linear,
    linear40: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule7 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant409: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant410: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant411: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant412: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant413: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant414: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant415: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear37 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear38 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear39 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear40 = LinearConfig::new(384, 384).with_bias(true).init(device);
        Self {
            constant409,
            constant410,
            constant60,
            constant61,
            linear35,
            constant411,
            constant412,
            constant413,
            linear36,
            constant414,
            constant415,
            constant64,
            constant65,
            linear37,
            linear38,
            linear39,
            linear40,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add46_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean23_out1 = { add46_out1.clone().mean_dim(2usize) };
        let sub13_out1 = add46_out1.sub(reducemean23_out1);
        let constant409_out1 = self.constant409.val();
        let pow12_out1 = sub13_out1
            .clone()
            .powf((constant409_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean24_out1 = { pow12_out1.mean_dim(2usize) };
        let constant410_out1 = self.constant410.val();
        let add47_out1 = reducemean24_out1
            .add((constant410_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt30_out1 = add47_out1.sqrt();
        let div23_out1 = sub13_out1.div(sqrt30_out1);
        let constant60_out1 = self.constant60.val();
        let mul35_out1 = div23_out1
            .mul((constant60_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant61_out1 = self.constant61.val();
        let add48_out1 = mul35_out1
            .add((constant61_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear35_out1 = self.linear35.forward(add48_out1.clone());
        let constant411_out1 = self.constant411.val();
        let div24_out1 = linear35_out1
            .clone()
            .div((constant411_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf6_out1 = div24_out1.erf();
        let constant412_out1 = self.constant412.val();
        let add49_out1 = erf6_out1
            .add((constant412_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul36_out1 = linear35_out1.mul(add49_out1);
        let constant413_out1 = self.constant413.val();
        let mul37_out1 = mul36_out1
            .mul((constant413_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear36_out1 = self.linear36.forward(mul37_out1);
        let add50_out1 = linear36_out1.add(add48_out1);
        let reducemean25_out1 = { add50_out1.clone().mean_dim(2usize) };
        let sub14_out1 = add50_out1.sub(reducemean25_out1);
        let constant414_out1 = self.constant414.val();
        let pow13_out1 = sub14_out1
            .clone()
            .powf((constant414_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean26_out1 = { pow13_out1.mean_dim(2usize) };
        let constant415_out1 = self.constant415.val();
        let add51_out1 = reducemean26_out1
            .add((constant415_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt31_out1 = add51_out1.sqrt();
        let div25_out1 = sub14_out1.div(sqrt31_out1);
        let constant64_out1 = self.constant64.val();
        let mul38_out1 = div25_out1
            .mul((constant64_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant65_out1 = self.constant65.val();
        let add52_out1 = mul38_out1
            .add((constant65_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape59_out1: [i64; 3] = {
            let axes = &add52_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather55_out1 = shape59_out1[0] as i64;
        let gather56_out1 = shape59_out1[1] as i64;
        let linear37_out1 = self.linear37.forward(add52_out1.clone());
        let shape61_out1: [i64; 3] = {
            let axes = &linear37_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather57_out1 = shape61_out1[0] as i64;
        let gather58_out1 = shape61_out1[1] as i64;
        let unsqueeze55_out1 = [gather57_out1 as i64];
        let unsqueeze56_out1 = [gather58_out1 as i64];
        let constant423_out1: [i64; 1] = [32i64];
        let constant422_out1: [i64; 1] = [12i64];
        let concat26_out1: [i64; 4usize] = [
            &unsqueeze55_out1[..],
            &unsqueeze56_out1[..],
            &constant422_out1[..],
            &constant423_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape26_out1 = linear37_out1.reshape(concat26_out1);
        let transpose25_out1 = reshape26_out1.permute([0, 2, 1, 3]);
        let linear38_out1 = self.linear38.forward(add52_out1.clone());
        let shape63_out1: [i64; 3] = {
            let axes = &linear38_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather59_out1 = shape63_out1[0] as i64;
        let gather60_out1 = shape63_out1[1] as i64;
        let unsqueeze57_out1 = [gather59_out1 as i64];
        let unsqueeze58_out1 = [gather60_out1 as i64];
        let constant429_out1: [i64; 1] = [32i64];
        let constant428_out1: [i64; 1] = [12i64];
        let concat27_out1: [i64; 4usize] = [
            &unsqueeze57_out1[..],
            &unsqueeze58_out1[..],
            &constant428_out1[..],
            &constant429_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape27_out1 = linear38_out1.reshape(concat27_out1);
        let linear39_out1 = self.linear39.forward(add52_out1.clone());
        let shape65_out1: [i64; 3] = {
            let axes = &linear39_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather61_out1 = shape65_out1[0] as i64;
        let gather62_out1 = shape65_out1[1] as i64;
        let unsqueeze59_out1 = [gather61_out1 as i64];
        let unsqueeze60_out1 = [gather62_out1 as i64];
        let constant435_out1: [i64; 1] = [32i64];
        let constant434_out1: [i64; 1] = [12i64];
        let concat28_out1: [i64; 4usize] = [
            &unsqueeze59_out1[..],
            &unsqueeze60_out1[..],
            &constant434_out1[..],
            &constant435_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape28_out1 = linear39_out1.reshape(concat28_out1);
        let transpose26_out1 = reshape28_out1.permute([0, 2, 1, 3]);
        let transpose27_out1 = reshape27_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul53_out1,)
        };
        let transpose28_out1 = matmul53_out1.permute([0, 2, 1, 3]);
        let unsqueeze61_out1 = [gather55_out1 as i64];
        let unsqueeze62_out1 = [gather56_out1 as i64];
        let constant441_out1: [i64; 1] = [384i64];
        let concat29_out1: [i64; 3usize] = [
            &unsqueeze61_out1[..],
            &unsqueeze62_out1[..],
            &constant441_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape29_out1 = transpose28_out1.reshape(concat29_out1);
        let linear40_out1 = self.linear40.forward(reshape29_out1);
        let add54_out1 = linear40_out1.add(add52_out1);
        add54_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule8 {
    constant442: burn::module::Param<Tensor<1>>,
    constant443: burn::module::Param<Tensor<1>>,
    constant70: burn::module::Param<Tensor<1>>,
    constant71: burn::module::Param<Tensor<1>>,
    linear41: Linear,
    constant444: burn::module::Param<Tensor<1>>,
    constant445: burn::module::Param<Tensor<1>>,
    constant446: burn::module::Param<Tensor<1>>,
    linear42: Linear,
    constant447: burn::module::Param<Tensor<1>>,
    constant448: burn::module::Param<Tensor<1>>,
    constant74: burn::module::Param<Tensor<1>>,
    constant75: burn::module::Param<Tensor<1>>,
    linear43: Linear,
    linear44: Linear,
    linear45: Linear,
    linear46: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule8 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant442: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant443: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant70: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant71: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear41 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant444: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant445: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant446: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear42 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        let constant447: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant448: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant74: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant75: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear43 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear44 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear45 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear46 = LinearConfig::new(384, 384).with_bias(true).init(device);
        Self {
            constant442,
            constant443,
            constant70,
            constant71,
            linear41,
            constant444,
            constant445,
            constant446,
            linear42,
            constant447,
            constant448,
            constant74,
            constant75,
            linear43,
            linear44,
            linear45,
            linear46,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add54_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean27_out1 = { add54_out1.clone().mean_dim(2usize) };
        let sub15_out1 = add54_out1.sub(reducemean27_out1);
        let constant442_out1 = self.constant442.val();
        let pow14_out1 = sub15_out1
            .clone()
            .powf((constant442_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean28_out1 = { pow14_out1.mean_dim(2usize) };
        let constant443_out1 = self.constant443.val();
        let add55_out1 = reducemean28_out1
            .add((constant443_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt35_out1 = add55_out1.sqrt();
        let div27_out1 = sub15_out1.div(sqrt35_out1);
        let constant70_out1 = self.constant70.val();
        let mul41_out1 = div27_out1
            .mul((constant70_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant71_out1 = self.constant71.val();
        let add56_out1 = mul41_out1
            .add((constant71_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear41_out1 = self.linear41.forward(add56_out1.clone());
        let constant444_out1 = self.constant444.val();
        let div28_out1 = linear41_out1
            .clone()
            .div((constant444_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf7_out1 = div28_out1.erf();
        let constant445_out1 = self.constant445.val();
        let add57_out1 = erf7_out1
            .add((constant445_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul42_out1 = linear41_out1.mul(add57_out1);
        let constant446_out1 = self.constant446.val();
        let mul43_out1 = mul42_out1
            .mul((constant446_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear42_out1 = self.linear42.forward(mul43_out1);
        let add58_out1 = linear42_out1.add(add56_out1);
        let reducemean29_out1 = { add58_out1.clone().mean_dim(2usize) };
        let sub16_out1 = add58_out1.sub(reducemean29_out1);
        let constant447_out1 = self.constant447.val();
        let pow15_out1 = sub16_out1
            .clone()
            .powf((constant447_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean30_out1 = { pow15_out1.mean_dim(2usize) };
        let constant448_out1 = self.constant448.val();
        let add59_out1 = reducemean30_out1
            .add((constant448_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt36_out1 = add59_out1.sqrt();
        let div29_out1 = sub16_out1.div(sqrt36_out1);
        let constant74_out1 = self.constant74.val();
        let mul44_out1 = div29_out1
            .mul((constant74_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant75_out1 = self.constant75.val();
        let add60_out1 = mul44_out1
            .add((constant75_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape68_out1: [i64; 3] = {
            let axes = &add60_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather63_out1 = shape68_out1[0] as i64;
        let gather64_out1 = shape68_out1[1] as i64;
        let linear43_out1 = self.linear43.forward(add60_out1.clone());
        let shape70_out1: [i64; 3] = {
            let axes = &linear43_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather65_out1 = shape70_out1[0] as i64;
        let gather66_out1 = shape70_out1[1] as i64;
        let unsqueeze63_out1 = [gather65_out1 as i64];
        let unsqueeze64_out1 = [gather66_out1 as i64];
        let constant456_out1: [i64; 1] = [32i64];
        let constant455_out1: [i64; 1] = [12i64];
        let concat30_out1: [i64; 4usize] = [
            &unsqueeze63_out1[..],
            &unsqueeze64_out1[..],
            &constant455_out1[..],
            &constant456_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape30_out1 = linear43_out1.reshape(concat30_out1);
        let transpose29_out1 = reshape30_out1.permute([0, 2, 1, 3]);
        let linear44_out1 = self.linear44.forward(add60_out1.clone());
        let shape72_out1: [i64; 3] = {
            let axes = &linear44_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather67_out1 = shape72_out1[0] as i64;
        let gather68_out1 = shape72_out1[1] as i64;
        let unsqueeze65_out1 = [gather67_out1 as i64];
        let unsqueeze66_out1 = [gather68_out1 as i64];
        let constant462_out1: [i64; 1] = [32i64];
        let constant461_out1: [i64; 1] = [12i64];
        let concat31_out1: [i64; 4usize] = [
            &unsqueeze65_out1[..],
            &unsqueeze66_out1[..],
            &constant461_out1[..],
            &constant462_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape31_out1 = linear44_out1.reshape(concat31_out1);
        let linear45_out1 = self.linear45.forward(add60_out1.clone());
        let shape74_out1: [i64; 3] = {
            let axes = &linear45_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather69_out1 = shape74_out1[0] as i64;
        let gather70_out1 = shape74_out1[1] as i64;
        let unsqueeze67_out1 = [gather69_out1 as i64];
        let unsqueeze68_out1 = [gather70_out1 as i64];
        let constant468_out1: [i64; 1] = [32i64];
        let constant467_out1: [i64; 1] = [12i64];
        let concat32_out1: [i64; 4usize] = [
            &unsqueeze67_out1[..],
            &unsqueeze68_out1[..],
            &constant467_out1[..],
            &constant468_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape32_out1 = linear45_out1.reshape(concat32_out1);
        let transpose30_out1 = reshape32_out1.permute([0, 2, 1, 3]);
        let transpose31_out1 = reshape31_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul61_out1,)
        };
        let transpose32_out1 = matmul61_out1.permute([0, 2, 1, 3]);
        let unsqueeze69_out1 = [gather63_out1 as i64];
        let unsqueeze70_out1 = [gather64_out1 as i64];
        let constant474_out1: [i64; 1] = [384i64];
        let concat33_out1: [i64; 3usize] = [
            &unsqueeze69_out1[..],
            &unsqueeze70_out1[..],
            &constant474_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape33_out1 = transpose32_out1.reshape(concat33_out1);
        let linear46_out1 = self.linear46.forward(reshape33_out1);
        let add62_out1 = linear46_out1.add(add60_out1);
        add62_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule9 {
    constant475: burn::module::Param<Tensor<1>>,
    constant476: burn::module::Param<Tensor<1>>,
    constant80: burn::module::Param<Tensor<1>>,
    constant81: burn::module::Param<Tensor<1>>,
    linear47: Linear,
    constant477: burn::module::Param<Tensor<1>>,
    constant478: burn::module::Param<Tensor<1>>,
    constant479: burn::module::Param<Tensor<1>>,
    linear48: Linear,
    constant480: burn::module::Param<Tensor<1>>,
    constant481: burn::module::Param<Tensor<1>>,
    constant84: burn::module::Param<Tensor<1>>,
    constant85: burn::module::Param<Tensor<1>>,
    linear49: Linear,
    linear50: Linear,
    linear51: Linear,
    linear52: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule9 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant475: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant476: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant80: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant81: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear47 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant477: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant478: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant479: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear48 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        let constant480: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant481: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant84: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant85: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear49 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear50 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear51 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear52 = LinearConfig::new(384, 384).with_bias(true).init(device);
        Self {
            constant475,
            constant476,
            constant80,
            constant81,
            linear47,
            constant477,
            constant478,
            constant479,
            linear48,
            constant480,
            constant481,
            constant84,
            constant85,
            linear49,
            linear50,
            linear51,
            linear52,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add62_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean31_out1 = { add62_out1.clone().mean_dim(2usize) };
        let sub17_out1 = add62_out1.sub(reducemean31_out1);
        let constant475_out1 = self.constant475.val();
        let pow16_out1 = sub17_out1
            .clone()
            .powf((constant475_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean32_out1 = { pow16_out1.mean_dim(2usize) };
        let constant476_out1 = self.constant476.val();
        let add63_out1 = reducemean32_out1
            .add((constant476_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt40_out1 = add63_out1.sqrt();
        let div31_out1 = sub17_out1.div(sqrt40_out1);
        let constant80_out1 = self.constant80.val();
        let mul47_out1 = div31_out1
            .mul((constant80_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant81_out1 = self.constant81.val();
        let add64_out1 = mul47_out1
            .add((constant81_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear47_out1 = self.linear47.forward(add64_out1.clone());
        let constant477_out1 = self.constant477.val();
        let div32_out1 = linear47_out1
            .clone()
            .div((constant477_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf8_out1 = div32_out1.erf();
        let constant478_out1 = self.constant478.val();
        let add65_out1 = erf8_out1
            .add((constant478_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul48_out1 = linear47_out1.mul(add65_out1);
        let constant479_out1 = self.constant479.val();
        let mul49_out1 = mul48_out1
            .mul((constant479_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear48_out1 = self.linear48.forward(mul49_out1);
        let add66_out1 = linear48_out1.add(add64_out1);
        let reducemean33_out1 = { add66_out1.clone().mean_dim(2usize) };
        let sub18_out1 = add66_out1.sub(reducemean33_out1);
        let constant480_out1 = self.constant480.val();
        let pow17_out1 = sub18_out1
            .clone()
            .powf((constant480_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean34_out1 = { pow17_out1.mean_dim(2usize) };
        let constant481_out1 = self.constant481.val();
        let add67_out1 = reducemean34_out1
            .add((constant481_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt41_out1 = add67_out1.sqrt();
        let div33_out1 = sub18_out1.div(sqrt41_out1);
        let constant84_out1 = self.constant84.val();
        let mul50_out1 = div33_out1
            .mul((constant84_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant85_out1 = self.constant85.val();
        let add68_out1 = mul50_out1
            .add((constant85_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape77_out1: [i64; 3] = {
            let axes = &add68_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather71_out1 = shape77_out1[0] as i64;
        let gather72_out1 = shape77_out1[1] as i64;
        let linear49_out1 = self.linear49.forward(add68_out1.clone());
        let shape79_out1: [i64; 3] = {
            let axes = &linear49_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather73_out1 = shape79_out1[0] as i64;
        let gather74_out1 = shape79_out1[1] as i64;
        let unsqueeze71_out1 = [gather73_out1 as i64];
        let unsqueeze72_out1 = [gather74_out1 as i64];
        let constant489_out1: [i64; 1] = [32i64];
        let constant488_out1: [i64; 1] = [12i64];
        let concat34_out1: [i64; 4usize] = [
            &unsqueeze71_out1[..],
            &unsqueeze72_out1[..],
            &constant488_out1[..],
            &constant489_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape34_out1 = linear49_out1.reshape(concat34_out1);
        let transpose33_out1 = reshape34_out1.permute([0, 2, 1, 3]);
        let linear50_out1 = self.linear50.forward(add68_out1.clone());
        let shape81_out1: [i64; 3] = {
            let axes = &linear50_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather75_out1 = shape81_out1[0] as i64;
        let gather76_out1 = shape81_out1[1] as i64;
        let unsqueeze73_out1 = [gather75_out1 as i64];
        let unsqueeze74_out1 = [gather76_out1 as i64];
        let constant495_out1: [i64; 1] = [32i64];
        let constant494_out1: [i64; 1] = [12i64];
        let concat35_out1: [i64; 4usize] = [
            &unsqueeze73_out1[..],
            &unsqueeze74_out1[..],
            &constant494_out1[..],
            &constant495_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape35_out1 = linear50_out1.reshape(concat35_out1);
        let linear51_out1 = self.linear51.forward(add68_out1.clone());
        let shape83_out1: [i64; 3] = {
            let axes = &linear51_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather77_out1 = shape83_out1[0] as i64;
        let gather78_out1 = shape83_out1[1] as i64;
        let unsqueeze75_out1 = [gather77_out1 as i64];
        let unsqueeze76_out1 = [gather78_out1 as i64];
        let constant501_out1: [i64; 1] = [32i64];
        let constant500_out1: [i64; 1] = [12i64];
        let concat36_out1: [i64; 4usize] = [
            &unsqueeze75_out1[..],
            &unsqueeze76_out1[..],
            &constant500_out1[..],
            &constant501_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape36_out1 = linear51_out1.reshape(concat36_out1);
        let transpose34_out1 = reshape36_out1.permute([0, 2, 1, 3]);
        let transpose35_out1 = reshape35_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul69_out1,)
        };
        let transpose36_out1 = matmul69_out1.permute([0, 2, 1, 3]);
        let unsqueeze77_out1 = [gather71_out1 as i64];
        let unsqueeze78_out1 = [gather72_out1 as i64];
        let constant507_out1: [i64; 1] = [384i64];
        let concat37_out1: [i64; 3usize] = [
            &unsqueeze77_out1[..],
            &unsqueeze78_out1[..],
            &constant507_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape37_out1 = transpose36_out1.reshape(concat37_out1);
        let linear52_out1 = self.linear52.forward(reshape37_out1);
        let add70_out1 = linear52_out1.add(add68_out1);
        add70_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule10 {
    constant508: burn::module::Param<Tensor<1>>,
    constant509: burn::module::Param<Tensor<1>>,
    constant90: burn::module::Param<Tensor<1>>,
    constant91: burn::module::Param<Tensor<1>>,
    linear53: Linear,
    constant510: burn::module::Param<Tensor<1>>,
    constant511: burn::module::Param<Tensor<1>>,
    constant512: burn::module::Param<Tensor<1>>,
    linear54: Linear,
    constant513: burn::module::Param<Tensor<1>>,
    constant514: burn::module::Param<Tensor<1>>,
    constant94: burn::module::Param<Tensor<1>>,
    constant95: burn::module::Param<Tensor<1>>,
    linear55: Linear,
    linear56: Linear,
    linear57: Linear,
    linear58: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule10 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant508: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant509: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant90: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant91: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear53 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant510: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant511: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant512: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear54 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        let constant513: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant514: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant94: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant95: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear55 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear56 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear57 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear58 = LinearConfig::new(384, 384).with_bias(true).init(device);
        Self {
            constant508,
            constant509,
            constant90,
            constant91,
            linear53,
            constant510,
            constant511,
            constant512,
            linear54,
            constant513,
            constant514,
            constant94,
            constant95,
            linear55,
            linear56,
            linear57,
            linear58,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add70_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean35_out1 = { add70_out1.clone().mean_dim(2usize) };
        let sub19_out1 = add70_out1.sub(reducemean35_out1);
        let constant508_out1 = self.constant508.val();
        let pow18_out1 = sub19_out1
            .clone()
            .powf((constant508_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean36_out1 = { pow18_out1.mean_dim(2usize) };
        let constant509_out1 = self.constant509.val();
        let add71_out1 = reducemean36_out1
            .add((constant509_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt45_out1 = add71_out1.sqrt();
        let div35_out1 = sub19_out1.div(sqrt45_out1);
        let constant90_out1 = self.constant90.val();
        let mul53_out1 = div35_out1
            .mul((constant90_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant91_out1 = self.constant91.val();
        let add72_out1 = mul53_out1
            .add((constant91_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear53_out1 = self.linear53.forward(add72_out1.clone());
        let constant510_out1 = self.constant510.val();
        let div36_out1 = linear53_out1
            .clone()
            .div((constant510_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf9_out1 = div36_out1.erf();
        let constant511_out1 = self.constant511.val();
        let add73_out1 = erf9_out1
            .add((constant511_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul54_out1 = linear53_out1.mul(add73_out1);
        let constant512_out1 = self.constant512.val();
        let mul55_out1 = mul54_out1
            .mul((constant512_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear54_out1 = self.linear54.forward(mul55_out1);
        let add74_out1 = linear54_out1.add(add72_out1);
        let reducemean37_out1 = { add74_out1.clone().mean_dim(2usize) };
        let sub20_out1 = add74_out1.sub(reducemean37_out1);
        let constant513_out1 = self.constant513.val();
        let pow19_out1 = sub20_out1
            .clone()
            .powf((constant513_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean38_out1 = { pow19_out1.mean_dim(2usize) };
        let constant514_out1 = self.constant514.val();
        let add75_out1 = reducemean38_out1
            .add((constant514_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt46_out1 = add75_out1.sqrt();
        let div37_out1 = sub20_out1.div(sqrt46_out1);
        let constant94_out1 = self.constant94.val();
        let mul56_out1 = div37_out1
            .mul((constant94_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant95_out1 = self.constant95.val();
        let add76_out1 = mul56_out1
            .add((constant95_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape86_out1: [i64; 3] = {
            let axes = &add76_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather79_out1 = shape86_out1[0] as i64;
        let gather80_out1 = shape86_out1[1] as i64;
        let linear55_out1 = self.linear55.forward(add76_out1.clone());
        let shape88_out1: [i64; 3] = {
            let axes = &linear55_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather81_out1 = shape88_out1[0] as i64;
        let gather82_out1 = shape88_out1[1] as i64;
        let unsqueeze79_out1 = [gather81_out1 as i64];
        let unsqueeze80_out1 = [gather82_out1 as i64];
        let constant522_out1: [i64; 1] = [32i64];
        let constant521_out1: [i64; 1] = [12i64];
        let concat38_out1: [i64; 4usize] = [
            &unsqueeze79_out1[..],
            &unsqueeze80_out1[..],
            &constant521_out1[..],
            &constant522_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape38_out1 = linear55_out1.reshape(concat38_out1);
        let transpose37_out1 = reshape38_out1.permute([0, 2, 1, 3]);
        let linear56_out1 = self.linear56.forward(add76_out1.clone());
        let shape90_out1: [i64; 3] = {
            let axes = &linear56_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather83_out1 = shape90_out1[0] as i64;
        let gather84_out1 = shape90_out1[1] as i64;
        let unsqueeze81_out1 = [gather83_out1 as i64];
        let unsqueeze82_out1 = [gather84_out1 as i64];
        let constant528_out1: [i64; 1] = [32i64];
        let constant527_out1: [i64; 1] = [12i64];
        let concat39_out1: [i64; 4usize] = [
            &unsqueeze81_out1[..],
            &unsqueeze82_out1[..],
            &constant527_out1[..],
            &constant528_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape39_out1 = linear56_out1.reshape(concat39_out1);
        let linear57_out1 = self.linear57.forward(add76_out1.clone());
        let shape92_out1: [i64; 3] = {
            let axes = &linear57_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather85_out1 = shape92_out1[0] as i64;
        let gather86_out1 = shape92_out1[1] as i64;
        let unsqueeze83_out1 = [gather85_out1 as i64];
        let unsqueeze84_out1 = [gather86_out1 as i64];
        let constant534_out1: [i64; 1] = [32i64];
        let constant533_out1: [i64; 1] = [12i64];
        let concat40_out1: [i64; 4usize] = [
            &unsqueeze83_out1[..],
            &unsqueeze84_out1[..],
            &constant533_out1[..],
            &constant534_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape40_out1 = linear57_out1.reshape(concat40_out1);
        let transpose38_out1 = reshape40_out1.permute([0, 2, 1, 3]);
        let transpose39_out1 = reshape39_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul77_out1,)
        };
        let transpose40_out1 = matmul77_out1.permute([0, 2, 1, 3]);
        let unsqueeze85_out1 = [gather79_out1 as i64];
        let unsqueeze86_out1 = [gather80_out1 as i64];
        let constant540_out1: [i64; 1] = [384i64];
        let concat41_out1: [i64; 3usize] = [
            &unsqueeze85_out1[..],
            &unsqueeze86_out1[..],
            &constant540_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape41_out1 = transpose40_out1.reshape(concat41_out1);
        let linear58_out1 = self.linear58.forward(reshape41_out1);
        let add78_out1 = linear58_out1.add(add76_out1);
        add78_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule11 {
    constant541: burn::module::Param<Tensor<1>>,
    constant542: burn::module::Param<Tensor<1>>,
    constant100: burn::module::Param<Tensor<1>>,
    constant101: burn::module::Param<Tensor<1>>,
    linear59: Linear,
    constant543: burn::module::Param<Tensor<1>>,
    constant544: burn::module::Param<Tensor<1>>,
    constant545: burn::module::Param<Tensor<1>>,
    linear60: Linear,
    constant546: burn::module::Param<Tensor<1>>,
    constant547: burn::module::Param<Tensor<1>>,
    constant104: burn::module::Param<Tensor<1>>,
    constant105: burn::module::Param<Tensor<1>>,
    linear61: Linear,
    linear62: Linear,
    linear63: Linear,
    linear64: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule11 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant541: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant542: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant100: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant101: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear59 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant543: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant544: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant545: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear60 = LinearConfig::new(1536, 384).with_bias(true).init(device);
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
                burn::tensor::TensorData::from([0.0000000000009999999960041972f64]),
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
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant105: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear61 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear62 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear63 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear64 = LinearConfig::new(384, 384).with_bias(true).init(device);
        Self {
            constant541,
            constant542,
            constant100,
            constant101,
            linear59,
            constant543,
            constant544,
            constant545,
            linear60,
            constant546,
            constant547,
            constant104,
            constant105,
            linear61,
            linear62,
            linear63,
            linear64,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add78_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean39_out1 = { add78_out1.clone().mean_dim(2usize) };
        let sub21_out1 = add78_out1.sub(reducemean39_out1);
        let constant541_out1 = self.constant541.val();
        let pow20_out1 = sub21_out1
            .clone()
            .powf((constant541_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean40_out1 = { pow20_out1.mean_dim(2usize) };
        let constant542_out1 = self.constant542.val();
        let add79_out1 = reducemean40_out1
            .add((constant542_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt50_out1 = add79_out1.sqrt();
        let div39_out1 = sub21_out1.div(sqrt50_out1);
        let constant100_out1 = self.constant100.val();
        let mul59_out1 = div39_out1
            .mul((constant100_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant101_out1 = self.constant101.val();
        let add80_out1 = mul59_out1
            .add((constant101_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear59_out1 = self.linear59.forward(add80_out1.clone());
        let constant543_out1 = self.constant543.val();
        let div40_out1 = linear59_out1
            .clone()
            .div((constant543_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf10_out1 = div40_out1.erf();
        let constant544_out1 = self.constant544.val();
        let add81_out1 = erf10_out1
            .add((constant544_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul60_out1 = linear59_out1.mul(add81_out1);
        let constant545_out1 = self.constant545.val();
        let mul61_out1 = mul60_out1
            .mul((constant545_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear60_out1 = self.linear60.forward(mul61_out1);
        let add82_out1 = linear60_out1.add(add80_out1);
        let reducemean41_out1 = { add82_out1.clone().mean_dim(2usize) };
        let sub22_out1 = add82_out1.sub(reducemean41_out1);
        let constant546_out1 = self.constant546.val();
        let pow21_out1 = sub22_out1
            .clone()
            .powf((constant546_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean42_out1 = { pow21_out1.mean_dim(2usize) };
        let constant547_out1 = self.constant547.val();
        let add83_out1 = reducemean42_out1
            .add((constant547_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt51_out1 = add83_out1.sqrt();
        let div41_out1 = sub22_out1.div(sqrt51_out1);
        let constant104_out1 = self.constant104.val();
        let mul62_out1 = div41_out1
            .mul((constant104_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant105_out1 = self.constant105.val();
        let add84_out1 = mul62_out1
            .add((constant105_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape95_out1: [i64; 3] = {
            let axes = &add84_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather87_out1 = shape95_out1[0] as i64;
        let gather88_out1 = shape95_out1[1] as i64;
        let linear61_out1 = self.linear61.forward(add84_out1.clone());
        let shape97_out1: [i64; 3] = {
            let axes = &linear61_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather89_out1 = shape97_out1[0] as i64;
        let gather90_out1 = shape97_out1[1] as i64;
        let unsqueeze87_out1 = [gather89_out1 as i64];
        let unsqueeze88_out1 = [gather90_out1 as i64];
        let constant555_out1: [i64; 1] = [32i64];
        let constant554_out1: [i64; 1] = [12i64];
        let concat42_out1: [i64; 4usize] = [
            &unsqueeze87_out1[..],
            &unsqueeze88_out1[..],
            &constant554_out1[..],
            &constant555_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape42_out1 = linear61_out1.reshape(concat42_out1);
        let transpose41_out1 = reshape42_out1.permute([0, 2, 1, 3]);
        let linear62_out1 = self.linear62.forward(add84_out1.clone());
        let shape99_out1: [i64; 3] = {
            let axes = &linear62_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather91_out1 = shape99_out1[0] as i64;
        let gather92_out1 = shape99_out1[1] as i64;
        let unsqueeze89_out1 = [gather91_out1 as i64];
        let unsqueeze90_out1 = [gather92_out1 as i64];
        let constant561_out1: [i64; 1] = [32i64];
        let constant560_out1: [i64; 1] = [12i64];
        let concat43_out1: [i64; 4usize] = [
            &unsqueeze89_out1[..],
            &unsqueeze90_out1[..],
            &constant560_out1[..],
            &constant561_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape43_out1 = linear62_out1.reshape(concat43_out1);
        let linear63_out1 = self.linear63.forward(add84_out1.clone());
        let shape101_out1: [i64; 3] = {
            let axes = &linear63_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather93_out1 = shape101_out1[0] as i64;
        let gather94_out1 = shape101_out1[1] as i64;
        let unsqueeze91_out1 = [gather93_out1 as i64];
        let unsqueeze92_out1 = [gather94_out1 as i64];
        let constant567_out1: [i64; 1] = [32i64];
        let constant566_out1: [i64; 1] = [12i64];
        let concat44_out1: [i64; 4usize] = [
            &unsqueeze91_out1[..],
            &unsqueeze92_out1[..],
            &constant566_out1[..],
            &constant567_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape44_out1 = linear63_out1.reshape(concat44_out1);
        let transpose42_out1 = reshape44_out1.permute([0, 2, 1, 3]);
        let transpose43_out1 = reshape43_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul85_out1,)
        };
        let transpose44_out1 = matmul85_out1.permute([0, 2, 1, 3]);
        let unsqueeze93_out1 = [gather87_out1 as i64];
        let unsqueeze94_out1 = [gather88_out1 as i64];
        let constant573_out1: [i64; 1] = [384i64];
        let concat45_out1: [i64; 3usize] = [
            &unsqueeze93_out1[..],
            &unsqueeze94_out1[..],
            &constant573_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape45_out1 = transpose44_out1.reshape(concat45_out1);
        let linear64_out1 = self.linear64.forward(reshape45_out1);
        let add86_out1 = linear64_out1.add(add84_out1);
        add86_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule12 {
    constant574: burn::module::Param<Tensor<1>>,
    constant575: burn::module::Param<Tensor<1>>,
    constant110: burn::module::Param<Tensor<1>>,
    constant111: burn::module::Param<Tensor<1>>,
    linear65: Linear,
    constant576: burn::module::Param<Tensor<1>>,
    constant577: burn::module::Param<Tensor<1>>,
    constant578: burn::module::Param<Tensor<1>>,
    linear66: Linear,
    constant579: burn::module::Param<Tensor<1>>,
    constant580: burn::module::Param<Tensor<1>>,
    constant114: burn::module::Param<Tensor<1>>,
    constant115: burn::module::Param<Tensor<1>>,
    linear67: Linear,
    linear68: Linear,
    linear69: Linear,
    linear70: Linear,
    constant607: burn::module::Param<Tensor<1>>,
    constant608: burn::module::Param<Tensor<1>>,
    constant120: burn::module::Param<Tensor<1>>,
    constant121: burn::module::Param<Tensor<1>>,
    linear71: Linear,
    constant609: burn::module::Param<Tensor<1>>,
    constant610: burn::module::Param<Tensor<1>>,
    constant611: burn::module::Param<Tensor<1>>,
    linear72: Linear,
    constant612: burn::module::Param<Tensor<1>>,
    constant613: burn::module::Param<Tensor<1>>,
    constant124: burn::module::Param<Tensor<1>>,
    constant125: burn::module::Param<Tensor<1>>,
    #[module(skip)]
    device: Device,
}
impl Submodule12 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant574: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant575: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant110: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant111: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear65 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant576: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant577: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant578: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear66 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        let constant579: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant580: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant114: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant115: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear67 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear68 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear69 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let linear70 = LinearConfig::new(384, 384).with_bias(true).init(device);
        let constant607: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant608: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant120: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant121: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let linear71 = LinearConfig::new(384, 1536).with_bias(true).init(device);
        let constant609: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant610: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant611: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let linear72 = LinearConfig::new(1536, 384).with_bias(true).init(device);
        let constant612: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant613: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant124: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        let constant125: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([384], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [384].into(),
        );
        Self {
            constant574,
            constant575,
            constant110,
            constant111,
            linear65,
            constant576,
            constant577,
            constant578,
            linear66,
            constant579,
            constant580,
            constant114,
            constant115,
            linear67,
            linear68,
            linear69,
            linear70,
            constant607,
            constant608,
            constant120,
            constant121,
            linear71,
            constant609,
            constant610,
            constant611,
            linear72,
            constant612,
            constant613,
            constant124,
            constant125,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add86_out1: Tensor<3>, where2_out1: Tensor<4>) -> Tensor<3> {
        let reducemean43_out1 = { add86_out1.clone().mean_dim(2usize) };
        let sub23_out1 = add86_out1.sub(reducemean43_out1);
        let constant574_out1 = self.constant574.val();
        let pow22_out1 = sub23_out1
            .clone()
            .powf((constant574_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean44_out1 = { pow22_out1.mean_dim(2usize) };
        let constant575_out1 = self.constant575.val();
        let add87_out1 = reducemean44_out1
            .add((constant575_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt55_out1 = add87_out1.sqrt();
        let div43_out1 = sub23_out1.div(sqrt55_out1);
        let constant110_out1 = self.constant110.val();
        let mul65_out1 = div43_out1
            .mul((constant110_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant111_out1 = self.constant111.val();
        let add88_out1 = mul65_out1
            .add((constant111_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear65_out1 = self.linear65.forward(add88_out1.clone());
        let constant576_out1 = self.constant576.val();
        let div44_out1 = linear65_out1
            .clone()
            .div((constant576_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf11_out1 = div44_out1.erf();
        let constant577_out1 = self.constant577.val();
        let add89_out1 = erf11_out1
            .add((constant577_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul66_out1 = linear65_out1.mul(add89_out1);
        let constant578_out1 = self.constant578.val();
        let mul67_out1 = mul66_out1
            .mul((constant578_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear66_out1 = self.linear66.forward(mul67_out1);
        let add90_out1 = linear66_out1.add(add88_out1);
        let reducemean45_out1 = { add90_out1.clone().mean_dim(2usize) };
        let sub24_out1 = add90_out1.sub(reducemean45_out1);
        let constant579_out1 = self.constant579.val();
        let pow23_out1 = sub24_out1
            .clone()
            .powf((constant579_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean46_out1 = { pow23_out1.mean_dim(2usize) };
        let constant580_out1 = self.constant580.val();
        let add91_out1 = reducemean46_out1
            .add((constant580_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt56_out1 = add91_out1.sqrt();
        let div45_out1 = sub24_out1.div(sqrt56_out1);
        let constant114_out1 = self.constant114.val();
        let mul68_out1 = div45_out1
            .mul((constant114_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant115_out1 = self.constant115.val();
        let add92_out1 = mul68_out1
            .add((constant115_out1).unsqueeze_dims(&[0isize, 1isize]));
        let shape104_out1: [i64; 3] = {
            let axes = &add92_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather95_out1 = shape104_out1[0] as i64;
        let gather96_out1 = shape104_out1[1] as i64;
        let linear67_out1 = self.linear67.forward(add92_out1.clone());
        let shape106_out1: [i64; 3] = {
            let axes = &linear67_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather97_out1 = shape106_out1[0] as i64;
        let gather98_out1 = shape106_out1[1] as i64;
        let unsqueeze95_out1 = [gather97_out1 as i64];
        let unsqueeze96_out1 = [gather98_out1 as i64];
        let constant588_out1: [i64; 1] = [32i64];
        let constant587_out1: [i64; 1] = [12i64];
        let concat46_out1: [i64; 4usize] = [
            &unsqueeze95_out1[..],
            &unsqueeze96_out1[..],
            &constant587_out1[..],
            &constant588_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape46_out1 = linear67_out1.reshape(concat46_out1);
        let transpose45_out1 = reshape46_out1.permute([0, 2, 1, 3]);
        let linear68_out1 = self.linear68.forward(add92_out1.clone());
        let shape108_out1: [i64; 3] = {
            let axes = &linear68_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather99_out1 = shape108_out1[0] as i64;
        let gather100_out1 = shape108_out1[1] as i64;
        let unsqueeze97_out1 = [gather99_out1 as i64];
        let unsqueeze98_out1 = [gather100_out1 as i64];
        let constant594_out1: [i64; 1] = [32i64];
        let constant593_out1: [i64; 1] = [12i64];
        let concat47_out1: [i64; 4usize] = [
            &unsqueeze97_out1[..],
            &unsqueeze98_out1[..],
            &constant593_out1[..],
            &constant594_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape47_out1 = linear68_out1.reshape(concat47_out1);
        let linear69_out1 = self.linear69.forward(add92_out1.clone());
        let shape110_out1: [i64; 3] = {
            let axes = &linear69_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather101_out1 = shape110_out1[0] as i64;
        let gather102_out1 = shape110_out1[1] as i64;
        let unsqueeze99_out1 = [gather101_out1 as i64];
        let unsqueeze100_out1 = [gather102_out1 as i64];
        let constant600_out1: [i64; 1] = [32i64];
        let constant599_out1: [i64; 1] = [12i64];
        let concat48_out1: [i64; 4usize] = [
            &unsqueeze99_out1[..],
            &unsqueeze100_out1[..],
            &constant599_out1[..],
            &constant600_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape48_out1 = linear69_out1.reshape(concat48_out1);
        let transpose46_out1 = reshape48_out1.permute([0, 2, 1, 3]);
        let transpose47_out1 = reshape47_out1.permute([0, 2, 3, 1]);
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
                Some(where2_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: None,
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul93_out1,)
        };
        let transpose48_out1 = matmul93_out1.permute([0, 2, 1, 3]);
        let unsqueeze101_out1 = [gather95_out1 as i64];
        let unsqueeze102_out1 = [gather96_out1 as i64];
        let constant606_out1: [i64; 1] = [384i64];
        let concat49_out1: [i64; 3usize] = [
            &unsqueeze101_out1[..],
            &unsqueeze102_out1[..],
            &constant606_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape49_out1 = transpose48_out1.reshape(concat49_out1);
        let linear70_out1 = self.linear70.forward(reshape49_out1);
        let add94_out1 = linear70_out1.add(add92_out1);
        let reducemean47_out1 = { add94_out1.clone().mean_dim(2usize) };
        let sub25_out1 = add94_out1.sub(reducemean47_out1);
        let constant607_out1 = self.constant607.val();
        let pow24_out1 = sub25_out1
            .clone()
            .powf((constant607_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean48_out1 = { pow24_out1.mean_dim(2usize) };
        let constant608_out1 = self.constant608.val();
        let add95_out1 = reducemean48_out1
            .add((constant608_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt60_out1 = add95_out1.sqrt();
        let div47_out1 = sub25_out1.div(sqrt60_out1);
        let constant120_out1 = self.constant120.val();
        let mul71_out1 = div47_out1
            .mul((constant120_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant121_out1 = self.constant121.val();
        let add96_out1 = mul71_out1
            .add((constant121_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear71_out1 = self.linear71.forward(add96_out1.clone());
        let constant609_out1 = self.constant609.val();
        let div48_out1 = linear71_out1
            .clone()
            .div((constant609_out1).unsqueeze_dims(&[0isize, 1isize]));
        let erf12_out1 = div48_out1.erf();
        let constant610_out1 = self.constant610.val();
        let add97_out1 = erf12_out1
            .add((constant610_out1).unsqueeze_dims(&[0isize, 1isize]));
        let mul72_out1 = linear71_out1.mul(add97_out1);
        let constant611_out1 = self.constant611.val();
        let mul73_out1 = mul72_out1
            .mul((constant611_out1).unsqueeze_dims(&[0isize, 1isize]));
        let linear72_out1 = self.linear72.forward(mul73_out1);
        let add98_out1 = linear72_out1.add(add96_out1);
        let reducemean49_out1 = { add98_out1.clone().mean_dim(2usize) };
        let sub26_out1 = add98_out1.sub(reducemean49_out1);
        let constant612_out1 = self.constant612.val();
        let pow25_out1 = sub26_out1
            .clone()
            .powf((constant612_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean50_out1 = { pow25_out1.mean_dim(2usize) };
        let constant613_out1 = self.constant613.val();
        let add99_out1 = reducemean50_out1
            .add((constant613_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt61_out1 = add99_out1.sqrt();
        let div49_out1 = sub26_out1.div(sqrt61_out1);
        let constant124_out1 = self.constant124.val();
        let mul74_out1 = div49_out1
            .mul((constant124_out1).unsqueeze_dims(&[0isize, 1isize]));
        let constant125_out1 = self.constant125.val();
        let add100_out1 = mul74_out1
            .add((constant125_out1).unsqueeze_dims(&[0isize, 1isize]));
        add100_out1
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
            device: device.clone(),
        }
    }

    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        input_ids: Tensor<2, Int>,
        attention_mask: Tensor<2, Int>,
        token_type_ids: Tensor<2, Int>,
    ) -> Tensor<3> {
        let (add6_out1, where2_out1) = self
            .submodule1
            .forward(input_ids, token_type_ids, attention_mask);
        let add14_out1 = self.submodule2.forward(add6_out1, where2_out1.clone());
        let add22_out1 = self.submodule3.forward(add14_out1, where2_out1.clone());
        let add30_out1 = self.submodule4.forward(add22_out1, where2_out1.clone());
        let add38_out1 = self.submodule5.forward(add30_out1, where2_out1.clone());
        let add46_out1 = self.submodule6.forward(add38_out1, where2_out1.clone());
        let add54_out1 = self.submodule7.forward(add46_out1, where2_out1.clone());
        let add62_out1 = self.submodule8.forward(add54_out1, where2_out1.clone());
        let add70_out1 = self.submodule9.forward(add62_out1, where2_out1.clone());
        let add78_out1 = self.submodule10.forward(add70_out1, where2_out1.clone());
        let add86_out1 = self.submodule11.forward(add78_out1, where2_out1.clone());
        let add100_out1 = self.submodule12.forward(add86_out1, where2_out1);
        add100_out1
    }
}
