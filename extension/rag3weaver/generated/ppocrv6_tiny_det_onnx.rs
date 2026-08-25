// Generated from ONNX "PaddlePaddle/PP-OCRv6_tiny_det_onnx inference.onnx" (auto_pad SAME_UPPER rewritten to explicit pads) by burn-onnx 0.22.0-pre.1
use burn::prelude::*;
use burn::nn::PaddingConfig2d;
use burn::nn::conv::Conv2d;
use burn::nn::conv::Conv2dConfig;
use burn::nn::conv::ConvTranspose2d;
use burn::nn::conv::ConvTranspose2dConfig;
use burn::nn::pool::AdaptiveAvgPool2d;
use burn::nn::pool::AdaptiveAvgPool2dConfig;
use burn::nn::pool::MaxPool2d;
use burn::nn::pool::MaxPool2dConfig;
use burn::tensor::Bytes;
use burn_store::BurnpackStore;
use burn_store::ModuleSnapshot;


#[derive(Module, Debug)]
pub struct Submodule1 {
    conv2d1: Conv2d,
    conv2d2: Conv2d,
    conv2d3: Conv2d,
    maxpool2d1: MaxPool2d,
    conv2d4: Conv2d,
    conv2d5: Conv2d,
    conv2d6: Conv2d,
    conv2d7: Conv2d,
    conv2d8: Conv2d,
    conv2d9: Conv2d,
    constant10: burn::module::Param<Tensor<1>>,
    constant11: burn::module::Param<Tensor<1>>,
    constant12: burn::module::Param<Tensor<1>>,
    conv2d10: Conv2d,
    conv2d11: Conv2d,
    conv2d12: Conv2d,
    conv2d13: Conv2d,
    conv2d14: Conv2d,
    conv2d15: Conv2d,
    conv2d16: Conv2d,
    conv2d17: Conv2d,
    conv2d18: Conv2d,
    conv2d19: Conv2d,
    conv2d20: Conv2d,
    conv2d21: Conv2d,
    conv2d22: Conv2d,
    conv2d23: Conv2d,
    conv2d24: Conv2d,
    conv2d25: Conv2d,
    conv2d26: Conv2d,
    conv2d27: Conv2d,
    conv2d28: Conv2d,
    conv2d29: Conv2d,
    conv2d30: Conv2d,
    conv2d31: Conv2d,
    conv2d32: Conv2d,
    conv2d33: Conv2d,
    conv2d34: Conv2d,
    conv2d35: Conv2d,
    conv2d36: Conv2d,
    conv2d37: Conv2d,
    conv2d38: Conv2d,
    conv2d39: Conv2d,
    conv2d40: Conv2d,
    conv2d41: Conv2d,
    conv2d42: Conv2d,
    conv2d43: Conv2d,
    conv2d44: Conv2d,
    conv2d45: Conv2d,
    conv2d46: Conv2d,
    conv2d47: Conv2d,
    conv2d48: Conv2d,
    conv2d49: Conv2d,
    conv2d50: Conv2d,
    conv2d51: Conv2d,
    conv2d52: Conv2d,
    conv2d53: Conv2d,
    conv2d54: Conv2d,
    #[module(skip)]
    device: Device,
}
impl Submodule1 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let conv2d1 = Conv2dConfig::new([3, 16], [3, 3])
            .with_stride([2, 2])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d2 = Conv2dConfig::new([16, 8], [2, 2])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(0, 0, 1, 1))
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d3 = Conv2dConfig::new([8, 16], [2, 2])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(0, 0, 1, 1))
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let maxpool2d1 = MaxPool2dConfig::new([2, 2])
            .with_strides([1, 1])
            .with_padding(PaddingConfig2d::Explicit(0, 0, 1, 1))
            .with_dilation([1, 1])
            .with_ceil_mode(false)
            .init();
        let conv2d4 = Conv2dConfig::new([32, 16], [3, 3])
            .with_stride([2, 2])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d5 = Conv2dConfig::new([16, 32], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d6 = Conv2dConfig::new([32, 32], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(32)
            .with_bias(true)
            .init(device);
        let conv2d7 = Conv2dConfig::new([32, 8], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d8 = Conv2dConfig::new([8, 32], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d9 = Conv2dConfig::new([32, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let constant10: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant11: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant12: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let conv2d10 = Conv2dConfig::new([64, 32], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d11 = Conv2dConfig::new([32, 32], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(32)
            .with_bias(true)
            .init(device);
        let conv2d12 = Conv2dConfig::new([32, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d13 = Conv2dConfig::new([64, 32], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d14 = Conv2dConfig::new([32, 32], [3, 3])
            .with_stride([2, 2])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(32)
            .with_bias(true)
            .init(device);
        let conv2d15 = Conv2dConfig::new([32, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d16 = Conv2dConfig::new([64, 48], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d17 = Conv2dConfig::new([48, 48], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(48)
            .with_bias(true)
            .init(device);
        let conv2d18 = Conv2dConfig::new([48, 12], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d19 = Conv2dConfig::new([12, 48], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d20 = Conv2dConfig::new([48, 96], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d21 = Conv2dConfig::new([96, 48], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d22 = Conv2dConfig::new([48, 48], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(48)
            .with_bias(true)
            .init(device);
        let conv2d23 = Conv2dConfig::new([48, 96], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d24 = Conv2dConfig::new([96, 48], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d25 = Conv2dConfig::new([48, 48], [3, 3])
            .with_stride([2, 2])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(48)
            .with_bias(true)
            .init(device);
        let conv2d26 = Conv2dConfig::new([48, 96], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d27 = Conv2dConfig::new([96, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d28 = Conv2dConfig::new([64, 64], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(64)
            .with_bias(true)
            .init(device);
        let conv2d29 = Conv2dConfig::new([64, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d30 = Conv2dConfig::new([16, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d31 = Conv2dConfig::new([64, 128], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d32 = Conv2dConfig::new([128, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d33 = Conv2dConfig::new([64, 64], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(64)
            .with_bias(true)
            .init(device);
        let conv2d34 = Conv2dConfig::new([64, 128], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d35 = Conv2dConfig::new([128, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d36 = Conv2dConfig::new([64, 64], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(64)
            .with_bias(true)
            .init(device);
        let conv2d37 = Conv2dConfig::new([64, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d38 = Conv2dConfig::new([16, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d39 = Conv2dConfig::new([64, 128], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d40 = Conv2dConfig::new([128, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d41 = Conv2dConfig::new([64, 64], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(64)
            .with_bias(true)
            .init(device);
        let conv2d42 = Conv2dConfig::new([64, 128], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d43 = Conv2dConfig::new([128, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d44 = Conv2dConfig::new([64, 64], [3, 3])
            .with_stride([2, 2])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(64)
            .with_bias(true)
            .init(device);
        let conv2d45 = Conv2dConfig::new([64, 128], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d46 = Conv2dConfig::new([128, 160], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d47 = Conv2dConfig::new([160, 160], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(160)
            .with_bias(true)
            .init(device);
        let conv2d48 = Conv2dConfig::new([160, 40], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d49 = Conv2dConfig::new([40, 160], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d50 = Conv2dConfig::new([160, 320], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d51 = Conv2dConfig::new([320, 160], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d52 = Conv2dConfig::new([160, 160], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(160)
            .with_bias(true)
            .init(device);
        let conv2d53 = Conv2dConfig::new([160, 320], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d54 = Conv2dConfig::new([320, 160], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        Self {
            conv2d1,
            conv2d2,
            conv2d3,
            maxpool2d1,
            conv2d4,
            conv2d5,
            conv2d6,
            conv2d7,
            conv2d8,
            conv2d9,
            constant10,
            constant11,
            constant12,
            conv2d10,
            conv2d11,
            conv2d12,
            conv2d13,
            conv2d14,
            conv2d15,
            conv2d16,
            conv2d17,
            conv2d18,
            conv2d19,
            conv2d20,
            conv2d21,
            conv2d22,
            conv2d23,
            conv2d24,
            conv2d25,
            conv2d26,
            conv2d27,
            conv2d28,
            conv2d29,
            conv2d30,
            conv2d31,
            conv2d32,
            conv2d33,
            conv2d34,
            conv2d35,
            conv2d36,
            conv2d37,
            conv2d38,
            conv2d39,
            conv2d40,
            conv2d41,
            conv2d42,
            conv2d43,
            conv2d44,
            conv2d45,
            conv2d46,
            conv2d47,
            conv2d48,
            conv2d49,
            conv2d50,
            conv2d51,
            conv2d52,
            conv2d53,
            conv2d54,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, x: Tensor<4>) -> (Tensor<4>, Tensor<4>, Tensor<4>, Tensor<4>) {
        let conv2d1_out1 = self.conv2d1.forward(x);
        let relu1_out1 = burn::tensor::activation::relu(conv2d1_out1);
        let conv2d2_out1 = self.conv2d2.forward(relu1_out1.clone());
        let relu2_out1 = burn::tensor::activation::relu(conv2d2_out1);
        let conv2d3_out1 = self.conv2d3.forward(relu2_out1);
        let relu3_out1 = burn::tensor::activation::relu(conv2d3_out1);
        let maxpool2d1_out1 = self.maxpool2d1.forward(relu1_out1);
        let concat1_out1 = burn::tensor::Tensor::cat(
            [maxpool2d1_out1, relu3_out1].into(),
            1,
        );
        let conv2d4_out1 = self.conv2d4.forward(concat1_out1);
        let relu4_out1 = burn::tensor::activation::relu(conv2d4_out1);
        let conv2d5_out1 = self.conv2d5.forward(relu4_out1);
        let relu5_out1 = burn::tensor::activation::relu(conv2d5_out1);
        let conv2d6_out1 = self.conv2d6.forward(relu5_out1);
        let reducemean1_out1 = {
            conv2d6_out1.clone().mean_dim(2usize).mean_dim(3usize)
        };
        let conv2d7_out1 = self.conv2d7.forward(reducemean1_out1);
        let relu6_out1 = burn::tensor::activation::relu(conv2d7_out1);
        let conv2d8_out1 = self.conv2d8.forward(relu6_out1);
        let hardsigmoid1_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d8_out1,
            0.16666670143604279,
            0.5,
        );
        let mul1_out1 = conv2d6_out1.mul(hardsigmoid1_out1);
        let conv2d9_out1 = self.conv2d9.forward(mul1_out1.clone());
        let constant10_out1 = self.constant10.val();
        let div1_out1 = conv2d9_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf1_out1 = div1_out1.erf();
        let constant11_out1 = self.constant11.val();
        let add1_out1 = erf1_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul2_out1 = conv2d9_out1.mul(add1_out1);
        let constant12_out1 = self.constant12.val();
        let mul3_out1 = mul2_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d10_out1 = self.conv2d10.forward(mul3_out1);
        let add2_out1 = mul1_out1.add(conv2d10_out1);
        let conv2d11_out1 = self.conv2d11.forward(add2_out1);
        let conv2d12_out1 = self.conv2d12.forward(conv2d11_out1.clone());
        let div2_out1 = conv2d12_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf2_out1 = div2_out1.erf();
        let add3_out1 = erf2_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul4_out1 = conv2d12_out1.mul(add3_out1);
        let mul5_out1 = mul4_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d13_out1 = self.conv2d13.forward(mul5_out1);
        let add4_out1 = conv2d11_out1.add(conv2d13_out1);
        let conv2d14_out1 = self.conv2d14.forward(add4_out1.clone());
        let conv2d15_out1 = self.conv2d15.forward(conv2d14_out1);
        let div3_out1 = conv2d15_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf3_out1 = div3_out1.erf();
        let add5_out1 = erf3_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul6_out1 = conv2d15_out1.mul(add5_out1);
        let mul7_out1 = mul6_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d16_out1 = self.conv2d16.forward(mul7_out1);
        let conv2d17_out1 = self.conv2d17.forward(conv2d16_out1);
        let reducemean2_out1 = {
            conv2d17_out1.clone().mean_dim(2usize).mean_dim(3usize)
        };
        let conv2d18_out1 = self.conv2d18.forward(reducemean2_out1);
        let relu7_out1 = burn::tensor::activation::relu(conv2d18_out1);
        let conv2d19_out1 = self.conv2d19.forward(relu7_out1);
        let hardsigmoid2_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d19_out1,
            0.16666670143604279,
            0.5,
        );
        let mul8_out1 = conv2d17_out1.mul(hardsigmoid2_out1);
        let conv2d20_out1 = self.conv2d20.forward(mul8_out1.clone());
        let div4_out1 = conv2d20_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf4_out1 = div4_out1.erf();
        let add6_out1 = erf4_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul9_out1 = conv2d20_out1.mul(add6_out1);
        let mul10_out1 = mul9_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d21_out1 = self.conv2d21.forward(mul10_out1);
        let add7_out1 = mul8_out1.add(conv2d21_out1);
        let conv2d22_out1 = self.conv2d22.forward(add7_out1);
        let conv2d23_out1 = self.conv2d23.forward(conv2d22_out1.clone());
        let div5_out1 = conv2d23_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf5_out1 = div5_out1.erf();
        let add8_out1 = erf5_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul11_out1 = conv2d23_out1.mul(add8_out1);
        let mul12_out1 = mul11_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d24_out1 = self.conv2d24.forward(mul12_out1);
        let add9_out1 = conv2d22_out1.add(conv2d24_out1);
        let conv2d25_out1 = self.conv2d25.forward(add9_out1.clone());
        let conv2d26_out1 = self.conv2d26.forward(conv2d25_out1);
        let div6_out1 = conv2d26_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf6_out1 = div6_out1.erf();
        let add10_out1 = erf6_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul13_out1 = conv2d26_out1.mul(add10_out1);
        let mul14_out1 = mul13_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d27_out1 = self.conv2d27.forward(mul14_out1);
        let conv2d28_out1 = self.conv2d28.forward(conv2d27_out1);
        let reducemean3_out1 = {
            conv2d28_out1.clone().mean_dim(2usize).mean_dim(3usize)
        };
        let conv2d29_out1 = self.conv2d29.forward(reducemean3_out1);
        let relu8_out1 = burn::tensor::activation::relu(conv2d29_out1);
        let conv2d30_out1 = self.conv2d30.forward(relu8_out1);
        let hardsigmoid3_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d30_out1,
            0.16666670143604279,
            0.5,
        );
        let mul15_out1 = conv2d28_out1.mul(hardsigmoid3_out1);
        let conv2d31_out1 = self.conv2d31.forward(mul15_out1.clone());
        let div7_out1 = conv2d31_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf7_out1 = div7_out1.erf();
        let add11_out1 = erf7_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul16_out1 = conv2d31_out1.mul(add11_out1);
        let mul17_out1 = mul16_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d32_out1 = self.conv2d32.forward(mul17_out1);
        let add12_out1 = mul15_out1.add(conv2d32_out1);
        let conv2d33_out1 = self.conv2d33.forward(add12_out1);
        let conv2d34_out1 = self.conv2d34.forward(conv2d33_out1.clone());
        let div8_out1 = conv2d34_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf8_out1 = div8_out1.erf();
        let add13_out1 = erf8_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul18_out1 = conv2d34_out1.mul(add13_out1);
        let mul19_out1 = mul18_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d35_out1 = self.conv2d35.forward(mul19_out1);
        let add14_out1 = conv2d33_out1.add(conv2d35_out1);
        let conv2d36_out1 = self.conv2d36.forward(add14_out1);
        let reducemean4_out1 = {
            conv2d36_out1.clone().mean_dim(2usize).mean_dim(3usize)
        };
        let conv2d37_out1 = self.conv2d37.forward(reducemean4_out1);
        let relu9_out1 = burn::tensor::activation::relu(conv2d37_out1);
        let conv2d38_out1 = self.conv2d38.forward(relu9_out1);
        let hardsigmoid4_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d38_out1,
            0.16666670143604279,
            0.5,
        );
        let mul20_out1 = conv2d36_out1.mul(hardsigmoid4_out1);
        let conv2d39_out1 = self.conv2d39.forward(mul20_out1.clone());
        let div9_out1 = conv2d39_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf9_out1 = div9_out1.erf();
        let add15_out1 = erf9_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul21_out1 = conv2d39_out1.mul(add15_out1);
        let mul22_out1 = mul21_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d40_out1 = self.conv2d40.forward(mul22_out1);
        let add16_out1 = mul20_out1.add(conv2d40_out1);
        let conv2d41_out1 = self.conv2d41.forward(add16_out1);
        let conv2d42_out1 = self.conv2d42.forward(conv2d41_out1.clone());
        let div10_out1 = conv2d42_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf10_out1 = div10_out1.erf();
        let add17_out1 = erf10_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul23_out1 = conv2d42_out1.mul(add17_out1);
        let mul24_out1 = mul23_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d43_out1 = self.conv2d43.forward(mul24_out1);
        let add18_out1 = conv2d41_out1.add(conv2d43_out1);
        let conv2d44_out1 = self.conv2d44.forward(add18_out1.clone());
        let conv2d45_out1 = self.conv2d45.forward(conv2d44_out1);
        let div11_out1 = conv2d45_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf11_out1 = div11_out1.erf();
        let add19_out1 = erf11_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul25_out1 = conv2d45_out1.mul(add19_out1);
        let mul26_out1 = mul25_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d46_out1 = self.conv2d46.forward(mul26_out1);
        let conv2d47_out1 = self.conv2d47.forward(conv2d46_out1);
        let reducemean5_out1 = {
            conv2d47_out1.clone().mean_dim(2usize).mean_dim(3usize)
        };
        let conv2d48_out1 = self.conv2d48.forward(reducemean5_out1);
        let relu10_out1 = burn::tensor::activation::relu(conv2d48_out1);
        let conv2d49_out1 = self.conv2d49.forward(relu10_out1);
        let hardsigmoid5_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d49_out1,
            0.16666670143604279,
            0.5,
        );
        let mul27_out1 = conv2d47_out1.mul(hardsigmoid5_out1);
        let conv2d50_out1 = self.conv2d50.forward(mul27_out1.clone());
        let div12_out1 = conv2d50_out1
            .clone()
            .div((constant10_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf12_out1 = div12_out1.erf();
        let add20_out1 = erf12_out1
            .add((constant11_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul28_out1 = conv2d50_out1.mul(add20_out1);
        let mul29_out1 = mul28_out1
            .mul((constant12_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d51_out1 = self.conv2d51.forward(mul29_out1);
        let add21_out1 = mul27_out1.add(conv2d51_out1);
        let conv2d52_out1 = self.conv2d52.forward(add21_out1);
        let conv2d53_out1 = self.conv2d53.forward(conv2d52_out1.clone());
        let div13_out1 = conv2d53_out1
            .clone()
            .div((constant10_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let erf13_out1 = div13_out1.erf();
        let add22_out1 = erf13_out1
            .add((constant11_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let mul30_out1 = conv2d53_out1.mul(add22_out1);
        let mul31_out1 = mul30_out1
            .mul((constant12_out1).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let conv2d54_out1 = self.conv2d54.forward(mul31_out1);
        let add23_out1 = conv2d52_out1.add(conv2d54_out1);
        (add23_out1, add18_out1, add9_out1, add4_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule2 {
    conv2d55: Conv2d,
    globalaveragepool1: AdaptiveAvgPool2d,
    conv2d56: Conv2d,
    conv2d57: Conv2d,
    conv2d58: Conv2d,
    globalaveragepool2: AdaptiveAvgPool2d,
    conv2d59: Conv2d,
    conv2d60: Conv2d,
    conv2d61: Conv2d,
    globalaveragepool3: AdaptiveAvgPool2d,
    conv2d62: Conv2d,
    conv2d63: Conv2d,
    conv2d64: Conv2d,
    globalaveragepool4: AdaptiveAvgPool2d,
    conv2d65: Conv2d,
    conv2d66: Conv2d,
    resize1: burn::nn::interpolate::Interpolate2d,
    resize2: burn::nn::interpolate::Interpolate2d,
    resize3: burn::nn::interpolate::Interpolate2d,
    conv2d67: Conv2d,
    conv2d68: Conv2d,
    globalaveragepool5: AdaptiveAvgPool2d,
    conv2d69: Conv2d,
    conv2d70: Conv2d,
    conv2d71: Conv2d,
    conv2d72: Conv2d,
    globalaveragepool6: AdaptiveAvgPool2d,
    conv2d73: Conv2d,
    conv2d74: Conv2d,
    conv2d75: Conv2d,
    conv2d76: Conv2d,
    globalaveragepool7: AdaptiveAvgPool2d,
    conv2d77: Conv2d,
    conv2d78: Conv2d,
    conv2d79: Conv2d,
    conv2d80: Conv2d,
    globalaveragepool8: AdaptiveAvgPool2d,
    conv2d81: Conv2d,
    conv2d82: Conv2d,
    resize4: burn::nn::interpolate::Interpolate2d,
    resize5: burn::nn::interpolate::Interpolate2d,
    resize6: burn::nn::interpolate::Interpolate2d,
    conv2d83: Conv2d,
    convtranspose2d1: ConvTranspose2d,
    constant92: burn::module::Param<Tensor<4>>,
    convtranspose2d2: ConvTranspose2d,
    constant94: burn::module::Param<Tensor<4>>,
    #[module(skip)]
    device: Device,
}
impl Submodule2 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let conv2d55 = Conv2dConfig::new([160, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let globalaveragepool1 = AdaptiveAvgPool2dConfig::new([1, 1]).init();
        let conv2d56 = Conv2dConfig::new([64, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d57 = Conv2dConfig::new([16, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d58 = Conv2dConfig::new([64, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let globalaveragepool2 = AdaptiveAvgPool2dConfig::new([1, 1]).init();
        let conv2d59 = Conv2dConfig::new([64, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d60 = Conv2dConfig::new([16, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d61 = Conv2dConfig::new([48, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let globalaveragepool3 = AdaptiveAvgPool2dConfig::new([1, 1]).init();
        let conv2d62 = Conv2dConfig::new([64, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d63 = Conv2dConfig::new([16, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d64 = Conv2dConfig::new([32, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let globalaveragepool4 = AdaptiveAvgPool2dConfig::new([1, 1]).init();
        let conv2d65 = Conv2dConfig::new([64, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d66 = Conv2dConfig::new([16, 64], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let resize1 = burn::nn::interpolate::Interpolate2dConfig::new()
            .with_output_size(None)
            .with_scale_factor(Some([2.0, 2.0]))
            .with_mode(burn::nn::interpolate::InterpolateMode::Nearest)
            .with_align_corners(false)
            .init();
        let resize2 = burn::nn::interpolate::Interpolate2dConfig::new()
            .with_output_size(None)
            .with_scale_factor(Some([2.0, 2.0]))
            .with_mode(burn::nn::interpolate::InterpolateMode::Nearest)
            .with_align_corners(false)
            .init();
        let resize3 = burn::nn::interpolate::Interpolate2dConfig::new()
            .with_output_size(None)
            .with_scale_factor(Some([2.0, 2.0]))
            .with_mode(burn::nn::interpolate::InterpolateMode::Nearest)
            .with_align_corners(false)
            .init();
        let conv2d67 = Conv2dConfig::new([64, 64], [5, 5])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(2, 2, 2, 2))
            .with_dilation([1, 1])
            .with_groups(64)
            .with_bias(true)
            .init(device);
        let conv2d68 = Conv2dConfig::new([64, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let globalaveragepool5 = AdaptiveAvgPool2dConfig::new([1, 1]).init();
        let conv2d69 = Conv2dConfig::new([16, 4], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d70 = Conv2dConfig::new([4, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d71 = Conv2dConfig::new([64, 64], [5, 5])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(2, 2, 2, 2))
            .with_dilation([1, 1])
            .with_groups(64)
            .with_bias(true)
            .init(device);
        let conv2d72 = Conv2dConfig::new([64, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let globalaveragepool6 = AdaptiveAvgPool2dConfig::new([1, 1]).init();
        let conv2d73 = Conv2dConfig::new([16, 4], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d74 = Conv2dConfig::new([4, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d75 = Conv2dConfig::new([64, 64], [5, 5])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(2, 2, 2, 2))
            .with_dilation([1, 1])
            .with_groups(64)
            .with_bias(true)
            .init(device);
        let conv2d76 = Conv2dConfig::new([64, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let globalaveragepool7 = AdaptiveAvgPool2dConfig::new([1, 1]).init();
        let conv2d77 = Conv2dConfig::new([16, 4], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d78 = Conv2dConfig::new([4, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d79 = Conv2dConfig::new([64, 64], [5, 5])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(2, 2, 2, 2))
            .with_dilation([1, 1])
            .with_groups(64)
            .with_bias(true)
            .init(device);
        let conv2d80 = Conv2dConfig::new([64, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let globalaveragepool8 = AdaptiveAvgPool2dConfig::new([1, 1]).init();
        let conv2d81 = Conv2dConfig::new([16, 4], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let conv2d82 = Conv2dConfig::new([4, 16], [1, 1])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Valid)
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let resize4 = burn::nn::interpolate::Interpolate2dConfig::new()
            .with_output_size(None)
            .with_scale_factor(Some([8.0, 8.0]))
            .with_mode(burn::nn::interpolate::InterpolateMode::Nearest)
            .with_align_corners(false)
            .init();
        let resize5 = burn::nn::interpolate::Interpolate2dConfig::new()
            .with_output_size(None)
            .with_scale_factor(Some([4.0, 4.0]))
            .with_mode(burn::nn::interpolate::InterpolateMode::Nearest)
            .with_align_corners(false)
            .init();
        let resize6 = burn::nn::interpolate::Interpolate2dConfig::new()
            .with_output_size(None)
            .with_scale_factor(Some([2.0, 2.0]))
            .with_mode(burn::nn::interpolate::InterpolateMode::Nearest)
            .with_align_corners(false)
            .init();
        let conv2d83 = Conv2dConfig::new([64, 16], [3, 3])
            .with_stride([1, 1])
            .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(true)
            .init(device);
        let convtranspose2d1 = ConvTranspose2dConfig::new([16, 16], [2, 2])
            .with_stride([2, 2])
            .with_padding([0, 0])
            .with_padding_out([0, 0])
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant92: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 16, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 16, 1, 1].into(),
        );
        let convtranspose2d2 = ConvTranspose2dConfig::new([16, 1], [2, 2])
            .with_stride([2, 2])
            .with_padding([0, 0])
            .with_padding_out([0, 0])
            .with_dilation([1, 1])
            .with_groups(1)
            .with_bias(false)
            .init(device);
        let constant94: burn::module::Param<Tensor<4>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                4,
            >::zeros([1, 1, 1, 1], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [1, 1, 1, 1].into(),
        );
        Self {
            conv2d55,
            globalaveragepool1,
            conv2d56,
            conv2d57,
            conv2d58,
            globalaveragepool2,
            conv2d59,
            conv2d60,
            conv2d61,
            globalaveragepool3,
            conv2d62,
            conv2d63,
            conv2d64,
            globalaveragepool4,
            conv2d65,
            conv2d66,
            resize1,
            resize2,
            resize3,
            conv2d67,
            conv2d68,
            globalaveragepool5,
            conv2d69,
            conv2d70,
            conv2d71,
            conv2d72,
            globalaveragepool6,
            conv2d73,
            conv2d74,
            conv2d75,
            conv2d76,
            globalaveragepool7,
            conv2d77,
            conv2d78,
            conv2d79,
            conv2d80,
            globalaveragepool8,
            conv2d81,
            conv2d82,
            resize4,
            resize5,
            resize6,
            conv2d83,
            convtranspose2d1,
            constant92,
            convtranspose2d2,
            constant94,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add23_out1: Tensor<4>,
        add18_out1: Tensor<4>,
        add9_out1: Tensor<4>,
        add4_out1: Tensor<4>,
    ) -> Tensor<4> {
        let conv2d55_out1 = self.conv2d55.forward(add23_out1);
        let globalaveragepool1_out1 = self
            .globalaveragepool1
            .forward(conv2d55_out1.clone());
        let conv2d56_out1 = self.conv2d56.forward(globalaveragepool1_out1);
        let relu11_out1 = burn::tensor::activation::relu(conv2d56_out1);
        let conv2d57_out1 = self.conv2d57.forward(relu11_out1);
        let hardsigmoid6_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d57_out1,
            0.20000000298023224,
            0.5,
        );
        let mul32_out1 = conv2d55_out1.clone().mul(hardsigmoid6_out1);
        let add24_out1 = conv2d55_out1.add(mul32_out1);
        let conv2d58_out1 = self.conv2d58.forward(add18_out1);
        let globalaveragepool2_out1 = self
            .globalaveragepool2
            .forward(conv2d58_out1.clone());
        let conv2d59_out1 = self.conv2d59.forward(globalaveragepool2_out1);
        let relu12_out1 = burn::tensor::activation::relu(conv2d59_out1);
        let conv2d60_out1 = self.conv2d60.forward(relu12_out1);
        let hardsigmoid7_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d60_out1,
            0.20000000298023224,
            0.5,
        );
        let mul33_out1 = conv2d58_out1.clone().mul(hardsigmoid7_out1);
        let add25_out1 = conv2d58_out1.add(mul33_out1);
        let conv2d61_out1 = self.conv2d61.forward(add9_out1);
        let globalaveragepool3_out1 = self
            .globalaveragepool3
            .forward(conv2d61_out1.clone());
        let conv2d62_out1 = self.conv2d62.forward(globalaveragepool3_out1);
        let relu13_out1 = burn::tensor::activation::relu(conv2d62_out1);
        let conv2d63_out1 = self.conv2d63.forward(relu13_out1);
        let hardsigmoid8_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d63_out1,
            0.20000000298023224,
            0.5,
        );
        let mul34_out1 = conv2d61_out1.clone().mul(hardsigmoid8_out1);
        let add26_out1 = conv2d61_out1.add(mul34_out1);
        let conv2d64_out1 = self.conv2d64.forward(add4_out1);
        let globalaveragepool4_out1 = self
            .globalaveragepool4
            .forward(conv2d64_out1.clone());
        let conv2d65_out1 = self.conv2d65.forward(globalaveragepool4_out1);
        let relu14_out1 = burn::tensor::activation::relu(conv2d65_out1);
        let conv2d66_out1 = self.conv2d66.forward(relu14_out1);
        let hardsigmoid9_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d66_out1,
            0.20000000298023224,
            0.5,
        );
        let mul35_out1 = conv2d64_out1.clone().mul(hardsigmoid9_out1);
        let add27_out1 = conv2d64_out1.add(mul35_out1);
        let resize1_out1 = self.resize1.forward(add24_out1.clone());
        let add28_out1 = add25_out1.add(resize1_out1);
        let resize2_out1 = self.resize2.forward(add28_out1.clone());
        let add29_out1 = add26_out1.add(resize2_out1);
        let resize3_out1 = self.resize3.forward(add29_out1.clone());
        let add30_out1 = add27_out1.add(resize3_out1);
        let conv2d67_out1 = self.conv2d67.forward(add24_out1);
        let conv2d68_out1 = self.conv2d68.forward(conv2d67_out1);
        let globalaveragepool5_out1 = self
            .globalaveragepool5
            .forward(conv2d68_out1.clone());
        let conv2d69_out1 = self.conv2d69.forward(globalaveragepool5_out1);
        let relu15_out1 = burn::tensor::activation::relu(conv2d69_out1);
        let conv2d70_out1 = self.conv2d70.forward(relu15_out1);
        let hardsigmoid10_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d70_out1,
            0.20000000298023224,
            0.5,
        );
        let mul36_out1 = conv2d68_out1.clone().mul(hardsigmoid10_out1);
        let add31_out1 = conv2d68_out1.add(mul36_out1);
        let conv2d71_out1 = self.conv2d71.forward(add28_out1);
        let conv2d72_out1 = self.conv2d72.forward(conv2d71_out1);
        let globalaveragepool6_out1 = self
            .globalaveragepool6
            .forward(conv2d72_out1.clone());
        let conv2d73_out1 = self.conv2d73.forward(globalaveragepool6_out1);
        let relu16_out1 = burn::tensor::activation::relu(conv2d73_out1);
        let conv2d74_out1 = self.conv2d74.forward(relu16_out1);
        let hardsigmoid11_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d74_out1,
            0.20000000298023224,
            0.5,
        );
        let mul37_out1 = conv2d72_out1.clone().mul(hardsigmoid11_out1);
        let add32_out1 = conv2d72_out1.add(mul37_out1);
        let conv2d75_out1 = self.conv2d75.forward(add29_out1);
        let conv2d76_out1 = self.conv2d76.forward(conv2d75_out1);
        let globalaveragepool7_out1 = self
            .globalaveragepool7
            .forward(conv2d76_out1.clone());
        let conv2d77_out1 = self.conv2d77.forward(globalaveragepool7_out1);
        let relu17_out1 = burn::tensor::activation::relu(conv2d77_out1);
        let conv2d78_out1 = self.conv2d78.forward(relu17_out1);
        let hardsigmoid12_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d78_out1,
            0.20000000298023224,
            0.5,
        );
        let mul38_out1 = conv2d76_out1.clone().mul(hardsigmoid12_out1);
        let add33_out1 = conv2d76_out1.add(mul38_out1);
        let conv2d79_out1 = self.conv2d79.forward(add30_out1);
        let conv2d80_out1 = self.conv2d80.forward(conv2d79_out1);
        let globalaveragepool8_out1 = self
            .globalaveragepool8
            .forward(conv2d80_out1.clone());
        let conv2d81_out1 = self.conv2d81.forward(globalaveragepool8_out1);
        let relu18_out1 = burn::tensor::activation::relu(conv2d81_out1);
        let conv2d82_out1 = self.conv2d82.forward(relu18_out1);
        let hardsigmoid13_out1 = burn::tensor::activation::hard_sigmoid(
            conv2d82_out1,
            0.20000000298023224,
            0.5,
        );
        let mul39_out1 = conv2d80_out1.clone().mul(hardsigmoid13_out1);
        let add34_out1 = conv2d80_out1.add(mul39_out1);
        let resize4_out1 = self.resize4.forward(add31_out1);
        let resize5_out1 = self.resize5.forward(add32_out1);
        let resize6_out1 = self.resize6.forward(add33_out1);
        let concat2_out1 = burn::tensor::Tensor::cat(
            [resize4_out1, resize5_out1, resize6_out1, add34_out1].into(),
            1,
        );
        let conv2d83_out1 = self.conv2d83.forward(concat2_out1);
        let relu19_out1 = burn::tensor::activation::relu(conv2d83_out1);
        let convtranspose2d1_out1 = self.convtranspose2d1.forward(relu19_out1);
        let constant92_out1 = self.constant92.val();
        let add35_out1 = convtranspose2d1_out1.add(constant92_out1);
        let relu20_out1 = burn::tensor::activation::relu(add35_out1);
        let convtranspose2d2_out1 = self.convtranspose2d2.forward(relu20_out1);
        let constant94_out1 = self.constant94.val();
        let add36_out1 = convtranspose2d2_out1.add(constant94_out1);
        let sigmoid1_out1 = burn::tensor::activation::sigmoid(add36_out1);
        sigmoid1_out1
    }
}

#[derive(Module, Debug)]
pub struct Model {
    submodule1: Submodule1,
    submodule2: Submodule2,
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
        Self {
            submodule1,
            submodule2,
            device: device.clone(),
        }
    }

    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(&self, x: Tensor<4>) -> Tensor<4> {
        let (add23_out1, add18_out1, add9_out1, add4_out1) = self.submodule1.forward(x);
        let sigmoid1_out1 = self
            .submodule2
            .forward(add23_out1, add18_out1, add9_out1, add4_out1);
        sigmoid1_out1
    }
}
