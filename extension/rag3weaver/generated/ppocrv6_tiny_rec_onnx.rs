// Generated from ONNX "PaddlePaddle/PP-OCRv6_tiny_rec_onnx inference.onnx" by burn-onnx 0.22.0-pre.1
use burn::prelude::*;
use burn::nn::BatchNorm;
use burn::nn::BatchNormConfig;
use burn::nn::Linear;
use burn::nn::LinearConfig;
use burn::nn::PaddingConfig2d;
use burn::nn::conv::Conv2d;
use burn::nn::conv::Conv2dConfig;
use burn::nn::pool::AvgPool2d;
use burn::nn::pool::AvgPool2dConfig;
use burn::tensor::Bytes;
use burn_store::BurnpackStore;
use burn_store::ModuleSnapshot;


#[derive(Module, Debug)]
pub struct Submodule1 {
    conv2d1: Conv2d,
    batchnormalization1: BatchNorm,
    constant6: burn::module::Param<Tensor<1>>,
    constant7: burn::module::Param<Tensor<1>>,
    constant8: burn::module::Param<Tensor<1>>,
    conv2d2: Conv2d,
    batchnormalization2: BatchNorm,
    conv2d3: Conv2d,
    constant15: burn::module::Param<Tensor<4>>,
    conv2d4: Conv2d,
    constant17: burn::module::Param<Tensor<4>>,
    conv2d5: Conv2d,
    constant19: burn::module::Param<Tensor<4>>,
    conv2d6: Conv2d,
    constant21: burn::module::Param<Tensor<4>>,
    constant22: burn::module::Param<Tensor<1>>,
    constant23: burn::module::Param<Tensor<1>>,
    constant24: burn::module::Param<Tensor<1>>,
    conv2d7: Conv2d,
    constant26: burn::module::Param<Tensor<4>>,
    conv2d8: Conv2d,
    constant28: burn::module::Param<Tensor<4>>,
    conv2d9: Conv2d,
    constant30: burn::module::Param<Tensor<4>>,
    constant31: burn::module::Param<Tensor<1>>,
    constant32: burn::module::Param<Tensor<1>>,
    constant33: burn::module::Param<Tensor<1>>,
    conv2d10: Conv2d,
    constant35: burn::module::Param<Tensor<4>>,
    conv2d11: Conv2d,
    constant37: burn::module::Param<Tensor<4>>,
    conv2d12: Conv2d,
    constant39: burn::module::Param<Tensor<4>>,
    #[module(skip)]
    device: Device,
}
impl Submodule1 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let conv2d1 = Conv2dConfig::new([3, 24], [3, 3])
            .with_stride([2, 2])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let batchnormalization1 = BatchNormConfig::new(24)
            .with_epsilon(0.000009999999747378752f64)
            .with_momentum(0.8999999761581421f64)
            .init(device);
        let constant6: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant7: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant8: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d2 = Conv2dConfig::new([24, 48], [3, 3])
            .with_stride([2, 2])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let batchnormalization2 = BatchNormConfig::new(48)
            .with_epsilon(0.000009999999747378752f64)
            .with_momentum(0.8999999761581421f64)
            .init(device);
        let conv2d3 = Conv2dConfig::new([48, 48], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(48)
            .with_bias(false)
            .init(device);
        let constant15: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 48, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 48, 1, 1].into(),
        );
        let conv2d4 = Conv2dConfig::new([48, 12], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant17: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 12, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 12, 1, 1].into(),
        );
        let conv2d5 = Conv2dConfig::new([12, 48], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant19: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 48, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 48, 1, 1].into(),
        );
        let conv2d6 = Conv2dConfig::new([48, 96], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant21: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 96, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 96, 1, 1].into(),
        );
        let constant22: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant23: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant24: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d7 = Conv2dConfig::new([96, 48], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant26: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 48, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 48, 1, 1].into(),
        );
        let conv2d8 = Conv2dConfig::new([48, 48], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(48)
            .with_bias(false)
            .init(device);
        let constant28: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 48, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 48, 1, 1].into(),
        );
        let conv2d9 = Conv2dConfig::new([48, 96], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant30: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 96, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 96, 1, 1].into(),
        );
        let constant31: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant32: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant33: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d10 = Conv2dConfig::new([96, 48], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant35: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 48, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 48, 1, 1].into(),
        );
        let conv2d11 = Conv2dConfig::new([48, 48], [3, 3])
            .with_stride([2, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(48)
            .with_bias(false)
            .init(device);
        let constant37: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 48, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 48, 1, 1].into(),
        );
        let conv2d12 = Conv2dConfig::new([48, 96], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant39: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 96, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 96, 1, 1].into(),
        );
        Self {
            conv2d1,
            batchnormalization1,
            constant6,
            constant7,
            constant8,
            conv2d2,
            batchnormalization2,
            conv2d3,
            constant15,
            conv2d4,
            constant17,
            conv2d5,
            constant19,
            conv2d6,
            constant21,
            constant22,
            constant23,
            constant24,
            conv2d7,
            constant26,
            conv2d8,
            constant28,
            conv2d9,
            constant30,
            constant31,
            constant32,
            constant33,
            conv2d10,
            constant35,
            conv2d11,
            constant37,
            conv2d12,
            constant39,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, x: Tensor<4>) -> Tensor<4> {
        let conv2d1_out1 = self.conv2d1.forward(x);
        let batchnormalization1_out1 = self.batchnormalization1.forward(conv2d1_out1);
        let constant6_out1 = self.constant6.val();
        let div1_out1 = batchnormalization1_out1
            .clone()
            .div((constant6_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf1_out1 = div1_out1.erf();
        let constant7_out1 = self.constant7.val();
        let add1_out1 = erf1_out1
            .add((constant7_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul1_out1 = batchnormalization1_out1.mul(add1_out1);
        let constant8_out1 = self.constant8.val();
        let mul2_out1 = mul1_out1
            .mul((constant8_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d2_out1 = self.conv2d2.forward(mul2_out1);
        let batchnormalization2_out1 = self.batchnormalization2.forward(conv2d2_out1);
        let conv2d3_out1 = self.conv2d3.forward(batchnormalization2_out1);
        let constant15_out1 = self.constant15.val();
        let add2_out1 = conv2d3_out1.add(constant15_out1);
        let reducemean1_out1 = { add2_out1.clone().mean_dim(2usize).mean_dim(3usize) };
        let conv2d4_out1 = self.conv2d4.forward(reducemean1_out1);
        let constant17_out1 = self.constant17.val();
        let add3_out1 = conv2d4_out1.add(constant17_out1);
        let relu1_out1 = burn::tensor::activation::relu(add3_out1);
        let conv2d5_out1 = self.conv2d5.forward(relu1_out1);
        let constant19_out1 = self.constant19.val();
        let add4_out1 = conv2d5_out1.add(constant19_out1);
        let hardsigmoid1_out1 = burn::tensor::activation::hard_sigmoid(
            add4_out1,
            0.16666670143604279,
            0.5,
        );
        let mul3_out1 = add2_out1.mul(hardsigmoid1_out1);
        let conv2d6_out1 = self.conv2d6.forward(mul3_out1.clone());
        let constant21_out1 = self.constant21.val();
        let add5_out1 = conv2d6_out1.add(constant21_out1);
        let constant22_out1 = self.constant22.val();
        let div2_out1 = add5_out1
            .clone()
            .div((constant22_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf2_out1 = div2_out1.erf();
        let constant23_out1 = self.constant23.val();
        let add6_out1 = erf2_out1
            .add((constant23_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul4_out1 = add5_out1.mul(add6_out1);
        let constant24_out1 = self.constant24.val();
        let mul5_out1 = mul4_out1
            .mul((constant24_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d7_out1 = self.conv2d7.forward(mul5_out1);
        let constant26_out1 = self.constant26.val();
        let add7_out1 = conv2d7_out1.add(constant26_out1);
        let add8_out1 = mul3_out1.add(add7_out1);
        let conv2d8_out1 = self.conv2d8.forward(add8_out1);
        let constant28_out1 = self.constant28.val();
        let add9_out1 = conv2d8_out1.add(constant28_out1);
        let conv2d9_out1 = self.conv2d9.forward(add9_out1.clone());
        let constant30_out1 = self.constant30.val();
        let add10_out1 = conv2d9_out1.add(constant30_out1);
        let constant31_out1 = self.constant31.val();
        let div3_out1 = add10_out1
            .clone()
            .div((constant31_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf3_out1 = div3_out1.erf();
        let constant32_out1 = self.constant32.val();
        let add11_out1 = erf3_out1
            .add((constant32_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul6_out1 = add10_out1.mul(add11_out1);
        let constant33_out1 = self.constant33.val();
        let mul7_out1 = mul6_out1
            .mul((constant33_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d10_out1 = self.conv2d10.forward(mul7_out1);
        let constant35_out1 = self.constant35.val();
        let add12_out1 = conv2d10_out1.add(constant35_out1);
        let add13_out1 = add9_out1.add(add12_out1);
        let conv2d11_out1 = self.conv2d11.forward(add13_out1);
        let constant37_out1 = self.constant37.val();
        let add14_out1 = conv2d11_out1.add(constant37_out1);
        let conv2d12_out1 = self.conv2d12.forward(add14_out1);
        let constant39_out1 = self.constant39.val();
        let add15_out1 = conv2d12_out1.add(constant39_out1);
        add15_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule2 {
    constant40: burn::module::Param<Tensor<1>>,
    constant41: burn::module::Param<Tensor<1>>,
    constant42: burn::module::Param<Tensor<1>>,
    conv2d13: Conv2d,
    constant44: burn::module::Param<Tensor<4>>,
    conv2d14: Conv2d,
    constant46: burn::module::Param<Tensor<4>>,
    conv2d15: Conv2d,
    constant48: burn::module::Param<Tensor<4>>,
    conv2d16: Conv2d,
    constant50: burn::module::Param<Tensor<4>>,
    conv2d17: Conv2d,
    constant52: burn::module::Param<Tensor<4>>,
    constant53: burn::module::Param<Tensor<1>>,
    constant54: burn::module::Param<Tensor<1>>,
    constant55: burn::module::Param<Tensor<1>>,
    conv2d18: Conv2d,
    constant57: burn::module::Param<Tensor<4>>,
    conv2d19: Conv2d,
    constant59: burn::module::Param<Tensor<4>>,
    conv2d20: Conv2d,
    constant61: burn::module::Param<Tensor<4>>,
    constant62: burn::module::Param<Tensor<1>>,
    constant63: burn::module::Param<Tensor<1>>,
    constant64: burn::module::Param<Tensor<1>>,
    conv2d21: Conv2d,
    constant66: burn::module::Param<Tensor<4>>,
    conv2d22: Conv2d,
    constant68: burn::module::Param<Tensor<4>>,
    conv2d23: Conv2d,
    constant70: burn::module::Param<Tensor<4>>,
    constant71: burn::module::Param<Tensor<1>>,
    constant72: burn::module::Param<Tensor<1>>,
    #[module(skip)]
    device: Device,
}
impl Submodule2 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant40: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant41: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant42: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d13 = Conv2dConfig::new([96, 96], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant44: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 96, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 96, 1, 1].into(),
        );
        let conv2d14 = Conv2dConfig::new([96, 96], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(96)
            .with_bias(false)
            .init(device);
        let constant46: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 96, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 96, 1, 1].into(),
        );
        let conv2d15 = Conv2dConfig::new([96, 24], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant48: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 24, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 24, 1, 1].into(),
        );
        let conv2d16 = Conv2dConfig::new([24, 96], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant50: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 96, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 96, 1, 1].into(),
        );
        let conv2d17 = Conv2dConfig::new([96, 192], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant52: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 192, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 192, 1, 1].into(),
        );
        let constant53: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant54: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant55: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d18 = Conv2dConfig::new([192, 96], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant57: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 96, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 96, 1, 1].into(),
        );
        let conv2d19 = Conv2dConfig::new([96, 96], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(96)
            .with_bias(false)
            .init(device);
        let constant59: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 96, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 96, 1, 1].into(),
        );
        let conv2d20 = Conv2dConfig::new([96, 192], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant61: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 192, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 192, 1, 1].into(),
        );
        let constant62: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant63: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant64: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d21 = Conv2dConfig::new([192, 96], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant66: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 96, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 96, 1, 1].into(),
        );
        let conv2d22 = Conv2dConfig::new([96, 96], [3, 3])
            .with_stride([2, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(96)
            .with_bias(false)
            .init(device);
        let constant68: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 96, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 96, 1, 1].into(),
        );
        let conv2d23 = Conv2dConfig::new([96, 192], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant70: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 192, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 192, 1, 1].into(),
        );
        let constant71: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant72: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
            constant40,
            constant41,
            constant42,
            conv2d13,
            constant44,
            conv2d14,
            constant46,
            conv2d15,
            constant48,
            conv2d16,
            constant50,
            conv2d17,
            constant52,
            constant53,
            constant54,
            constant55,
            conv2d18,
            constant57,
            conv2d19,
            constant59,
            conv2d20,
            constant61,
            constant62,
            constant63,
            constant64,
            conv2d21,
            constant66,
            conv2d22,
            constant68,
            conv2d23,
            constant70,
            constant71,
            constant72,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, add15_out1: Tensor<4>) -> Tensor<4> {
        let constant40_out1 = self.constant40.val();
        let div4_out1 = add15_out1
            .clone()
            .div((constant40_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf4_out1 = div4_out1.erf();
        let constant41_out1 = self.constant41.val();
        let add16_out1 = erf4_out1
            .add((constant41_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul8_out1 = add15_out1.mul(add16_out1);
        let constant42_out1 = self.constant42.val();
        let mul9_out1 = mul8_out1
            .mul((constant42_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d13_out1 = self.conv2d13.forward(mul9_out1);
        let constant44_out1 = self.constant44.val();
        let add17_out1 = conv2d13_out1.add(constant44_out1);
        let conv2d14_out1 = self.conv2d14.forward(add17_out1);
        let constant46_out1 = self.constant46.val();
        let add18_out1 = conv2d14_out1.add(constant46_out1);
        let reducemean2_out1 = { add18_out1.clone().mean_dim(2usize).mean_dim(3usize) };
        let conv2d15_out1 = self.conv2d15.forward(reducemean2_out1);
        let constant48_out1 = self.constant48.val();
        let add19_out1 = conv2d15_out1.add(constant48_out1);
        let relu2_out1 = burn::tensor::activation::relu(add19_out1);
        let conv2d16_out1 = self.conv2d16.forward(relu2_out1);
        let constant50_out1 = self.constant50.val();
        let add20_out1 = conv2d16_out1.add(constant50_out1);
        let hardsigmoid2_out1 = burn::tensor::activation::hard_sigmoid(
            add20_out1,
            0.16666670143604279,
            0.5,
        );
        let mul10_out1 = add18_out1.mul(hardsigmoid2_out1);
        let conv2d17_out1 = self.conv2d17.forward(mul10_out1.clone());
        let constant52_out1 = self.constant52.val();
        let add21_out1 = conv2d17_out1.add(constant52_out1);
        let constant53_out1 = self.constant53.val();
        let div5_out1 = add21_out1
            .clone()
            .div((constant53_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf5_out1 = div5_out1.erf();
        let constant54_out1 = self.constant54.val();
        let add22_out1 = erf5_out1
            .add((constant54_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul11_out1 = add21_out1.mul(add22_out1);
        let constant55_out1 = self.constant55.val();
        let mul12_out1 = mul11_out1
            .mul((constant55_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d18_out1 = self.conv2d18.forward(mul12_out1);
        let constant57_out1 = self.constant57.val();
        let add23_out1 = conv2d18_out1.add(constant57_out1);
        let add24_out1 = mul10_out1.add(add23_out1);
        let conv2d19_out1 = self.conv2d19.forward(add24_out1);
        let constant59_out1 = self.constant59.val();
        let add25_out1 = conv2d19_out1.add(constant59_out1);
        let conv2d20_out1 = self.conv2d20.forward(add25_out1.clone());
        let constant61_out1 = self.constant61.val();
        let add26_out1 = conv2d20_out1.add(constant61_out1);
        let constant62_out1 = self.constant62.val();
        let div6_out1 = add26_out1
            .clone()
            .div((constant62_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf6_out1 = div6_out1.erf();
        let constant63_out1 = self.constant63.val();
        let add27_out1 = erf6_out1
            .add((constant63_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul13_out1 = add26_out1.mul(add27_out1);
        let constant64_out1 = self.constant64.val();
        let mul14_out1 = mul13_out1
            .mul((constant64_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d21_out1 = self.conv2d21.forward(mul14_out1);
        let constant66_out1 = self.constant66.val();
        let add28_out1 = conv2d21_out1.add(constant66_out1);
        let add29_out1 = add25_out1.add(add28_out1);
        let conv2d22_out1 = self.conv2d22.forward(add29_out1);
        let constant68_out1 = self.constant68.val();
        let add30_out1 = conv2d22_out1.add(constant68_out1);
        let conv2d23_out1 = self.conv2d23.forward(add30_out1);
        let constant70_out1 = self.constant70.val();
        let add31_out1 = conv2d23_out1.add(constant70_out1);
        let constant71_out1 = self.constant71.val();
        let div7_out1 = add31_out1
            .clone()
            .div((constant71_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf7_out1 = div7_out1.erf();
        let constant72_out1 = self.constant72.val();
        let add32_out1 = erf7_out1
            .add((constant72_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul15_out1 = add31_out1.mul(add32_out1);
        mul15_out1
    }
}
#[derive(Module, Debug)]
pub struct Submodule3 {
    constant73: burn::module::Param<Tensor<1>>,
    conv2d24: Conv2d,
    constant75: burn::module::Param<Tensor<4>>,
    conv2d25: Conv2d,
    constant77: burn::module::Param<Tensor<4>>,
    conv2d26: Conv2d,
    constant79: burn::module::Param<Tensor<4>>,
    conv2d27: Conv2d,
    constant81: burn::module::Param<Tensor<4>>,
    conv2d28: Conv2d,
    constant83: burn::module::Param<Tensor<4>>,
    constant84: burn::module::Param<Tensor<1>>,
    constant85: burn::module::Param<Tensor<1>>,
    constant86: burn::module::Param<Tensor<1>>,
    conv2d29: Conv2d,
    constant88: burn::module::Param<Tensor<4>>,
    conv2d30: Conv2d,
    constant90: burn::module::Param<Tensor<4>>,
    conv2d31: Conv2d,
    constant92: burn::module::Param<Tensor<4>>,
    constant93: burn::module::Param<Tensor<1>>,
    constant94: burn::module::Param<Tensor<1>>,
    constant95: burn::module::Param<Tensor<1>>,
    conv2d32: Conv2d,
    constant97: burn::module::Param<Tensor<4>>,
    conv2d33: Conv2d,
    constant99: burn::module::Param<Tensor<4>>,
    conv2d34: Conv2d,
    constant101: burn::module::Param<Tensor<4>>,
    constant102: burn::module::Param<Tensor<1>>,
    constant103: burn::module::Param<Tensor<1>>,
    constant104: burn::module::Param<Tensor<1>>,
    conv2d35: Conv2d,
    constant106: burn::module::Param<Tensor<4>>,
    averagepool2d1: AvgPool2d,
    conv2d36: Conv2d,
    batchnormalization3: BatchNorm,
    conv2d37: Conv2d,
    batchnormalization4: BatchNorm,
    linear1: Linear,
    linear2: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule3 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant73: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d24 = Conv2dConfig::new([192, 160], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant75: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 160, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 160, 1, 1].into(),
        );
        let conv2d25 = Conv2dConfig::new([160, 160], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(160)
            .with_bias(false)
            .init(device);
        let constant77: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 160, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 160, 1, 1].into(),
        );
        let conv2d26 = Conv2dConfig::new([160, 40], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant79: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 40, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 40, 1, 1].into(),
        );
        let conv2d27 = Conv2dConfig::new([40, 160], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant81: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 160, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 160, 1, 1].into(),
        );
        let conv2d28 = Conv2dConfig::new([160, 320], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant83: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 320, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 320, 1, 1].into(),
        );
        let constant84: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant85: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant86: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d29 = Conv2dConfig::new([320, 160], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant88: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 160, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 160, 1, 1].into(),
        );
        let conv2d30 = Conv2dConfig::new([160, 160], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(160)
            .with_bias(false)
            .init(device);
        let constant90: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 160, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 160, 1, 1].into(),
        );
        let conv2d31 = Conv2dConfig::new([160, 320], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant92: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 320, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 320, 1, 1].into(),
        );
        let constant93: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant94: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant95: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d32 = Conv2dConfig::new([320, 160], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant97: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 160, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 160, 1, 1].into(),
        );
        let conv2d33 = Conv2dConfig::new([160, 160], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(160)
            .with_bias(false)
            .init(device);
        let constant99: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 160, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 160, 1, 1].into(),
        );
        let conv2d34 = Conv2dConfig::new([160, 320], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant101: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 320, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 320, 1, 1].into(),
        );
        let constant102: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant103: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant104: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d35 = Conv2dConfig::new([320, 160], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant106: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 160, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 160, 1, 1].into(),
        );
        let averagepool2d1 = AvgPool2dConfig::new([3, 2])
            .with_strides([3, 2])
            .with_padding(PaddingConfig2d::Valid)
            .with_count_include_pad(false)
            .with_ceil_mode(false)
            .init();
        let conv2d36 = Conv2dConfig::new([160, 160], [1, 5])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(0, 2, 0, 2))
            .with_dilation([1, 1])
            .with_groups(160)
            .with_bias(false)
            .init(device);
        let batchnormalization3 = BatchNormConfig::new(160)
            .with_epsilon(0.000009999999747378752f64)
            .with_momentum(0.8999999761581421f64)
            .init(device);
        let conv2d37 = Conv2dConfig::new([160, 160], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let batchnormalization4 = BatchNormConfig::new(160)
            .with_epsilon(0.000009999999747378752f64)
            .with_momentum(0.8999999761581421f64)
            .init(device);
        let linear1 = LinearConfig::new(160, 80).with_bias(true).init(device);
        let linear2 = LinearConfig::new(80, 6906).with_bias(true).init(device);
        Self {
            constant73,
            conv2d24,
            constant75,
            conv2d25,
            constant77,
            conv2d26,
            constant79,
            conv2d27,
            constant81,
            conv2d28,
            constant83,
            constant84,
            constant85,
            constant86,
            conv2d29,
            constant88,
            conv2d30,
            constant90,
            conv2d31,
            constant92,
            constant93,
            constant94,
            constant95,
            conv2d32,
            constant97,
            conv2d33,
            constant99,
            conv2d34,
            constant101,
            constant102,
            constant103,
            constant104,
            conv2d35,
            constant106,
            averagepool2d1,
            conv2d36,
            batchnormalization3,
            conv2d37,
            batchnormalization4,
            linear1,
            linear2,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, mul15_out1: Tensor<4>) -> Tensor<3> {
        let constant73_out1 = self.constant73.val();
        let mul16_out1 = mul15_out1
            .mul((constant73_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d24_out1 = self.conv2d24.forward(mul16_out1);
        let constant75_out1 = self.constant75.val();
        let add33_out1 = conv2d24_out1.add(constant75_out1);
        let conv2d25_out1 = self.conv2d25.forward(add33_out1);
        let constant77_out1 = self.constant77.val();
        let add34_out1 = conv2d25_out1.add(constant77_out1);
        let reducemean3_out1 = { add34_out1.clone().mean_dim(2usize).mean_dim(3usize) };
        let conv2d26_out1 = self.conv2d26.forward(reducemean3_out1);
        let constant79_out1 = self.constant79.val();
        let add35_out1 = conv2d26_out1.add(constant79_out1);
        let relu3_out1 = burn::tensor::activation::relu(add35_out1);
        let conv2d27_out1 = self.conv2d27.forward(relu3_out1);
        let constant81_out1 = self.constant81.val();
        let add36_out1 = conv2d27_out1.add(constant81_out1);
        let hardsigmoid3_out1 = burn::tensor::activation::hard_sigmoid(
            add36_out1,
            0.16666670143604279,
            0.5,
        );
        let mul17_out1 = add34_out1.mul(hardsigmoid3_out1);
        let conv2d28_out1 = self.conv2d28.forward(mul17_out1.clone());
        let constant83_out1 = self.constant83.val();
        let add37_out1 = conv2d28_out1.add(constant83_out1);
        let constant84_out1 = self.constant84.val();
        let div8_out1 = add37_out1
            .clone()
            .div((constant84_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf8_out1 = div8_out1.erf();
        let constant85_out1 = self.constant85.val();
        let add38_out1 = erf8_out1
            .add((constant85_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul18_out1 = add37_out1.mul(add38_out1);
        let constant86_out1 = self.constant86.val();
        let mul19_out1 = mul18_out1
            .mul((constant86_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d29_out1 = self.conv2d29.forward(mul19_out1);
        let constant88_out1 = self.constant88.val();
        let add39_out1 = conv2d29_out1.add(constant88_out1);
        let add40_out1 = mul17_out1.add(add39_out1);
        let conv2d30_out1 = self.conv2d30.forward(add40_out1);
        let constant90_out1 = self.constant90.val();
        let add41_out1 = conv2d30_out1.add(constant90_out1);
        let conv2d31_out1 = self.conv2d31.forward(add41_out1.clone());
        let constant92_out1 = self.constant92.val();
        let add42_out1 = conv2d31_out1.add(constant92_out1);
        let constant93_out1 = self.constant93.val();
        let div9_out1 = add42_out1
            .clone()
            .div((constant93_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf9_out1 = div9_out1.erf();
        let constant94_out1 = self.constant94.val();
        let add43_out1 = erf9_out1
            .add((constant94_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul20_out1 = add42_out1.mul(add43_out1);
        let constant95_out1 = self.constant95.val();
        let mul21_out1 = mul20_out1
            .mul((constant95_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d32_out1 = self.conv2d32.forward(mul21_out1);
        let constant97_out1 = self.constant97.val();
        let add44_out1 = conv2d32_out1.add(constant97_out1);
        let add45_out1 = add41_out1.add(add44_out1);
        let conv2d33_out1 = self.conv2d33.forward(add45_out1);
        let constant99_out1 = self.constant99.val();
        let add46_out1 = conv2d33_out1.add(constant99_out1);
        let conv2d34_out1 = self.conv2d34.forward(add46_out1.clone());
        let constant101_out1 = self.constant101.val();
        let add47_out1 = conv2d34_out1.add(constant101_out1);
        let constant102_out1 = self.constant102.val();
        let div10_out1 = add47_out1
            .clone()
            .div((constant102_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf10_out1 = div10_out1.erf();
        let constant103_out1 = self.constant103.val();
        let add48_out1 = erf10_out1
            .add((constant103_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul22_out1 = add47_out1.mul(add48_out1);
        let constant104_out1 = self.constant104.val();
        let mul23_out1 = mul22_out1
            .mul((constant104_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d35_out1 = self.conv2d35.forward(mul23_out1);
        let constant106_out1 = self.constant106.val();
        let add49_out1 = conv2d35_out1.add(constant106_out1);
        let add50_out1 = add46_out1.add(add49_out1);
        let averagepool2d1_out1 = self.averagepool2d1.forward(add50_out1);
        let squeeze1_out1 = averagepool2d1_out1.squeeze_dims::<3>(&[2]);
        let transpose1_out1 = squeeze1_out1.permute([0, 2, 1]);
        let transpose2_out1 = transpose1_out1.permute([0, 2, 1]);
        let unsqueeze1_out1: Tensor<4> = transpose2_out1.unsqueeze_dims::<4>(&[2]);
        let conv2d36_out1 = self.conv2d36.forward(unsqueeze1_out1);
        let squeeze2_out1 = conv2d36_out1.squeeze_dims::<3>(&[2]);
        let batchnormalization3_out1 = self.batchnormalization3.forward(squeeze2_out1);
        let hardsigmoid4_out1 = burn::tensor::activation::hard_sigmoid(
            batchnormalization3_out1.clone(),
            0.1666666716337204,
            0.5,
        );
        let mul24_out1 = hardsigmoid4_out1.mul(batchnormalization3_out1);
        let unsqueeze2_out1: Tensor<4> = mul24_out1.unsqueeze_dims::<4>(&[2]);
        let conv2d37_out1 = self.conv2d37.forward(unsqueeze2_out1);
        let squeeze3_out1 = conv2d37_out1.squeeze_dims::<3>(&[2]);
        let batchnormalization4_out1 = self.batchnormalization4.forward(squeeze3_out1);
        let hardsigmoid5_out1 = burn::tensor::activation::hard_sigmoid(
            batchnormalization4_out1.clone(),
            0.1666666716337204,
            0.5,
        );
        let mul25_out1 = hardsigmoid5_out1.mul(batchnormalization4_out1);
        let transpose3_out1 = mul25_out1.permute([0, 2, 1]);
        let linear1_out1 = self.linear1.forward(transpose3_out1);
        let linear2_out1 = self.linear2.forward(linear1_out1);
        let softmax1_out1 = burn::tensor::activation::softmax(linear2_out1, 2);
        softmax1_out1
    }
}

#[derive(Module, Debug)]
pub struct Model {
    submodule1: Submodule1,
    submodule2: Submodule2,
    submodule3: Submodule3,
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
        Self {
            submodule1,
            submodule2,
            submodule3,
            device: device.clone(),
        }
    }

    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, x: Tensor<4>) -> Tensor<3> {
        let add15_out1 = self.submodule1.forward(x);
        let mul15_out1 = self.submodule2.forward(add15_out1);
        let softmax1_out1 = self.submodule3.forward(mul15_out1);
        softmax1_out1
    }
}
