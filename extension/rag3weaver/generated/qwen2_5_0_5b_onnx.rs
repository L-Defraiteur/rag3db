// Generated from ONNX "onnx-community/Qwen2.5-0.5B-Instruct onnx/model.onnx" by burn-onnx
//
// Une rustine, appliquée à la main sur la sortie de burn-onnx 0.22.0-pre.1
// (décrite dans generated/README.md) : le chemin `ScatterND` émet
// `alloc::vec::Vec` sans déclarer le crate.
//
// ⚠ C'est l'export **f32** et non `model_fp16.onnx`, à dessein : l'export fp16
// d'onnx-community est numériquement dégradé — mesuré, il complète « The
// capital of France is » par « is is is » là où le f32 rend « Paris. It is the
// largest city in Europe ». Voir generated/README.md.
extern crate alloc;
use burn::prelude::*;
use burn::nn::Linear;
use burn::nn::LinearConfig;
use burn::tensor::Bytes;
use burn_store::BurnpackStore;
use burn_store::ModuleSnapshot;


#[derive(Module, Debug)]
pub struct Submodule1 {
    constant1: burn::module::Param<Tensor<2>>,
    constant29: burn::module::Param<Tensor<1>>,
    constant37: burn::module::Param<Tensor<1>>,
    constant43: burn::module::Param<Tensor<1>>,
    constant44: burn::module::Param<Tensor<1, Int>>,
    constant46: burn::module::Param<Tensor<1, Int>>,
    constant47: burn::module::Param<Tensor<1>>,
    linear1: Linear,
    linear2: Linear,
    linear3: Linear,
    constant54: burn::module::Param<Tensor<1>>,
    constant55: burn::module::Param<Tensor<1>>,
    constant56: burn::module::Param<Tensor<1>>,
    constant108: burn::module::Param<Tensor<2>>,
    constant112: burn::module::Param<Tensor<2>>,
    constant121: burn::module::Param<Tensor<1, Int>>,
    constant122: burn::module::Param<Tensor<1, Int>>,
    constant125: burn::module::Param<Tensor<1, Int>>,
    constant126: burn::module::Param<Tensor<1, Int>>,
    constant102: burn::module::Param<Tensor<3, Int>>,
    constant142: burn::module::Param<Tensor<1, Int>>,
    constant143: burn::module::Param<Tensor<1, Int>>,
    linear4: Linear,
    constant151: burn::module::Param<Tensor<1>>,
    constant152: burn::module::Param<Tensor<1>>,
    constant153: burn::module::Param<Tensor<1>>,
    constant154: burn::module::Param<Tensor<1>>,
    linear5: Linear,
    linear6: Linear,
    linear7: Linear,
    constant158: burn::module::Param<Tensor<1>>,
    constant159: burn::module::Param<Tensor<1>>,
    constant160: burn::module::Param<Tensor<1>>,
    constant161: burn::module::Param<Tensor<1>>,
    linear8: Linear,
    linear9: Linear,
    linear10: Linear,
    constant167: burn::module::Param<Tensor<1>>,
    constant168: burn::module::Param<Tensor<1>>,
    constant169: burn::module::Param<Tensor<1>>,
    constant211: burn::module::Param<Tensor<1, Int>>,
    constant214: burn::module::Param<Tensor<1, Int>>,
    constant226: burn::module::Param<Tensor<1, Int>>,
    constant227: burn::module::Param<Tensor<1, Int>>,
    linear11: Linear,
    constant235: burn::module::Param<Tensor<1>>,
    constant236: burn::module::Param<Tensor<1>>,
    constant237: burn::module::Param<Tensor<1>>,
    constant238: burn::module::Param<Tensor<1>>,
    linear12: Linear,
    linear13: Linear,
    linear14: Linear,
    constant242: burn::module::Param<Tensor<1>>,
    constant243: burn::module::Param<Tensor<1>>,
    constant244: burn::module::Param<Tensor<1>>,
    constant245: burn::module::Param<Tensor<1>>,
    linear15: Linear,
    linear16: Linear,
    linear17: Linear,
    constant251: burn::module::Param<Tensor<1>>,
    constant252: burn::module::Param<Tensor<1>>,
    constant253: burn::module::Param<Tensor<1>>,
    constant295: burn::module::Param<Tensor<1, Int>>,
    constant298: burn::module::Param<Tensor<1, Int>>,
    constant310: burn::module::Param<Tensor<1, Int>>,
    constant311: burn::module::Param<Tensor<1, Int>>,
    linear18: Linear,
    constant319: burn::module::Param<Tensor<1>>,
    constant320: burn::module::Param<Tensor<1>>,
    constant321: burn::module::Param<Tensor<1>>,
    constant322: burn::module::Param<Tensor<1>>,
    linear19: Linear,
    linear20: Linear,
    linear21: Linear,
    constant326: burn::module::Param<Tensor<1>>,
    constant327: burn::module::Param<Tensor<1>>,
    constant328: burn::module::Param<Tensor<1>>,
    constant329: burn::module::Param<Tensor<1>>,
    linear22: Linear,
    linear23: Linear,
    linear24: Linear,
    constant335: burn::module::Param<Tensor<1>>,
    constant336: burn::module::Param<Tensor<1>>,
    constant337: burn::module::Param<Tensor<1>>,
    constant379: burn::module::Param<Tensor<1, Int>>,
    constant382: burn::module::Param<Tensor<1, Int>>,
    constant394: burn::module::Param<Tensor<1, Int>>,
    constant395: burn::module::Param<Tensor<1, Int>>,
    linear25: Linear,
    constant403: burn::module::Param<Tensor<1>>,
    constant404: burn::module::Param<Tensor<1>>,
    constant405: burn::module::Param<Tensor<1>>,
    constant406: burn::module::Param<Tensor<1>>,
    linear26: Linear,
    linear27: Linear,
    linear28: Linear,
    constant410: burn::module::Param<Tensor<1>>,
    constant411: burn::module::Param<Tensor<1>>,
    constant412: burn::module::Param<Tensor<1>>,
    constant413: burn::module::Param<Tensor<1>>,
    linear29: Linear,
    linear30: Linear,
    linear31: Linear,
    constant419: burn::module::Param<Tensor<1>>,
    constant420: burn::module::Param<Tensor<1>>,
    constant421: burn::module::Param<Tensor<1>>,
    constant463: burn::module::Param<Tensor<1, Int>>,
    constant466: burn::module::Param<Tensor<1, Int>>,
    constant478: burn::module::Param<Tensor<1, Int>>,
    constant479: burn::module::Param<Tensor<1, Int>>,
    linear32: Linear,
    constant487: burn::module::Param<Tensor<1>>,
    constant488: burn::module::Param<Tensor<1>>,
    constant489: burn::module::Param<Tensor<1>>,
    constant490: burn::module::Param<Tensor<1>>,
    linear33: Linear,
    linear34: Linear,
    linear35: Linear,
    constant494: burn::module::Param<Tensor<1>>,
    constant495: burn::module::Param<Tensor<1>>,
    constant496: burn::module::Param<Tensor<1>>,
    constant497: burn::module::Param<Tensor<1>>,
    linear36: Linear,
    linear37: Linear,
    linear38: Linear,
    constant503: burn::module::Param<Tensor<1>>,
    constant504: burn::module::Param<Tensor<1>>,
    constant505: burn::module::Param<Tensor<1>>,
    constant547: burn::module::Param<Tensor<1, Int>>,
    constant550: burn::module::Param<Tensor<1, Int>>,
    constant562: burn::module::Param<Tensor<1, Int>>,
    constant563: burn::module::Param<Tensor<1, Int>>,
    linear39: Linear,
    constant571: burn::module::Param<Tensor<1>>,
    constant572: burn::module::Param<Tensor<1>>,
    constant573: burn::module::Param<Tensor<1>>,
    constant574: burn::module::Param<Tensor<1>>,
    linear40: Linear,
    linear41: Linear,
    linear42: Linear,
    constant578: burn::module::Param<Tensor<1>>,
    constant579: burn::module::Param<Tensor<1>>,
    constant580: burn::module::Param<Tensor<1>>,
    constant581: burn::module::Param<Tensor<1>>,
    linear43: Linear,
    linear44: Linear,
    linear45: Linear,
    constant587: burn::module::Param<Tensor<1>>,
    constant588: burn::module::Param<Tensor<1>>,
    constant589: burn::module::Param<Tensor<1>>,
    constant631: burn::module::Param<Tensor<1, Int>>,
    constant634: burn::module::Param<Tensor<1, Int>>,
    constant646: burn::module::Param<Tensor<1, Int>>,
    constant647: burn::module::Param<Tensor<1, Int>>,
    linear46: Linear,
    constant655: burn::module::Param<Tensor<1>>,
    constant656: burn::module::Param<Tensor<1>>,
    constant657: burn::module::Param<Tensor<1>>,
    constant658: burn::module::Param<Tensor<1>>,
    linear47: Linear,
    linear48: Linear,
    linear49: Linear,
    constant662: burn::module::Param<Tensor<1>>,
    constant663: burn::module::Param<Tensor<1>>,
    constant664: burn::module::Param<Tensor<1>>,
    constant665: burn::module::Param<Tensor<1>>,
    linear50: Linear,
    linear51: Linear,
    linear52: Linear,
    constant671: burn::module::Param<Tensor<1>>,
    constant672: burn::module::Param<Tensor<1>>,
    constant673: burn::module::Param<Tensor<1>>,
    constant715: burn::module::Param<Tensor<1, Int>>,
    constant718: burn::module::Param<Tensor<1, Int>>,
    constant730: burn::module::Param<Tensor<1, Int>>,
    constant731: burn::module::Param<Tensor<1, Int>>,
    linear53: Linear,
    constant739: burn::module::Param<Tensor<1>>,
    constant740: burn::module::Param<Tensor<1>>,
    constant741: burn::module::Param<Tensor<1>>,
    constant742: burn::module::Param<Tensor<1>>,
    linear54: Linear,
    linear55: Linear,
    linear56: Linear,
    constant746: burn::module::Param<Tensor<1>>,
    constant747: burn::module::Param<Tensor<1>>,
    constant748: burn::module::Param<Tensor<1>>,
    constant749: burn::module::Param<Tensor<1>>,
    linear57: Linear,
    linear58: Linear,
    linear59: Linear,
    constant755: burn::module::Param<Tensor<1>>,
    constant756: burn::module::Param<Tensor<1>>,
    constant757: burn::module::Param<Tensor<1>>,
    constant799: burn::module::Param<Tensor<1, Int>>,
    constant802: burn::module::Param<Tensor<1, Int>>,
    constant814: burn::module::Param<Tensor<1, Int>>,
    constant815: burn::module::Param<Tensor<1, Int>>,
    linear60: Linear,
    constant823: burn::module::Param<Tensor<1>>,
    constant824: burn::module::Param<Tensor<1>>,
    constant825: burn::module::Param<Tensor<1>>,
    constant826: burn::module::Param<Tensor<1>>,
    linear61: Linear,
    linear62: Linear,
    linear63: Linear,
    constant830: burn::module::Param<Tensor<1>>,
    constant831: burn::module::Param<Tensor<1>>,
    constant832: burn::module::Param<Tensor<1>>,
    constant833: burn::module::Param<Tensor<1>>,
    linear64: Linear,
    linear65: Linear,
    linear66: Linear,
    constant839: burn::module::Param<Tensor<1>>,
    constant840: burn::module::Param<Tensor<1>>,
    constant841: burn::module::Param<Tensor<1>>,
    constant883: burn::module::Param<Tensor<1, Int>>,
    constant886: burn::module::Param<Tensor<1, Int>>,
    constant898: burn::module::Param<Tensor<1, Int>>,
    constant899: burn::module::Param<Tensor<1, Int>>,
    linear67: Linear,
    constant907: burn::module::Param<Tensor<1>>,
    constant908: burn::module::Param<Tensor<1>>,
    constant909: burn::module::Param<Tensor<1>>,
    constant910: burn::module::Param<Tensor<1>>,
    linear68: Linear,
    linear69: Linear,
    linear70: Linear,
    constant914: burn::module::Param<Tensor<1>>,
    constant915: burn::module::Param<Tensor<1>>,
    constant916: burn::module::Param<Tensor<1>>,
    constant917: burn::module::Param<Tensor<1>>,
    linear71: Linear,
    linear72: Linear,
    linear73: Linear,
    constant923: burn::module::Param<Tensor<1>>,
    constant924: burn::module::Param<Tensor<1>>,
    constant925: burn::module::Param<Tensor<1>>,
    constant967: burn::module::Param<Tensor<1, Int>>,
    constant970: burn::module::Param<Tensor<1, Int>>,
    constant982: burn::module::Param<Tensor<1, Int>>,
    constant983: burn::module::Param<Tensor<1, Int>>,
    linear74: Linear,
    constant991: burn::module::Param<Tensor<1>>,
    constant992: burn::module::Param<Tensor<1>>,
    constant993: burn::module::Param<Tensor<1>>,
    constant994: burn::module::Param<Tensor<1>>,
    linear75: Linear,
    linear76: Linear,
    linear77: Linear,
    constant998: burn::module::Param<Tensor<1>>,
    constant999: burn::module::Param<Tensor<1>>,
    constant1000: burn::module::Param<Tensor<1>>,
    constant1001: burn::module::Param<Tensor<1>>,
    linear78: Linear,
    linear79: Linear,
    linear80: Linear,
    constant1007: burn::module::Param<Tensor<1>>,
    constant1008: burn::module::Param<Tensor<1>>,
    constant1009: burn::module::Param<Tensor<1>>,
    constant1051: burn::module::Param<Tensor<1, Int>>,
    constant1054: burn::module::Param<Tensor<1, Int>>,
    constant1066: burn::module::Param<Tensor<1, Int>>,
    constant1067: burn::module::Param<Tensor<1, Int>>,
    linear81: Linear,
    constant1075: burn::module::Param<Tensor<1>>,
    constant1076: burn::module::Param<Tensor<1>>,
    constant1077: burn::module::Param<Tensor<1>>,
    constant1078: burn::module::Param<Tensor<1>>,
    linear82: Linear,
    linear83: Linear,
    linear84: Linear,
    constant1082: burn::module::Param<Tensor<1>>,
    constant1083: burn::module::Param<Tensor<1>>,
    constant1084: burn::module::Param<Tensor<1>>,
    constant1085: burn::module::Param<Tensor<1>>,
    linear85: Linear,
    linear86: Linear,
    linear87: Linear,
    constant1091: burn::module::Param<Tensor<1>>,
    constant1092: burn::module::Param<Tensor<1>>,
    constant1093: burn::module::Param<Tensor<1>>,
    constant1135: burn::module::Param<Tensor<1, Int>>,
    constant1138: burn::module::Param<Tensor<1, Int>>,
    constant1150: burn::module::Param<Tensor<1, Int>>,
    constant1151: burn::module::Param<Tensor<1, Int>>,
    linear88: Linear,
    constant1159: burn::module::Param<Tensor<1>>,
    constant1160: burn::module::Param<Tensor<1>>,
    constant1161: burn::module::Param<Tensor<1>>,
    constant1162: burn::module::Param<Tensor<1>>,
    linear89: Linear,
    linear90: Linear,
    linear91: Linear,
    constant1166: burn::module::Param<Tensor<1>>,
    constant1167: burn::module::Param<Tensor<1>>,
    constant1168: burn::module::Param<Tensor<1>>,
    constant1169: burn::module::Param<Tensor<1>>,
    linear92: Linear,
    linear93: Linear,
    linear94: Linear,
    constant1175: burn::module::Param<Tensor<1>>,
    constant1176: burn::module::Param<Tensor<1>>,
    constant1177: burn::module::Param<Tensor<1>>,
    constant1219: burn::module::Param<Tensor<1, Int>>,
    constant1222: burn::module::Param<Tensor<1, Int>>,
    constant1234: burn::module::Param<Tensor<1, Int>>,
    constant1235: burn::module::Param<Tensor<1, Int>>,
    linear95: Linear,
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
            >::zeros([151936, 896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [151936, 896].into(),
        );
        let constant29: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant37: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant43: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant44: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([4], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [4].into(),
        );
        let constant46: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([4], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [4].into(),
        );
        let constant47: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear1 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear2 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear3 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant54: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant55: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant56: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant108: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([32768, 64], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [32768, 64].into(),
        );
        let constant112: burn::module::Param<Tensor<2>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                2,
            >::zeros([32768, 64], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [32768, 64].into(),
        );
        let constant121: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant122: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([4], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [4].into(),
        );
        let constant125: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant126: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([4], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [4].into(),
        );
        let constant102: burn::module::Param<Tensor<3, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                3,
                Int,
            >::zeros([1, 1, 1], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [1, 1, 1].into(),
        );
        let constant142: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant143: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear4 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant151: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant152: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant153: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant154: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear5 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear6 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear7 = LinearConfig::new(4864, 896).with_bias(false).init(device);
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
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
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
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant161: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear8 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear9 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear10 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant167: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant168: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant169: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant211: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant214: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant226: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant227: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear11 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant235: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant236: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
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
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear12 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear13 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear14 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant242: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant243: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
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
            >::from_data(
                burn::tensor::TensorData::from([1f64]),
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
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear15 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear16 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear17 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant251: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant252: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant253: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant295: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant298: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant310: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant311: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear18 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant319: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant320: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
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
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear19 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear20 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear21 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant326: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant327: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant328: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant329: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear22 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear23 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear24 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant335: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant336: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant337: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant379: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant382: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant394: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant395: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear25 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant403: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant404: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
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
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant406: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear26 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear27 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear28 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant410: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant411: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
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
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear29 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear30 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear31 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant419: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant420: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant421: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant463: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant466: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant478: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant479: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear32 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant487: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant488: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant489: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant490: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear33 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear34 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear35 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant494: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant495: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
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
                burn::tensor::TensorData::from([1f64]),
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
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear36 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear37 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear38 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant503: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant504: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant505: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant547: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant550: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant562: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant563: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear39 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant571: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant572: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant573: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant574: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear40 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear41 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear42 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant578: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant579: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
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
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant581: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear43 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear44 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear45 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant587: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant588: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant589: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant631: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant634: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant646: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant647: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear46 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant655: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant656: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant657: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant658: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear47 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear48 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear49 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant662: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant663: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant664: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant665: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear50 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear51 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear52 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant671: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant672: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant673: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant715: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant718: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant730: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant731: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear53 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant739: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant740: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant741: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant742: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear54 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear55 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear56 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant746: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant747: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
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
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant749: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear57 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear58 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear59 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant755: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant756: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant757: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant799: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant802: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant814: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant815: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear60 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant823: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant824: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant825: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant826: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear61 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear62 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear63 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant830: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant831: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant832: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant833: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear64 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear65 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear66 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant839: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant840: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant841: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant883: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant886: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant898: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant899: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear67 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant907: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant908: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
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
                burn::tensor::TensorData::from([1f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant910: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear68 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear69 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear70 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant914: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant915: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant916: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant917: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear71 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear72 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear73 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant923: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant924: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant925: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant967: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant970: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant982: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant983: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear74 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant991: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant992: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant993: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant994: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear75 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear76 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear77 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant998: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant999: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1000: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1001: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear78 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear79 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear80 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1007: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1008: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1009: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1051: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1054: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1066: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1067: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear81 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant1075: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1076: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1077: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1078: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear82 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear83 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear84 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1082: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1083: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1084: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1085: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear85 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear86 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear87 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1091: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1092: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1093: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1135: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1138: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1150: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1151: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear88 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant1159: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1160: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1161: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1162: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear89 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear90 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear91 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1166: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1167: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1168: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1169: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear92 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear93 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear94 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1175: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1176: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1177: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1219: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1222: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1234: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1235: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear95 = LinearConfig::new(896, 896).with_bias(false).init(device);
        Self {
            constant1,
            constant29,
            constant37,
            constant43,
            constant44,
            constant46,
            constant47,
            linear1,
            linear2,
            linear3,
            constant54,
            constant55,
            constant56,
            constant108,
            constant112,
            constant121,
            constant122,
            constant125,
            constant126,
            constant102,
            constant142,
            constant143,
            linear4,
            constant151,
            constant152,
            constant153,
            constant154,
            linear5,
            linear6,
            linear7,
            constant158,
            constant159,
            constant160,
            constant161,
            linear8,
            linear9,
            linear10,
            constant167,
            constant168,
            constant169,
            constant211,
            constant214,
            constant226,
            constant227,
            linear11,
            constant235,
            constant236,
            constant237,
            constant238,
            linear12,
            linear13,
            linear14,
            constant242,
            constant243,
            constant244,
            constant245,
            linear15,
            linear16,
            linear17,
            constant251,
            constant252,
            constant253,
            constant295,
            constant298,
            constant310,
            constant311,
            linear18,
            constant319,
            constant320,
            constant321,
            constant322,
            linear19,
            linear20,
            linear21,
            constant326,
            constant327,
            constant328,
            constant329,
            linear22,
            linear23,
            linear24,
            constant335,
            constant336,
            constant337,
            constant379,
            constant382,
            constant394,
            constant395,
            linear25,
            constant403,
            constant404,
            constant405,
            constant406,
            linear26,
            linear27,
            linear28,
            constant410,
            constant411,
            constant412,
            constant413,
            linear29,
            linear30,
            linear31,
            constant419,
            constant420,
            constant421,
            constant463,
            constant466,
            constant478,
            constant479,
            linear32,
            constant487,
            constant488,
            constant489,
            constant490,
            linear33,
            linear34,
            linear35,
            constant494,
            constant495,
            constant496,
            constant497,
            linear36,
            linear37,
            linear38,
            constant503,
            constant504,
            constant505,
            constant547,
            constant550,
            constant562,
            constant563,
            linear39,
            constant571,
            constant572,
            constant573,
            constant574,
            linear40,
            linear41,
            linear42,
            constant578,
            constant579,
            constant580,
            constant581,
            linear43,
            linear44,
            linear45,
            constant587,
            constant588,
            constant589,
            constant631,
            constant634,
            constant646,
            constant647,
            linear46,
            constant655,
            constant656,
            constant657,
            constant658,
            linear47,
            linear48,
            linear49,
            constant662,
            constant663,
            constant664,
            constant665,
            linear50,
            linear51,
            linear52,
            constant671,
            constant672,
            constant673,
            constant715,
            constant718,
            constant730,
            constant731,
            linear53,
            constant739,
            constant740,
            constant741,
            constant742,
            linear54,
            linear55,
            linear56,
            constant746,
            constant747,
            constant748,
            constant749,
            linear57,
            linear58,
            linear59,
            constant755,
            constant756,
            constant757,
            constant799,
            constant802,
            constant814,
            constant815,
            linear60,
            constant823,
            constant824,
            constant825,
            constant826,
            linear61,
            linear62,
            linear63,
            constant830,
            constant831,
            constant832,
            constant833,
            linear64,
            linear65,
            linear66,
            constant839,
            constant840,
            constant841,
            constant883,
            constant886,
            constant898,
            constant899,
            linear67,
            constant907,
            constant908,
            constant909,
            constant910,
            linear68,
            linear69,
            linear70,
            constant914,
            constant915,
            constant916,
            constant917,
            linear71,
            linear72,
            linear73,
            constant923,
            constant924,
            constant925,
            constant967,
            constant970,
            constant982,
            constant983,
            linear74,
            constant991,
            constant992,
            constant993,
            constant994,
            linear75,
            linear76,
            linear77,
            constant998,
            constant999,
            constant1000,
            constant1001,
            linear78,
            linear79,
            linear80,
            constant1007,
            constant1008,
            constant1009,
            constant1051,
            constant1054,
            constant1066,
            constant1067,
            linear81,
            constant1075,
            constant1076,
            constant1077,
            constant1078,
            linear82,
            linear83,
            linear84,
            constant1082,
            constant1083,
            constant1084,
            constant1085,
            linear85,
            linear86,
            linear87,
            constant1091,
            constant1092,
            constant1093,
            constant1135,
            constant1138,
            constant1150,
            constant1151,
            linear88,
            constant1159,
            constant1160,
            constant1161,
            constant1162,
            linear89,
            linear90,
            linear91,
            constant1166,
            constant1167,
            constant1168,
            constant1169,
            linear92,
            linear93,
            linear94,
            constant1175,
            constant1176,
            constant1177,
            constant1219,
            constant1222,
            constant1234,
            constant1235,
            linear95,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        input_ids: Tensor<2, Int>,
        past_key_values_0_key: Tensor<4>,
        attention_mask: Tensor<2, Int>,
        past_key_values_1_key: Tensor<4>,
        past_key_values_2_key: Tensor<4>,
        past_key_values_3_key: Tensor<4>,
        past_key_values_4_key: Tensor<4>,
        past_key_values_5_key: Tensor<4>,
        past_key_values_6_key: Tensor<4>,
        past_key_values_7_key: Tensor<4>,
        past_key_values_8_key: Tensor<4>,
        past_key_values_9_key: Tensor<4>,
        past_key_values_10_key: Tensor<4>,
        past_key_values_11_key: Tensor<4>,
        past_key_values_12_key: Tensor<4>,
        past_key_values_13_key: Tensor<4>,
        past_key_values_14_key: Tensor<4>,
        past_key_values_15_key: Tensor<4>,
        past_key_values_16_key: Tensor<4>,
        past_key_values_17_key: Tensor<4>,
        past_key_values_18_key: Tensor<4>,
        past_key_values_19_key: Tensor<4>,
        past_key_values_20_key: Tensor<4>,
        past_key_values_21_key: Tensor<4>,
        past_key_values_22_key: Tensor<4>,
        past_key_values_23_key: Tensor<4>,
        past_key_values_0_value: Tensor<4>,
        position_ids: Tensor<2, Int>,
        past_key_values_1_value: Tensor<4>,
        past_key_values_2_value: Tensor<4>,
        past_key_values_3_value: Tensor<4>,
        past_key_values_4_value: Tensor<4>,
        past_key_values_5_value: Tensor<4>,
        past_key_values_6_value: Tensor<4>,
        past_key_values_7_value: Tensor<4>,
        past_key_values_8_value: Tensor<4>,
        past_key_values_9_value: Tensor<4>,
        past_key_values_10_value: Tensor<4>,
        past_key_values_11_value: Tensor<4>,
        past_key_values_12_value: Tensor<4>,
        past_key_values_13_value: Tensor<4>,
    ) -> (
        Tensor<3>,
        i64,
        Tensor<2>,
        Tensor<2>,
        Tensor<4>,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        Tensor<2>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
    ) {
        let constant1_out1 = self.constant1.val();
        let gather1_out1 = constant1_out1.clone().take::<2, 3>(0, input_ids);
        let shape1_out1: [i64; 4] = {
            let axes = &past_key_values_0_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape2_out1: [i64; 2] = {
            let axes = &attention_mask.clone().dims()[0..2];
            let mut output = [0i64; 2];
            for i in 0..2 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape3_out1: [i64; 4] = {
            let axes = &past_key_values_1_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape4_out1: [i64; 4] = {
            let axes = &past_key_values_2_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape5_out1: [i64; 4] = {
            let axes = &past_key_values_3_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape6_out1: [i64; 4] = {
            let axes = &past_key_values_4_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape7_out1: [i64; 4] = {
            let axes = &past_key_values_5_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape8_out1: [i64; 4] = {
            let axes = &past_key_values_6_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape9_out1: [i64; 4] = {
            let axes = &past_key_values_7_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape10_out1: [i64; 4] = {
            let axes = &past_key_values_8_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape11_out1: [i64; 4] = {
            let axes = &past_key_values_9_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape12_out1: [i64; 4] = {
            let axes = &past_key_values_10_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape13_out1: [i64; 4] = {
            let axes = &past_key_values_11_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape14_out1: [i64; 4] = {
            let axes = &past_key_values_12_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape15_out1: [i64; 4] = {
            let axes = &past_key_values_13_key.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape16_out1: [i64; 4] = {
            let axes = &past_key_values_14_key.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape17_out1: [i64; 4] = {
            let axes = &past_key_values_15_key.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape18_out1: [i64; 4] = {
            let axes = &past_key_values_16_key.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape19_out1: [i64; 4] = {
            let axes = &past_key_values_17_key.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape20_out1: [i64; 4] = {
            let axes = &past_key_values_18_key.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape21_out1: [i64; 4] = {
            let axes = &past_key_values_19_key.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape22_out1: [i64; 4] = {
            let axes = &past_key_values_20_key.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape23_out1: [i64; 4] = {
            let axes = &past_key_values_21_key.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape24_out1: [i64; 4] = {
            let axes = &past_key_values_22_key.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let shape25_out1: [i64; 4] = {
            let axes = &past_key_values_23_key.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let transpose1_out1 = constant1_out1.permute([1, 0]);
        let unsqueeze1_out1: Tensor<3, Int> = attention_mask.unsqueeze_dims::<3>(&[1]);
        let gather2_out1 = shape1_out1[2] as i64;
        let shape26_out1: [i64; 3] = {
            let axes = &gather1_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather3_out1 = shape2_out1[1] as i64;
        let gather4_out1 = shape3_out1[2] as i64;
        let gather5_out1 = shape4_out1[2] as i64;
        let gather6_out1 = shape5_out1[2] as i64;
        let gather7_out1 = shape6_out1[2] as i64;
        let gather8_out1 = shape7_out1[2] as i64;
        let gather9_out1 = shape8_out1[2] as i64;
        let gather10_out1 = shape9_out1[2] as i64;
        let gather11_out1 = shape10_out1[2] as i64;
        let gather12_out1 = shape11_out1[2] as i64;
        let gather13_out1 = shape12_out1[2] as i64;
        let gather14_out1 = shape13_out1[2] as i64;
        let gather15_out1 = shape14_out1[2] as i64;
        let gather16_out1 = shape15_out1[2] as i64;
        let gather17_out1 = shape16_out1[2] as i64;
        let gather18_out1 = shape17_out1[2] as i64;
        let gather19_out1 = shape18_out1[2] as i64;
        let gather20_out1 = shape19_out1[2] as i64;
        let gather21_out1 = shape20_out1[2] as i64;
        let gather22_out1 = shape21_out1[2] as i64;
        let gather23_out1 = shape22_out1[2] as i64;
        let gather24_out1 = shape23_out1[2] as i64;
        let gather25_out1 = shape24_out1[2] as i64;
        let gather26_out1 = shape25_out1[2] as i64;
        let unsqueeze2_out1: Tensor<4, Int> = unsqueeze1_out1.unsqueeze_dims::<4>(&[2]);
        let constant29_out1 = self.constant29.val();
        let pow1_out1 = gather1_out1
            .clone()
            .powf((constant29_out1).unsqueeze_dims(&[0isize, 1isize]));
        let gather27_out1 = shape26_out1[1] as i64;
        let unsqueeze3_out1 = [gather3_out1 as i64];
        let gather28_out1 = shape26_out1[0] as i64;
        let cast1_out1 = unsqueeze2_out1.float().cast(burn::tensor::DType::F32);
        let range1_out1 = {
            let __start = 0i64;
            let __limit = gather3_out1;
            let __delta = 1i64;
            assert!(__delta != 0);
            let __n = ((__limit - __start) as f64 / __delta as f64).ceil().max(0.0)
                as i64;
            Tensor::arange(0..__n, &self.device)
                .cast(burn::tensor::DType::I64)
                .mul_scalar(__delta)
                .add_scalar(__start)
        };
        let reducemean1_out1 = { pow1_out1.mean_dim(2usize) };
        let add1_out1 = gather2_out1 + gather27_out1;
        let unsqueeze4_out1 = [gather27_out1 as i64];
        let unsqueeze5_out1 = [gather28_out1 as i64];
        let constant37_out1 = self.constant37.val();
        let add2_out1 = reducemean1_out1
            .add((constant37_out1).unsqueeze_dims(&[0isize, 1isize]));
        let concat1_out1: [i64; 2usize] = [&unsqueeze4_out1[..], &unsqueeze3_out1[..]]
            .concat()
            .try_into()
            .unwrap();
        let constant38_out1: [i64; 1] = [1i64];
        let constant39_out1: [i64; 1] = [-1i64];
        let constant40_out1: [i64; 1] = [-1i64];
        let concat2_out1: [i64; 4usize] = [
            &unsqueeze5_out1[..],
            &constant38_out1[..],
            &constant39_out1[..],
            &constant40_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let range2_out1 = {
            let __start = gather2_out1;
            let __limit = add1_out1;
            let __delta = 1i64;
            assert!(__delta != 0);
            let __n = ((__limit - __start) as f64 / __delta as f64).ceil().max(0.0)
                as i64;
            Tensor::arange(0..__n, &self.device)
                .cast(burn::tensor::DType::I64)
                .mul_scalar(__delta)
                .add_scalar(__start)
        };
        let sqrt1_out1 = add2_out1.sqrt();
        let constantofshape1_out1 = Tensor::<
            1,
        >::from_data(
                burn::tensor::TensorData::from([
                    -340282350000000000000000000000000000000f32 as f64,
                ]),
                (&self.device, burn::tensor::DType::F32),
            )
            .reshape([1, 1])
            .expand(concat1_out1);
        let reshape1_out1 = range2_out1.reshape([-1, 1]);
        let constant43_out1 = self.constant43.val();
        let div1_out1 = (constant43_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt1_out1);
        let constant44_out1 = self.constant44.val();
        let equal1_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat2_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant44_out1)
        };
        let trilu1_out1 = constantofshape1_out1.triu(1);
        let greater1_out1 = (range1_out1)
            .unsqueeze_dims(&[0isize])
            .greater(reshape1_out1);
        let mul1_out1 = gather1_out1.clone().mul(div1_out1);
        let constant46_out1 = self.constant46.val();
        let where1_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat2_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal1_out1, constant46_out1);
        let cast2_out1 = greater1_out1.float().cast(burn::tensor::DType::F32);
        let constant47_out1 = self.constant47.val();
        let mul2_out1 = (constant47_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul1_out1);
        let mul3_out1 = trilu1_out1.mul(cast2_out1);
        let shape27_out1: [i64; 3] = {
            let axes = &mul2_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear1_out1 = self.linear1.forward(mul2_out1.clone());
        let linear2_out1 = self.linear2.forward(mul2_out1.clone());
        let linear3_out1 = self.linear3.forward(mul2_out1);
        let unsqueeze6_out1: Tensor<3> = mul3_out1.unsqueeze_dims::<3>(&[0]);
        let gather29_out1 = shape27_out1[0] as i64;
        let gather30_out1 = shape27_out1[1] as i64;
        let constant54_out1 = self.constant54.val();
        let add3_out1 = (constant54_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear1_out1);
        let constant55_out1 = self.constant55.val();
        let add4_out1 = (constant55_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear2_out1);
        let constant56_out1 = self.constant56.val();
        let add5_out1 = (constant56_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear3_out1);
        let unsqueeze7_out1: Tensor<4> = unsqueeze6_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze8_out1 = [gather29_out1 as i64];
        let unsqueeze9_out1 = [gather30_out1 as i64];
        let expand1_out1 = {
            let onnx_shape: [i64; 4usize] = TryInto::<
                [i64; 4usize],
            >::try_into(where1_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze7_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..4usize {
                let dim_offset = 4usize - 4usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze7_out1.expand(shape)
        };
        let constant60_out1: [i64; 1] = [14i64];
        let constant61_out1: [i64; 1] = [64i64];
        let concat3_out1: [i64; 4usize] = [
            &unsqueeze8_out1[..],
            &unsqueeze9_out1[..],
            &constant60_out1[..],
            &constant61_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant62_out1: [i64; 1] = [2i64];
        let constant63_out1: [i64; 1] = [64i64];
        let concat4_out1: [i64; 4usize] = [
            &unsqueeze8_out1[..],
            &unsqueeze9_out1[..],
            &constant62_out1[..],
            &constant63_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant64_out1: [i64; 1] = [896i64];
        let concat5_out1: [i64; 3usize] = [
            &unsqueeze8_out1[..],
            &unsqueeze9_out1[..],
            &constant64_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let slice1_out1 = expand1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze3_out1[0]]);
        let shape28_out1: [i64; 4] = {
            let axes = &expand1_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let reshape2_out1 = add3_out1.reshape(concat3_out1);
        let reshape3_out1 = add4_out1.reshape(concat4_out1);
        let reshape4_out1 = add5_out1.reshape(concat4_out1);
        let add6_out1 = slice1_out1.clone().add(cast1_out1);
        let shape29_out1: [i64; 4] = {
            let axes = &slice1_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather31_out1 = shape28_out1[0] as i64;
        let gather32_out1 = shape28_out1[2] as i64;
        let gather33_out1 = shape28_out1[3] as i64;
        let transpose2_out1 = reshape2_out1.permute([0, 2, 1, 3]);
        let transpose3_out1 = reshape3_out1.permute([0, 2, 1, 3]);
        let transpose4_out1 = reshape4_out1.permute([0, 2, 1, 3]);
        let constant71_out1 = 0f32;
        let equal2_out1 = add6_out1.equal_elem(constant71_out1);
        let range3_out1 = {
            let __start = 0i64;
            let __limit = gather31_out1;
            let __delta = 1i64;
            assert!(__delta != 0);
            let __n = ((__limit - __start) as f64 / __delta as f64).ceil().max(0.0)
                as i64;
            Tensor::arange(0..__n, &self.device)
                .cast(burn::tensor::DType::I64)
                .mul_scalar(__delta)
                .add_scalar(__start)
        };
        let range4_out1 = {
            let __start = 0i64;
            let __limit = gather32_out1;
            let __delta = 1i64;
            assert!(__delta != 0);
            let __n = ((__limit - __start) as f64 / __delta as f64).ceil().max(0.0)
                as i64;
            Tensor::arange(0..__n, &self.device)
                .cast(burn::tensor::DType::I64)
                .mul_scalar(__delta)
                .add_scalar(__start)
        };
        let range5_out1 = {
            let __start = 0i64;
            let __limit = gather33_out1;
            let __delta = 1i64;
            assert!(__delta != 0);
            let __n = ((__limit - __start) as f64 / __delta as f64).ceil().max(0.0)
                as i64;
            Tensor::arange(0..__n, &self.device)
                .cast(burn::tensor::DType::I64)
                .mul_scalar(__delta)
                .add_scalar(__start)
        };
        let shape30_out1: [i64; 4] = {
            let axes = &transpose3_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat6_out1 = burn::tensor::Tensor::cat(
            [past_key_values_0_value, transpose4_out1].into(),
            2,
        );
        let slice2_out1 = transpose2_out1.clone().slice(s![.., .., .., 0..32]);
        let slice3_out1 = transpose2_out1.clone().slice(s![.., .., .., 32..]);
        let slice4_out1 = transpose3_out1.clone().slice(s![.., .., .., 0..32]);
        let slice5_out1 = transpose3_out1.clone().slice(s![.., .., .., 32..]);
        let constant94_out1 = -340282350000000000000000000000000000000f32;
        let where2_out1 = slice1_out1.mask_fill(equal2_out1, constant94_out1);
        let slice6_out1 = range5_out1.slice(s![0..unsqueeze3_out1[0]]);
        let reshape5_out1 = range3_out1.reshape([-1, 1, 1, 1]);
        let reshape6_out1 = range4_out1.reshape([-1, 1]);
        let gather34_out1 = shape30_out1[2] as i64;
        let shape31_out1: [i64; 4] = {
            let axes = &concat6_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze10_out1: Tensor<5> = concat6_out1.clone().unsqueeze_dims::<5>(&[2]);
        let neg1_out1 = slice3_out1.neg();
        let neg2_out1 = slice5_out1.neg();
        let expand2_out1 = {
            let onnx_shape: [i64; 4usize] = shape29_out1;
            let input_dims = where2_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..4usize {
                let dim_offset = 4usize - 4usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            where2_out1.expand(shape)
        };
        let add8_out1 = gather34_out1 + gather2_out1;
        let gather35_out1 = shape31_out1[0] as i64;
        let gather36_out1 = shape31_out1[2] as i64;
        let concat7_out1 = burn::tensor::Tensor::cat([neg1_out1, slice2_out1].into(), 3);
        let concat8_out1 = burn::tensor::Tensor::cat([neg2_out1, slice4_out1].into(), 3);
        let add9_out1 = reshape5_out1
            .clone()
            .add((reshape6_out1.clone()).unsqueeze_dims(&[0isize, 1isize]));
        let unsqueeze11_out1 = [add8_out1 as i64];
        let unsqueeze12_out1 = [gather35_out1 as i64];
        let unsqueeze13_out1 = [gather36_out1 as i64];
        let add10_out1 = add9_out1
            .add((slice6_out1.clone()).unsqueeze_dims(&[0isize, 1isize, 2isize]));
        let constant108_out1 = self.constant108.val();
        let slice7_out1 = constant108_out1.clone().slice(s![0..unsqueeze11_out1[0], ..]);
        let constant112_out1 = self.constant112.val();
        let slice8_out1 = constant112_out1.clone().slice(s![0..unsqueeze11_out1[0], ..]);
        let constant116_out1: [i64; 1] = [2i64];
        let constant117_out1: [i64; 1] = [7i64];
        let constant118_out1: [i64; 1] = [64i64];
        let concat9_out1: [i64; 5usize] = [
            &unsqueeze12_out1[..],
            &constant116_out1[..],
            &constant117_out1[..],
            &unsqueeze13_out1[..],
            &constant118_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant119_out1: [i64; 1] = [14i64];
        let constant120_out1: [i64; 1] = [64i64];
        let concat10_out1: [i64; 4usize] = [
            &unsqueeze12_out1[..],
            &constant119_out1[..],
            &unsqueeze13_out1[..],
            &constant120_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let shape32_out1: [i64; 4] = {
            let axes = &add10_out1.dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather37_out1 = slice7_out1.take::<2, 3>(0, position_ids.clone());
        let gather38_out1 = slice8_out1.take::<2, 3>(0, position_ids.clone());
        let constant121_out1 = self.constant121.val();
        let equal3_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat9_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant121_out1)
        };
        let constant122_out1 = self.constant122.val();
        let equal4_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(shape32_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant122_out1)
        };
        let unsqueeze14_out1: Tensor<4> = gather37_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze15_out1: Tensor<4> = gather38_out1.unsqueeze_dims::<4>(&[1]);
        let constant125_out1 = self.constant125.val();
        let where3_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat9_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal3_out1, constant125_out1);
        let constant126_out1 = self.constant126.val();
        let where4_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&shape32_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal4_out1, constant126_out1);
        let mul4_out1 = transpose2_out1.mul(unsqueeze14_out1.clone());
        let mul5_out1 = transpose3_out1.mul(unsqueeze14_out1);
        let mul6_out1 = concat7_out1.mul(unsqueeze15_out1.clone());
        let mul7_out1 = concat8_out1.mul(unsqueeze15_out1);
        let expand3_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where3_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze10_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze10_out1.expand(shape)
        };
        let expand4_out1 = {
            let onnx_shape: [i64; 4usize] = TryInto::<
                [i64; 4usize],
            >::try_into(where4_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = reshape5_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..4usize {
                let dim_offset = 4usize - 4usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            reshape5_out1.expand(shape)
        };
        let constant102_out1 = self.constant102.val();
        let expand5_out1 = {
            let onnx_shape: [i64; 4usize] = TryInto::<
                [i64; 4usize],
            >::try_into(where4_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = constant102_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..3usize {
                let dim_offset = 4usize - 3usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            constant102_out1.expand(shape)
        };
        let expand6_out1 = {
            let onnx_shape: [i64; 4usize] = TryInto::<
                [i64; 4usize],
            >::try_into(where4_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = reshape6_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..2usize {
                let dim_offset = 4usize - 2usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            reshape6_out1.expand(shape)
        };
        let expand7_out1 = {
            let onnx_shape: [i64; 4usize] = TryInto::<
                [i64; 4usize],
            >::try_into(where4_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = slice6_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..1usize {
                let dim_offset = 4usize - 1usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            slice6_out1.expand(shape)
        };
        let add11_out1 = mul4_out1.add(mul6_out1);
        let add12_out1 = mul5_out1.add(mul7_out1);
        let reshape8_out1 = expand3_out1.reshape(concat10_out1);
        let unsqueeze16_out1: Tensor<5, Int> = expand4_out1.unsqueeze_dims::<5>(&[-1]);
        let unsqueeze17_out1: Tensor<5, Int> = expand5_out1.unsqueeze_dims::<5>(&[-1]);
        let unsqueeze18_out1: Tensor<5, Int> = expand6_out1.unsqueeze_dims::<5>(&[-1]);
        let unsqueeze19_out1: Tensor<5, Int> = expand7_out1.unsqueeze_dims::<5>(&[-1]);
        let concat11_out1 = burn::tensor::Tensor::cat(
            [past_key_values_0_key, add12_out1].into(),
            2,
        );
        let concat12_out1 = burn::tensor::Tensor::cat(
            [unsqueeze16_out1, unsqueeze17_out1, unsqueeze18_out1, unsqueeze19_out1]
                .into(),
            4,
        );
        let shape33_out1: [i64; 4] = {
            let axes = &concat11_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze20_out1: Tensor<5> = concat11_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let scatternd1_out1 = {
            let __nd_data_dims = expand1_out1.dims();
            let __nd_indices = concat12_out1.cast(burn::tensor::DType::I64);
            let __nd_idx_dims = __nd_indices.dims();
            let __nd_k = __nd_idx_dims[5 - 1];
            let mut __nd_dim_sizes: alloc::vec::Vec<i64> = alloc::vec::Vec::with_capacity(
                __nd_k,
            );
            for __nd_i in 0..__nd_k {
                __nd_dim_sizes.push(__nd_data_dims[0 + __nd_i] as i64);
            }
            let mut __nd_bcast_shape = [1usize; 5];
            __nd_bcast_shape[5 - 1] = __nd_k;
            let __nd_dims_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                    burn::tensor::TensorData::from(__nd_dim_sizes.as_slice()),
                    (&self.device, burn::tensor::DType::I64),
                )
                .reshape(__nd_bcast_shape);
            let __nd_mask = __nd_indices.clone().lower_elem(0i64);
            let __nd_corrected = __nd_indices.clone() + __nd_dims_tensor;
            let __nd_indices_norm = __nd_indices.mask_where(__nd_mask, __nd_corrected);
            expand1_out1
                .scatter_nd(
                    __nd_indices_norm,
                    expand2_out1,
                    burn::tensor::IndexingUpdateOp::Assign,
                )
        };
        let gather39_out1 = shape33_out1[0] as i64;
        let gather40_out1 = shape33_out1[2] as i64;
        let unsqueeze21_out1 = [gather39_out1 as i64];
        let unsqueeze22_out1 = [gather40_out1 as i64];
        let constant137_out1: [i64; 1] = [2i64];
        let constant138_out1: [i64; 1] = [7i64];
        let constant139_out1: [i64; 1] = [64i64];
        let concat13_out1: [i64; 5usize] = [
            &unsqueeze21_out1[..],
            &constant137_out1[..],
            &constant138_out1[..],
            &unsqueeze22_out1[..],
            &constant139_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant140_out1: [i64; 1] = [14i64];
        let constant141_out1: [i64; 1] = [64i64];
        let concat14_out1: [i64; 4usize] = [
            &unsqueeze21_out1[..],
            &constant140_out1[..],
            &unsqueeze22_out1[..],
            &constant141_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant142_out1 = self.constant142.val();
        let equal5_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat13_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant142_out1)
        };
        let constant143_out1 = self.constant143.val();
        let where5_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat13_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal5_out1, constant143_out1);
        let expand8_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where5_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze20_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze20_out1.expand(shape)
        };
        let reshape9_out1 = expand8_out1.reshape(concat14_out1);
        let shape34_out1: [i64; 4] = {
            let axes = &reshape9_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather41_out1 = shape34_out1[2] as i64;
        let unsqueeze23_out1 = [gather41_out1 as i64];
        let slice9_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze23_out1[0]]);
        let (matmul5_out1,) = {
            let q = add11_out1;
            let k = reshape9_out1;
            let v = reshape8_out1;
            let matmul5_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice9_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul5_out1,)
        };
        let transpose6_out1 = matmul5_out1.permute([0, 2, 1, 3]);
        let reshape10_out1 = transpose6_out1.reshape(concat5_out1);
        let linear4_out1 = self.linear4.forward(reshape10_out1);
        let add14_out1 = gather1_out1.add(linear4_out1);
        let constant151_out1 = self.constant151.val();
        let pow2_out1 = add14_out1
            .clone()
            .powf((constant151_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean2_out1 = { pow2_out1.mean_dim(2usize) };
        let constant152_out1 = self.constant152.val();
        let add15_out1 = reducemean2_out1
            .add((constant152_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt2_out1 = add15_out1.sqrt();
        let constant153_out1 = self.constant153.val();
        let div2_out1 = (constant153_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt2_out1);
        let mul10_out1 = add14_out1.clone().mul(div2_out1);
        let constant154_out1 = self.constant154.val();
        let mul11_out1 = (constant154_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul10_out1);
        let linear5_out1 = self.linear5.forward(mul11_out1.clone());
        let linear6_out1 = self.linear6.forward(mul11_out1);
        let sigmoid1_out1 = burn::tensor::activation::sigmoid(linear5_out1.clone());
        let mul12_out1 = linear5_out1.mul(sigmoid1_out1);
        let mul13_out1 = mul12_out1.mul(linear6_out1);
        let linear7_out1 = self.linear7.forward(mul13_out1);
        let add16_out1 = add14_out1.add(linear7_out1);
        let constant158_out1 = self.constant158.val();
        let pow3_out1 = add16_out1
            .clone()
            .powf((constant158_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean3_out1 = { pow3_out1.mean_dim(2usize) };
        let constant159_out1 = self.constant159.val();
        let add17_out1 = reducemean3_out1
            .add((constant159_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt3_out1 = add17_out1.sqrt();
        let constant160_out1 = self.constant160.val();
        let div3_out1 = (constant160_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt3_out1);
        let mul14_out1 = add16_out1.clone().mul(div3_out1);
        let constant161_out1 = self.constant161.val();
        let mul15_out1 = (constant161_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul14_out1);
        let shape35_out1: [i64; 3] = {
            let axes = &mul15_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear8_out1 = self.linear8.forward(mul15_out1.clone());
        let linear9_out1 = self.linear9.forward(mul15_out1.clone());
        let linear10_out1 = self.linear10.forward(mul15_out1);
        let gather42_out1 = shape35_out1[0] as i64;
        let gather43_out1 = shape35_out1[1] as i64;
        let constant167_out1 = self.constant167.val();
        let add18_out1 = (constant167_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear8_out1);
        let constant168_out1 = self.constant168.val();
        let add19_out1 = (constant168_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear9_out1);
        let constant169_out1 = self.constant169.val();
        let add20_out1 = (constant169_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear10_out1);
        let unsqueeze24_out1 = [gather42_out1 as i64];
        let unsqueeze25_out1 = [gather43_out1 as i64];
        let constant172_out1: [i64; 1] = [14i64];
        let constant173_out1: [i64; 1] = [64i64];
        let concat15_out1: [i64; 4usize] = [
            &unsqueeze24_out1[..],
            &unsqueeze25_out1[..],
            &constant172_out1[..],
            &constant173_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant174_out1: [i64; 1] = [2i64];
        let constant175_out1: [i64; 1] = [64i64];
        let concat16_out1: [i64; 4usize] = [
            &unsqueeze24_out1[..],
            &unsqueeze25_out1[..],
            &constant174_out1[..],
            &constant175_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant176_out1: [i64; 1] = [896i64];
        let concat17_out1: [i64; 3usize] = [
            &unsqueeze24_out1[..],
            &unsqueeze25_out1[..],
            &constant176_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape11_out1 = add18_out1.reshape(concat15_out1);
        let reshape12_out1 = add19_out1.reshape(concat16_out1);
        let reshape13_out1 = add20_out1.reshape(concat16_out1);
        let transpose7_out1 = reshape11_out1.permute([0, 2, 1, 3]);
        let transpose8_out1 = reshape12_out1.permute([0, 2, 1, 3]);
        let transpose9_out1 = reshape13_out1.permute([0, 2, 1, 3]);
        let shape36_out1: [i64; 4] = {
            let axes = &transpose8_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat18_out1 = burn::tensor::Tensor::cat(
            [past_key_values_1_value, transpose9_out1].into(),
            2,
        );
        let slice10_out1 = transpose7_out1.clone().slice(s![.., .., .., 0..32]);
        let slice11_out1 = transpose7_out1.clone().slice(s![.., .., .., 32..]);
        let slice12_out1 = transpose8_out1.clone().slice(s![.., .., .., 0..32]);
        let slice13_out1 = transpose8_out1.clone().slice(s![.., .., .., 32..]);
        let gather44_out1 = shape36_out1[2] as i64;
        let shape37_out1: [i64; 4] = {
            let axes = &concat18_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze26_out1: Tensor<5> = concat18_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg3_out1 = slice11_out1.neg();
        let neg4_out1 = slice13_out1.neg();
        let add21_out1 = gather44_out1 + gather4_out1;
        let gather45_out1 = shape37_out1[0] as i64;
        let gather46_out1 = shape37_out1[2] as i64;
        let concat19_out1 = burn::tensor::Tensor::cat(
            [neg3_out1, slice10_out1].into(),
            3,
        );
        let concat20_out1 = burn::tensor::Tensor::cat(
            [neg4_out1, slice12_out1].into(),
            3,
        );
        let unsqueeze27_out1 = [add21_out1 as i64];
        let unsqueeze28_out1 = [gather45_out1 as i64];
        let unsqueeze29_out1 = [gather46_out1 as i64];
        let slice14_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze27_out1[0], ..]);
        let slice15_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze27_out1[0], ..]);
        let constant206_out1: [i64; 1] = [2i64];
        let constant207_out1: [i64; 1] = [7i64];
        let constant208_out1: [i64; 1] = [64i64];
        let concat21_out1: [i64; 5usize] = [
            &unsqueeze28_out1[..],
            &constant206_out1[..],
            &constant207_out1[..],
            &unsqueeze29_out1[..],
            &constant208_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant209_out1: [i64; 1] = [14i64];
        let constant210_out1: [i64; 1] = [64i64];
        let concat22_out1: [i64; 4usize] = [
            &unsqueeze28_out1[..],
            &constant209_out1[..],
            &unsqueeze29_out1[..],
            &constant210_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather47_out1 = slice14_out1.take::<2, 3>(0, position_ids.clone());
        let gather48_out1 = slice15_out1.take::<2, 3>(0, position_ids.clone());
        let constant211_out1 = self.constant211.val();
        let equal6_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat21_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant211_out1)
        };
        let unsqueeze30_out1: Tensor<4> = gather47_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze31_out1: Tensor<4> = gather48_out1.unsqueeze_dims::<4>(&[1]);
        let constant214_out1 = self.constant214.val();
        let where6_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat21_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal6_out1, constant214_out1);
        let mul16_out1 = transpose7_out1.mul(unsqueeze30_out1.clone());
        let mul17_out1 = transpose8_out1.mul(unsqueeze30_out1);
        let mul18_out1 = concat19_out1.mul(unsqueeze31_out1.clone());
        let mul19_out1 = concat20_out1.mul(unsqueeze31_out1);
        let expand9_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where6_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze26_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze26_out1.expand(shape)
        };
        let add22_out1 = mul16_out1.add(mul18_out1);
        let add23_out1 = mul17_out1.add(mul19_out1);
        let reshape14_out1 = expand9_out1.reshape(concat22_out1);
        let concat23_out1 = burn::tensor::Tensor::cat(
            [past_key_values_1_key, add23_out1].into(),
            2,
        );
        let shape38_out1: [i64; 4] = {
            let axes = &concat23_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze32_out1: Tensor<5> = concat23_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather49_out1 = shape38_out1[0] as i64;
        let gather50_out1 = shape38_out1[2] as i64;
        let unsqueeze33_out1 = [gather49_out1 as i64];
        let unsqueeze34_out1 = [gather50_out1 as i64];
        let constant221_out1: [i64; 1] = [2i64];
        let constant222_out1: [i64; 1] = [7i64];
        let constant223_out1: [i64; 1] = [64i64];
        let concat24_out1: [i64; 5usize] = [
            &unsqueeze33_out1[..],
            &constant221_out1[..],
            &constant222_out1[..],
            &unsqueeze34_out1[..],
            &constant223_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant224_out1: [i64; 1] = [14i64];
        let constant225_out1: [i64; 1] = [64i64];
        let concat25_out1: [i64; 4usize] = [
            &unsqueeze33_out1[..],
            &constant224_out1[..],
            &unsqueeze34_out1[..],
            &constant225_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant226_out1 = self.constant226.val();
        let equal7_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat24_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant226_out1)
        };
        let constant227_out1 = self.constant227.val();
        let where7_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat24_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal7_out1, constant227_out1);
        let expand10_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where7_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze32_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze32_out1.expand(shape)
        };
        let reshape15_out1 = expand10_out1.reshape(concat25_out1);
        let shape39_out1: [i64; 4] = {
            let axes = &reshape15_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather51_out1 = shape39_out1[2] as i64;
        let unsqueeze35_out1 = [gather51_out1 as i64];
        let slice16_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze35_out1[0]]);
        let (matmul14_out1,) = {
            let q = add22_out1;
            let k = reshape15_out1;
            let v = reshape14_out1;
            let matmul14_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice16_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul14_out1,)
        };
        let transpose11_out1 = matmul14_out1.permute([0, 2, 1, 3]);
        let reshape16_out1 = transpose11_out1.reshape(concat17_out1);
        let linear11_out1 = self.linear11.forward(reshape16_out1);
        let add25_out1 = add16_out1.add(linear11_out1);
        let constant235_out1 = self.constant235.val();
        let pow4_out1 = add25_out1
            .clone()
            .powf((constant235_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean4_out1 = { pow4_out1.mean_dim(2usize) };
        let constant236_out1 = self.constant236.val();
        let add26_out1 = reducemean4_out1
            .add((constant236_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt4_out1 = add26_out1.sqrt();
        let constant237_out1 = self.constant237.val();
        let div4_out1 = (constant237_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt4_out1);
        let mul22_out1 = add25_out1.clone().mul(div4_out1);
        let constant238_out1 = self.constant238.val();
        let mul23_out1 = (constant238_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul22_out1);
        let linear12_out1 = self.linear12.forward(mul23_out1.clone());
        let linear13_out1 = self.linear13.forward(mul23_out1);
        let sigmoid2_out1 = burn::tensor::activation::sigmoid(linear12_out1.clone());
        let mul24_out1 = linear12_out1.mul(sigmoid2_out1);
        let mul25_out1 = mul24_out1.mul(linear13_out1);
        let linear14_out1 = self.linear14.forward(mul25_out1);
        let add27_out1 = add25_out1.add(linear14_out1);
        let constant242_out1 = self.constant242.val();
        let pow5_out1 = add27_out1
            .clone()
            .powf((constant242_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean5_out1 = { pow5_out1.mean_dim(2usize) };
        let constant243_out1 = self.constant243.val();
        let add28_out1 = reducemean5_out1
            .add((constant243_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt5_out1 = add28_out1.sqrt();
        let constant244_out1 = self.constant244.val();
        let div5_out1 = (constant244_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt5_out1);
        let mul26_out1 = add27_out1.clone().mul(div5_out1);
        let constant245_out1 = self.constant245.val();
        let mul27_out1 = (constant245_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul26_out1);
        let shape40_out1: [i64; 3] = {
            let axes = &mul27_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear15_out1 = self.linear15.forward(mul27_out1.clone());
        let linear16_out1 = self.linear16.forward(mul27_out1.clone());
        let linear17_out1 = self.linear17.forward(mul27_out1);
        let gather52_out1 = shape40_out1[0] as i64;
        let gather53_out1 = shape40_out1[1] as i64;
        let constant251_out1 = self.constant251.val();
        let add29_out1 = (constant251_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear15_out1);
        let constant252_out1 = self.constant252.val();
        let add30_out1 = (constant252_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear16_out1);
        let constant253_out1 = self.constant253.val();
        let add31_out1 = (constant253_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear17_out1);
        let unsqueeze36_out1 = [gather52_out1 as i64];
        let unsqueeze37_out1 = [gather53_out1 as i64];
        let constant256_out1: [i64; 1] = [14i64];
        let constant257_out1: [i64; 1] = [64i64];
        let concat26_out1: [i64; 4usize] = [
            &unsqueeze36_out1[..],
            &unsqueeze37_out1[..],
            &constant256_out1[..],
            &constant257_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant258_out1: [i64; 1] = [2i64];
        let constant259_out1: [i64; 1] = [64i64];
        let concat27_out1: [i64; 4usize] = [
            &unsqueeze36_out1[..],
            &unsqueeze37_out1[..],
            &constant258_out1[..],
            &constant259_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant260_out1: [i64; 1] = [896i64];
        let concat28_out1: [i64; 3usize] = [
            &unsqueeze36_out1[..],
            &unsqueeze37_out1[..],
            &constant260_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape17_out1 = add29_out1.reshape(concat26_out1);
        let reshape18_out1 = add30_out1.reshape(concat27_out1);
        let reshape19_out1 = add31_out1.reshape(concat27_out1);
        let transpose12_out1 = reshape17_out1.permute([0, 2, 1, 3]);
        let transpose13_out1 = reshape18_out1.permute([0, 2, 1, 3]);
        let transpose14_out1 = reshape19_out1.permute([0, 2, 1, 3]);
        let shape41_out1: [i64; 4] = {
            let axes = &transpose13_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat29_out1 = burn::tensor::Tensor::cat(
            [past_key_values_2_value, transpose14_out1].into(),
            2,
        );
        let slice17_out1 = transpose12_out1.clone().slice(s![.., .., .., 0..32]);
        let slice18_out1 = transpose12_out1.clone().slice(s![.., .., .., 32..]);
        let slice19_out1 = transpose13_out1.clone().slice(s![.., .., .., 0..32]);
        let slice20_out1 = transpose13_out1.clone().slice(s![.., .., .., 32..]);
        let gather54_out1 = shape41_out1[2] as i64;
        let shape42_out1: [i64; 4] = {
            let axes = &concat29_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze38_out1: Tensor<5> = concat29_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg5_out1 = slice18_out1.neg();
        let neg6_out1 = slice20_out1.neg();
        let add32_out1 = gather54_out1 + gather5_out1;
        let gather55_out1 = shape42_out1[0] as i64;
        let gather56_out1 = shape42_out1[2] as i64;
        let concat30_out1 = burn::tensor::Tensor::cat(
            [neg5_out1, slice17_out1].into(),
            3,
        );
        let concat31_out1 = burn::tensor::Tensor::cat(
            [neg6_out1, slice19_out1].into(),
            3,
        );
        let unsqueeze39_out1 = [add32_out1 as i64];
        let unsqueeze40_out1 = [gather55_out1 as i64];
        let unsqueeze41_out1 = [gather56_out1 as i64];
        let slice21_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze39_out1[0], ..]);
        let slice22_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze39_out1[0], ..]);
        let constant290_out1: [i64; 1] = [2i64];
        let constant291_out1: [i64; 1] = [7i64];
        let constant292_out1: [i64; 1] = [64i64];
        let concat32_out1: [i64; 5usize] = [
            &unsqueeze40_out1[..],
            &constant290_out1[..],
            &constant291_out1[..],
            &unsqueeze41_out1[..],
            &constant292_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant293_out1: [i64; 1] = [14i64];
        let constant294_out1: [i64; 1] = [64i64];
        let concat33_out1: [i64; 4usize] = [
            &unsqueeze40_out1[..],
            &constant293_out1[..],
            &unsqueeze41_out1[..],
            &constant294_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather57_out1 = slice21_out1.take::<2, 3>(0, position_ids.clone());
        let gather58_out1 = slice22_out1.take::<2, 3>(0, position_ids.clone());
        let constant295_out1 = self.constant295.val();
        let equal8_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat32_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant295_out1)
        };
        let unsqueeze42_out1: Tensor<4> = gather57_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze43_out1: Tensor<4> = gather58_out1.unsqueeze_dims::<4>(&[1]);
        let constant298_out1 = self.constant298.val();
        let where8_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat32_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal8_out1, constant298_out1);
        let mul28_out1 = transpose12_out1.mul(unsqueeze42_out1.clone());
        let mul29_out1 = transpose13_out1.mul(unsqueeze42_out1);
        let mul30_out1 = concat30_out1.mul(unsqueeze43_out1.clone());
        let mul31_out1 = concat31_out1.mul(unsqueeze43_out1);
        let expand11_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where8_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze38_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze38_out1.expand(shape)
        };
        let add33_out1 = mul28_out1.add(mul30_out1);
        let add34_out1 = mul29_out1.add(mul31_out1);
        let reshape20_out1 = expand11_out1.reshape(concat33_out1);
        let concat34_out1 = burn::tensor::Tensor::cat(
            [past_key_values_2_key, add34_out1].into(),
            2,
        );
        let shape43_out1: [i64; 4] = {
            let axes = &concat34_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze44_out1: Tensor<5> = concat34_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather59_out1 = shape43_out1[0] as i64;
        let gather60_out1 = shape43_out1[2] as i64;
        let unsqueeze45_out1 = [gather59_out1 as i64];
        let unsqueeze46_out1 = [gather60_out1 as i64];
        let constant305_out1: [i64; 1] = [2i64];
        let constant306_out1: [i64; 1] = [7i64];
        let constant307_out1: [i64; 1] = [64i64];
        let concat35_out1: [i64; 5usize] = [
            &unsqueeze45_out1[..],
            &constant305_out1[..],
            &constant306_out1[..],
            &unsqueeze46_out1[..],
            &constant307_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant308_out1: [i64; 1] = [14i64];
        let constant309_out1: [i64; 1] = [64i64];
        let concat36_out1: [i64; 4usize] = [
            &unsqueeze45_out1[..],
            &constant308_out1[..],
            &unsqueeze46_out1[..],
            &constant309_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant310_out1 = self.constant310.val();
        let equal9_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat35_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant310_out1)
        };
        let constant311_out1 = self.constant311.val();
        let where9_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat35_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal9_out1, constant311_out1);
        let expand12_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where9_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze44_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze44_out1.expand(shape)
        };
        let reshape21_out1 = expand12_out1.reshape(concat36_out1);
        let shape44_out1: [i64; 4] = {
            let axes = &reshape21_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather61_out1 = shape44_out1[2] as i64;
        let unsqueeze47_out1 = [gather61_out1 as i64];
        let slice23_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze47_out1[0]]);
        let (matmul23_out1,) = {
            let q = add33_out1;
            let k = reshape21_out1;
            let v = reshape20_out1;
            let matmul23_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice23_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul23_out1,)
        };
        let transpose16_out1 = matmul23_out1.permute([0, 2, 1, 3]);
        let reshape22_out1 = transpose16_out1.reshape(concat28_out1);
        let linear18_out1 = self.linear18.forward(reshape22_out1);
        let add36_out1 = add27_out1.add(linear18_out1);
        let constant319_out1 = self.constant319.val();
        let pow6_out1 = add36_out1
            .clone()
            .powf((constant319_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean6_out1 = { pow6_out1.mean_dim(2usize) };
        let constant320_out1 = self.constant320.val();
        let add37_out1 = reducemean6_out1
            .add((constant320_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt6_out1 = add37_out1.sqrt();
        let constant321_out1 = self.constant321.val();
        let div6_out1 = (constant321_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt6_out1);
        let mul34_out1 = add36_out1.clone().mul(div6_out1);
        let constant322_out1 = self.constant322.val();
        let mul35_out1 = (constant322_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul34_out1);
        let linear19_out1 = self.linear19.forward(mul35_out1.clone());
        let linear20_out1 = self.linear20.forward(mul35_out1);
        let sigmoid3_out1 = burn::tensor::activation::sigmoid(linear19_out1.clone());
        let mul36_out1 = linear19_out1.mul(sigmoid3_out1);
        let mul37_out1 = mul36_out1.mul(linear20_out1);
        let linear21_out1 = self.linear21.forward(mul37_out1);
        let add38_out1 = add36_out1.add(linear21_out1);
        let constant326_out1 = self.constant326.val();
        let pow7_out1 = add38_out1
            .clone()
            .powf((constant326_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean7_out1 = { pow7_out1.mean_dim(2usize) };
        let constant327_out1 = self.constant327.val();
        let add39_out1 = reducemean7_out1
            .add((constant327_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt7_out1 = add39_out1.sqrt();
        let constant328_out1 = self.constant328.val();
        let div7_out1 = (constant328_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt7_out1);
        let mul38_out1 = add38_out1.clone().mul(div7_out1);
        let constant329_out1 = self.constant329.val();
        let mul39_out1 = (constant329_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul38_out1);
        let shape45_out1: [i64; 3] = {
            let axes = &mul39_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear22_out1 = self.linear22.forward(mul39_out1.clone());
        let linear23_out1 = self.linear23.forward(mul39_out1.clone());
        let linear24_out1 = self.linear24.forward(mul39_out1);
        let gather62_out1 = shape45_out1[0] as i64;
        let gather63_out1 = shape45_out1[1] as i64;
        let constant335_out1 = self.constant335.val();
        let add40_out1 = (constant335_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear22_out1);
        let constant336_out1 = self.constant336.val();
        let add41_out1 = (constant336_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear23_out1);
        let constant337_out1 = self.constant337.val();
        let add42_out1 = (constant337_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear24_out1);
        let unsqueeze48_out1 = [gather62_out1 as i64];
        let unsqueeze49_out1 = [gather63_out1 as i64];
        let constant340_out1: [i64; 1] = [14i64];
        let constant341_out1: [i64; 1] = [64i64];
        let concat37_out1: [i64; 4usize] = [
            &unsqueeze48_out1[..],
            &unsqueeze49_out1[..],
            &constant340_out1[..],
            &constant341_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant342_out1: [i64; 1] = [2i64];
        let constant343_out1: [i64; 1] = [64i64];
        let concat38_out1: [i64; 4usize] = [
            &unsqueeze48_out1[..],
            &unsqueeze49_out1[..],
            &constant342_out1[..],
            &constant343_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant344_out1: [i64; 1] = [896i64];
        let concat39_out1: [i64; 3usize] = [
            &unsqueeze48_out1[..],
            &unsqueeze49_out1[..],
            &constant344_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape23_out1 = add40_out1.reshape(concat37_out1);
        let reshape24_out1 = add41_out1.reshape(concat38_out1);
        let reshape25_out1 = add42_out1.reshape(concat38_out1);
        let transpose17_out1 = reshape23_out1.permute([0, 2, 1, 3]);
        let transpose18_out1 = reshape24_out1.permute([0, 2, 1, 3]);
        let transpose19_out1 = reshape25_out1.permute([0, 2, 1, 3]);
        let shape46_out1: [i64; 4] = {
            let axes = &transpose18_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat40_out1 = burn::tensor::Tensor::cat(
            [past_key_values_3_value, transpose19_out1].into(),
            2,
        );
        let slice24_out1 = transpose17_out1.clone().slice(s![.., .., .., 0..32]);
        let slice25_out1 = transpose17_out1.clone().slice(s![.., .., .., 32..]);
        let slice26_out1 = transpose18_out1.clone().slice(s![.., .., .., 0..32]);
        let slice27_out1 = transpose18_out1.clone().slice(s![.., .., .., 32..]);
        let gather64_out1 = shape46_out1[2] as i64;
        let shape47_out1: [i64; 4] = {
            let axes = &concat40_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze50_out1: Tensor<5> = concat40_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg7_out1 = slice25_out1.neg();
        let neg8_out1 = slice27_out1.neg();
        let add43_out1 = gather64_out1 + gather6_out1;
        let gather65_out1 = shape47_out1[0] as i64;
        let gather66_out1 = shape47_out1[2] as i64;
        let concat41_out1 = burn::tensor::Tensor::cat(
            [neg7_out1, slice24_out1].into(),
            3,
        );
        let concat42_out1 = burn::tensor::Tensor::cat(
            [neg8_out1, slice26_out1].into(),
            3,
        );
        let unsqueeze51_out1 = [add43_out1 as i64];
        let unsqueeze52_out1 = [gather65_out1 as i64];
        let unsqueeze53_out1 = [gather66_out1 as i64];
        let slice28_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze51_out1[0], ..]);
        let slice29_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze51_out1[0], ..]);
        let constant374_out1: [i64; 1] = [2i64];
        let constant375_out1: [i64; 1] = [7i64];
        let constant376_out1: [i64; 1] = [64i64];
        let concat43_out1: [i64; 5usize] = [
            &unsqueeze52_out1[..],
            &constant374_out1[..],
            &constant375_out1[..],
            &unsqueeze53_out1[..],
            &constant376_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant377_out1: [i64; 1] = [14i64];
        let constant378_out1: [i64; 1] = [64i64];
        let concat44_out1: [i64; 4usize] = [
            &unsqueeze52_out1[..],
            &constant377_out1[..],
            &unsqueeze53_out1[..],
            &constant378_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather67_out1 = slice28_out1.take::<2, 3>(0, position_ids.clone());
        let gather68_out1 = slice29_out1.take::<2, 3>(0, position_ids.clone());
        let constant379_out1 = self.constant379.val();
        let equal10_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat43_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant379_out1)
        };
        let unsqueeze54_out1: Tensor<4> = gather67_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze55_out1: Tensor<4> = gather68_out1.unsqueeze_dims::<4>(&[1]);
        let constant382_out1 = self.constant382.val();
        let where10_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat43_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal10_out1, constant382_out1);
        let mul40_out1 = transpose17_out1.mul(unsqueeze54_out1.clone());
        let mul41_out1 = transpose18_out1.mul(unsqueeze54_out1);
        let mul42_out1 = concat41_out1.mul(unsqueeze55_out1.clone());
        let mul43_out1 = concat42_out1.mul(unsqueeze55_out1);
        let expand13_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where10_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze50_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze50_out1.expand(shape)
        };
        let add44_out1 = mul40_out1.add(mul42_out1);
        let add45_out1 = mul41_out1.add(mul43_out1);
        let reshape26_out1 = expand13_out1.reshape(concat44_out1);
        let concat45_out1 = burn::tensor::Tensor::cat(
            [past_key_values_3_key, add45_out1].into(),
            2,
        );
        let shape48_out1: [i64; 4] = {
            let axes = &concat45_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze56_out1: Tensor<5> = concat45_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather69_out1 = shape48_out1[0] as i64;
        let gather70_out1 = shape48_out1[2] as i64;
        let unsqueeze57_out1 = [gather69_out1 as i64];
        let unsqueeze58_out1 = [gather70_out1 as i64];
        let constant389_out1: [i64; 1] = [2i64];
        let constant390_out1: [i64; 1] = [7i64];
        let constant391_out1: [i64; 1] = [64i64];
        let concat46_out1: [i64; 5usize] = [
            &unsqueeze57_out1[..],
            &constant389_out1[..],
            &constant390_out1[..],
            &unsqueeze58_out1[..],
            &constant391_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant392_out1: [i64; 1] = [14i64];
        let constant393_out1: [i64; 1] = [64i64];
        let concat47_out1: [i64; 4usize] = [
            &unsqueeze57_out1[..],
            &constant392_out1[..],
            &unsqueeze58_out1[..],
            &constant393_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant394_out1 = self.constant394.val();
        let equal11_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat46_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant394_out1)
        };
        let constant395_out1 = self.constant395.val();
        let where11_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat46_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal11_out1, constant395_out1);
        let expand14_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where11_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze56_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze56_out1.expand(shape)
        };
        let reshape27_out1 = expand14_out1.reshape(concat47_out1);
        let shape49_out1: [i64; 4] = {
            let axes = &reshape27_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather71_out1 = shape49_out1[2] as i64;
        let unsqueeze59_out1 = [gather71_out1 as i64];
        let slice30_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze59_out1[0]]);
        let (matmul32_out1,) = {
            let q = add44_out1;
            let k = reshape27_out1;
            let v = reshape26_out1;
            let matmul32_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice30_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul32_out1,)
        };
        let transpose21_out1 = matmul32_out1.permute([0, 2, 1, 3]);
        let reshape28_out1 = transpose21_out1.reshape(concat39_out1);
        let linear25_out1 = self.linear25.forward(reshape28_out1);
        let add47_out1 = add38_out1.add(linear25_out1);
        let constant403_out1 = self.constant403.val();
        let pow8_out1 = add47_out1
            .clone()
            .powf((constant403_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean8_out1 = { pow8_out1.mean_dim(2usize) };
        let constant404_out1 = self.constant404.val();
        let add48_out1 = reducemean8_out1
            .add((constant404_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt8_out1 = add48_out1.sqrt();
        let constant405_out1 = self.constant405.val();
        let div8_out1 = (constant405_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt8_out1);
        let mul46_out1 = add47_out1.clone().mul(div8_out1);
        let constant406_out1 = self.constant406.val();
        let mul47_out1 = (constant406_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul46_out1);
        let linear26_out1 = self.linear26.forward(mul47_out1.clone());
        let linear27_out1 = self.linear27.forward(mul47_out1);
        let sigmoid4_out1 = burn::tensor::activation::sigmoid(linear26_out1.clone());
        let mul48_out1 = linear26_out1.mul(sigmoid4_out1);
        let mul49_out1 = mul48_out1.mul(linear27_out1);
        let linear28_out1 = self.linear28.forward(mul49_out1);
        let add49_out1 = add47_out1.add(linear28_out1);
        let constant410_out1 = self.constant410.val();
        let pow9_out1 = add49_out1
            .clone()
            .powf((constant410_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean9_out1 = { pow9_out1.mean_dim(2usize) };
        let constant411_out1 = self.constant411.val();
        let add50_out1 = reducemean9_out1
            .add((constant411_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt9_out1 = add50_out1.sqrt();
        let constant412_out1 = self.constant412.val();
        let div9_out1 = (constant412_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt9_out1);
        let mul50_out1 = add49_out1.clone().mul(div9_out1);
        let constant413_out1 = self.constant413.val();
        let mul51_out1 = (constant413_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul50_out1);
        let shape50_out1: [i64; 3] = {
            let axes = &mul51_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear29_out1 = self.linear29.forward(mul51_out1.clone());
        let linear30_out1 = self.linear30.forward(mul51_out1.clone());
        let linear31_out1 = self.linear31.forward(mul51_out1);
        let gather72_out1 = shape50_out1[0] as i64;
        let gather73_out1 = shape50_out1[1] as i64;
        let constant419_out1 = self.constant419.val();
        let add51_out1 = (constant419_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear29_out1);
        let constant420_out1 = self.constant420.val();
        let add52_out1 = (constant420_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear30_out1);
        let constant421_out1 = self.constant421.val();
        let add53_out1 = (constant421_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear31_out1);
        let unsqueeze60_out1 = [gather72_out1 as i64];
        let unsqueeze61_out1 = [gather73_out1 as i64];
        let constant424_out1: [i64; 1] = [14i64];
        let constant425_out1: [i64; 1] = [64i64];
        let concat48_out1: [i64; 4usize] = [
            &unsqueeze60_out1[..],
            &unsqueeze61_out1[..],
            &constant424_out1[..],
            &constant425_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant426_out1: [i64; 1] = [2i64];
        let constant427_out1: [i64; 1] = [64i64];
        let concat49_out1: [i64; 4usize] = [
            &unsqueeze60_out1[..],
            &unsqueeze61_out1[..],
            &constant426_out1[..],
            &constant427_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant428_out1: [i64; 1] = [896i64];
        let concat50_out1: [i64; 3usize] = [
            &unsqueeze60_out1[..],
            &unsqueeze61_out1[..],
            &constant428_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape29_out1 = add51_out1.reshape(concat48_out1);
        let reshape30_out1 = add52_out1.reshape(concat49_out1);
        let reshape31_out1 = add53_out1.reshape(concat49_out1);
        let transpose22_out1 = reshape29_out1.permute([0, 2, 1, 3]);
        let transpose23_out1 = reshape30_out1.permute([0, 2, 1, 3]);
        let transpose24_out1 = reshape31_out1.permute([0, 2, 1, 3]);
        let shape51_out1: [i64; 4] = {
            let axes = &transpose23_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat51_out1 = burn::tensor::Tensor::cat(
            [past_key_values_4_value, transpose24_out1].into(),
            2,
        );
        let slice31_out1 = transpose22_out1.clone().slice(s![.., .., .., 0..32]);
        let slice32_out1 = transpose22_out1.clone().slice(s![.., .., .., 32..]);
        let slice33_out1 = transpose23_out1.clone().slice(s![.., .., .., 0..32]);
        let slice34_out1 = transpose23_out1.clone().slice(s![.., .., .., 32..]);
        let gather74_out1 = shape51_out1[2] as i64;
        let shape52_out1: [i64; 4] = {
            let axes = &concat51_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze62_out1: Tensor<5> = concat51_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg9_out1 = slice32_out1.neg();
        let neg10_out1 = slice34_out1.neg();
        let add54_out1 = gather74_out1 + gather7_out1;
        let gather75_out1 = shape52_out1[0] as i64;
        let gather76_out1 = shape52_out1[2] as i64;
        let concat52_out1 = burn::tensor::Tensor::cat(
            [neg9_out1, slice31_out1].into(),
            3,
        );
        let concat53_out1 = burn::tensor::Tensor::cat(
            [neg10_out1, slice33_out1].into(),
            3,
        );
        let unsqueeze63_out1 = [add54_out1 as i64];
        let unsqueeze64_out1 = [gather75_out1 as i64];
        let unsqueeze65_out1 = [gather76_out1 as i64];
        let slice35_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze63_out1[0], ..]);
        let slice36_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze63_out1[0], ..]);
        let constant458_out1: [i64; 1] = [2i64];
        let constant459_out1: [i64; 1] = [7i64];
        let constant460_out1: [i64; 1] = [64i64];
        let concat54_out1: [i64; 5usize] = [
            &unsqueeze64_out1[..],
            &constant458_out1[..],
            &constant459_out1[..],
            &unsqueeze65_out1[..],
            &constant460_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant461_out1: [i64; 1] = [14i64];
        let constant462_out1: [i64; 1] = [64i64];
        let concat55_out1: [i64; 4usize] = [
            &unsqueeze64_out1[..],
            &constant461_out1[..],
            &unsqueeze65_out1[..],
            &constant462_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather77_out1 = slice35_out1.take::<2, 3>(0, position_ids.clone());
        let gather78_out1 = slice36_out1.take::<2, 3>(0, position_ids.clone());
        let constant463_out1 = self.constant463.val();
        let equal12_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat54_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant463_out1)
        };
        let unsqueeze66_out1: Tensor<4> = gather77_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze67_out1: Tensor<4> = gather78_out1.unsqueeze_dims::<4>(&[1]);
        let constant466_out1 = self.constant466.val();
        let where12_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat54_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal12_out1, constant466_out1);
        let mul52_out1 = transpose22_out1.mul(unsqueeze66_out1.clone());
        let mul53_out1 = transpose23_out1.mul(unsqueeze66_out1);
        let mul54_out1 = concat52_out1.mul(unsqueeze67_out1.clone());
        let mul55_out1 = concat53_out1.mul(unsqueeze67_out1);
        let expand15_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where12_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze62_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze62_out1.expand(shape)
        };
        let add55_out1 = mul52_out1.add(mul54_out1);
        let add56_out1 = mul53_out1.add(mul55_out1);
        let reshape32_out1 = expand15_out1.reshape(concat55_out1);
        let concat56_out1 = burn::tensor::Tensor::cat(
            [past_key_values_4_key, add56_out1].into(),
            2,
        );
        let shape53_out1: [i64; 4] = {
            let axes = &concat56_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze68_out1: Tensor<5> = concat56_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather79_out1 = shape53_out1[0] as i64;
        let gather80_out1 = shape53_out1[2] as i64;
        let unsqueeze69_out1 = [gather79_out1 as i64];
        let unsqueeze70_out1 = [gather80_out1 as i64];
        let constant473_out1: [i64; 1] = [2i64];
        let constant474_out1: [i64; 1] = [7i64];
        let constant475_out1: [i64; 1] = [64i64];
        let concat57_out1: [i64; 5usize] = [
            &unsqueeze69_out1[..],
            &constant473_out1[..],
            &constant474_out1[..],
            &unsqueeze70_out1[..],
            &constant475_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant476_out1: [i64; 1] = [14i64];
        let constant477_out1: [i64; 1] = [64i64];
        let concat58_out1: [i64; 4usize] = [
            &unsqueeze69_out1[..],
            &constant476_out1[..],
            &unsqueeze70_out1[..],
            &constant477_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant478_out1 = self.constant478.val();
        let equal13_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat57_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant478_out1)
        };
        let constant479_out1 = self.constant479.val();
        let where13_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat57_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal13_out1, constant479_out1);
        let expand16_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where13_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze68_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze68_out1.expand(shape)
        };
        let reshape33_out1 = expand16_out1.reshape(concat58_out1);
        let shape54_out1: [i64; 4] = {
            let axes = &reshape33_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather81_out1 = shape54_out1[2] as i64;
        let unsqueeze71_out1 = [gather81_out1 as i64];
        let slice37_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze71_out1[0]]);
        let (matmul41_out1,) = {
            let q = add55_out1;
            let k = reshape33_out1;
            let v = reshape32_out1;
            let matmul41_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice37_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul41_out1,)
        };
        let transpose26_out1 = matmul41_out1.permute([0, 2, 1, 3]);
        let reshape34_out1 = transpose26_out1.reshape(concat50_out1);
        let linear32_out1 = self.linear32.forward(reshape34_out1);
        let add58_out1 = add49_out1.add(linear32_out1);
        let constant487_out1 = self.constant487.val();
        let pow10_out1 = add58_out1
            .clone()
            .powf((constant487_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean10_out1 = { pow10_out1.mean_dim(2usize) };
        let constant488_out1 = self.constant488.val();
        let add59_out1 = reducemean10_out1
            .add((constant488_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt10_out1 = add59_out1.sqrt();
        let constant489_out1 = self.constant489.val();
        let div10_out1 = (constant489_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt10_out1);
        let mul58_out1 = add58_out1.clone().mul(div10_out1);
        let constant490_out1 = self.constant490.val();
        let mul59_out1 = (constant490_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul58_out1);
        let linear33_out1 = self.linear33.forward(mul59_out1.clone());
        let linear34_out1 = self.linear34.forward(mul59_out1);
        let sigmoid5_out1 = burn::tensor::activation::sigmoid(linear33_out1.clone());
        let mul60_out1 = linear33_out1.mul(sigmoid5_out1);
        let mul61_out1 = mul60_out1.mul(linear34_out1);
        let linear35_out1 = self.linear35.forward(mul61_out1);
        let add60_out1 = add58_out1.add(linear35_out1);
        let constant494_out1 = self.constant494.val();
        let pow11_out1 = add60_out1
            .clone()
            .powf((constant494_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean11_out1 = { pow11_out1.mean_dim(2usize) };
        let constant495_out1 = self.constant495.val();
        let add61_out1 = reducemean11_out1
            .add((constant495_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt11_out1 = add61_out1.sqrt();
        let constant496_out1 = self.constant496.val();
        let div11_out1 = (constant496_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt11_out1);
        let mul62_out1 = add60_out1.clone().mul(div11_out1);
        let constant497_out1 = self.constant497.val();
        let mul63_out1 = (constant497_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul62_out1);
        let shape55_out1: [i64; 3] = {
            let axes = &mul63_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear36_out1 = self.linear36.forward(mul63_out1.clone());
        let linear37_out1 = self.linear37.forward(mul63_out1.clone());
        let linear38_out1 = self.linear38.forward(mul63_out1);
        let gather82_out1 = shape55_out1[0] as i64;
        let gather83_out1 = shape55_out1[1] as i64;
        let constant503_out1 = self.constant503.val();
        let add62_out1 = (constant503_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear36_out1);
        let constant504_out1 = self.constant504.val();
        let add63_out1 = (constant504_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear37_out1);
        let constant505_out1 = self.constant505.val();
        let add64_out1 = (constant505_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear38_out1);
        let unsqueeze72_out1 = [gather82_out1 as i64];
        let unsqueeze73_out1 = [gather83_out1 as i64];
        let constant508_out1: [i64; 1] = [14i64];
        let constant509_out1: [i64; 1] = [64i64];
        let concat59_out1: [i64; 4usize] = [
            &unsqueeze72_out1[..],
            &unsqueeze73_out1[..],
            &constant508_out1[..],
            &constant509_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant510_out1: [i64; 1] = [2i64];
        let constant511_out1: [i64; 1] = [64i64];
        let concat60_out1: [i64; 4usize] = [
            &unsqueeze72_out1[..],
            &unsqueeze73_out1[..],
            &constant510_out1[..],
            &constant511_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant512_out1: [i64; 1] = [896i64];
        let concat61_out1: [i64; 3usize] = [
            &unsqueeze72_out1[..],
            &unsqueeze73_out1[..],
            &constant512_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape35_out1 = add62_out1.reshape(concat59_out1);
        let reshape36_out1 = add63_out1.reshape(concat60_out1);
        let reshape37_out1 = add64_out1.reshape(concat60_out1);
        let transpose27_out1 = reshape35_out1.permute([0, 2, 1, 3]);
        let transpose28_out1 = reshape36_out1.permute([0, 2, 1, 3]);
        let transpose29_out1 = reshape37_out1.permute([0, 2, 1, 3]);
        let shape56_out1: [i64; 4] = {
            let axes = &transpose28_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat62_out1 = burn::tensor::Tensor::cat(
            [past_key_values_5_value, transpose29_out1].into(),
            2,
        );
        let slice38_out1 = transpose27_out1.clone().slice(s![.., .., .., 0..32]);
        let slice39_out1 = transpose27_out1.clone().slice(s![.., .., .., 32..]);
        let slice40_out1 = transpose28_out1.clone().slice(s![.., .., .., 0..32]);
        let slice41_out1 = transpose28_out1.clone().slice(s![.., .., .., 32..]);
        let gather84_out1 = shape56_out1[2] as i64;
        let shape57_out1: [i64; 4] = {
            let axes = &concat62_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze74_out1: Tensor<5> = concat62_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg11_out1 = slice39_out1.neg();
        let neg12_out1 = slice41_out1.neg();
        let add65_out1 = gather84_out1 + gather8_out1;
        let gather85_out1 = shape57_out1[0] as i64;
        let gather86_out1 = shape57_out1[2] as i64;
        let concat63_out1 = burn::tensor::Tensor::cat(
            [neg11_out1, slice38_out1].into(),
            3,
        );
        let concat64_out1 = burn::tensor::Tensor::cat(
            [neg12_out1, slice40_out1].into(),
            3,
        );
        let unsqueeze75_out1 = [add65_out1 as i64];
        let unsqueeze76_out1 = [gather85_out1 as i64];
        let unsqueeze77_out1 = [gather86_out1 as i64];
        let slice42_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze75_out1[0], ..]);
        let slice43_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze75_out1[0], ..]);
        let constant542_out1: [i64; 1] = [2i64];
        let constant543_out1: [i64; 1] = [7i64];
        let constant544_out1: [i64; 1] = [64i64];
        let concat65_out1: [i64; 5usize] = [
            &unsqueeze76_out1[..],
            &constant542_out1[..],
            &constant543_out1[..],
            &unsqueeze77_out1[..],
            &constant544_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant545_out1: [i64; 1] = [14i64];
        let constant546_out1: [i64; 1] = [64i64];
        let concat66_out1: [i64; 4usize] = [
            &unsqueeze76_out1[..],
            &constant545_out1[..],
            &unsqueeze77_out1[..],
            &constant546_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather87_out1 = slice42_out1.take::<2, 3>(0, position_ids.clone());
        let gather88_out1 = slice43_out1.take::<2, 3>(0, position_ids.clone());
        let constant547_out1 = self.constant547.val();
        let equal14_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat65_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant547_out1)
        };
        let unsqueeze78_out1: Tensor<4> = gather87_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze79_out1: Tensor<4> = gather88_out1.unsqueeze_dims::<4>(&[1]);
        let constant550_out1 = self.constant550.val();
        let where14_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat65_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal14_out1, constant550_out1);
        let mul64_out1 = transpose27_out1.mul(unsqueeze78_out1.clone());
        let mul65_out1 = transpose28_out1.mul(unsqueeze78_out1);
        let mul66_out1 = concat63_out1.mul(unsqueeze79_out1.clone());
        let mul67_out1 = concat64_out1.mul(unsqueeze79_out1);
        let expand17_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where14_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze74_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze74_out1.expand(shape)
        };
        let add66_out1 = mul64_out1.add(mul66_out1);
        let add67_out1 = mul65_out1.add(mul67_out1);
        let reshape38_out1 = expand17_out1.reshape(concat66_out1);
        let concat67_out1 = burn::tensor::Tensor::cat(
            [past_key_values_5_key, add67_out1].into(),
            2,
        );
        let shape58_out1: [i64; 4] = {
            let axes = &concat67_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze80_out1: Tensor<5> = concat67_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather89_out1 = shape58_out1[0] as i64;
        let gather90_out1 = shape58_out1[2] as i64;
        let unsqueeze81_out1 = [gather89_out1 as i64];
        let unsqueeze82_out1 = [gather90_out1 as i64];
        let constant557_out1: [i64; 1] = [2i64];
        let constant558_out1: [i64; 1] = [7i64];
        let constant559_out1: [i64; 1] = [64i64];
        let concat68_out1: [i64; 5usize] = [
            &unsqueeze81_out1[..],
            &constant557_out1[..],
            &constant558_out1[..],
            &unsqueeze82_out1[..],
            &constant559_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant560_out1: [i64; 1] = [14i64];
        let constant561_out1: [i64; 1] = [64i64];
        let concat69_out1: [i64; 4usize] = [
            &unsqueeze81_out1[..],
            &constant560_out1[..],
            &unsqueeze82_out1[..],
            &constant561_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant562_out1 = self.constant562.val();
        let equal15_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat68_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant562_out1)
        };
        let constant563_out1 = self.constant563.val();
        let where15_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat68_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal15_out1, constant563_out1);
        let expand18_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where15_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze80_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze80_out1.expand(shape)
        };
        let reshape39_out1 = expand18_out1.reshape(concat69_out1);
        let shape59_out1: [i64; 4] = {
            let axes = &reshape39_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather91_out1 = shape59_out1[2] as i64;
        let unsqueeze83_out1 = [gather91_out1 as i64];
        let slice44_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze83_out1[0]]);
        let (matmul50_out1,) = {
            let q = add66_out1;
            let k = reshape39_out1;
            let v = reshape38_out1;
            let matmul50_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice44_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul50_out1,)
        };
        let transpose31_out1 = matmul50_out1.permute([0, 2, 1, 3]);
        let reshape40_out1 = transpose31_out1.reshape(concat61_out1);
        let linear39_out1 = self.linear39.forward(reshape40_out1);
        let add69_out1 = add60_out1.add(linear39_out1);
        let constant571_out1 = self.constant571.val();
        let pow12_out1 = add69_out1
            .clone()
            .powf((constant571_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean12_out1 = { pow12_out1.mean_dim(2usize) };
        let constant572_out1 = self.constant572.val();
        let add70_out1 = reducemean12_out1
            .add((constant572_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt12_out1 = add70_out1.sqrt();
        let constant573_out1 = self.constant573.val();
        let div12_out1 = (constant573_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt12_out1);
        let mul70_out1 = add69_out1.clone().mul(div12_out1);
        let constant574_out1 = self.constant574.val();
        let mul71_out1 = (constant574_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul70_out1);
        let linear40_out1 = self.linear40.forward(mul71_out1.clone());
        let linear41_out1 = self.linear41.forward(mul71_out1);
        let sigmoid6_out1 = burn::tensor::activation::sigmoid(linear40_out1.clone());
        let mul72_out1 = linear40_out1.mul(sigmoid6_out1);
        let mul73_out1 = mul72_out1.mul(linear41_out1);
        let linear42_out1 = self.linear42.forward(mul73_out1);
        let add71_out1 = add69_out1.add(linear42_out1);
        let constant578_out1 = self.constant578.val();
        let pow13_out1 = add71_out1
            .clone()
            .powf((constant578_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean13_out1 = { pow13_out1.mean_dim(2usize) };
        let constant579_out1 = self.constant579.val();
        let add72_out1 = reducemean13_out1
            .add((constant579_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt13_out1 = add72_out1.sqrt();
        let constant580_out1 = self.constant580.val();
        let div13_out1 = (constant580_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt13_out1);
        let mul74_out1 = add71_out1.clone().mul(div13_out1);
        let constant581_out1 = self.constant581.val();
        let mul75_out1 = (constant581_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul74_out1);
        let shape60_out1: [i64; 3] = {
            let axes = &mul75_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear43_out1 = self.linear43.forward(mul75_out1.clone());
        let linear44_out1 = self.linear44.forward(mul75_out1.clone());
        let linear45_out1 = self.linear45.forward(mul75_out1);
        let gather92_out1 = shape60_out1[0] as i64;
        let gather93_out1 = shape60_out1[1] as i64;
        let constant587_out1 = self.constant587.val();
        let add73_out1 = (constant587_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear43_out1);
        let constant588_out1 = self.constant588.val();
        let add74_out1 = (constant588_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear44_out1);
        let constant589_out1 = self.constant589.val();
        let add75_out1 = (constant589_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear45_out1);
        let unsqueeze84_out1 = [gather92_out1 as i64];
        let unsqueeze85_out1 = [gather93_out1 as i64];
        let constant592_out1: [i64; 1] = [14i64];
        let constant593_out1: [i64; 1] = [64i64];
        let concat70_out1: [i64; 4usize] = [
            &unsqueeze84_out1[..],
            &unsqueeze85_out1[..],
            &constant592_out1[..],
            &constant593_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant594_out1: [i64; 1] = [2i64];
        let constant595_out1: [i64; 1] = [64i64];
        let concat71_out1: [i64; 4usize] = [
            &unsqueeze84_out1[..],
            &unsqueeze85_out1[..],
            &constant594_out1[..],
            &constant595_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant596_out1: [i64; 1] = [896i64];
        let concat72_out1: [i64; 3usize] = [
            &unsqueeze84_out1[..],
            &unsqueeze85_out1[..],
            &constant596_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape41_out1 = add73_out1.reshape(concat70_out1);
        let reshape42_out1 = add74_out1.reshape(concat71_out1);
        let reshape43_out1 = add75_out1.reshape(concat71_out1);
        let transpose32_out1 = reshape41_out1.permute([0, 2, 1, 3]);
        let transpose33_out1 = reshape42_out1.permute([0, 2, 1, 3]);
        let transpose34_out1 = reshape43_out1.permute([0, 2, 1, 3]);
        let shape61_out1: [i64; 4] = {
            let axes = &transpose33_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat73_out1 = burn::tensor::Tensor::cat(
            [past_key_values_6_value, transpose34_out1].into(),
            2,
        );
        let slice45_out1 = transpose32_out1.clone().slice(s![.., .., .., 0..32]);
        let slice46_out1 = transpose32_out1.clone().slice(s![.., .., .., 32..]);
        let slice47_out1 = transpose33_out1.clone().slice(s![.., .., .., 0..32]);
        let slice48_out1 = transpose33_out1.clone().slice(s![.., .., .., 32..]);
        let gather94_out1 = shape61_out1[2] as i64;
        let shape62_out1: [i64; 4] = {
            let axes = &concat73_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze86_out1: Tensor<5> = concat73_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg13_out1 = slice46_out1.neg();
        let neg14_out1 = slice48_out1.neg();
        let add76_out1 = gather94_out1 + gather9_out1;
        let gather95_out1 = shape62_out1[0] as i64;
        let gather96_out1 = shape62_out1[2] as i64;
        let concat74_out1 = burn::tensor::Tensor::cat(
            [neg13_out1, slice45_out1].into(),
            3,
        );
        let concat75_out1 = burn::tensor::Tensor::cat(
            [neg14_out1, slice47_out1].into(),
            3,
        );
        let unsqueeze87_out1 = [add76_out1 as i64];
        let unsqueeze88_out1 = [gather95_out1 as i64];
        let unsqueeze89_out1 = [gather96_out1 as i64];
        let slice49_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze87_out1[0], ..]);
        let slice50_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze87_out1[0], ..]);
        let constant626_out1: [i64; 1] = [2i64];
        let constant627_out1: [i64; 1] = [7i64];
        let constant628_out1: [i64; 1] = [64i64];
        let concat76_out1: [i64; 5usize] = [
            &unsqueeze88_out1[..],
            &constant626_out1[..],
            &constant627_out1[..],
            &unsqueeze89_out1[..],
            &constant628_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant629_out1: [i64; 1] = [14i64];
        let constant630_out1: [i64; 1] = [64i64];
        let concat77_out1: [i64; 4usize] = [
            &unsqueeze88_out1[..],
            &constant629_out1[..],
            &unsqueeze89_out1[..],
            &constant630_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather97_out1 = slice49_out1.take::<2, 3>(0, position_ids.clone());
        let gather98_out1 = slice50_out1.take::<2, 3>(0, position_ids.clone());
        let constant631_out1 = self.constant631.val();
        let equal16_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat76_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant631_out1)
        };
        let unsqueeze90_out1: Tensor<4> = gather97_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze91_out1: Tensor<4> = gather98_out1.unsqueeze_dims::<4>(&[1]);
        let constant634_out1 = self.constant634.val();
        let where16_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat76_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal16_out1, constant634_out1);
        let mul76_out1 = transpose32_out1.mul(unsqueeze90_out1.clone());
        let mul77_out1 = transpose33_out1.mul(unsqueeze90_out1);
        let mul78_out1 = concat74_out1.mul(unsqueeze91_out1.clone());
        let mul79_out1 = concat75_out1.mul(unsqueeze91_out1);
        let expand19_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where16_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze86_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze86_out1.expand(shape)
        };
        let add77_out1 = mul76_out1.add(mul78_out1);
        let add78_out1 = mul77_out1.add(mul79_out1);
        let reshape44_out1 = expand19_out1.reshape(concat77_out1);
        let concat78_out1 = burn::tensor::Tensor::cat(
            [past_key_values_6_key, add78_out1].into(),
            2,
        );
        let shape63_out1: [i64; 4] = {
            let axes = &concat78_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze92_out1: Tensor<5> = concat78_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather99_out1 = shape63_out1[0] as i64;
        let gather100_out1 = shape63_out1[2] as i64;
        let unsqueeze93_out1 = [gather99_out1 as i64];
        let unsqueeze94_out1 = [gather100_out1 as i64];
        let constant641_out1: [i64; 1] = [2i64];
        let constant642_out1: [i64; 1] = [7i64];
        let constant643_out1: [i64; 1] = [64i64];
        let concat79_out1: [i64; 5usize] = [
            &unsqueeze93_out1[..],
            &constant641_out1[..],
            &constant642_out1[..],
            &unsqueeze94_out1[..],
            &constant643_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant644_out1: [i64; 1] = [14i64];
        let constant645_out1: [i64; 1] = [64i64];
        let concat80_out1: [i64; 4usize] = [
            &unsqueeze93_out1[..],
            &constant644_out1[..],
            &unsqueeze94_out1[..],
            &constant645_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant646_out1 = self.constant646.val();
        let equal17_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat79_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant646_out1)
        };
        let constant647_out1 = self.constant647.val();
        let where17_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat79_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal17_out1, constant647_out1);
        let expand20_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where17_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze92_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze92_out1.expand(shape)
        };
        let reshape45_out1 = expand20_out1.reshape(concat80_out1);
        let shape64_out1: [i64; 4] = {
            let axes = &reshape45_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather101_out1 = shape64_out1[2] as i64;
        let unsqueeze95_out1 = [gather101_out1 as i64];
        let slice51_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze95_out1[0]]);
        let (matmul59_out1,) = {
            let q = add77_out1;
            let k = reshape45_out1;
            let v = reshape44_out1;
            let matmul59_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice51_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul59_out1,)
        };
        let transpose36_out1 = matmul59_out1.permute([0, 2, 1, 3]);
        let reshape46_out1 = transpose36_out1.reshape(concat72_out1);
        let linear46_out1 = self.linear46.forward(reshape46_out1);
        let add80_out1 = add71_out1.add(linear46_out1);
        let constant655_out1 = self.constant655.val();
        let pow14_out1 = add80_out1
            .clone()
            .powf((constant655_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean14_out1 = { pow14_out1.mean_dim(2usize) };
        let constant656_out1 = self.constant656.val();
        let add81_out1 = reducemean14_out1
            .add((constant656_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt14_out1 = add81_out1.sqrt();
        let constant657_out1 = self.constant657.val();
        let div14_out1 = (constant657_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt14_out1);
        let mul82_out1 = add80_out1.clone().mul(div14_out1);
        let constant658_out1 = self.constant658.val();
        let mul83_out1 = (constant658_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul82_out1);
        let linear47_out1 = self.linear47.forward(mul83_out1.clone());
        let linear48_out1 = self.linear48.forward(mul83_out1);
        let sigmoid7_out1 = burn::tensor::activation::sigmoid(linear47_out1.clone());
        let mul84_out1 = linear47_out1.mul(sigmoid7_out1);
        let mul85_out1 = mul84_out1.mul(linear48_out1);
        let linear49_out1 = self.linear49.forward(mul85_out1);
        let add82_out1 = add80_out1.add(linear49_out1);
        let constant662_out1 = self.constant662.val();
        let pow15_out1 = add82_out1
            .clone()
            .powf((constant662_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean15_out1 = { pow15_out1.mean_dim(2usize) };
        let constant663_out1 = self.constant663.val();
        let add83_out1 = reducemean15_out1
            .add((constant663_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt15_out1 = add83_out1.sqrt();
        let constant664_out1 = self.constant664.val();
        let div15_out1 = (constant664_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt15_out1);
        let mul86_out1 = add82_out1.clone().mul(div15_out1);
        let constant665_out1 = self.constant665.val();
        let mul87_out1 = (constant665_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul86_out1);
        let shape65_out1: [i64; 3] = {
            let axes = &mul87_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear50_out1 = self.linear50.forward(mul87_out1.clone());
        let linear51_out1 = self.linear51.forward(mul87_out1.clone());
        let linear52_out1 = self.linear52.forward(mul87_out1);
        let gather102_out1 = shape65_out1[0] as i64;
        let gather103_out1 = shape65_out1[1] as i64;
        let constant671_out1 = self.constant671.val();
        let add84_out1 = (constant671_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear50_out1);
        let constant672_out1 = self.constant672.val();
        let add85_out1 = (constant672_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear51_out1);
        let constant673_out1 = self.constant673.val();
        let add86_out1 = (constant673_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear52_out1);
        let unsqueeze96_out1 = [gather102_out1 as i64];
        let unsqueeze97_out1 = [gather103_out1 as i64];
        let constant676_out1: [i64; 1] = [14i64];
        let constant677_out1: [i64; 1] = [64i64];
        let concat81_out1: [i64; 4usize] = [
            &unsqueeze96_out1[..],
            &unsqueeze97_out1[..],
            &constant676_out1[..],
            &constant677_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant678_out1: [i64; 1] = [2i64];
        let constant679_out1: [i64; 1] = [64i64];
        let concat82_out1: [i64; 4usize] = [
            &unsqueeze96_out1[..],
            &unsqueeze97_out1[..],
            &constant678_out1[..],
            &constant679_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant680_out1: [i64; 1] = [896i64];
        let concat83_out1: [i64; 3usize] = [
            &unsqueeze96_out1[..],
            &unsqueeze97_out1[..],
            &constant680_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape47_out1 = add84_out1.reshape(concat81_out1);
        let reshape48_out1 = add85_out1.reshape(concat82_out1);
        let reshape49_out1 = add86_out1.reshape(concat82_out1);
        let transpose37_out1 = reshape47_out1.permute([0, 2, 1, 3]);
        let transpose38_out1 = reshape48_out1.permute([0, 2, 1, 3]);
        let transpose39_out1 = reshape49_out1.permute([0, 2, 1, 3]);
        let shape66_out1: [i64; 4] = {
            let axes = &transpose38_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat84_out1 = burn::tensor::Tensor::cat(
            [past_key_values_7_value, transpose39_out1].into(),
            2,
        );
        let slice52_out1 = transpose37_out1.clone().slice(s![.., .., .., 0..32]);
        let slice53_out1 = transpose37_out1.clone().slice(s![.., .., .., 32..]);
        let slice54_out1 = transpose38_out1.clone().slice(s![.., .., .., 0..32]);
        let slice55_out1 = transpose38_out1.clone().slice(s![.., .., .., 32..]);
        let gather104_out1 = shape66_out1[2] as i64;
        let shape67_out1: [i64; 4] = {
            let axes = &concat84_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze98_out1: Tensor<5> = concat84_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg15_out1 = slice53_out1.neg();
        let neg16_out1 = slice55_out1.neg();
        let add87_out1 = gather104_out1 + gather10_out1;
        let gather105_out1 = shape67_out1[0] as i64;
        let gather106_out1 = shape67_out1[2] as i64;
        let concat85_out1 = burn::tensor::Tensor::cat(
            [neg15_out1, slice52_out1].into(),
            3,
        );
        let concat86_out1 = burn::tensor::Tensor::cat(
            [neg16_out1, slice54_out1].into(),
            3,
        );
        let unsqueeze99_out1 = [add87_out1 as i64];
        let unsqueeze100_out1 = [gather105_out1 as i64];
        let unsqueeze101_out1 = [gather106_out1 as i64];
        let slice56_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze99_out1[0], ..]);
        let slice57_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze99_out1[0], ..]);
        let constant710_out1: [i64; 1] = [2i64];
        let constant711_out1: [i64; 1] = [7i64];
        let constant712_out1: [i64; 1] = [64i64];
        let concat87_out1: [i64; 5usize] = [
            &unsqueeze100_out1[..],
            &constant710_out1[..],
            &constant711_out1[..],
            &unsqueeze101_out1[..],
            &constant712_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant713_out1: [i64; 1] = [14i64];
        let constant714_out1: [i64; 1] = [64i64];
        let concat88_out1: [i64; 4usize] = [
            &unsqueeze100_out1[..],
            &constant713_out1[..],
            &unsqueeze101_out1[..],
            &constant714_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather107_out1 = slice56_out1.take::<2, 3>(0, position_ids.clone());
        let gather108_out1 = slice57_out1.take::<2, 3>(0, position_ids.clone());
        let constant715_out1 = self.constant715.val();
        let equal18_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat87_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant715_out1)
        };
        let unsqueeze102_out1: Tensor<4> = gather107_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze103_out1: Tensor<4> = gather108_out1.unsqueeze_dims::<4>(&[1]);
        let constant718_out1 = self.constant718.val();
        let where18_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat87_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal18_out1, constant718_out1);
        let mul88_out1 = transpose37_out1.mul(unsqueeze102_out1.clone());
        let mul89_out1 = transpose38_out1.mul(unsqueeze102_out1);
        let mul90_out1 = concat85_out1.mul(unsqueeze103_out1.clone());
        let mul91_out1 = concat86_out1.mul(unsqueeze103_out1);
        let expand21_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where18_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze98_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze98_out1.expand(shape)
        };
        let add88_out1 = mul88_out1.add(mul90_out1);
        let add89_out1 = mul89_out1.add(mul91_out1);
        let reshape50_out1 = expand21_out1.reshape(concat88_out1);
        let concat89_out1 = burn::tensor::Tensor::cat(
            [past_key_values_7_key, add89_out1].into(),
            2,
        );
        let shape68_out1: [i64; 4] = {
            let axes = &concat89_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze104_out1: Tensor<5> = concat89_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather109_out1 = shape68_out1[0] as i64;
        let gather110_out1 = shape68_out1[2] as i64;
        let unsqueeze105_out1 = [gather109_out1 as i64];
        let unsqueeze106_out1 = [gather110_out1 as i64];
        let constant725_out1: [i64; 1] = [2i64];
        let constant726_out1: [i64; 1] = [7i64];
        let constant727_out1: [i64; 1] = [64i64];
        let concat90_out1: [i64; 5usize] = [
            &unsqueeze105_out1[..],
            &constant725_out1[..],
            &constant726_out1[..],
            &unsqueeze106_out1[..],
            &constant727_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant728_out1: [i64; 1] = [14i64];
        let constant729_out1: [i64; 1] = [64i64];
        let concat91_out1: [i64; 4usize] = [
            &unsqueeze105_out1[..],
            &constant728_out1[..],
            &unsqueeze106_out1[..],
            &constant729_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant730_out1 = self.constant730.val();
        let equal19_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat90_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant730_out1)
        };
        let constant731_out1 = self.constant731.val();
        let where19_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat90_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal19_out1, constant731_out1);
        let expand22_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where19_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze104_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze104_out1.expand(shape)
        };
        let reshape51_out1 = expand22_out1.reshape(concat91_out1);
        let shape69_out1: [i64; 4] = {
            let axes = &reshape51_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather111_out1 = shape69_out1[2] as i64;
        let unsqueeze107_out1 = [gather111_out1 as i64];
        let slice58_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze107_out1[0]]);
        let (matmul68_out1,) = {
            let q = add88_out1;
            let k = reshape51_out1;
            let v = reshape50_out1;
            let matmul68_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice58_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul68_out1,)
        };
        let transpose41_out1 = matmul68_out1.permute([0, 2, 1, 3]);
        let reshape52_out1 = transpose41_out1.reshape(concat83_out1);
        let linear53_out1 = self.linear53.forward(reshape52_out1);
        let add91_out1 = add82_out1.add(linear53_out1);
        let constant739_out1 = self.constant739.val();
        let pow16_out1 = add91_out1
            .clone()
            .powf((constant739_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean16_out1 = { pow16_out1.mean_dim(2usize) };
        let constant740_out1 = self.constant740.val();
        let add92_out1 = reducemean16_out1
            .add((constant740_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt16_out1 = add92_out1.sqrt();
        let constant741_out1 = self.constant741.val();
        let div16_out1 = (constant741_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt16_out1);
        let mul94_out1 = add91_out1.clone().mul(div16_out1);
        let constant742_out1 = self.constant742.val();
        let mul95_out1 = (constant742_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul94_out1);
        let linear54_out1 = self.linear54.forward(mul95_out1.clone());
        let linear55_out1 = self.linear55.forward(mul95_out1);
        let sigmoid8_out1 = burn::tensor::activation::sigmoid(linear54_out1.clone());
        let mul96_out1 = linear54_out1.mul(sigmoid8_out1);
        let mul97_out1 = mul96_out1.mul(linear55_out1);
        let linear56_out1 = self.linear56.forward(mul97_out1);
        let add93_out1 = add91_out1.add(linear56_out1);
        let constant746_out1 = self.constant746.val();
        let pow17_out1 = add93_out1
            .clone()
            .powf((constant746_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean17_out1 = { pow17_out1.mean_dim(2usize) };
        let constant747_out1 = self.constant747.val();
        let add94_out1 = reducemean17_out1
            .add((constant747_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt17_out1 = add94_out1.sqrt();
        let constant748_out1 = self.constant748.val();
        let div17_out1 = (constant748_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt17_out1);
        let mul98_out1 = add93_out1.clone().mul(div17_out1);
        let constant749_out1 = self.constant749.val();
        let mul99_out1 = (constant749_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul98_out1);
        let shape70_out1: [i64; 3] = {
            let axes = &mul99_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear57_out1 = self.linear57.forward(mul99_out1.clone());
        let linear58_out1 = self.linear58.forward(mul99_out1.clone());
        let linear59_out1 = self.linear59.forward(mul99_out1);
        let gather112_out1 = shape70_out1[0] as i64;
        let gather113_out1 = shape70_out1[1] as i64;
        let constant755_out1 = self.constant755.val();
        let add95_out1 = (constant755_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear57_out1);
        let constant756_out1 = self.constant756.val();
        let add96_out1 = (constant756_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear58_out1);
        let constant757_out1 = self.constant757.val();
        let add97_out1 = (constant757_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear59_out1);
        let unsqueeze108_out1 = [gather112_out1 as i64];
        let unsqueeze109_out1 = [gather113_out1 as i64];
        let constant760_out1: [i64; 1] = [14i64];
        let constant761_out1: [i64; 1] = [64i64];
        let concat92_out1: [i64; 4usize] = [
            &unsqueeze108_out1[..],
            &unsqueeze109_out1[..],
            &constant760_out1[..],
            &constant761_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant762_out1: [i64; 1] = [2i64];
        let constant763_out1: [i64; 1] = [64i64];
        let concat93_out1: [i64; 4usize] = [
            &unsqueeze108_out1[..],
            &unsqueeze109_out1[..],
            &constant762_out1[..],
            &constant763_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant764_out1: [i64; 1] = [896i64];
        let concat94_out1: [i64; 3usize] = [
            &unsqueeze108_out1[..],
            &unsqueeze109_out1[..],
            &constant764_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape53_out1 = add95_out1.reshape(concat92_out1);
        let reshape54_out1 = add96_out1.reshape(concat93_out1);
        let reshape55_out1 = add97_out1.reshape(concat93_out1);
        let transpose42_out1 = reshape53_out1.permute([0, 2, 1, 3]);
        let transpose43_out1 = reshape54_out1.permute([0, 2, 1, 3]);
        let transpose44_out1 = reshape55_out1.permute([0, 2, 1, 3]);
        let shape71_out1: [i64; 4] = {
            let axes = &transpose43_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat95_out1 = burn::tensor::Tensor::cat(
            [past_key_values_8_value, transpose44_out1].into(),
            2,
        );
        let slice59_out1 = transpose42_out1.clone().slice(s![.., .., .., 0..32]);
        let slice60_out1 = transpose42_out1.clone().slice(s![.., .., .., 32..]);
        let slice61_out1 = transpose43_out1.clone().slice(s![.., .., .., 0..32]);
        let slice62_out1 = transpose43_out1.clone().slice(s![.., .., .., 32..]);
        let gather114_out1 = shape71_out1[2] as i64;
        let shape72_out1: [i64; 4] = {
            let axes = &concat95_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze110_out1: Tensor<5> = concat95_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg17_out1 = slice60_out1.neg();
        let neg18_out1 = slice62_out1.neg();
        let add98_out1 = gather114_out1 + gather11_out1;
        let gather115_out1 = shape72_out1[0] as i64;
        let gather116_out1 = shape72_out1[2] as i64;
        let concat96_out1 = burn::tensor::Tensor::cat(
            [neg17_out1, slice59_out1].into(),
            3,
        );
        let concat97_out1 = burn::tensor::Tensor::cat(
            [neg18_out1, slice61_out1].into(),
            3,
        );
        let unsqueeze111_out1 = [add98_out1 as i64];
        let unsqueeze112_out1 = [gather115_out1 as i64];
        let unsqueeze113_out1 = [gather116_out1 as i64];
        let slice63_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze111_out1[0], ..]);
        let slice64_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze111_out1[0], ..]);
        let constant794_out1: [i64; 1] = [2i64];
        let constant795_out1: [i64; 1] = [7i64];
        let constant796_out1: [i64; 1] = [64i64];
        let concat98_out1: [i64; 5usize] = [
            &unsqueeze112_out1[..],
            &constant794_out1[..],
            &constant795_out1[..],
            &unsqueeze113_out1[..],
            &constant796_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant797_out1: [i64; 1] = [14i64];
        let constant798_out1: [i64; 1] = [64i64];
        let concat99_out1: [i64; 4usize] = [
            &unsqueeze112_out1[..],
            &constant797_out1[..],
            &unsqueeze113_out1[..],
            &constant798_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather117_out1 = slice63_out1.take::<2, 3>(0, position_ids.clone());
        let gather118_out1 = slice64_out1.take::<2, 3>(0, position_ids.clone());
        let constant799_out1 = self.constant799.val();
        let equal20_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat98_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant799_out1)
        };
        let unsqueeze114_out1: Tensor<4> = gather117_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze115_out1: Tensor<4> = gather118_out1.unsqueeze_dims::<4>(&[1]);
        let constant802_out1 = self.constant802.val();
        let where20_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat98_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal20_out1, constant802_out1);
        let mul100_out1 = transpose42_out1.mul(unsqueeze114_out1.clone());
        let mul101_out1 = transpose43_out1.mul(unsqueeze114_out1);
        let mul102_out1 = concat96_out1.mul(unsqueeze115_out1.clone());
        let mul103_out1 = concat97_out1.mul(unsqueeze115_out1);
        let expand23_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where20_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze110_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze110_out1.expand(shape)
        };
        let add99_out1 = mul100_out1.add(mul102_out1);
        let add100_out1 = mul101_out1.add(mul103_out1);
        let reshape56_out1 = expand23_out1.reshape(concat99_out1);
        let concat100_out1 = burn::tensor::Tensor::cat(
            [past_key_values_8_key, add100_out1].into(),
            2,
        );
        let shape73_out1: [i64; 4] = {
            let axes = &concat100_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze116_out1: Tensor<5> = concat100_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather119_out1 = shape73_out1[0] as i64;
        let gather120_out1 = shape73_out1[2] as i64;
        let unsqueeze117_out1 = [gather119_out1 as i64];
        let unsqueeze118_out1 = [gather120_out1 as i64];
        let constant809_out1: [i64; 1] = [2i64];
        let constant810_out1: [i64; 1] = [7i64];
        let constant811_out1: [i64; 1] = [64i64];
        let concat101_out1: [i64; 5usize] = [
            &unsqueeze117_out1[..],
            &constant809_out1[..],
            &constant810_out1[..],
            &unsqueeze118_out1[..],
            &constant811_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant812_out1: [i64; 1] = [14i64];
        let constant813_out1: [i64; 1] = [64i64];
        let concat102_out1: [i64; 4usize] = [
            &unsqueeze117_out1[..],
            &constant812_out1[..],
            &unsqueeze118_out1[..],
            &constant813_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant814_out1 = self.constant814.val();
        let equal21_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat101_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant814_out1)
        };
        let constant815_out1 = self.constant815.val();
        let where21_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat101_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal21_out1, constant815_out1);
        let expand24_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where21_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze116_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze116_out1.expand(shape)
        };
        let reshape57_out1 = expand24_out1.reshape(concat102_out1);
        let shape74_out1: [i64; 4] = {
            let axes = &reshape57_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather121_out1 = shape74_out1[2] as i64;
        let unsqueeze119_out1 = [gather121_out1 as i64];
        let slice65_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze119_out1[0]]);
        let (matmul77_out1,) = {
            let q = add99_out1;
            let k = reshape57_out1;
            let v = reshape56_out1;
            let matmul77_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice65_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul77_out1,)
        };
        let transpose46_out1 = matmul77_out1.permute([0, 2, 1, 3]);
        let reshape58_out1 = transpose46_out1.reshape(concat94_out1);
        let linear60_out1 = self.linear60.forward(reshape58_out1);
        let add102_out1 = add93_out1.add(linear60_out1);
        let constant823_out1 = self.constant823.val();
        let pow18_out1 = add102_out1
            .clone()
            .powf((constant823_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean18_out1 = { pow18_out1.mean_dim(2usize) };
        let constant824_out1 = self.constant824.val();
        let add103_out1 = reducemean18_out1
            .add((constant824_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt18_out1 = add103_out1.sqrt();
        let constant825_out1 = self.constant825.val();
        let div18_out1 = (constant825_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt18_out1);
        let mul106_out1 = add102_out1.clone().mul(div18_out1);
        let constant826_out1 = self.constant826.val();
        let mul107_out1 = (constant826_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul106_out1);
        let linear61_out1 = self.linear61.forward(mul107_out1.clone());
        let linear62_out1 = self.linear62.forward(mul107_out1);
        let sigmoid9_out1 = burn::tensor::activation::sigmoid(linear61_out1.clone());
        let mul108_out1 = linear61_out1.mul(sigmoid9_out1);
        let mul109_out1 = mul108_out1.mul(linear62_out1);
        let linear63_out1 = self.linear63.forward(mul109_out1);
        let add104_out1 = add102_out1.add(linear63_out1);
        let constant830_out1 = self.constant830.val();
        let pow19_out1 = add104_out1
            .clone()
            .powf((constant830_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean19_out1 = { pow19_out1.mean_dim(2usize) };
        let constant831_out1 = self.constant831.val();
        let add105_out1 = reducemean19_out1
            .add((constant831_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt19_out1 = add105_out1.sqrt();
        let constant832_out1 = self.constant832.val();
        let div19_out1 = (constant832_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt19_out1);
        let mul110_out1 = add104_out1.clone().mul(div19_out1);
        let constant833_out1 = self.constant833.val();
        let mul111_out1 = (constant833_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul110_out1);
        let shape75_out1: [i64; 3] = {
            let axes = &mul111_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear64_out1 = self.linear64.forward(mul111_out1.clone());
        let linear65_out1 = self.linear65.forward(mul111_out1.clone());
        let linear66_out1 = self.linear66.forward(mul111_out1);
        let gather122_out1 = shape75_out1[0] as i64;
        let gather123_out1 = shape75_out1[1] as i64;
        let constant839_out1 = self.constant839.val();
        let add106_out1 = (constant839_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear64_out1);
        let constant840_out1 = self.constant840.val();
        let add107_out1 = (constant840_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear65_out1);
        let constant841_out1 = self.constant841.val();
        let add108_out1 = (constant841_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear66_out1);
        let unsqueeze120_out1 = [gather122_out1 as i64];
        let unsqueeze121_out1 = [gather123_out1 as i64];
        let constant844_out1: [i64; 1] = [14i64];
        let constant845_out1: [i64; 1] = [64i64];
        let concat103_out1: [i64; 4usize] = [
            &unsqueeze120_out1[..],
            &unsqueeze121_out1[..],
            &constant844_out1[..],
            &constant845_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant846_out1: [i64; 1] = [2i64];
        let constant847_out1: [i64; 1] = [64i64];
        let concat104_out1: [i64; 4usize] = [
            &unsqueeze120_out1[..],
            &unsqueeze121_out1[..],
            &constant846_out1[..],
            &constant847_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant848_out1: [i64; 1] = [896i64];
        let concat105_out1: [i64; 3usize] = [
            &unsqueeze120_out1[..],
            &unsqueeze121_out1[..],
            &constant848_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape59_out1 = add106_out1.reshape(concat103_out1);
        let reshape60_out1 = add107_out1.reshape(concat104_out1);
        let reshape61_out1 = add108_out1.reshape(concat104_out1);
        let transpose47_out1 = reshape59_out1.permute([0, 2, 1, 3]);
        let transpose48_out1 = reshape60_out1.permute([0, 2, 1, 3]);
        let transpose49_out1 = reshape61_out1.permute([0, 2, 1, 3]);
        let shape76_out1: [i64; 4] = {
            let axes = &transpose48_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat106_out1 = burn::tensor::Tensor::cat(
            [past_key_values_9_value, transpose49_out1].into(),
            2,
        );
        let slice66_out1 = transpose47_out1.clone().slice(s![.., .., .., 0..32]);
        let slice67_out1 = transpose47_out1.clone().slice(s![.., .., .., 32..]);
        let slice68_out1 = transpose48_out1.clone().slice(s![.., .., .., 0..32]);
        let slice69_out1 = transpose48_out1.clone().slice(s![.., .., .., 32..]);
        let gather124_out1 = shape76_out1[2] as i64;
        let shape77_out1: [i64; 4] = {
            let axes = &concat106_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze122_out1: Tensor<5> = concat106_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg19_out1 = slice67_out1.neg();
        let neg20_out1 = slice69_out1.neg();
        let add109_out1 = gather124_out1 + gather12_out1;
        let gather125_out1 = shape77_out1[0] as i64;
        let gather126_out1 = shape77_out1[2] as i64;
        let concat107_out1 = burn::tensor::Tensor::cat(
            [neg19_out1, slice66_out1].into(),
            3,
        );
        let concat108_out1 = burn::tensor::Tensor::cat(
            [neg20_out1, slice68_out1].into(),
            3,
        );
        let unsqueeze123_out1 = [add109_out1 as i64];
        let unsqueeze124_out1 = [gather125_out1 as i64];
        let unsqueeze125_out1 = [gather126_out1 as i64];
        let slice70_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze123_out1[0], ..]);
        let slice71_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze123_out1[0], ..]);
        let constant878_out1: [i64; 1] = [2i64];
        let constant879_out1: [i64; 1] = [7i64];
        let constant880_out1: [i64; 1] = [64i64];
        let concat109_out1: [i64; 5usize] = [
            &unsqueeze124_out1[..],
            &constant878_out1[..],
            &constant879_out1[..],
            &unsqueeze125_out1[..],
            &constant880_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant881_out1: [i64; 1] = [14i64];
        let constant882_out1: [i64; 1] = [64i64];
        let concat110_out1: [i64; 4usize] = [
            &unsqueeze124_out1[..],
            &constant881_out1[..],
            &unsqueeze125_out1[..],
            &constant882_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather127_out1 = slice70_out1.take::<2, 3>(0, position_ids.clone());
        let gather128_out1 = slice71_out1.take::<2, 3>(0, position_ids.clone());
        let constant883_out1 = self.constant883.val();
        let equal22_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat109_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant883_out1)
        };
        let unsqueeze126_out1: Tensor<4> = gather127_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze127_out1: Tensor<4> = gather128_out1.unsqueeze_dims::<4>(&[1]);
        let constant886_out1 = self.constant886.val();
        let where22_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat109_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal22_out1, constant886_out1);
        let mul112_out1 = transpose47_out1.mul(unsqueeze126_out1.clone());
        let mul113_out1 = transpose48_out1.mul(unsqueeze126_out1);
        let mul114_out1 = concat107_out1.mul(unsqueeze127_out1.clone());
        let mul115_out1 = concat108_out1.mul(unsqueeze127_out1);
        let expand25_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where22_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze122_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze122_out1.expand(shape)
        };
        let add110_out1 = mul112_out1.add(mul114_out1);
        let add111_out1 = mul113_out1.add(mul115_out1);
        let reshape62_out1 = expand25_out1.reshape(concat110_out1);
        let concat111_out1 = burn::tensor::Tensor::cat(
            [past_key_values_9_key, add111_out1].into(),
            2,
        );
        let shape78_out1: [i64; 4] = {
            let axes = &concat111_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze128_out1: Tensor<5> = concat111_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather129_out1 = shape78_out1[0] as i64;
        let gather130_out1 = shape78_out1[2] as i64;
        let unsqueeze129_out1 = [gather129_out1 as i64];
        let unsqueeze130_out1 = [gather130_out1 as i64];
        let constant893_out1: [i64; 1] = [2i64];
        let constant894_out1: [i64; 1] = [7i64];
        let constant895_out1: [i64; 1] = [64i64];
        let concat112_out1: [i64; 5usize] = [
            &unsqueeze129_out1[..],
            &constant893_out1[..],
            &constant894_out1[..],
            &unsqueeze130_out1[..],
            &constant895_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant896_out1: [i64; 1] = [14i64];
        let constant897_out1: [i64; 1] = [64i64];
        let concat113_out1: [i64; 4usize] = [
            &unsqueeze129_out1[..],
            &constant896_out1[..],
            &unsqueeze130_out1[..],
            &constant897_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant898_out1 = self.constant898.val();
        let equal23_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat112_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant898_out1)
        };
        let constant899_out1 = self.constant899.val();
        let where23_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat112_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal23_out1, constant899_out1);
        let expand26_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where23_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze128_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze128_out1.expand(shape)
        };
        let reshape63_out1 = expand26_out1.reshape(concat113_out1);
        let shape79_out1: [i64; 4] = {
            let axes = &reshape63_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather131_out1 = shape79_out1[2] as i64;
        let unsqueeze131_out1 = [gather131_out1 as i64];
        let slice72_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze131_out1[0]]);
        let (matmul86_out1,) = {
            let q = add110_out1;
            let k = reshape63_out1;
            let v = reshape62_out1;
            let matmul86_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice72_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul86_out1,)
        };
        let transpose51_out1 = matmul86_out1.permute([0, 2, 1, 3]);
        let reshape64_out1 = transpose51_out1.reshape(concat105_out1);
        let linear67_out1 = self.linear67.forward(reshape64_out1);
        let add113_out1 = add104_out1.add(linear67_out1);
        let constant907_out1 = self.constant907.val();
        let pow20_out1 = add113_out1
            .clone()
            .powf((constant907_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean20_out1 = { pow20_out1.mean_dim(2usize) };
        let constant908_out1 = self.constant908.val();
        let add114_out1 = reducemean20_out1
            .add((constant908_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt20_out1 = add114_out1.sqrt();
        let constant909_out1 = self.constant909.val();
        let div20_out1 = (constant909_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt20_out1);
        let mul118_out1 = add113_out1.clone().mul(div20_out1);
        let constant910_out1 = self.constant910.val();
        let mul119_out1 = (constant910_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul118_out1);
        let linear68_out1 = self.linear68.forward(mul119_out1.clone());
        let linear69_out1 = self.linear69.forward(mul119_out1);
        let sigmoid10_out1 = burn::tensor::activation::sigmoid(linear68_out1.clone());
        let mul120_out1 = linear68_out1.mul(sigmoid10_out1);
        let mul121_out1 = mul120_out1.mul(linear69_out1);
        let linear70_out1 = self.linear70.forward(mul121_out1);
        let add115_out1 = add113_out1.add(linear70_out1);
        let constant914_out1 = self.constant914.val();
        let pow21_out1 = add115_out1
            .clone()
            .powf((constant914_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean21_out1 = { pow21_out1.mean_dim(2usize) };
        let constant915_out1 = self.constant915.val();
        let add116_out1 = reducemean21_out1
            .add((constant915_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt21_out1 = add116_out1.sqrt();
        let constant916_out1 = self.constant916.val();
        let div21_out1 = (constant916_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt21_out1);
        let mul122_out1 = add115_out1.clone().mul(div21_out1);
        let constant917_out1 = self.constant917.val();
        let mul123_out1 = (constant917_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul122_out1);
        let shape80_out1: [i64; 3] = {
            let axes = &mul123_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear71_out1 = self.linear71.forward(mul123_out1.clone());
        let linear72_out1 = self.linear72.forward(mul123_out1.clone());
        let linear73_out1 = self.linear73.forward(mul123_out1);
        let gather132_out1 = shape80_out1[0] as i64;
        let gather133_out1 = shape80_out1[1] as i64;
        let constant923_out1 = self.constant923.val();
        let add117_out1 = (constant923_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear71_out1);
        let constant924_out1 = self.constant924.val();
        let add118_out1 = (constant924_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear72_out1);
        let constant925_out1 = self.constant925.val();
        let add119_out1 = (constant925_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear73_out1);
        let unsqueeze132_out1 = [gather132_out1 as i64];
        let unsqueeze133_out1 = [gather133_out1 as i64];
        let constant928_out1: [i64; 1] = [14i64];
        let constant929_out1: [i64; 1] = [64i64];
        let concat114_out1: [i64; 4usize] = [
            &unsqueeze132_out1[..],
            &unsqueeze133_out1[..],
            &constant928_out1[..],
            &constant929_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant930_out1: [i64; 1] = [2i64];
        let constant931_out1: [i64; 1] = [64i64];
        let concat115_out1: [i64; 4usize] = [
            &unsqueeze132_out1[..],
            &unsqueeze133_out1[..],
            &constant930_out1[..],
            &constant931_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant932_out1: [i64; 1] = [896i64];
        let concat116_out1: [i64; 3usize] = [
            &unsqueeze132_out1[..],
            &unsqueeze133_out1[..],
            &constant932_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape65_out1 = add117_out1.reshape(concat114_out1);
        let reshape66_out1 = add118_out1.reshape(concat115_out1);
        let reshape67_out1 = add119_out1.reshape(concat115_out1);
        let transpose52_out1 = reshape65_out1.permute([0, 2, 1, 3]);
        let transpose53_out1 = reshape66_out1.permute([0, 2, 1, 3]);
        let transpose54_out1 = reshape67_out1.permute([0, 2, 1, 3]);
        let shape81_out1: [i64; 4] = {
            let axes = &transpose53_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat117_out1 = burn::tensor::Tensor::cat(
            [past_key_values_10_value, transpose54_out1].into(),
            2,
        );
        let slice73_out1 = transpose52_out1.clone().slice(s![.., .., .., 0..32]);
        let slice74_out1 = transpose52_out1.clone().slice(s![.., .., .., 32..]);
        let slice75_out1 = transpose53_out1.clone().slice(s![.., .., .., 0..32]);
        let slice76_out1 = transpose53_out1.clone().slice(s![.., .., .., 32..]);
        let gather134_out1 = shape81_out1[2] as i64;
        let shape82_out1: [i64; 4] = {
            let axes = &concat117_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze134_out1: Tensor<5> = concat117_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg21_out1 = slice74_out1.neg();
        let neg22_out1 = slice76_out1.neg();
        let add120_out1 = gather134_out1 + gather13_out1;
        let gather135_out1 = shape82_out1[0] as i64;
        let gather136_out1 = shape82_out1[2] as i64;
        let concat118_out1 = burn::tensor::Tensor::cat(
            [neg21_out1, slice73_out1].into(),
            3,
        );
        let concat119_out1 = burn::tensor::Tensor::cat(
            [neg22_out1, slice75_out1].into(),
            3,
        );
        let unsqueeze135_out1 = [add120_out1 as i64];
        let unsqueeze136_out1 = [gather135_out1 as i64];
        let unsqueeze137_out1 = [gather136_out1 as i64];
        let slice77_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze135_out1[0], ..]);
        let slice78_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze135_out1[0], ..]);
        let constant962_out1: [i64; 1] = [2i64];
        let constant963_out1: [i64; 1] = [7i64];
        let constant964_out1: [i64; 1] = [64i64];
        let concat120_out1: [i64; 5usize] = [
            &unsqueeze136_out1[..],
            &constant962_out1[..],
            &constant963_out1[..],
            &unsqueeze137_out1[..],
            &constant964_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant965_out1: [i64; 1] = [14i64];
        let constant966_out1: [i64; 1] = [64i64];
        let concat121_out1: [i64; 4usize] = [
            &unsqueeze136_out1[..],
            &constant965_out1[..],
            &unsqueeze137_out1[..],
            &constant966_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather137_out1 = slice77_out1.take::<2, 3>(0, position_ids.clone());
        let gather138_out1 = slice78_out1.take::<2, 3>(0, position_ids.clone());
        let constant967_out1 = self.constant967.val();
        let equal24_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat120_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant967_out1)
        };
        let unsqueeze138_out1: Tensor<4> = gather137_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze139_out1: Tensor<4> = gather138_out1.unsqueeze_dims::<4>(&[1]);
        let constant970_out1 = self.constant970.val();
        let where24_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat120_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal24_out1, constant970_out1);
        let mul124_out1 = transpose52_out1.mul(unsqueeze138_out1.clone());
        let mul125_out1 = transpose53_out1.mul(unsqueeze138_out1);
        let mul126_out1 = concat118_out1.mul(unsqueeze139_out1.clone());
        let mul127_out1 = concat119_out1.mul(unsqueeze139_out1);
        let expand27_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where24_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze134_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze134_out1.expand(shape)
        };
        let add121_out1 = mul124_out1.add(mul126_out1);
        let add122_out1 = mul125_out1.add(mul127_out1);
        let reshape68_out1 = expand27_out1.reshape(concat121_out1);
        let concat122_out1 = burn::tensor::Tensor::cat(
            [past_key_values_10_key, add122_out1].into(),
            2,
        );
        let shape83_out1: [i64; 4] = {
            let axes = &concat122_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze140_out1: Tensor<5> = concat122_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather139_out1 = shape83_out1[0] as i64;
        let gather140_out1 = shape83_out1[2] as i64;
        let unsqueeze141_out1 = [gather139_out1 as i64];
        let unsqueeze142_out1 = [gather140_out1 as i64];
        let constant977_out1: [i64; 1] = [2i64];
        let constant978_out1: [i64; 1] = [7i64];
        let constant979_out1: [i64; 1] = [64i64];
        let concat123_out1: [i64; 5usize] = [
            &unsqueeze141_out1[..],
            &constant977_out1[..],
            &constant978_out1[..],
            &unsqueeze142_out1[..],
            &constant979_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant980_out1: [i64; 1] = [14i64];
        let constant981_out1: [i64; 1] = [64i64];
        let concat124_out1: [i64; 4usize] = [
            &unsqueeze141_out1[..],
            &constant980_out1[..],
            &unsqueeze142_out1[..],
            &constant981_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant982_out1 = self.constant982.val();
        let equal25_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat123_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant982_out1)
        };
        let constant983_out1 = self.constant983.val();
        let where25_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat123_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal25_out1, constant983_out1);
        let expand28_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where25_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze140_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze140_out1.expand(shape)
        };
        let reshape69_out1 = expand28_out1.reshape(concat124_out1);
        let shape84_out1: [i64; 4] = {
            let axes = &reshape69_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather141_out1 = shape84_out1[2] as i64;
        let unsqueeze143_out1 = [gather141_out1 as i64];
        let slice79_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze143_out1[0]]);
        let (matmul95_out1,) = {
            let q = add121_out1;
            let k = reshape69_out1;
            let v = reshape68_out1;
            let matmul95_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice79_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul95_out1,)
        };
        let transpose56_out1 = matmul95_out1.permute([0, 2, 1, 3]);
        let reshape70_out1 = transpose56_out1.reshape(concat116_out1);
        let linear74_out1 = self.linear74.forward(reshape70_out1);
        let add124_out1 = add115_out1.add(linear74_out1);
        let constant991_out1 = self.constant991.val();
        let pow22_out1 = add124_out1
            .clone()
            .powf((constant991_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean22_out1 = { pow22_out1.mean_dim(2usize) };
        let constant992_out1 = self.constant992.val();
        let add125_out1 = reducemean22_out1
            .add((constant992_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt22_out1 = add125_out1.sqrt();
        let constant993_out1 = self.constant993.val();
        let div22_out1 = (constant993_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt22_out1);
        let mul130_out1 = add124_out1.clone().mul(div22_out1);
        let constant994_out1 = self.constant994.val();
        let mul131_out1 = (constant994_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul130_out1);
        let linear75_out1 = self.linear75.forward(mul131_out1.clone());
        let linear76_out1 = self.linear76.forward(mul131_out1);
        let sigmoid11_out1 = burn::tensor::activation::sigmoid(linear75_out1.clone());
        let mul132_out1 = linear75_out1.mul(sigmoid11_out1);
        let mul133_out1 = mul132_out1.mul(linear76_out1);
        let linear77_out1 = self.linear77.forward(mul133_out1);
        let add126_out1 = add124_out1.add(linear77_out1);
        let constant998_out1 = self.constant998.val();
        let pow23_out1 = add126_out1
            .clone()
            .powf((constant998_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean23_out1 = { pow23_out1.mean_dim(2usize) };
        let constant999_out1 = self.constant999.val();
        let add127_out1 = reducemean23_out1
            .add((constant999_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt23_out1 = add127_out1.sqrt();
        let constant1000_out1 = self.constant1000.val();
        let div23_out1 = (constant1000_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt23_out1);
        let mul134_out1 = add126_out1.clone().mul(div23_out1);
        let constant1001_out1 = self.constant1001.val();
        let mul135_out1 = (constant1001_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul134_out1);
        let shape85_out1: [i64; 3] = {
            let axes = &mul135_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear78_out1 = self.linear78.forward(mul135_out1.clone());
        let linear79_out1 = self.linear79.forward(mul135_out1.clone());
        let linear80_out1 = self.linear80.forward(mul135_out1);
        let gather142_out1 = shape85_out1[0] as i64;
        let gather143_out1 = shape85_out1[1] as i64;
        let constant1007_out1 = self.constant1007.val();
        let add128_out1 = (constant1007_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear78_out1);
        let constant1008_out1 = self.constant1008.val();
        let add129_out1 = (constant1008_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear79_out1);
        let constant1009_out1 = self.constant1009.val();
        let add130_out1 = (constant1009_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear80_out1);
        let unsqueeze144_out1 = [gather142_out1 as i64];
        let unsqueeze145_out1 = [gather143_out1 as i64];
        let constant1012_out1: [i64; 1] = [14i64];
        let constant1013_out1: [i64; 1] = [64i64];
        let concat125_out1: [i64; 4usize] = [
            &unsqueeze144_out1[..],
            &unsqueeze145_out1[..],
            &constant1012_out1[..],
            &constant1013_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1014_out1: [i64; 1] = [2i64];
        let constant1015_out1: [i64; 1] = [64i64];
        let concat126_out1: [i64; 4usize] = [
            &unsqueeze144_out1[..],
            &unsqueeze145_out1[..],
            &constant1014_out1[..],
            &constant1015_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1016_out1: [i64; 1] = [896i64];
        let concat127_out1: [i64; 3usize] = [
            &unsqueeze144_out1[..],
            &unsqueeze145_out1[..],
            &constant1016_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape71_out1 = add128_out1.reshape(concat125_out1);
        let reshape72_out1 = add129_out1.reshape(concat126_out1);
        let reshape73_out1 = add130_out1.reshape(concat126_out1);
        let transpose57_out1 = reshape71_out1.permute([0, 2, 1, 3]);
        let transpose58_out1 = reshape72_out1.permute([0, 2, 1, 3]);
        let transpose59_out1 = reshape73_out1.permute([0, 2, 1, 3]);
        let shape86_out1: [i64; 4] = {
            let axes = &transpose58_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat128_out1 = burn::tensor::Tensor::cat(
            [past_key_values_11_value, transpose59_out1].into(),
            2,
        );
        let slice80_out1 = transpose57_out1.clone().slice(s![.., .., .., 0..32]);
        let slice81_out1 = transpose57_out1.clone().slice(s![.., .., .., 32..]);
        let slice82_out1 = transpose58_out1.clone().slice(s![.., .., .., 0..32]);
        let slice83_out1 = transpose58_out1.clone().slice(s![.., .., .., 32..]);
        let gather144_out1 = shape86_out1[2] as i64;
        let shape87_out1: [i64; 4] = {
            let axes = &concat128_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze146_out1: Tensor<5> = concat128_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg23_out1 = slice81_out1.neg();
        let neg24_out1 = slice83_out1.neg();
        let add131_out1 = gather144_out1 + gather14_out1;
        let gather145_out1 = shape87_out1[0] as i64;
        let gather146_out1 = shape87_out1[2] as i64;
        let concat129_out1 = burn::tensor::Tensor::cat(
            [neg23_out1, slice80_out1].into(),
            3,
        );
        let concat130_out1 = burn::tensor::Tensor::cat(
            [neg24_out1, slice82_out1].into(),
            3,
        );
        let unsqueeze147_out1 = [add131_out1 as i64];
        let unsqueeze148_out1 = [gather145_out1 as i64];
        let unsqueeze149_out1 = [gather146_out1 as i64];
        let slice84_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze147_out1[0], ..]);
        let slice85_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze147_out1[0], ..]);
        let constant1046_out1: [i64; 1] = [2i64];
        let constant1047_out1: [i64; 1] = [7i64];
        let constant1048_out1: [i64; 1] = [64i64];
        let concat131_out1: [i64; 5usize] = [
            &unsqueeze148_out1[..],
            &constant1046_out1[..],
            &constant1047_out1[..],
            &unsqueeze149_out1[..],
            &constant1048_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1049_out1: [i64; 1] = [14i64];
        let constant1050_out1: [i64; 1] = [64i64];
        let concat132_out1: [i64; 4usize] = [
            &unsqueeze148_out1[..],
            &constant1049_out1[..],
            &unsqueeze149_out1[..],
            &constant1050_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather147_out1 = slice84_out1.take::<2, 3>(0, position_ids.clone());
        let gather148_out1 = slice85_out1.take::<2, 3>(0, position_ids.clone());
        let constant1051_out1 = self.constant1051.val();
        let equal26_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat131_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1051_out1)
        };
        let unsqueeze150_out1: Tensor<4> = gather147_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze151_out1: Tensor<4> = gather148_out1.unsqueeze_dims::<4>(&[1]);
        let constant1054_out1 = self.constant1054.val();
        let where26_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat131_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal26_out1, constant1054_out1);
        let mul136_out1 = transpose57_out1.mul(unsqueeze150_out1.clone());
        let mul137_out1 = transpose58_out1.mul(unsqueeze150_out1);
        let mul138_out1 = concat129_out1.mul(unsqueeze151_out1.clone());
        let mul139_out1 = concat130_out1.mul(unsqueeze151_out1);
        let expand29_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where26_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze146_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze146_out1.expand(shape)
        };
        let add132_out1 = mul136_out1.add(mul138_out1);
        let add133_out1 = mul137_out1.add(mul139_out1);
        let reshape74_out1 = expand29_out1.reshape(concat132_out1);
        let concat133_out1 = burn::tensor::Tensor::cat(
            [past_key_values_11_key, add133_out1].into(),
            2,
        );
        let shape88_out1: [i64; 4] = {
            let axes = &concat133_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze152_out1: Tensor<5> = concat133_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather149_out1 = shape88_out1[0] as i64;
        let gather150_out1 = shape88_out1[2] as i64;
        let unsqueeze153_out1 = [gather149_out1 as i64];
        let unsqueeze154_out1 = [gather150_out1 as i64];
        let constant1061_out1: [i64; 1] = [2i64];
        let constant1062_out1: [i64; 1] = [7i64];
        let constant1063_out1: [i64; 1] = [64i64];
        let concat134_out1: [i64; 5usize] = [
            &unsqueeze153_out1[..],
            &constant1061_out1[..],
            &constant1062_out1[..],
            &unsqueeze154_out1[..],
            &constant1063_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1064_out1: [i64; 1] = [14i64];
        let constant1065_out1: [i64; 1] = [64i64];
        let concat135_out1: [i64; 4usize] = [
            &unsqueeze153_out1[..],
            &constant1064_out1[..],
            &unsqueeze154_out1[..],
            &constant1065_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1066_out1 = self.constant1066.val();
        let equal27_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat134_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1066_out1)
        };
        let constant1067_out1 = self.constant1067.val();
        let where27_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat134_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal27_out1, constant1067_out1);
        let expand30_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where27_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze152_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze152_out1.expand(shape)
        };
        let reshape75_out1 = expand30_out1.reshape(concat135_out1);
        let shape89_out1: [i64; 4] = {
            let axes = &reshape75_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather151_out1 = shape89_out1[2] as i64;
        let unsqueeze155_out1 = [gather151_out1 as i64];
        let slice86_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze155_out1[0]]);
        let (matmul104_out1,) = {
            let q = add132_out1;
            let k = reshape75_out1;
            let v = reshape74_out1;
            let matmul104_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice86_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul104_out1,)
        };
        let transpose61_out1 = matmul104_out1.permute([0, 2, 1, 3]);
        let reshape76_out1 = transpose61_out1.reshape(concat127_out1);
        let linear81_out1 = self.linear81.forward(reshape76_out1);
        let add135_out1 = add126_out1.add(linear81_out1);
        let constant1075_out1 = self.constant1075.val();
        let pow24_out1 = add135_out1
            .clone()
            .powf((constant1075_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean24_out1 = { pow24_out1.mean_dim(2usize) };
        let constant1076_out1 = self.constant1076.val();
        let add136_out1 = reducemean24_out1
            .add((constant1076_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt24_out1 = add136_out1.sqrt();
        let constant1077_out1 = self.constant1077.val();
        let div24_out1 = (constant1077_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt24_out1);
        let mul142_out1 = add135_out1.clone().mul(div24_out1);
        let constant1078_out1 = self.constant1078.val();
        let mul143_out1 = (constant1078_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul142_out1);
        let linear82_out1 = self.linear82.forward(mul143_out1.clone());
        let linear83_out1 = self.linear83.forward(mul143_out1);
        let sigmoid12_out1 = burn::tensor::activation::sigmoid(linear82_out1.clone());
        let mul144_out1 = linear82_out1.mul(sigmoid12_out1);
        let mul145_out1 = mul144_out1.mul(linear83_out1);
        let linear84_out1 = self.linear84.forward(mul145_out1);
        let add137_out1 = add135_out1.add(linear84_out1);
        let constant1082_out1 = self.constant1082.val();
        let pow25_out1 = add137_out1
            .clone()
            .powf((constant1082_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean25_out1 = { pow25_out1.mean_dim(2usize) };
        let constant1083_out1 = self.constant1083.val();
        let add138_out1 = reducemean25_out1
            .add((constant1083_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt25_out1 = add138_out1.sqrt();
        let constant1084_out1 = self.constant1084.val();
        let div25_out1 = (constant1084_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt25_out1);
        let mul146_out1 = add137_out1.clone().mul(div25_out1);
        let constant1085_out1 = self.constant1085.val();
        let mul147_out1 = (constant1085_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul146_out1);
        let shape90_out1: [i64; 3] = {
            let axes = &mul147_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear85_out1 = self.linear85.forward(mul147_out1.clone());
        let linear86_out1 = self.linear86.forward(mul147_out1.clone());
        let linear87_out1 = self.linear87.forward(mul147_out1);
        let gather152_out1 = shape90_out1[0] as i64;
        let gather153_out1 = shape90_out1[1] as i64;
        let constant1091_out1 = self.constant1091.val();
        let add139_out1 = (constant1091_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear85_out1);
        let constant1092_out1 = self.constant1092.val();
        let add140_out1 = (constant1092_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear86_out1);
        let constant1093_out1 = self.constant1093.val();
        let add141_out1 = (constant1093_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear87_out1);
        let unsqueeze156_out1 = [gather152_out1 as i64];
        let unsqueeze157_out1 = [gather153_out1 as i64];
        let constant1096_out1: [i64; 1] = [14i64];
        let constant1097_out1: [i64; 1] = [64i64];
        let concat136_out1: [i64; 4usize] = [
            &unsqueeze156_out1[..],
            &unsqueeze157_out1[..],
            &constant1096_out1[..],
            &constant1097_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1098_out1: [i64; 1] = [2i64];
        let constant1099_out1: [i64; 1] = [64i64];
        let concat137_out1: [i64; 4usize] = [
            &unsqueeze156_out1[..],
            &unsqueeze157_out1[..],
            &constant1098_out1[..],
            &constant1099_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1100_out1: [i64; 1] = [896i64];
        let concat138_out1: [i64; 3usize] = [
            &unsqueeze156_out1[..],
            &unsqueeze157_out1[..],
            &constant1100_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape77_out1 = add139_out1.reshape(concat136_out1);
        let reshape78_out1 = add140_out1.reshape(concat137_out1);
        let reshape79_out1 = add141_out1.reshape(concat137_out1);
        let transpose62_out1 = reshape77_out1.permute([0, 2, 1, 3]);
        let transpose63_out1 = reshape78_out1.permute([0, 2, 1, 3]);
        let transpose64_out1 = reshape79_out1.permute([0, 2, 1, 3]);
        let shape91_out1: [i64; 4] = {
            let axes = &transpose63_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat139_out1 = burn::tensor::Tensor::cat(
            [past_key_values_12_value, transpose64_out1].into(),
            2,
        );
        let slice87_out1 = transpose62_out1.clone().slice(s![.., .., .., 0..32]);
        let slice88_out1 = transpose62_out1.clone().slice(s![.., .., .., 32..]);
        let slice89_out1 = transpose63_out1.clone().slice(s![.., .., .., 0..32]);
        let slice90_out1 = transpose63_out1.clone().slice(s![.., .., .., 32..]);
        let gather154_out1 = shape91_out1[2] as i64;
        let shape92_out1: [i64; 4] = {
            let axes = &concat139_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze158_out1: Tensor<5> = concat139_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg25_out1 = slice88_out1.neg();
        let neg26_out1 = slice90_out1.neg();
        let add142_out1 = gather154_out1 + gather15_out1;
        let gather155_out1 = shape92_out1[0] as i64;
        let gather156_out1 = shape92_out1[2] as i64;
        let concat140_out1 = burn::tensor::Tensor::cat(
            [neg25_out1, slice87_out1].into(),
            3,
        );
        let concat141_out1 = burn::tensor::Tensor::cat(
            [neg26_out1, slice89_out1].into(),
            3,
        );
        let unsqueeze159_out1 = [add142_out1 as i64];
        let unsqueeze160_out1 = [gather155_out1 as i64];
        let unsqueeze161_out1 = [gather156_out1 as i64];
        let slice91_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze159_out1[0], ..]);
        let slice92_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze159_out1[0], ..]);
        let constant1130_out1: [i64; 1] = [2i64];
        let constant1131_out1: [i64; 1] = [7i64];
        let constant1132_out1: [i64; 1] = [64i64];
        let concat142_out1: [i64; 5usize] = [
            &unsqueeze160_out1[..],
            &constant1130_out1[..],
            &constant1131_out1[..],
            &unsqueeze161_out1[..],
            &constant1132_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1133_out1: [i64; 1] = [14i64];
        let constant1134_out1: [i64; 1] = [64i64];
        let concat143_out1: [i64; 4usize] = [
            &unsqueeze160_out1[..],
            &constant1133_out1[..],
            &unsqueeze161_out1[..],
            &constant1134_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather157_out1 = slice91_out1.take::<2, 3>(0, position_ids.clone());
        let gather158_out1 = slice92_out1.take::<2, 3>(0, position_ids.clone());
        let constant1135_out1 = self.constant1135.val();
        let equal28_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat142_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1135_out1)
        };
        let unsqueeze162_out1: Tensor<4> = gather157_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze163_out1: Tensor<4> = gather158_out1.unsqueeze_dims::<4>(&[1]);
        let constant1138_out1 = self.constant1138.val();
        let where28_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat142_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal28_out1, constant1138_out1);
        let mul148_out1 = transpose62_out1.mul(unsqueeze162_out1.clone());
        let mul149_out1 = transpose63_out1.mul(unsqueeze162_out1);
        let mul150_out1 = concat140_out1.mul(unsqueeze163_out1.clone());
        let mul151_out1 = concat141_out1.mul(unsqueeze163_out1);
        let expand31_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where28_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze158_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze158_out1.expand(shape)
        };
        let add143_out1 = mul148_out1.add(mul150_out1);
        let add144_out1 = mul149_out1.add(mul151_out1);
        let reshape80_out1 = expand31_out1.reshape(concat143_out1);
        let concat144_out1 = burn::tensor::Tensor::cat(
            [past_key_values_12_key, add144_out1].into(),
            2,
        );
        let shape93_out1: [i64; 4] = {
            let axes = &concat144_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze164_out1: Tensor<5> = concat144_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather159_out1 = shape93_out1[0] as i64;
        let gather160_out1 = shape93_out1[2] as i64;
        let unsqueeze165_out1 = [gather159_out1 as i64];
        let unsqueeze166_out1 = [gather160_out1 as i64];
        let constant1145_out1: [i64; 1] = [2i64];
        let constant1146_out1: [i64; 1] = [7i64];
        let constant1147_out1: [i64; 1] = [64i64];
        let concat145_out1: [i64; 5usize] = [
            &unsqueeze165_out1[..],
            &constant1145_out1[..],
            &constant1146_out1[..],
            &unsqueeze166_out1[..],
            &constant1147_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1148_out1: [i64; 1] = [14i64];
        let constant1149_out1: [i64; 1] = [64i64];
        let concat146_out1: [i64; 4usize] = [
            &unsqueeze165_out1[..],
            &constant1148_out1[..],
            &unsqueeze166_out1[..],
            &constant1149_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1150_out1 = self.constant1150.val();
        let equal29_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat145_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1150_out1)
        };
        let constant1151_out1 = self.constant1151.val();
        let where29_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat145_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal29_out1, constant1151_out1);
        let expand32_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where29_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze164_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze164_out1.expand(shape)
        };
        let reshape81_out1 = expand32_out1.reshape(concat146_out1);
        let shape94_out1: [i64; 4] = {
            let axes = &reshape81_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather161_out1 = shape94_out1[2] as i64;
        let unsqueeze167_out1 = [gather161_out1 as i64];
        let slice93_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze167_out1[0]]);
        let (matmul113_out1,) = {
            let q = add143_out1;
            let k = reshape81_out1;
            let v = reshape80_out1;
            let matmul113_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice93_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul113_out1,)
        };
        let transpose66_out1 = matmul113_out1.permute([0, 2, 1, 3]);
        let reshape82_out1 = transpose66_out1.reshape(concat138_out1);
        let linear88_out1 = self.linear88.forward(reshape82_out1);
        let add146_out1 = add137_out1.add(linear88_out1);
        let constant1159_out1 = self.constant1159.val();
        let pow26_out1 = add146_out1
            .clone()
            .powf((constant1159_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean26_out1 = { pow26_out1.mean_dim(2usize) };
        let constant1160_out1 = self.constant1160.val();
        let add147_out1 = reducemean26_out1
            .add((constant1160_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt26_out1 = add147_out1.sqrt();
        let constant1161_out1 = self.constant1161.val();
        let div26_out1 = (constant1161_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt26_out1);
        let mul154_out1 = add146_out1.clone().mul(div26_out1);
        let constant1162_out1 = self.constant1162.val();
        let mul155_out1 = (constant1162_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul154_out1);
        let linear89_out1 = self.linear89.forward(mul155_out1.clone());
        let linear90_out1 = self.linear90.forward(mul155_out1);
        let sigmoid13_out1 = burn::tensor::activation::sigmoid(linear89_out1.clone());
        let mul156_out1 = linear89_out1.mul(sigmoid13_out1);
        let mul157_out1 = mul156_out1.mul(linear90_out1);
        let linear91_out1 = self.linear91.forward(mul157_out1);
        let add148_out1 = add146_out1.add(linear91_out1);
        let constant1166_out1 = self.constant1166.val();
        let pow27_out1 = add148_out1
            .clone()
            .powf((constant1166_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean27_out1 = { pow27_out1.mean_dim(2usize) };
        let constant1167_out1 = self.constant1167.val();
        let add149_out1 = reducemean27_out1
            .add((constant1167_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt27_out1 = add149_out1.sqrt();
        let constant1168_out1 = self.constant1168.val();
        let div27_out1 = (constant1168_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt27_out1);
        let mul158_out1 = add148_out1.clone().mul(div27_out1);
        let constant1169_out1 = self.constant1169.val();
        let mul159_out1 = (constant1169_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul158_out1);
        let shape95_out1: [i64; 3] = {
            let axes = &mul159_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear92_out1 = self.linear92.forward(mul159_out1.clone());
        let linear93_out1 = self.linear93.forward(mul159_out1.clone());
        let linear94_out1 = self.linear94.forward(mul159_out1);
        let gather162_out1 = shape95_out1[0] as i64;
        let gather163_out1 = shape95_out1[1] as i64;
        let constant1175_out1 = self.constant1175.val();
        let add150_out1 = (constant1175_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear92_out1);
        let constant1176_out1 = self.constant1176.val();
        let add151_out1 = (constant1176_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear93_out1);
        let constant1177_out1 = self.constant1177.val();
        let add152_out1 = (constant1177_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear94_out1);
        let unsqueeze168_out1 = [gather162_out1 as i64];
        let unsqueeze169_out1 = [gather163_out1 as i64];
        let constant1180_out1: [i64; 1] = [14i64];
        let constant1181_out1: [i64; 1] = [64i64];
        let concat147_out1: [i64; 4usize] = [
            &unsqueeze168_out1[..],
            &unsqueeze169_out1[..],
            &constant1180_out1[..],
            &constant1181_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1182_out1: [i64; 1] = [2i64];
        let constant1183_out1: [i64; 1] = [64i64];
        let concat148_out1: [i64; 4usize] = [
            &unsqueeze168_out1[..],
            &unsqueeze169_out1[..],
            &constant1182_out1[..],
            &constant1183_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1184_out1: [i64; 1] = [896i64];
        let concat149_out1: [i64; 3usize] = [
            &unsqueeze168_out1[..],
            &unsqueeze169_out1[..],
            &constant1184_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape83_out1 = add150_out1.reshape(concat147_out1);
        let reshape84_out1 = add151_out1.reshape(concat148_out1);
        let reshape85_out1 = add152_out1.reshape(concat148_out1);
        let transpose67_out1 = reshape83_out1.permute([0, 2, 1, 3]);
        let transpose68_out1 = reshape84_out1.permute([0, 2, 1, 3]);
        let transpose69_out1 = reshape85_out1.permute([0, 2, 1, 3]);
        let shape96_out1: [i64; 4] = {
            let axes = &transpose68_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat150_out1 = burn::tensor::Tensor::cat(
            [past_key_values_13_value, transpose69_out1].into(),
            2,
        );
        let slice94_out1 = transpose67_out1.clone().slice(s![.., .., .., 0..32]);
        let slice95_out1 = transpose67_out1.clone().slice(s![.., .., .., 32..]);
        let slice96_out1 = transpose68_out1.clone().slice(s![.., .., .., 0..32]);
        let slice97_out1 = transpose68_out1.clone().slice(s![.., .., .., 32..]);
        let gather164_out1 = shape96_out1[2] as i64;
        let shape97_out1: [i64; 4] = {
            let axes = &concat150_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze170_out1: Tensor<5> = concat150_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg27_out1 = slice95_out1.neg();
        let neg28_out1 = slice97_out1.neg();
        let add153_out1 = gather164_out1 + gather16_out1;
        let gather165_out1 = shape97_out1[0] as i64;
        let gather166_out1 = shape97_out1[2] as i64;
        let concat151_out1 = burn::tensor::Tensor::cat(
            [neg27_out1, slice94_out1].into(),
            3,
        );
        let concat152_out1 = burn::tensor::Tensor::cat(
            [neg28_out1, slice96_out1].into(),
            3,
        );
        let unsqueeze171_out1 = [add153_out1 as i64];
        let unsqueeze172_out1 = [gather165_out1 as i64];
        let unsqueeze173_out1 = [gather166_out1 as i64];
        let slice98_out1 = constant108_out1
            .clone()
            .slice(s![0..unsqueeze171_out1[0], ..]);
        let slice99_out1 = constant112_out1
            .clone()
            .slice(s![0..unsqueeze171_out1[0], ..]);
        let constant1214_out1: [i64; 1] = [2i64];
        let constant1215_out1: [i64; 1] = [7i64];
        let constant1216_out1: [i64; 1] = [64i64];
        let concat153_out1: [i64; 5usize] = [
            &unsqueeze172_out1[..],
            &constant1214_out1[..],
            &constant1215_out1[..],
            &unsqueeze173_out1[..],
            &constant1216_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1217_out1: [i64; 1] = [14i64];
        let constant1218_out1: [i64; 1] = [64i64];
        let concat154_out1: [i64; 4usize] = [
            &unsqueeze172_out1[..],
            &constant1217_out1[..],
            &unsqueeze173_out1[..],
            &constant1218_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather167_out1 = slice98_out1.take::<2, 3>(0, position_ids.clone());
        let gather168_out1 = slice99_out1.take::<2, 3>(0, position_ids);
        let constant1219_out1 = self.constant1219.val();
        let equal30_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat153_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1219_out1)
        };
        let unsqueeze174_out1: Tensor<4> = gather167_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze175_out1: Tensor<4> = gather168_out1.unsqueeze_dims::<4>(&[1]);
        let constant1222_out1 = self.constant1222.val();
        let where30_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat153_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal30_out1, constant1222_out1);
        let mul160_out1 = transpose67_out1.mul(unsqueeze174_out1.clone());
        let mul161_out1 = transpose68_out1.mul(unsqueeze174_out1);
        let mul162_out1 = concat151_out1.mul(unsqueeze175_out1.clone());
        let mul163_out1 = concat152_out1.mul(unsqueeze175_out1);
        let expand33_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where30_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze170_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze170_out1.expand(shape)
        };
        let add154_out1 = mul160_out1.add(mul162_out1);
        let add155_out1 = mul161_out1.add(mul163_out1);
        let reshape86_out1 = expand33_out1.reshape(concat154_out1);
        let concat155_out1 = burn::tensor::Tensor::cat(
            [past_key_values_13_key, add155_out1].into(),
            2,
        );
        let shape98_out1: [i64; 4] = {
            let axes = &concat155_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze176_out1: Tensor<5> = concat155_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather169_out1 = shape98_out1[0] as i64;
        let gather170_out1 = shape98_out1[2] as i64;
        let unsqueeze177_out1 = [gather169_out1 as i64];
        let unsqueeze178_out1 = [gather170_out1 as i64];
        let constant1229_out1: [i64; 1] = [2i64];
        let constant1230_out1: [i64; 1] = [7i64];
        let constant1231_out1: [i64; 1] = [64i64];
        let concat156_out1: [i64; 5usize] = [
            &unsqueeze177_out1[..],
            &constant1229_out1[..],
            &constant1230_out1[..],
            &unsqueeze178_out1[..],
            &constant1231_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1232_out1: [i64; 1] = [14i64];
        let constant1233_out1: [i64; 1] = [64i64];
        let concat157_out1: [i64; 4usize] = [
            &unsqueeze177_out1[..],
            &constant1232_out1[..],
            &unsqueeze178_out1[..],
            &constant1233_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1234_out1 = self.constant1234.val();
        let equal31_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat156_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1234_out1)
        };
        let constant1235_out1 = self.constant1235.val();
        let where31_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat156_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal31_out1, constant1235_out1);
        let expand34_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where31_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze176_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze176_out1.expand(shape)
        };
        let reshape87_out1 = expand34_out1.reshape(concat157_out1);
        let shape99_out1: [i64; 4] = {
            let axes = &reshape87_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather171_out1 = shape99_out1[2] as i64;
        let unsqueeze179_out1 = [gather171_out1 as i64];
        let slice100_out1 = scatternd1_out1
            .clone()
            .slice(s![.., .., .., 0..unsqueeze179_out1[0]]);
        let (matmul122_out1,) = {
            let q = add154_out1;
            let k = reshape87_out1;
            let v = reshape86_out1;
            let matmul122_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice100_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul122_out1,)
        };
        let transpose71_out1 = matmul122_out1.permute([0, 2, 1, 3]);
        let reshape88_out1 = transpose71_out1.reshape(concat149_out1);
        let linear95_out1 = self.linear95.forward(reshape88_out1);
        let add157_out1 = add148_out1.add(linear95_out1);
        (
            add157_out1,
            gather17_out1,
            constant108_out1,
            constant112_out1,
            scatternd1_out1,
            gather18_out1,
            gather19_out1,
            gather20_out1,
            gather21_out1,
            gather22_out1,
            gather23_out1,
            gather24_out1,
            gather25_out1,
            gather26_out1,
            transpose1_out1,
            concat11_out1,
            concat6_out1,
            concat23_out1,
            concat18_out1,
            concat34_out1,
            concat29_out1,
            concat45_out1,
            concat40_out1,
            concat56_out1,
            concat51_out1,
            concat67_out1,
            concat62_out1,
            concat78_out1,
            concat73_out1,
            concat89_out1,
            concat84_out1,
            concat100_out1,
            concat95_out1,
            concat111_out1,
            concat106_out1,
            concat122_out1,
            concat117_out1,
            concat133_out1,
            concat128_out1,
            concat144_out1,
            concat139_out1,
            concat155_out1,
            concat150_out1,
        )
    }
}
#[derive(Module, Debug)]
pub struct Submodule2 {
    constant1243: burn::module::Param<Tensor<1>>,
    constant1244: burn::module::Param<Tensor<1>>,
    constant1245: burn::module::Param<Tensor<1>>,
    constant1246: burn::module::Param<Tensor<1>>,
    linear96: Linear,
    linear97: Linear,
    linear98: Linear,
    constant1250: burn::module::Param<Tensor<1>>,
    constant1251: burn::module::Param<Tensor<1>>,
    constant1252: burn::module::Param<Tensor<1>>,
    constant1253: burn::module::Param<Tensor<1>>,
    linear99: Linear,
    linear100: Linear,
    linear101: Linear,
    constant1259: burn::module::Param<Tensor<1>>,
    constant1260: burn::module::Param<Tensor<1>>,
    constant1261: burn::module::Param<Tensor<1>>,
    constant1303: burn::module::Param<Tensor<1, Int>>,
    constant1306: burn::module::Param<Tensor<1, Int>>,
    constant1318: burn::module::Param<Tensor<1, Int>>,
    constant1319: burn::module::Param<Tensor<1, Int>>,
    linear102: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule2 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1243: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1244: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1245: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1246: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear96 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear97 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear98 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1250: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1251: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1252: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1253: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear99 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear100 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear101 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1259: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1260: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1261: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1303: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1306: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1318: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1319: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear102 = LinearConfig::new(896, 896).with_bias(false).init(device);
        Self {
            constant1243,
            constant1244,
            constant1245,
            constant1246,
            linear96,
            linear97,
            linear98,
            constant1250,
            constant1251,
            constant1252,
            constant1253,
            linear99,
            linear100,
            linear101,
            constant1259,
            constant1260,
            constant1261,
            constant1303,
            constant1306,
            constant1318,
            constant1319,
            linear102,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add157_out1: Tensor<3>,
        past_key_values_14_value: Tensor<4>,
        gather17_out1: i64,
        constant108_out1: Tensor<2>,
        constant112_out1: Tensor<2>,
        position_ids: Tensor<2, Int>,
        past_key_values_14_key: Tensor<4>,
        scatternd1_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<4>, Tensor<4>) {
        let constant1243_out1 = self.constant1243.val();
        let pow28_out1 = add157_out1
            .clone()
            .powf((constant1243_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean28_out1 = { pow28_out1.mean_dim(2usize) };
        let constant1244_out1 = self.constant1244.val();
        let add158_out1 = reducemean28_out1
            .add((constant1244_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt28_out1 = add158_out1.sqrt();
        let constant1245_out1 = self.constant1245.val();
        let div28_out1 = (constant1245_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt28_out1);
        let mul166_out1 = add157_out1.clone().mul(div28_out1);
        let constant1246_out1 = self.constant1246.val();
        let mul167_out1 = (constant1246_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul166_out1);
        let linear96_out1 = self.linear96.forward(mul167_out1.clone());
        let linear97_out1 = self.linear97.forward(mul167_out1);
        let sigmoid14_out1 = burn::tensor::activation::sigmoid(linear96_out1.clone());
        let mul168_out1 = linear96_out1.mul(sigmoid14_out1);
        let mul169_out1 = mul168_out1.mul(linear97_out1);
        let linear98_out1 = self.linear98.forward(mul169_out1);
        let add159_out1 = add157_out1.add(linear98_out1);
        let constant1250_out1 = self.constant1250.val();
        let pow29_out1 = add159_out1
            .clone()
            .powf((constant1250_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean29_out1 = { pow29_out1.mean_dim(2usize) };
        let constant1251_out1 = self.constant1251.val();
        let add160_out1 = reducemean29_out1
            .add((constant1251_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt29_out1 = add160_out1.sqrt();
        let constant1252_out1 = self.constant1252.val();
        let div29_out1 = (constant1252_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt29_out1);
        let mul170_out1 = add159_out1.clone().mul(div29_out1);
        let constant1253_out1 = self.constant1253.val();
        let mul171_out1 = (constant1253_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul170_out1);
        let shape100_out1: [i64; 3] = {
            let axes = &mul171_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear99_out1 = self.linear99.forward(mul171_out1.clone());
        let linear100_out1 = self.linear100.forward(mul171_out1.clone());
        let linear101_out1 = self.linear101.forward(mul171_out1);
        let gather172_out1 = shape100_out1[0] as i64;
        let gather173_out1 = shape100_out1[1] as i64;
        let constant1259_out1 = self.constant1259.val();
        let add161_out1 = (constant1259_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear99_out1);
        let constant1260_out1 = self.constant1260.val();
        let add162_out1 = (constant1260_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear100_out1);
        let constant1261_out1 = self.constant1261.val();
        let add163_out1 = (constant1261_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear101_out1);
        let unsqueeze180_out1 = [gather172_out1 as i64];
        let unsqueeze181_out1 = [gather173_out1 as i64];
        let constant1264_out1: [i64; 1] = [14i64];
        let constant1265_out1: [i64; 1] = [64i64];
        let concat158_out1: [i64; 4usize] = [
            &unsqueeze180_out1[..],
            &unsqueeze181_out1[..],
            &constant1264_out1[..],
            &constant1265_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1266_out1: [i64; 1] = [2i64];
        let constant1267_out1: [i64; 1] = [64i64];
        let concat159_out1: [i64; 4usize] = [
            &unsqueeze180_out1[..],
            &unsqueeze181_out1[..],
            &constant1266_out1[..],
            &constant1267_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1268_out1: [i64; 1] = [896i64];
        let concat160_out1: [i64; 3usize] = [
            &unsqueeze180_out1[..],
            &unsqueeze181_out1[..],
            &constant1268_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape89_out1 = add161_out1.reshape(concat158_out1);
        let reshape90_out1 = add162_out1.reshape(concat159_out1);
        let reshape91_out1 = add163_out1.reshape(concat159_out1);
        let transpose72_out1 = reshape89_out1.permute([0, 2, 1, 3]);
        let transpose73_out1 = reshape90_out1.permute([0, 2, 1, 3]);
        let transpose74_out1 = reshape91_out1.permute([0, 2, 1, 3]);
        let shape101_out1: [i64; 4] = {
            let axes = &transpose73_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat161_out1 = burn::tensor::Tensor::cat(
            [past_key_values_14_value, transpose74_out1].into(),
            2,
        );
        let slice101_out1 = transpose72_out1.clone().slice(s![.., .., .., 0..32]);
        let slice102_out1 = transpose72_out1.clone().slice(s![.., .., .., 32..]);
        let slice103_out1 = transpose73_out1.clone().slice(s![.., .., .., 0..32]);
        let slice104_out1 = transpose73_out1.clone().slice(s![.., .., .., 32..]);
        let gather174_out1 = shape101_out1[2] as i64;
        let shape102_out1: [i64; 4] = {
            let axes = &concat161_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze182_out1: Tensor<5> = concat161_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg29_out1 = slice102_out1.neg();
        let neg30_out1 = slice104_out1.neg();
        let add164_out1 = gather174_out1 + gather17_out1;
        let gather175_out1 = shape102_out1[0] as i64;
        let gather176_out1 = shape102_out1[2] as i64;
        let concat162_out1 = burn::tensor::Tensor::cat(
            [neg29_out1, slice101_out1].into(),
            3,
        );
        let concat163_out1 = burn::tensor::Tensor::cat(
            [neg30_out1, slice103_out1].into(),
            3,
        );
        let unsqueeze183_out1 = [add164_out1 as i64];
        let unsqueeze184_out1 = [gather175_out1 as i64];
        let unsqueeze185_out1 = [gather176_out1 as i64];
        let slice105_out1 = constant108_out1.slice(s![0..unsqueeze183_out1[0], ..]);
        let slice106_out1 = constant112_out1.slice(s![0..unsqueeze183_out1[0], ..]);
        let constant1298_out1: [i64; 1] = [2i64];
        let constant1299_out1: [i64; 1] = [7i64];
        let constant1300_out1: [i64; 1] = [64i64];
        let concat164_out1: [i64; 5usize] = [
            &unsqueeze184_out1[..],
            &constant1298_out1[..],
            &constant1299_out1[..],
            &unsqueeze185_out1[..],
            &constant1300_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1301_out1: [i64; 1] = [14i64];
        let constant1302_out1: [i64; 1] = [64i64];
        let concat165_out1: [i64; 4usize] = [
            &unsqueeze184_out1[..],
            &constant1301_out1[..],
            &unsqueeze185_out1[..],
            &constant1302_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather177_out1 = slice105_out1.take::<2, 3>(0, position_ids.clone());
        let gather178_out1 = slice106_out1.take::<2, 3>(0, position_ids);
        let constant1303_out1 = self.constant1303.val();
        let equal32_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat164_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1303_out1)
        };
        let unsqueeze186_out1: Tensor<4> = gather177_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze187_out1: Tensor<4> = gather178_out1.unsqueeze_dims::<4>(&[1]);
        let constant1306_out1 = self.constant1306.val();
        let where32_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat164_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal32_out1, constant1306_out1);
        let mul172_out1 = transpose72_out1.mul(unsqueeze186_out1.clone());
        let mul173_out1 = transpose73_out1.mul(unsqueeze186_out1);
        let mul174_out1 = concat162_out1.mul(unsqueeze187_out1.clone());
        let mul175_out1 = concat163_out1.mul(unsqueeze187_out1);
        let expand35_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where32_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze182_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze182_out1.expand(shape)
        };
        let add165_out1 = mul172_out1.add(mul174_out1);
        let add166_out1 = mul173_out1.add(mul175_out1);
        let reshape92_out1 = expand35_out1.reshape(concat165_out1);
        let concat166_out1 = burn::tensor::Tensor::cat(
            [past_key_values_14_key, add166_out1].into(),
            2,
        );
        let shape103_out1: [i64; 4] = {
            let axes = &concat166_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze188_out1: Tensor<5> = concat166_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather179_out1 = shape103_out1[0] as i64;
        let gather180_out1 = shape103_out1[2] as i64;
        let unsqueeze189_out1 = [gather179_out1 as i64];
        let unsqueeze190_out1 = [gather180_out1 as i64];
        let constant1313_out1: [i64; 1] = [2i64];
        let constant1314_out1: [i64; 1] = [7i64];
        let constant1315_out1: [i64; 1] = [64i64];
        let concat167_out1: [i64; 5usize] = [
            &unsqueeze189_out1[..],
            &constant1313_out1[..],
            &constant1314_out1[..],
            &unsqueeze190_out1[..],
            &constant1315_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1316_out1: [i64; 1] = [14i64];
        let constant1317_out1: [i64; 1] = [64i64];
        let concat168_out1: [i64; 4usize] = [
            &unsqueeze189_out1[..],
            &constant1316_out1[..],
            &unsqueeze190_out1[..],
            &constant1317_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1318_out1 = self.constant1318.val();
        let equal33_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat167_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1318_out1)
        };
        let constant1319_out1 = self.constant1319.val();
        let where33_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat167_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal33_out1, constant1319_out1);
        let expand36_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where33_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze188_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze188_out1.expand(shape)
        };
        let reshape93_out1 = expand36_out1.reshape(concat168_out1);
        let shape104_out1: [i64; 4] = {
            let axes = &reshape93_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather181_out1 = shape104_out1[2] as i64;
        let unsqueeze191_out1 = [gather181_out1 as i64];
        let slice107_out1 = scatternd1_out1
            .slice(s![.., .., .., 0..unsqueeze191_out1[0]]);
        let (matmul131_out1,) = {
            let q = add165_out1;
            let k = reshape93_out1;
            let v = reshape92_out1;
            let matmul131_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice107_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul131_out1,)
        };
        let transpose76_out1 = matmul131_out1.permute([0, 2, 1, 3]);
        let reshape94_out1 = transpose76_out1.reshape(concat160_out1);
        let linear102_out1 = self.linear102.forward(reshape94_out1);
        let add168_out1 = add159_out1.add(linear102_out1);
        (add168_out1, concat166_out1, concat161_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule3 {
    constant1327: burn::module::Param<Tensor<1>>,
    constant1328: burn::module::Param<Tensor<1>>,
    constant1329: burn::module::Param<Tensor<1>>,
    constant1330: burn::module::Param<Tensor<1>>,
    linear103: Linear,
    linear104: Linear,
    linear105: Linear,
    constant1334: burn::module::Param<Tensor<1>>,
    constant1335: burn::module::Param<Tensor<1>>,
    constant1336: burn::module::Param<Tensor<1>>,
    constant1337: burn::module::Param<Tensor<1>>,
    linear106: Linear,
    linear107: Linear,
    linear108: Linear,
    constant1343: burn::module::Param<Tensor<1>>,
    constant1344: burn::module::Param<Tensor<1>>,
    constant1345: burn::module::Param<Tensor<1>>,
    constant1387: burn::module::Param<Tensor<1, Int>>,
    constant1390: burn::module::Param<Tensor<1, Int>>,
    constant1402: burn::module::Param<Tensor<1, Int>>,
    constant1403: burn::module::Param<Tensor<1, Int>>,
    linear109: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule3 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1327: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1328: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1329: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1330: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear103 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear104 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear105 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1334: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1335: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1336: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1337: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear106 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear107 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear108 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1343: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1344: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1345: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1387: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1390: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1402: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1403: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear109 = LinearConfig::new(896, 896).with_bias(false).init(device);
        Self {
            constant1327,
            constant1328,
            constant1329,
            constant1330,
            linear103,
            linear104,
            linear105,
            constant1334,
            constant1335,
            constant1336,
            constant1337,
            linear106,
            linear107,
            linear108,
            constant1343,
            constant1344,
            constant1345,
            constant1387,
            constant1390,
            constant1402,
            constant1403,
            linear109,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add168_out1: Tensor<3>,
        past_key_values_15_value: Tensor<4>,
        gather18_out1: i64,
        constant108_out1: Tensor<2>,
        constant112_out1: Tensor<2>,
        position_ids: Tensor<2, Int>,
        past_key_values_15_key: Tensor<4>,
        scatternd1_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<4>, Tensor<4>) {
        let constant1327_out1 = self.constant1327.val();
        let pow30_out1 = add168_out1
            .clone()
            .powf((constant1327_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean30_out1 = { pow30_out1.mean_dim(2usize) };
        let constant1328_out1 = self.constant1328.val();
        let add169_out1 = reducemean30_out1
            .add((constant1328_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt30_out1 = add169_out1.sqrt();
        let constant1329_out1 = self.constant1329.val();
        let div30_out1 = (constant1329_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt30_out1);
        let mul178_out1 = add168_out1.clone().mul(div30_out1);
        let constant1330_out1 = self.constant1330.val();
        let mul179_out1 = (constant1330_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul178_out1);
        let linear103_out1 = self.linear103.forward(mul179_out1.clone());
        let linear104_out1 = self.linear104.forward(mul179_out1);
        let sigmoid15_out1 = burn::tensor::activation::sigmoid(linear103_out1.clone());
        let mul180_out1 = linear103_out1.mul(sigmoid15_out1);
        let mul181_out1 = mul180_out1.mul(linear104_out1);
        let linear105_out1 = self.linear105.forward(mul181_out1);
        let add170_out1 = add168_out1.add(linear105_out1);
        let constant1334_out1 = self.constant1334.val();
        let pow31_out1 = add170_out1
            .clone()
            .powf((constant1334_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean31_out1 = { pow31_out1.mean_dim(2usize) };
        let constant1335_out1 = self.constant1335.val();
        let add171_out1 = reducemean31_out1
            .add((constant1335_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt31_out1 = add171_out1.sqrt();
        let constant1336_out1 = self.constant1336.val();
        let div31_out1 = (constant1336_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt31_out1);
        let mul182_out1 = add170_out1.clone().mul(div31_out1);
        let constant1337_out1 = self.constant1337.val();
        let mul183_out1 = (constant1337_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul182_out1);
        let shape105_out1: [i64; 3] = {
            let axes = &mul183_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear106_out1 = self.linear106.forward(mul183_out1.clone());
        let linear107_out1 = self.linear107.forward(mul183_out1.clone());
        let linear108_out1 = self.linear108.forward(mul183_out1);
        let gather182_out1 = shape105_out1[0] as i64;
        let gather183_out1 = shape105_out1[1] as i64;
        let constant1343_out1 = self.constant1343.val();
        let add172_out1 = (constant1343_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear106_out1);
        let constant1344_out1 = self.constant1344.val();
        let add173_out1 = (constant1344_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear107_out1);
        let constant1345_out1 = self.constant1345.val();
        let add174_out1 = (constant1345_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear108_out1);
        let unsqueeze192_out1 = [gather182_out1 as i64];
        let unsqueeze193_out1 = [gather183_out1 as i64];
        let constant1348_out1: [i64; 1] = [14i64];
        let constant1349_out1: [i64; 1] = [64i64];
        let concat169_out1: [i64; 4usize] = [
            &unsqueeze192_out1[..],
            &unsqueeze193_out1[..],
            &constant1348_out1[..],
            &constant1349_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1350_out1: [i64; 1] = [2i64];
        let constant1351_out1: [i64; 1] = [64i64];
        let concat170_out1: [i64; 4usize] = [
            &unsqueeze192_out1[..],
            &unsqueeze193_out1[..],
            &constant1350_out1[..],
            &constant1351_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1352_out1: [i64; 1] = [896i64];
        let concat171_out1: [i64; 3usize] = [
            &unsqueeze192_out1[..],
            &unsqueeze193_out1[..],
            &constant1352_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape95_out1 = add172_out1.reshape(concat169_out1);
        let reshape96_out1 = add173_out1.reshape(concat170_out1);
        let reshape97_out1 = add174_out1.reshape(concat170_out1);
        let transpose77_out1 = reshape95_out1.permute([0, 2, 1, 3]);
        let transpose78_out1 = reshape96_out1.permute([0, 2, 1, 3]);
        let transpose79_out1 = reshape97_out1.permute([0, 2, 1, 3]);
        let shape106_out1: [i64; 4] = {
            let axes = &transpose78_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat172_out1 = burn::tensor::Tensor::cat(
            [past_key_values_15_value, transpose79_out1].into(),
            2,
        );
        let slice108_out1 = transpose77_out1.clone().slice(s![.., .., .., 0..32]);
        let slice109_out1 = transpose77_out1.clone().slice(s![.., .., .., 32..]);
        let slice110_out1 = transpose78_out1.clone().slice(s![.., .., .., 0..32]);
        let slice111_out1 = transpose78_out1.clone().slice(s![.., .., .., 32..]);
        let gather184_out1 = shape106_out1[2] as i64;
        let shape107_out1: [i64; 4] = {
            let axes = &concat172_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze194_out1: Tensor<5> = concat172_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg31_out1 = slice109_out1.neg();
        let neg32_out1 = slice111_out1.neg();
        let add175_out1 = gather184_out1 + gather18_out1;
        let gather185_out1 = shape107_out1[0] as i64;
        let gather186_out1 = shape107_out1[2] as i64;
        let concat173_out1 = burn::tensor::Tensor::cat(
            [neg31_out1, slice108_out1].into(),
            3,
        );
        let concat174_out1 = burn::tensor::Tensor::cat(
            [neg32_out1, slice110_out1].into(),
            3,
        );
        let unsqueeze195_out1 = [add175_out1 as i64];
        let unsqueeze196_out1 = [gather185_out1 as i64];
        let unsqueeze197_out1 = [gather186_out1 as i64];
        let slice112_out1 = constant108_out1.slice(s![0..unsqueeze195_out1[0], ..]);
        let slice113_out1 = constant112_out1.slice(s![0..unsqueeze195_out1[0], ..]);
        let constant1382_out1: [i64; 1] = [2i64];
        let constant1383_out1: [i64; 1] = [7i64];
        let constant1384_out1: [i64; 1] = [64i64];
        let concat175_out1: [i64; 5usize] = [
            &unsqueeze196_out1[..],
            &constant1382_out1[..],
            &constant1383_out1[..],
            &unsqueeze197_out1[..],
            &constant1384_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1385_out1: [i64; 1] = [14i64];
        let constant1386_out1: [i64; 1] = [64i64];
        let concat176_out1: [i64; 4usize] = [
            &unsqueeze196_out1[..],
            &constant1385_out1[..],
            &unsqueeze197_out1[..],
            &constant1386_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather187_out1 = slice112_out1.take::<2, 3>(0, position_ids.clone());
        let gather188_out1 = slice113_out1.take::<2, 3>(0, position_ids);
        let constant1387_out1 = self.constant1387.val();
        let equal34_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat175_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1387_out1)
        };
        let unsqueeze198_out1: Tensor<4> = gather187_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze199_out1: Tensor<4> = gather188_out1.unsqueeze_dims::<4>(&[1]);
        let constant1390_out1 = self.constant1390.val();
        let where34_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat175_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal34_out1, constant1390_out1);
        let mul184_out1 = transpose77_out1.mul(unsqueeze198_out1.clone());
        let mul185_out1 = transpose78_out1.mul(unsqueeze198_out1);
        let mul186_out1 = concat173_out1.mul(unsqueeze199_out1.clone());
        let mul187_out1 = concat174_out1.mul(unsqueeze199_out1);
        let expand37_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where34_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze194_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze194_out1.expand(shape)
        };
        let add176_out1 = mul184_out1.add(mul186_out1);
        let add177_out1 = mul185_out1.add(mul187_out1);
        let reshape98_out1 = expand37_out1.reshape(concat176_out1);
        let concat177_out1 = burn::tensor::Tensor::cat(
            [past_key_values_15_key, add177_out1].into(),
            2,
        );
        let shape108_out1: [i64; 4] = {
            let axes = &concat177_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze200_out1: Tensor<5> = concat177_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather189_out1 = shape108_out1[0] as i64;
        let gather190_out1 = shape108_out1[2] as i64;
        let unsqueeze201_out1 = [gather189_out1 as i64];
        let unsqueeze202_out1 = [gather190_out1 as i64];
        let constant1397_out1: [i64; 1] = [2i64];
        let constant1398_out1: [i64; 1] = [7i64];
        let constant1399_out1: [i64; 1] = [64i64];
        let concat178_out1: [i64; 5usize] = [
            &unsqueeze201_out1[..],
            &constant1397_out1[..],
            &constant1398_out1[..],
            &unsqueeze202_out1[..],
            &constant1399_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1400_out1: [i64; 1] = [14i64];
        let constant1401_out1: [i64; 1] = [64i64];
        let concat179_out1: [i64; 4usize] = [
            &unsqueeze201_out1[..],
            &constant1400_out1[..],
            &unsqueeze202_out1[..],
            &constant1401_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1402_out1 = self.constant1402.val();
        let equal35_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat178_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1402_out1)
        };
        let constant1403_out1 = self.constant1403.val();
        let where35_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat178_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal35_out1, constant1403_out1);
        let expand38_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where35_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze200_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze200_out1.expand(shape)
        };
        let reshape99_out1 = expand38_out1.reshape(concat179_out1);
        let shape109_out1: [i64; 4] = {
            let axes = &reshape99_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather191_out1 = shape109_out1[2] as i64;
        let unsqueeze203_out1 = [gather191_out1 as i64];
        let slice114_out1 = scatternd1_out1
            .slice(s![.., .., .., 0..unsqueeze203_out1[0]]);
        let (matmul140_out1,) = {
            let q = add176_out1;
            let k = reshape99_out1;
            let v = reshape98_out1;
            let matmul140_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice114_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul140_out1,)
        };
        let transpose81_out1 = matmul140_out1.permute([0, 2, 1, 3]);
        let reshape100_out1 = transpose81_out1.reshape(concat171_out1);
        let linear109_out1 = self.linear109.forward(reshape100_out1);
        let add179_out1 = add170_out1.add(linear109_out1);
        (add179_out1, concat177_out1, concat172_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule4 {
    constant1411: burn::module::Param<Tensor<1>>,
    constant1412: burn::module::Param<Tensor<1>>,
    constant1413: burn::module::Param<Tensor<1>>,
    constant1414: burn::module::Param<Tensor<1>>,
    linear110: Linear,
    linear111: Linear,
    linear112: Linear,
    constant1418: burn::module::Param<Tensor<1>>,
    constant1419: burn::module::Param<Tensor<1>>,
    constant1420: burn::module::Param<Tensor<1>>,
    constant1421: burn::module::Param<Tensor<1>>,
    linear113: Linear,
    linear114: Linear,
    linear115: Linear,
    constant1427: burn::module::Param<Tensor<1>>,
    constant1428: burn::module::Param<Tensor<1>>,
    constant1429: burn::module::Param<Tensor<1>>,
    constant1471: burn::module::Param<Tensor<1, Int>>,
    constant1474: burn::module::Param<Tensor<1, Int>>,
    constant1486: burn::module::Param<Tensor<1, Int>>,
    constant1487: burn::module::Param<Tensor<1, Int>>,
    linear116: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule4 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1411: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1412: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1413: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1414: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear110 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear111 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear112 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1418: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1419: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1420: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1421: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear113 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear114 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear115 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1427: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1428: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1429: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1471: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1474: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1486: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1487: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear116 = LinearConfig::new(896, 896).with_bias(false).init(device);
        Self {
            constant1411,
            constant1412,
            constant1413,
            constant1414,
            linear110,
            linear111,
            linear112,
            constant1418,
            constant1419,
            constant1420,
            constant1421,
            linear113,
            linear114,
            linear115,
            constant1427,
            constant1428,
            constant1429,
            constant1471,
            constant1474,
            constant1486,
            constant1487,
            linear116,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add179_out1: Tensor<3>,
        past_key_values_16_value: Tensor<4>,
        gather19_out1: i64,
        constant108_out1: Tensor<2>,
        constant112_out1: Tensor<2>,
        position_ids: Tensor<2, Int>,
        past_key_values_16_key: Tensor<4>,
        scatternd1_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<4>, Tensor<4>) {
        let constant1411_out1 = self.constant1411.val();
        let pow32_out1 = add179_out1
            .clone()
            .powf((constant1411_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean32_out1 = { pow32_out1.mean_dim(2usize) };
        let constant1412_out1 = self.constant1412.val();
        let add180_out1 = reducemean32_out1
            .add((constant1412_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt32_out1 = add180_out1.sqrt();
        let constant1413_out1 = self.constant1413.val();
        let div32_out1 = (constant1413_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt32_out1);
        let mul190_out1 = add179_out1.clone().mul(div32_out1);
        let constant1414_out1 = self.constant1414.val();
        let mul191_out1 = (constant1414_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul190_out1);
        let linear110_out1 = self.linear110.forward(mul191_out1.clone());
        let linear111_out1 = self.linear111.forward(mul191_out1);
        let sigmoid16_out1 = burn::tensor::activation::sigmoid(linear110_out1.clone());
        let mul192_out1 = linear110_out1.mul(sigmoid16_out1);
        let mul193_out1 = mul192_out1.mul(linear111_out1);
        let linear112_out1 = self.linear112.forward(mul193_out1);
        let add181_out1 = add179_out1.add(linear112_out1);
        let constant1418_out1 = self.constant1418.val();
        let pow33_out1 = add181_out1
            .clone()
            .powf((constant1418_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean33_out1 = { pow33_out1.mean_dim(2usize) };
        let constant1419_out1 = self.constant1419.val();
        let add182_out1 = reducemean33_out1
            .add((constant1419_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt33_out1 = add182_out1.sqrt();
        let constant1420_out1 = self.constant1420.val();
        let div33_out1 = (constant1420_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt33_out1);
        let mul194_out1 = add181_out1.clone().mul(div33_out1);
        let constant1421_out1 = self.constant1421.val();
        let mul195_out1 = (constant1421_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul194_out1);
        let shape110_out1: [i64; 3] = {
            let axes = &mul195_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear113_out1 = self.linear113.forward(mul195_out1.clone());
        let linear114_out1 = self.linear114.forward(mul195_out1.clone());
        let linear115_out1 = self.linear115.forward(mul195_out1);
        let gather192_out1 = shape110_out1[0] as i64;
        let gather193_out1 = shape110_out1[1] as i64;
        let constant1427_out1 = self.constant1427.val();
        let add183_out1 = (constant1427_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear113_out1);
        let constant1428_out1 = self.constant1428.val();
        let add184_out1 = (constant1428_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear114_out1);
        let constant1429_out1 = self.constant1429.val();
        let add185_out1 = (constant1429_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear115_out1);
        let unsqueeze204_out1 = [gather192_out1 as i64];
        let unsqueeze205_out1 = [gather193_out1 as i64];
        let constant1432_out1: [i64; 1] = [14i64];
        let constant1433_out1: [i64; 1] = [64i64];
        let concat180_out1: [i64; 4usize] = [
            &unsqueeze204_out1[..],
            &unsqueeze205_out1[..],
            &constant1432_out1[..],
            &constant1433_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1434_out1: [i64; 1] = [2i64];
        let constant1435_out1: [i64; 1] = [64i64];
        let concat181_out1: [i64; 4usize] = [
            &unsqueeze204_out1[..],
            &unsqueeze205_out1[..],
            &constant1434_out1[..],
            &constant1435_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1436_out1: [i64; 1] = [896i64];
        let concat182_out1: [i64; 3usize] = [
            &unsqueeze204_out1[..],
            &unsqueeze205_out1[..],
            &constant1436_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape101_out1 = add183_out1.reshape(concat180_out1);
        let reshape102_out1 = add184_out1.reshape(concat181_out1);
        let reshape103_out1 = add185_out1.reshape(concat181_out1);
        let transpose82_out1 = reshape101_out1.permute([0, 2, 1, 3]);
        let transpose83_out1 = reshape102_out1.permute([0, 2, 1, 3]);
        let transpose84_out1 = reshape103_out1.permute([0, 2, 1, 3]);
        let shape111_out1: [i64; 4] = {
            let axes = &transpose83_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat183_out1 = burn::tensor::Tensor::cat(
            [past_key_values_16_value, transpose84_out1].into(),
            2,
        );
        let slice115_out1 = transpose82_out1.clone().slice(s![.., .., .., 0..32]);
        let slice116_out1 = transpose82_out1.clone().slice(s![.., .., .., 32..]);
        let slice117_out1 = transpose83_out1.clone().slice(s![.., .., .., 0..32]);
        let slice118_out1 = transpose83_out1.clone().slice(s![.., .., .., 32..]);
        let gather194_out1 = shape111_out1[2] as i64;
        let shape112_out1: [i64; 4] = {
            let axes = &concat183_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze206_out1: Tensor<5> = concat183_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg33_out1 = slice116_out1.neg();
        let neg34_out1 = slice118_out1.neg();
        let add186_out1 = gather194_out1 + gather19_out1;
        let gather195_out1 = shape112_out1[0] as i64;
        let gather196_out1 = shape112_out1[2] as i64;
        let concat184_out1 = burn::tensor::Tensor::cat(
            [neg33_out1, slice115_out1].into(),
            3,
        );
        let concat185_out1 = burn::tensor::Tensor::cat(
            [neg34_out1, slice117_out1].into(),
            3,
        );
        let unsqueeze207_out1 = [add186_out1 as i64];
        let unsqueeze208_out1 = [gather195_out1 as i64];
        let unsqueeze209_out1 = [gather196_out1 as i64];
        let slice119_out1 = constant108_out1.slice(s![0..unsqueeze207_out1[0], ..]);
        let slice120_out1 = constant112_out1.slice(s![0..unsqueeze207_out1[0], ..]);
        let constant1466_out1: [i64; 1] = [2i64];
        let constant1467_out1: [i64; 1] = [7i64];
        let constant1468_out1: [i64; 1] = [64i64];
        let concat186_out1: [i64; 5usize] = [
            &unsqueeze208_out1[..],
            &constant1466_out1[..],
            &constant1467_out1[..],
            &unsqueeze209_out1[..],
            &constant1468_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1469_out1: [i64; 1] = [14i64];
        let constant1470_out1: [i64; 1] = [64i64];
        let concat187_out1: [i64; 4usize] = [
            &unsqueeze208_out1[..],
            &constant1469_out1[..],
            &unsqueeze209_out1[..],
            &constant1470_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather197_out1 = slice119_out1.take::<2, 3>(0, position_ids.clone());
        let gather198_out1 = slice120_out1.take::<2, 3>(0, position_ids);
        let constant1471_out1 = self.constant1471.val();
        let equal36_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat186_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1471_out1)
        };
        let unsqueeze210_out1: Tensor<4> = gather197_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze211_out1: Tensor<4> = gather198_out1.unsqueeze_dims::<4>(&[1]);
        let constant1474_out1 = self.constant1474.val();
        let where36_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat186_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal36_out1, constant1474_out1);
        let mul196_out1 = transpose82_out1.mul(unsqueeze210_out1.clone());
        let mul197_out1 = transpose83_out1.mul(unsqueeze210_out1);
        let mul198_out1 = concat184_out1.mul(unsqueeze211_out1.clone());
        let mul199_out1 = concat185_out1.mul(unsqueeze211_out1);
        let expand39_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where36_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze206_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze206_out1.expand(shape)
        };
        let add187_out1 = mul196_out1.add(mul198_out1);
        let add188_out1 = mul197_out1.add(mul199_out1);
        let reshape104_out1 = expand39_out1.reshape(concat187_out1);
        let concat188_out1 = burn::tensor::Tensor::cat(
            [past_key_values_16_key, add188_out1].into(),
            2,
        );
        let shape113_out1: [i64; 4] = {
            let axes = &concat188_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze212_out1: Tensor<5> = concat188_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather199_out1 = shape113_out1[0] as i64;
        let gather200_out1 = shape113_out1[2] as i64;
        let unsqueeze213_out1 = [gather199_out1 as i64];
        let unsqueeze214_out1 = [gather200_out1 as i64];
        let constant1481_out1: [i64; 1] = [2i64];
        let constant1482_out1: [i64; 1] = [7i64];
        let constant1483_out1: [i64; 1] = [64i64];
        let concat189_out1: [i64; 5usize] = [
            &unsqueeze213_out1[..],
            &constant1481_out1[..],
            &constant1482_out1[..],
            &unsqueeze214_out1[..],
            &constant1483_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1484_out1: [i64; 1] = [14i64];
        let constant1485_out1: [i64; 1] = [64i64];
        let concat190_out1: [i64; 4usize] = [
            &unsqueeze213_out1[..],
            &constant1484_out1[..],
            &unsqueeze214_out1[..],
            &constant1485_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1486_out1 = self.constant1486.val();
        let equal37_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat189_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1486_out1)
        };
        let constant1487_out1 = self.constant1487.val();
        let where37_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat189_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal37_out1, constant1487_out1);
        let expand40_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where37_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze212_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze212_out1.expand(shape)
        };
        let reshape105_out1 = expand40_out1.reshape(concat190_out1);
        let shape114_out1: [i64; 4] = {
            let axes = &reshape105_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather201_out1 = shape114_out1[2] as i64;
        let unsqueeze215_out1 = [gather201_out1 as i64];
        let slice121_out1 = scatternd1_out1
            .slice(s![.., .., .., 0..unsqueeze215_out1[0]]);
        let (matmul149_out1,) = {
            let q = add187_out1;
            let k = reshape105_out1;
            let v = reshape104_out1;
            let matmul149_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice121_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul149_out1,)
        };
        let transpose86_out1 = matmul149_out1.permute([0, 2, 1, 3]);
        let reshape106_out1 = transpose86_out1.reshape(concat182_out1);
        let linear116_out1 = self.linear116.forward(reshape106_out1);
        let add190_out1 = add181_out1.add(linear116_out1);
        (add190_out1, concat188_out1, concat183_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule5 {
    constant1495: burn::module::Param<Tensor<1>>,
    constant1496: burn::module::Param<Tensor<1>>,
    constant1497: burn::module::Param<Tensor<1>>,
    constant1498: burn::module::Param<Tensor<1>>,
    linear117: Linear,
    linear118: Linear,
    linear119: Linear,
    constant1502: burn::module::Param<Tensor<1>>,
    constant1503: burn::module::Param<Tensor<1>>,
    constant1504: burn::module::Param<Tensor<1>>,
    constant1505: burn::module::Param<Tensor<1>>,
    linear120: Linear,
    linear121: Linear,
    linear122: Linear,
    constant1511: burn::module::Param<Tensor<1>>,
    constant1512: burn::module::Param<Tensor<1>>,
    constant1513: burn::module::Param<Tensor<1>>,
    constant1555: burn::module::Param<Tensor<1, Int>>,
    constant1558: burn::module::Param<Tensor<1, Int>>,
    constant1570: burn::module::Param<Tensor<1, Int>>,
    constant1571: burn::module::Param<Tensor<1, Int>>,
    linear123: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule5 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1495: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1496: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1497: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1498: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear117 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear118 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear119 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1502: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1503: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1504: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1505: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear120 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear121 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear122 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1511: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1512: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1513: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1555: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1558: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1570: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1571: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear123 = LinearConfig::new(896, 896).with_bias(false).init(device);
        Self {
            constant1495,
            constant1496,
            constant1497,
            constant1498,
            linear117,
            linear118,
            linear119,
            constant1502,
            constant1503,
            constant1504,
            constant1505,
            linear120,
            linear121,
            linear122,
            constant1511,
            constant1512,
            constant1513,
            constant1555,
            constant1558,
            constant1570,
            constant1571,
            linear123,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add190_out1: Tensor<3>,
        past_key_values_17_value: Tensor<4>,
        gather20_out1: i64,
        constant108_out1: Tensor<2>,
        constant112_out1: Tensor<2>,
        position_ids: Tensor<2, Int>,
        past_key_values_17_key: Tensor<4>,
        scatternd1_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<4>, Tensor<4>) {
        let constant1495_out1 = self.constant1495.val();
        let pow34_out1 = add190_out1
            .clone()
            .powf((constant1495_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean34_out1 = { pow34_out1.mean_dim(2usize) };
        let constant1496_out1 = self.constant1496.val();
        let add191_out1 = reducemean34_out1
            .add((constant1496_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt34_out1 = add191_out1.sqrt();
        let constant1497_out1 = self.constant1497.val();
        let div34_out1 = (constant1497_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt34_out1);
        let mul202_out1 = add190_out1.clone().mul(div34_out1);
        let constant1498_out1 = self.constant1498.val();
        let mul203_out1 = (constant1498_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul202_out1);
        let linear117_out1 = self.linear117.forward(mul203_out1.clone());
        let linear118_out1 = self.linear118.forward(mul203_out1);
        let sigmoid17_out1 = burn::tensor::activation::sigmoid(linear117_out1.clone());
        let mul204_out1 = linear117_out1.mul(sigmoid17_out1);
        let mul205_out1 = mul204_out1.mul(linear118_out1);
        let linear119_out1 = self.linear119.forward(mul205_out1);
        let add192_out1 = add190_out1.add(linear119_out1);
        let constant1502_out1 = self.constant1502.val();
        let pow35_out1 = add192_out1
            .clone()
            .powf((constant1502_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean35_out1 = { pow35_out1.mean_dim(2usize) };
        let constant1503_out1 = self.constant1503.val();
        let add193_out1 = reducemean35_out1
            .add((constant1503_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt35_out1 = add193_out1.sqrt();
        let constant1504_out1 = self.constant1504.val();
        let div35_out1 = (constant1504_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt35_out1);
        let mul206_out1 = add192_out1.clone().mul(div35_out1);
        let constant1505_out1 = self.constant1505.val();
        let mul207_out1 = (constant1505_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul206_out1);
        let shape115_out1: [i64; 3] = {
            let axes = &mul207_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear120_out1 = self.linear120.forward(mul207_out1.clone());
        let linear121_out1 = self.linear121.forward(mul207_out1.clone());
        let linear122_out1 = self.linear122.forward(mul207_out1);
        let gather202_out1 = shape115_out1[0] as i64;
        let gather203_out1 = shape115_out1[1] as i64;
        let constant1511_out1 = self.constant1511.val();
        let add194_out1 = (constant1511_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear120_out1);
        let constant1512_out1 = self.constant1512.val();
        let add195_out1 = (constant1512_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear121_out1);
        let constant1513_out1 = self.constant1513.val();
        let add196_out1 = (constant1513_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear122_out1);
        let unsqueeze216_out1 = [gather202_out1 as i64];
        let unsqueeze217_out1 = [gather203_out1 as i64];
        let constant1516_out1: [i64; 1] = [14i64];
        let constant1517_out1: [i64; 1] = [64i64];
        let concat191_out1: [i64; 4usize] = [
            &unsqueeze216_out1[..],
            &unsqueeze217_out1[..],
            &constant1516_out1[..],
            &constant1517_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1518_out1: [i64; 1] = [2i64];
        let constant1519_out1: [i64; 1] = [64i64];
        let concat192_out1: [i64; 4usize] = [
            &unsqueeze216_out1[..],
            &unsqueeze217_out1[..],
            &constant1518_out1[..],
            &constant1519_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1520_out1: [i64; 1] = [896i64];
        let concat193_out1: [i64; 3usize] = [
            &unsqueeze216_out1[..],
            &unsqueeze217_out1[..],
            &constant1520_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape107_out1 = add194_out1.reshape(concat191_out1);
        let reshape108_out1 = add195_out1.reshape(concat192_out1);
        let reshape109_out1 = add196_out1.reshape(concat192_out1);
        let transpose87_out1 = reshape107_out1.permute([0, 2, 1, 3]);
        let transpose88_out1 = reshape108_out1.permute([0, 2, 1, 3]);
        let transpose89_out1 = reshape109_out1.permute([0, 2, 1, 3]);
        let shape116_out1: [i64; 4] = {
            let axes = &transpose88_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat194_out1 = burn::tensor::Tensor::cat(
            [past_key_values_17_value, transpose89_out1].into(),
            2,
        );
        let slice122_out1 = transpose87_out1.clone().slice(s![.., .., .., 0..32]);
        let slice123_out1 = transpose87_out1.clone().slice(s![.., .., .., 32..]);
        let slice124_out1 = transpose88_out1.clone().slice(s![.., .., .., 0..32]);
        let slice125_out1 = transpose88_out1.clone().slice(s![.., .., .., 32..]);
        let gather204_out1 = shape116_out1[2] as i64;
        let shape117_out1: [i64; 4] = {
            let axes = &concat194_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze218_out1: Tensor<5> = concat194_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg35_out1 = slice123_out1.neg();
        let neg36_out1 = slice125_out1.neg();
        let add197_out1 = gather204_out1 + gather20_out1;
        let gather205_out1 = shape117_out1[0] as i64;
        let gather206_out1 = shape117_out1[2] as i64;
        let concat195_out1 = burn::tensor::Tensor::cat(
            [neg35_out1, slice122_out1].into(),
            3,
        );
        let concat196_out1 = burn::tensor::Tensor::cat(
            [neg36_out1, slice124_out1].into(),
            3,
        );
        let unsqueeze219_out1 = [add197_out1 as i64];
        let unsqueeze220_out1 = [gather205_out1 as i64];
        let unsqueeze221_out1 = [gather206_out1 as i64];
        let slice126_out1 = constant108_out1.slice(s![0..unsqueeze219_out1[0], ..]);
        let slice127_out1 = constant112_out1.slice(s![0..unsqueeze219_out1[0], ..]);
        let constant1550_out1: [i64; 1] = [2i64];
        let constant1551_out1: [i64; 1] = [7i64];
        let constant1552_out1: [i64; 1] = [64i64];
        let concat197_out1: [i64; 5usize] = [
            &unsqueeze220_out1[..],
            &constant1550_out1[..],
            &constant1551_out1[..],
            &unsqueeze221_out1[..],
            &constant1552_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1553_out1: [i64; 1] = [14i64];
        let constant1554_out1: [i64; 1] = [64i64];
        let concat198_out1: [i64; 4usize] = [
            &unsqueeze220_out1[..],
            &constant1553_out1[..],
            &unsqueeze221_out1[..],
            &constant1554_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather207_out1 = slice126_out1.take::<2, 3>(0, position_ids.clone());
        let gather208_out1 = slice127_out1.take::<2, 3>(0, position_ids);
        let constant1555_out1 = self.constant1555.val();
        let equal38_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat197_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1555_out1)
        };
        let unsqueeze222_out1: Tensor<4> = gather207_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze223_out1: Tensor<4> = gather208_out1.unsqueeze_dims::<4>(&[1]);
        let constant1558_out1 = self.constant1558.val();
        let where38_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat197_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal38_out1, constant1558_out1);
        let mul208_out1 = transpose87_out1.mul(unsqueeze222_out1.clone());
        let mul209_out1 = transpose88_out1.mul(unsqueeze222_out1);
        let mul210_out1 = concat195_out1.mul(unsqueeze223_out1.clone());
        let mul211_out1 = concat196_out1.mul(unsqueeze223_out1);
        let expand41_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where38_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze218_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze218_out1.expand(shape)
        };
        let add198_out1 = mul208_out1.add(mul210_out1);
        let add199_out1 = mul209_out1.add(mul211_out1);
        let reshape110_out1 = expand41_out1.reshape(concat198_out1);
        let concat199_out1 = burn::tensor::Tensor::cat(
            [past_key_values_17_key, add199_out1].into(),
            2,
        );
        let shape118_out1: [i64; 4] = {
            let axes = &concat199_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze224_out1: Tensor<5> = concat199_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather209_out1 = shape118_out1[0] as i64;
        let gather210_out1 = shape118_out1[2] as i64;
        let unsqueeze225_out1 = [gather209_out1 as i64];
        let unsqueeze226_out1 = [gather210_out1 as i64];
        let constant1565_out1: [i64; 1] = [2i64];
        let constant1566_out1: [i64; 1] = [7i64];
        let constant1567_out1: [i64; 1] = [64i64];
        let concat200_out1: [i64; 5usize] = [
            &unsqueeze225_out1[..],
            &constant1565_out1[..],
            &constant1566_out1[..],
            &unsqueeze226_out1[..],
            &constant1567_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1568_out1: [i64; 1] = [14i64];
        let constant1569_out1: [i64; 1] = [64i64];
        let concat201_out1: [i64; 4usize] = [
            &unsqueeze225_out1[..],
            &constant1568_out1[..],
            &unsqueeze226_out1[..],
            &constant1569_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1570_out1 = self.constant1570.val();
        let equal39_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat200_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1570_out1)
        };
        let constant1571_out1 = self.constant1571.val();
        let where39_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat200_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal39_out1, constant1571_out1);
        let expand42_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where39_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze224_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze224_out1.expand(shape)
        };
        let reshape111_out1 = expand42_out1.reshape(concat201_out1);
        let shape119_out1: [i64; 4] = {
            let axes = &reshape111_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather211_out1 = shape119_out1[2] as i64;
        let unsqueeze227_out1 = [gather211_out1 as i64];
        let slice128_out1 = scatternd1_out1
            .slice(s![.., .., .., 0..unsqueeze227_out1[0]]);
        let (matmul158_out1,) = {
            let q = add198_out1;
            let k = reshape111_out1;
            let v = reshape110_out1;
            let matmul158_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice128_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul158_out1,)
        };
        let transpose91_out1 = matmul158_out1.permute([0, 2, 1, 3]);
        let reshape112_out1 = transpose91_out1.reshape(concat193_out1);
        let linear123_out1 = self.linear123.forward(reshape112_out1);
        let add201_out1 = add192_out1.add(linear123_out1);
        (add201_out1, concat199_out1, concat194_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule6 {
    constant1579: burn::module::Param<Tensor<1>>,
    constant1580: burn::module::Param<Tensor<1>>,
    constant1581: burn::module::Param<Tensor<1>>,
    constant1582: burn::module::Param<Tensor<1>>,
    linear124: Linear,
    linear125: Linear,
    linear126: Linear,
    constant1586: burn::module::Param<Tensor<1>>,
    constant1587: burn::module::Param<Tensor<1>>,
    constant1588: burn::module::Param<Tensor<1>>,
    constant1589: burn::module::Param<Tensor<1>>,
    linear127: Linear,
    linear128: Linear,
    linear129: Linear,
    constant1595: burn::module::Param<Tensor<1>>,
    constant1596: burn::module::Param<Tensor<1>>,
    constant1597: burn::module::Param<Tensor<1>>,
    constant1639: burn::module::Param<Tensor<1, Int>>,
    constant1642: burn::module::Param<Tensor<1, Int>>,
    constant1654: burn::module::Param<Tensor<1, Int>>,
    constant1655: burn::module::Param<Tensor<1, Int>>,
    linear130: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule6 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1579: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1580: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1581: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1582: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear124 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear125 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear126 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1586: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1587: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1588: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1589: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear127 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear128 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear129 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1595: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1596: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1597: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1639: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1642: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1654: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1655: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear130 = LinearConfig::new(896, 896).with_bias(false).init(device);
        Self {
            constant1579,
            constant1580,
            constant1581,
            constant1582,
            linear124,
            linear125,
            linear126,
            constant1586,
            constant1587,
            constant1588,
            constant1589,
            linear127,
            linear128,
            linear129,
            constant1595,
            constant1596,
            constant1597,
            constant1639,
            constant1642,
            constant1654,
            constant1655,
            linear130,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add201_out1: Tensor<3>,
        past_key_values_18_value: Tensor<4>,
        gather21_out1: i64,
        constant108_out1: Tensor<2>,
        constant112_out1: Tensor<2>,
        position_ids: Tensor<2, Int>,
        past_key_values_18_key: Tensor<4>,
        scatternd1_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<4>, Tensor<4>) {
        let constant1579_out1 = self.constant1579.val();
        let pow36_out1 = add201_out1
            .clone()
            .powf((constant1579_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean36_out1 = { pow36_out1.mean_dim(2usize) };
        let constant1580_out1 = self.constant1580.val();
        let add202_out1 = reducemean36_out1
            .add((constant1580_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt36_out1 = add202_out1.sqrt();
        let constant1581_out1 = self.constant1581.val();
        let div36_out1 = (constant1581_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt36_out1);
        let mul214_out1 = add201_out1.clone().mul(div36_out1);
        let constant1582_out1 = self.constant1582.val();
        let mul215_out1 = (constant1582_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul214_out1);
        let linear124_out1 = self.linear124.forward(mul215_out1.clone());
        let linear125_out1 = self.linear125.forward(mul215_out1);
        let sigmoid18_out1 = burn::tensor::activation::sigmoid(linear124_out1.clone());
        let mul216_out1 = linear124_out1.mul(sigmoid18_out1);
        let mul217_out1 = mul216_out1.mul(linear125_out1);
        let linear126_out1 = self.linear126.forward(mul217_out1);
        let add203_out1 = add201_out1.add(linear126_out1);
        let constant1586_out1 = self.constant1586.val();
        let pow37_out1 = add203_out1
            .clone()
            .powf((constant1586_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean37_out1 = { pow37_out1.mean_dim(2usize) };
        let constant1587_out1 = self.constant1587.val();
        let add204_out1 = reducemean37_out1
            .add((constant1587_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt37_out1 = add204_out1.sqrt();
        let constant1588_out1 = self.constant1588.val();
        let div37_out1 = (constant1588_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt37_out1);
        let mul218_out1 = add203_out1.clone().mul(div37_out1);
        let constant1589_out1 = self.constant1589.val();
        let mul219_out1 = (constant1589_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul218_out1);
        let shape120_out1: [i64; 3] = {
            let axes = &mul219_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear127_out1 = self.linear127.forward(mul219_out1.clone());
        let linear128_out1 = self.linear128.forward(mul219_out1.clone());
        let linear129_out1 = self.linear129.forward(mul219_out1);
        let gather212_out1 = shape120_out1[0] as i64;
        let gather213_out1 = shape120_out1[1] as i64;
        let constant1595_out1 = self.constant1595.val();
        let add205_out1 = (constant1595_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear127_out1);
        let constant1596_out1 = self.constant1596.val();
        let add206_out1 = (constant1596_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear128_out1);
        let constant1597_out1 = self.constant1597.val();
        let add207_out1 = (constant1597_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear129_out1);
        let unsqueeze228_out1 = [gather212_out1 as i64];
        let unsqueeze229_out1 = [gather213_out1 as i64];
        let constant1600_out1: [i64; 1] = [14i64];
        let constant1601_out1: [i64; 1] = [64i64];
        let concat202_out1: [i64; 4usize] = [
            &unsqueeze228_out1[..],
            &unsqueeze229_out1[..],
            &constant1600_out1[..],
            &constant1601_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1602_out1: [i64; 1] = [2i64];
        let constant1603_out1: [i64; 1] = [64i64];
        let concat203_out1: [i64; 4usize] = [
            &unsqueeze228_out1[..],
            &unsqueeze229_out1[..],
            &constant1602_out1[..],
            &constant1603_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1604_out1: [i64; 1] = [896i64];
        let concat204_out1: [i64; 3usize] = [
            &unsqueeze228_out1[..],
            &unsqueeze229_out1[..],
            &constant1604_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape113_out1 = add205_out1.reshape(concat202_out1);
        let reshape114_out1 = add206_out1.reshape(concat203_out1);
        let reshape115_out1 = add207_out1.reshape(concat203_out1);
        let transpose92_out1 = reshape113_out1.permute([0, 2, 1, 3]);
        let transpose93_out1 = reshape114_out1.permute([0, 2, 1, 3]);
        let transpose94_out1 = reshape115_out1.permute([0, 2, 1, 3]);
        let shape121_out1: [i64; 4] = {
            let axes = &transpose93_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat205_out1 = burn::tensor::Tensor::cat(
            [past_key_values_18_value, transpose94_out1].into(),
            2,
        );
        let slice129_out1 = transpose92_out1.clone().slice(s![.., .., .., 0..32]);
        let slice130_out1 = transpose92_out1.clone().slice(s![.., .., .., 32..]);
        let slice131_out1 = transpose93_out1.clone().slice(s![.., .., .., 0..32]);
        let slice132_out1 = transpose93_out1.clone().slice(s![.., .., .., 32..]);
        let gather214_out1 = shape121_out1[2] as i64;
        let shape122_out1: [i64; 4] = {
            let axes = &concat205_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze230_out1: Tensor<5> = concat205_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg37_out1 = slice130_out1.neg();
        let neg38_out1 = slice132_out1.neg();
        let add208_out1 = gather214_out1 + gather21_out1;
        let gather215_out1 = shape122_out1[0] as i64;
        let gather216_out1 = shape122_out1[2] as i64;
        let concat206_out1 = burn::tensor::Tensor::cat(
            [neg37_out1, slice129_out1].into(),
            3,
        );
        let concat207_out1 = burn::tensor::Tensor::cat(
            [neg38_out1, slice131_out1].into(),
            3,
        );
        let unsqueeze231_out1 = [add208_out1 as i64];
        let unsqueeze232_out1 = [gather215_out1 as i64];
        let unsqueeze233_out1 = [gather216_out1 as i64];
        let slice133_out1 = constant108_out1.slice(s![0..unsqueeze231_out1[0], ..]);
        let slice134_out1 = constant112_out1.slice(s![0..unsqueeze231_out1[0], ..]);
        let constant1634_out1: [i64; 1] = [2i64];
        let constant1635_out1: [i64; 1] = [7i64];
        let constant1636_out1: [i64; 1] = [64i64];
        let concat208_out1: [i64; 5usize] = [
            &unsqueeze232_out1[..],
            &constant1634_out1[..],
            &constant1635_out1[..],
            &unsqueeze233_out1[..],
            &constant1636_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1637_out1: [i64; 1] = [14i64];
        let constant1638_out1: [i64; 1] = [64i64];
        let concat209_out1: [i64; 4usize] = [
            &unsqueeze232_out1[..],
            &constant1637_out1[..],
            &unsqueeze233_out1[..],
            &constant1638_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather217_out1 = slice133_out1.take::<2, 3>(0, position_ids.clone());
        let gather218_out1 = slice134_out1.take::<2, 3>(0, position_ids);
        let constant1639_out1 = self.constant1639.val();
        let equal40_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat208_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1639_out1)
        };
        let unsqueeze234_out1: Tensor<4> = gather217_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze235_out1: Tensor<4> = gather218_out1.unsqueeze_dims::<4>(&[1]);
        let constant1642_out1 = self.constant1642.val();
        let where40_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat208_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal40_out1, constant1642_out1);
        let mul220_out1 = transpose92_out1.mul(unsqueeze234_out1.clone());
        let mul221_out1 = transpose93_out1.mul(unsqueeze234_out1);
        let mul222_out1 = concat206_out1.mul(unsqueeze235_out1.clone());
        let mul223_out1 = concat207_out1.mul(unsqueeze235_out1);
        let expand43_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where40_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze230_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze230_out1.expand(shape)
        };
        let add209_out1 = mul220_out1.add(mul222_out1);
        let add210_out1 = mul221_out1.add(mul223_out1);
        let reshape116_out1 = expand43_out1.reshape(concat209_out1);
        let concat210_out1 = burn::tensor::Tensor::cat(
            [past_key_values_18_key, add210_out1].into(),
            2,
        );
        let shape123_out1: [i64; 4] = {
            let axes = &concat210_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze236_out1: Tensor<5> = concat210_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather219_out1 = shape123_out1[0] as i64;
        let gather220_out1 = shape123_out1[2] as i64;
        let unsqueeze237_out1 = [gather219_out1 as i64];
        let unsqueeze238_out1 = [gather220_out1 as i64];
        let constant1649_out1: [i64; 1] = [2i64];
        let constant1650_out1: [i64; 1] = [7i64];
        let constant1651_out1: [i64; 1] = [64i64];
        let concat211_out1: [i64; 5usize] = [
            &unsqueeze237_out1[..],
            &constant1649_out1[..],
            &constant1650_out1[..],
            &unsqueeze238_out1[..],
            &constant1651_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1652_out1: [i64; 1] = [14i64];
        let constant1653_out1: [i64; 1] = [64i64];
        let concat212_out1: [i64; 4usize] = [
            &unsqueeze237_out1[..],
            &constant1652_out1[..],
            &unsqueeze238_out1[..],
            &constant1653_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1654_out1 = self.constant1654.val();
        let equal41_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat211_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1654_out1)
        };
        let constant1655_out1 = self.constant1655.val();
        let where41_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat211_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal41_out1, constant1655_out1);
        let expand44_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where41_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze236_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze236_out1.expand(shape)
        };
        let reshape117_out1 = expand44_out1.reshape(concat212_out1);
        let shape124_out1: [i64; 4] = {
            let axes = &reshape117_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather221_out1 = shape124_out1[2] as i64;
        let unsqueeze239_out1 = [gather221_out1 as i64];
        let slice135_out1 = scatternd1_out1
            .slice(s![.., .., .., 0..unsqueeze239_out1[0]]);
        let (matmul167_out1,) = {
            let q = add209_out1;
            let k = reshape117_out1;
            let v = reshape116_out1;
            let matmul167_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice135_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul167_out1,)
        };
        let transpose96_out1 = matmul167_out1.permute([0, 2, 1, 3]);
        let reshape118_out1 = transpose96_out1.reshape(concat204_out1);
        let linear130_out1 = self.linear130.forward(reshape118_out1);
        let add212_out1 = add203_out1.add(linear130_out1);
        (add212_out1, concat210_out1, concat205_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule7 {
    constant1663: burn::module::Param<Tensor<1>>,
    constant1664: burn::module::Param<Tensor<1>>,
    constant1665: burn::module::Param<Tensor<1>>,
    constant1666: burn::module::Param<Tensor<1>>,
    linear131: Linear,
    linear132: Linear,
    linear133: Linear,
    constant1670: burn::module::Param<Tensor<1>>,
    constant1671: burn::module::Param<Tensor<1>>,
    constant1672: burn::module::Param<Tensor<1>>,
    constant1673: burn::module::Param<Tensor<1>>,
    linear134: Linear,
    linear135: Linear,
    linear136: Linear,
    constant1679: burn::module::Param<Tensor<1>>,
    constant1680: burn::module::Param<Tensor<1>>,
    constant1681: burn::module::Param<Tensor<1>>,
    constant1723: burn::module::Param<Tensor<1, Int>>,
    constant1726: burn::module::Param<Tensor<1, Int>>,
    constant1738: burn::module::Param<Tensor<1, Int>>,
    constant1739: burn::module::Param<Tensor<1, Int>>,
    linear137: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule7 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1663: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1664: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1665: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1666: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear131 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear132 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear133 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1670: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1671: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1672: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1673: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear134 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear135 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear136 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1679: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1680: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1681: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1723: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1726: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1738: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1739: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear137 = LinearConfig::new(896, 896).with_bias(false).init(device);
        Self {
            constant1663,
            constant1664,
            constant1665,
            constant1666,
            linear131,
            linear132,
            linear133,
            constant1670,
            constant1671,
            constant1672,
            constant1673,
            linear134,
            linear135,
            linear136,
            constant1679,
            constant1680,
            constant1681,
            constant1723,
            constant1726,
            constant1738,
            constant1739,
            linear137,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add212_out1: Tensor<3>,
        past_key_values_19_value: Tensor<4>,
        gather22_out1: i64,
        constant108_out1: Tensor<2>,
        constant112_out1: Tensor<2>,
        position_ids: Tensor<2, Int>,
        past_key_values_19_key: Tensor<4>,
        scatternd1_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<4>, Tensor<4>) {
        let constant1663_out1 = self.constant1663.val();
        let pow38_out1 = add212_out1
            .clone()
            .powf((constant1663_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean38_out1 = { pow38_out1.mean_dim(2usize) };
        let constant1664_out1 = self.constant1664.val();
        let add213_out1 = reducemean38_out1
            .add((constant1664_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt38_out1 = add213_out1.sqrt();
        let constant1665_out1 = self.constant1665.val();
        let div38_out1 = (constant1665_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt38_out1);
        let mul226_out1 = add212_out1.clone().mul(div38_out1);
        let constant1666_out1 = self.constant1666.val();
        let mul227_out1 = (constant1666_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul226_out1);
        let linear131_out1 = self.linear131.forward(mul227_out1.clone());
        let linear132_out1 = self.linear132.forward(mul227_out1);
        let sigmoid19_out1 = burn::tensor::activation::sigmoid(linear131_out1.clone());
        let mul228_out1 = linear131_out1.mul(sigmoid19_out1);
        let mul229_out1 = mul228_out1.mul(linear132_out1);
        let linear133_out1 = self.linear133.forward(mul229_out1);
        let add214_out1 = add212_out1.add(linear133_out1);
        let constant1670_out1 = self.constant1670.val();
        let pow39_out1 = add214_out1
            .clone()
            .powf((constant1670_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean39_out1 = { pow39_out1.mean_dim(2usize) };
        let constant1671_out1 = self.constant1671.val();
        let add215_out1 = reducemean39_out1
            .add((constant1671_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt39_out1 = add215_out1.sqrt();
        let constant1672_out1 = self.constant1672.val();
        let div39_out1 = (constant1672_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt39_out1);
        let mul230_out1 = add214_out1.clone().mul(div39_out1);
        let constant1673_out1 = self.constant1673.val();
        let mul231_out1 = (constant1673_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul230_out1);
        let shape125_out1: [i64; 3] = {
            let axes = &mul231_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear134_out1 = self.linear134.forward(mul231_out1.clone());
        let linear135_out1 = self.linear135.forward(mul231_out1.clone());
        let linear136_out1 = self.linear136.forward(mul231_out1);
        let gather222_out1 = shape125_out1[0] as i64;
        let gather223_out1 = shape125_out1[1] as i64;
        let constant1679_out1 = self.constant1679.val();
        let add216_out1 = (constant1679_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear134_out1);
        let constant1680_out1 = self.constant1680.val();
        let add217_out1 = (constant1680_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear135_out1);
        let constant1681_out1 = self.constant1681.val();
        let add218_out1 = (constant1681_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear136_out1);
        let unsqueeze240_out1 = [gather222_out1 as i64];
        let unsqueeze241_out1 = [gather223_out1 as i64];
        let constant1684_out1: [i64; 1] = [14i64];
        let constant1685_out1: [i64; 1] = [64i64];
        let concat213_out1: [i64; 4usize] = [
            &unsqueeze240_out1[..],
            &unsqueeze241_out1[..],
            &constant1684_out1[..],
            &constant1685_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1686_out1: [i64; 1] = [2i64];
        let constant1687_out1: [i64; 1] = [64i64];
        let concat214_out1: [i64; 4usize] = [
            &unsqueeze240_out1[..],
            &unsqueeze241_out1[..],
            &constant1686_out1[..],
            &constant1687_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1688_out1: [i64; 1] = [896i64];
        let concat215_out1: [i64; 3usize] = [
            &unsqueeze240_out1[..],
            &unsqueeze241_out1[..],
            &constant1688_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape119_out1 = add216_out1.reshape(concat213_out1);
        let reshape120_out1 = add217_out1.reshape(concat214_out1);
        let reshape121_out1 = add218_out1.reshape(concat214_out1);
        let transpose97_out1 = reshape119_out1.permute([0, 2, 1, 3]);
        let transpose98_out1 = reshape120_out1.permute([0, 2, 1, 3]);
        let transpose99_out1 = reshape121_out1.permute([0, 2, 1, 3]);
        let shape126_out1: [i64; 4] = {
            let axes = &transpose98_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat216_out1 = burn::tensor::Tensor::cat(
            [past_key_values_19_value, transpose99_out1].into(),
            2,
        );
        let slice136_out1 = transpose97_out1.clone().slice(s![.., .., .., 0..32]);
        let slice137_out1 = transpose97_out1.clone().slice(s![.., .., .., 32..]);
        let slice138_out1 = transpose98_out1.clone().slice(s![.., .., .., 0..32]);
        let slice139_out1 = transpose98_out1.clone().slice(s![.., .., .., 32..]);
        let gather224_out1 = shape126_out1[2] as i64;
        let shape127_out1: [i64; 4] = {
            let axes = &concat216_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze242_out1: Tensor<5> = concat216_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg39_out1 = slice137_out1.neg();
        let neg40_out1 = slice139_out1.neg();
        let add219_out1 = gather224_out1 + gather22_out1;
        let gather225_out1 = shape127_out1[0] as i64;
        let gather226_out1 = shape127_out1[2] as i64;
        let concat217_out1 = burn::tensor::Tensor::cat(
            [neg39_out1, slice136_out1].into(),
            3,
        );
        let concat218_out1 = burn::tensor::Tensor::cat(
            [neg40_out1, slice138_out1].into(),
            3,
        );
        let unsqueeze243_out1 = [add219_out1 as i64];
        let unsqueeze244_out1 = [gather225_out1 as i64];
        let unsqueeze245_out1 = [gather226_out1 as i64];
        let slice140_out1 = constant108_out1.slice(s![0..unsqueeze243_out1[0], ..]);
        let slice141_out1 = constant112_out1.slice(s![0..unsqueeze243_out1[0], ..]);
        let constant1718_out1: [i64; 1] = [2i64];
        let constant1719_out1: [i64; 1] = [7i64];
        let constant1720_out1: [i64; 1] = [64i64];
        let concat219_out1: [i64; 5usize] = [
            &unsqueeze244_out1[..],
            &constant1718_out1[..],
            &constant1719_out1[..],
            &unsqueeze245_out1[..],
            &constant1720_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1721_out1: [i64; 1] = [14i64];
        let constant1722_out1: [i64; 1] = [64i64];
        let concat220_out1: [i64; 4usize] = [
            &unsqueeze244_out1[..],
            &constant1721_out1[..],
            &unsqueeze245_out1[..],
            &constant1722_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather227_out1 = slice140_out1.take::<2, 3>(0, position_ids.clone());
        let gather228_out1 = slice141_out1.take::<2, 3>(0, position_ids);
        let constant1723_out1 = self.constant1723.val();
        let equal42_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat219_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1723_out1)
        };
        let unsqueeze246_out1: Tensor<4> = gather227_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze247_out1: Tensor<4> = gather228_out1.unsqueeze_dims::<4>(&[1]);
        let constant1726_out1 = self.constant1726.val();
        let where42_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat219_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal42_out1, constant1726_out1);
        let mul232_out1 = transpose97_out1.mul(unsqueeze246_out1.clone());
        let mul233_out1 = transpose98_out1.mul(unsqueeze246_out1);
        let mul234_out1 = concat217_out1.mul(unsqueeze247_out1.clone());
        let mul235_out1 = concat218_out1.mul(unsqueeze247_out1);
        let expand45_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where42_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze242_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze242_out1.expand(shape)
        };
        let add220_out1 = mul232_out1.add(mul234_out1);
        let add221_out1 = mul233_out1.add(mul235_out1);
        let reshape122_out1 = expand45_out1.reshape(concat220_out1);
        let concat221_out1 = burn::tensor::Tensor::cat(
            [past_key_values_19_key, add221_out1].into(),
            2,
        );
        let shape128_out1: [i64; 4] = {
            let axes = &concat221_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze248_out1: Tensor<5> = concat221_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather229_out1 = shape128_out1[0] as i64;
        let gather230_out1 = shape128_out1[2] as i64;
        let unsqueeze249_out1 = [gather229_out1 as i64];
        let unsqueeze250_out1 = [gather230_out1 as i64];
        let constant1733_out1: [i64; 1] = [2i64];
        let constant1734_out1: [i64; 1] = [7i64];
        let constant1735_out1: [i64; 1] = [64i64];
        let concat222_out1: [i64; 5usize] = [
            &unsqueeze249_out1[..],
            &constant1733_out1[..],
            &constant1734_out1[..],
            &unsqueeze250_out1[..],
            &constant1735_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1736_out1: [i64; 1] = [14i64];
        let constant1737_out1: [i64; 1] = [64i64];
        let concat223_out1: [i64; 4usize] = [
            &unsqueeze249_out1[..],
            &constant1736_out1[..],
            &unsqueeze250_out1[..],
            &constant1737_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1738_out1 = self.constant1738.val();
        let equal43_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat222_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1738_out1)
        };
        let constant1739_out1 = self.constant1739.val();
        let where43_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat222_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal43_out1, constant1739_out1);
        let expand46_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where43_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze248_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze248_out1.expand(shape)
        };
        let reshape123_out1 = expand46_out1.reshape(concat223_out1);
        let shape129_out1: [i64; 4] = {
            let axes = &reshape123_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather231_out1 = shape129_out1[2] as i64;
        let unsqueeze251_out1 = [gather231_out1 as i64];
        let slice142_out1 = scatternd1_out1
            .slice(s![.., .., .., 0..unsqueeze251_out1[0]]);
        let (matmul176_out1,) = {
            let q = add220_out1;
            let k = reshape123_out1;
            let v = reshape122_out1;
            let matmul176_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice142_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul176_out1,)
        };
        let transpose101_out1 = matmul176_out1.permute([0, 2, 1, 3]);
        let reshape124_out1 = transpose101_out1.reshape(concat215_out1);
        let linear137_out1 = self.linear137.forward(reshape124_out1);
        let add223_out1 = add214_out1.add(linear137_out1);
        (add223_out1, concat221_out1, concat216_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule8 {
    constant1747: burn::module::Param<Tensor<1>>,
    constant1748: burn::module::Param<Tensor<1>>,
    constant1749: burn::module::Param<Tensor<1>>,
    constant1750: burn::module::Param<Tensor<1>>,
    linear138: Linear,
    linear139: Linear,
    linear140: Linear,
    constant1754: burn::module::Param<Tensor<1>>,
    constant1755: burn::module::Param<Tensor<1>>,
    constant1756: burn::module::Param<Tensor<1>>,
    constant1757: burn::module::Param<Tensor<1>>,
    linear141: Linear,
    linear142: Linear,
    linear143: Linear,
    constant1763: burn::module::Param<Tensor<1>>,
    constant1764: burn::module::Param<Tensor<1>>,
    constant1765: burn::module::Param<Tensor<1>>,
    constant1807: burn::module::Param<Tensor<1, Int>>,
    constant1810: burn::module::Param<Tensor<1, Int>>,
    constant1822: burn::module::Param<Tensor<1, Int>>,
    constant1823: burn::module::Param<Tensor<1, Int>>,
    linear144: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule8 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1747: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1748: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1749: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1750: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear138 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear139 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear140 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1754: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1755: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1756: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1757: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear141 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear142 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear143 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1763: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1764: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1765: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1807: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1810: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1822: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1823: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear144 = LinearConfig::new(896, 896).with_bias(false).init(device);
        Self {
            constant1747,
            constant1748,
            constant1749,
            constant1750,
            linear138,
            linear139,
            linear140,
            constant1754,
            constant1755,
            constant1756,
            constant1757,
            linear141,
            linear142,
            linear143,
            constant1763,
            constant1764,
            constant1765,
            constant1807,
            constant1810,
            constant1822,
            constant1823,
            linear144,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add223_out1: Tensor<3>,
        past_key_values_20_value: Tensor<4>,
        gather23_out1: i64,
        constant108_out1: Tensor<2>,
        constant112_out1: Tensor<2>,
        position_ids: Tensor<2, Int>,
        past_key_values_20_key: Tensor<4>,
        scatternd1_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<4>, Tensor<4>) {
        let constant1747_out1 = self.constant1747.val();
        let pow40_out1 = add223_out1
            .clone()
            .powf((constant1747_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean40_out1 = { pow40_out1.mean_dim(2usize) };
        let constant1748_out1 = self.constant1748.val();
        let add224_out1 = reducemean40_out1
            .add((constant1748_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt40_out1 = add224_out1.sqrt();
        let constant1749_out1 = self.constant1749.val();
        let div40_out1 = (constant1749_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt40_out1);
        let mul238_out1 = add223_out1.clone().mul(div40_out1);
        let constant1750_out1 = self.constant1750.val();
        let mul239_out1 = (constant1750_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul238_out1);
        let linear138_out1 = self.linear138.forward(mul239_out1.clone());
        let linear139_out1 = self.linear139.forward(mul239_out1);
        let sigmoid20_out1 = burn::tensor::activation::sigmoid(linear138_out1.clone());
        let mul240_out1 = linear138_out1.mul(sigmoid20_out1);
        let mul241_out1 = mul240_out1.mul(linear139_out1);
        let linear140_out1 = self.linear140.forward(mul241_out1);
        let add225_out1 = add223_out1.add(linear140_out1);
        let constant1754_out1 = self.constant1754.val();
        let pow41_out1 = add225_out1
            .clone()
            .powf((constant1754_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean41_out1 = { pow41_out1.mean_dim(2usize) };
        let constant1755_out1 = self.constant1755.val();
        let add226_out1 = reducemean41_out1
            .add((constant1755_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt41_out1 = add226_out1.sqrt();
        let constant1756_out1 = self.constant1756.val();
        let div41_out1 = (constant1756_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt41_out1);
        let mul242_out1 = add225_out1.clone().mul(div41_out1);
        let constant1757_out1 = self.constant1757.val();
        let mul243_out1 = (constant1757_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul242_out1);
        let shape130_out1: [i64; 3] = {
            let axes = &mul243_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear141_out1 = self.linear141.forward(mul243_out1.clone());
        let linear142_out1 = self.linear142.forward(mul243_out1.clone());
        let linear143_out1 = self.linear143.forward(mul243_out1);
        let gather232_out1 = shape130_out1[0] as i64;
        let gather233_out1 = shape130_out1[1] as i64;
        let constant1763_out1 = self.constant1763.val();
        let add227_out1 = (constant1763_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear141_out1);
        let constant1764_out1 = self.constant1764.val();
        let add228_out1 = (constant1764_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear142_out1);
        let constant1765_out1 = self.constant1765.val();
        let add229_out1 = (constant1765_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear143_out1);
        let unsqueeze252_out1 = [gather232_out1 as i64];
        let unsqueeze253_out1 = [gather233_out1 as i64];
        let constant1768_out1: [i64; 1] = [14i64];
        let constant1769_out1: [i64; 1] = [64i64];
        let concat224_out1: [i64; 4usize] = [
            &unsqueeze252_out1[..],
            &unsqueeze253_out1[..],
            &constant1768_out1[..],
            &constant1769_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1770_out1: [i64; 1] = [2i64];
        let constant1771_out1: [i64; 1] = [64i64];
        let concat225_out1: [i64; 4usize] = [
            &unsqueeze252_out1[..],
            &unsqueeze253_out1[..],
            &constant1770_out1[..],
            &constant1771_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1772_out1: [i64; 1] = [896i64];
        let concat226_out1: [i64; 3usize] = [
            &unsqueeze252_out1[..],
            &unsqueeze253_out1[..],
            &constant1772_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape125_out1 = add227_out1.reshape(concat224_out1);
        let reshape126_out1 = add228_out1.reshape(concat225_out1);
        let reshape127_out1 = add229_out1.reshape(concat225_out1);
        let transpose102_out1 = reshape125_out1.permute([0, 2, 1, 3]);
        let transpose103_out1 = reshape126_out1.permute([0, 2, 1, 3]);
        let transpose104_out1 = reshape127_out1.permute([0, 2, 1, 3]);
        let shape131_out1: [i64; 4] = {
            let axes = &transpose103_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat227_out1 = burn::tensor::Tensor::cat(
            [past_key_values_20_value, transpose104_out1].into(),
            2,
        );
        let slice143_out1 = transpose102_out1.clone().slice(s![.., .., .., 0..32]);
        let slice144_out1 = transpose102_out1.clone().slice(s![.., .., .., 32..]);
        let slice145_out1 = transpose103_out1.clone().slice(s![.., .., .., 0..32]);
        let slice146_out1 = transpose103_out1.clone().slice(s![.., .., .., 32..]);
        let gather234_out1 = shape131_out1[2] as i64;
        let shape132_out1: [i64; 4] = {
            let axes = &concat227_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze254_out1: Tensor<5> = concat227_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg41_out1 = slice144_out1.neg();
        let neg42_out1 = slice146_out1.neg();
        let add230_out1 = gather234_out1 + gather23_out1;
        let gather235_out1 = shape132_out1[0] as i64;
        let gather236_out1 = shape132_out1[2] as i64;
        let concat228_out1 = burn::tensor::Tensor::cat(
            [neg41_out1, slice143_out1].into(),
            3,
        );
        let concat229_out1 = burn::tensor::Tensor::cat(
            [neg42_out1, slice145_out1].into(),
            3,
        );
        let unsqueeze255_out1 = [add230_out1 as i64];
        let unsqueeze256_out1 = [gather235_out1 as i64];
        let unsqueeze257_out1 = [gather236_out1 as i64];
        let slice147_out1 = constant108_out1.slice(s![0..unsqueeze255_out1[0], ..]);
        let slice148_out1 = constant112_out1.slice(s![0..unsqueeze255_out1[0], ..]);
        let constant1802_out1: [i64; 1] = [2i64];
        let constant1803_out1: [i64; 1] = [7i64];
        let constant1804_out1: [i64; 1] = [64i64];
        let concat230_out1: [i64; 5usize] = [
            &unsqueeze256_out1[..],
            &constant1802_out1[..],
            &constant1803_out1[..],
            &unsqueeze257_out1[..],
            &constant1804_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1805_out1: [i64; 1] = [14i64];
        let constant1806_out1: [i64; 1] = [64i64];
        let concat231_out1: [i64; 4usize] = [
            &unsqueeze256_out1[..],
            &constant1805_out1[..],
            &unsqueeze257_out1[..],
            &constant1806_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather237_out1 = slice147_out1.take::<2, 3>(0, position_ids.clone());
        let gather238_out1 = slice148_out1.take::<2, 3>(0, position_ids);
        let constant1807_out1 = self.constant1807.val();
        let equal44_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat230_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1807_out1)
        };
        let unsqueeze258_out1: Tensor<4> = gather237_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze259_out1: Tensor<4> = gather238_out1.unsqueeze_dims::<4>(&[1]);
        let constant1810_out1 = self.constant1810.val();
        let where44_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat230_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal44_out1, constant1810_out1);
        let mul244_out1 = transpose102_out1.mul(unsqueeze258_out1.clone());
        let mul245_out1 = transpose103_out1.mul(unsqueeze258_out1);
        let mul246_out1 = concat228_out1.mul(unsqueeze259_out1.clone());
        let mul247_out1 = concat229_out1.mul(unsqueeze259_out1);
        let expand47_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where44_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze254_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze254_out1.expand(shape)
        };
        let add231_out1 = mul244_out1.add(mul246_out1);
        let add232_out1 = mul245_out1.add(mul247_out1);
        let reshape128_out1 = expand47_out1.reshape(concat231_out1);
        let concat232_out1 = burn::tensor::Tensor::cat(
            [past_key_values_20_key, add232_out1].into(),
            2,
        );
        let shape133_out1: [i64; 4] = {
            let axes = &concat232_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze260_out1: Tensor<5> = concat232_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather239_out1 = shape133_out1[0] as i64;
        let gather240_out1 = shape133_out1[2] as i64;
        let unsqueeze261_out1 = [gather239_out1 as i64];
        let unsqueeze262_out1 = [gather240_out1 as i64];
        let constant1817_out1: [i64; 1] = [2i64];
        let constant1818_out1: [i64; 1] = [7i64];
        let constant1819_out1: [i64; 1] = [64i64];
        let concat233_out1: [i64; 5usize] = [
            &unsqueeze261_out1[..],
            &constant1817_out1[..],
            &constant1818_out1[..],
            &unsqueeze262_out1[..],
            &constant1819_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1820_out1: [i64; 1] = [14i64];
        let constant1821_out1: [i64; 1] = [64i64];
        let concat234_out1: [i64; 4usize] = [
            &unsqueeze261_out1[..],
            &constant1820_out1[..],
            &unsqueeze262_out1[..],
            &constant1821_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1822_out1 = self.constant1822.val();
        let equal45_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat233_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1822_out1)
        };
        let constant1823_out1 = self.constant1823.val();
        let where45_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat233_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal45_out1, constant1823_out1);
        let expand48_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where45_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze260_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze260_out1.expand(shape)
        };
        let reshape129_out1 = expand48_out1.reshape(concat234_out1);
        let shape134_out1: [i64; 4] = {
            let axes = &reshape129_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather241_out1 = shape134_out1[2] as i64;
        let unsqueeze263_out1 = [gather241_out1 as i64];
        let slice149_out1 = scatternd1_out1
            .slice(s![.., .., .., 0..unsqueeze263_out1[0]]);
        let (matmul185_out1,) = {
            let q = add231_out1;
            let k = reshape129_out1;
            let v = reshape128_out1;
            let matmul185_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice149_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul185_out1,)
        };
        let transpose106_out1 = matmul185_out1.permute([0, 2, 1, 3]);
        let reshape130_out1 = transpose106_out1.reshape(concat226_out1);
        let linear144_out1 = self.linear144.forward(reshape130_out1);
        let add234_out1 = add225_out1.add(linear144_out1);
        (add234_out1, concat232_out1, concat227_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule9 {
    constant1831: burn::module::Param<Tensor<1>>,
    constant1832: burn::module::Param<Tensor<1>>,
    constant1833: burn::module::Param<Tensor<1>>,
    constant1834: burn::module::Param<Tensor<1>>,
    linear145: Linear,
    linear146: Linear,
    linear147: Linear,
    constant1838: burn::module::Param<Tensor<1>>,
    constant1839: burn::module::Param<Tensor<1>>,
    constant1840: burn::module::Param<Tensor<1>>,
    constant1841: burn::module::Param<Tensor<1>>,
    linear148: Linear,
    linear149: Linear,
    linear150: Linear,
    constant1847: burn::module::Param<Tensor<1>>,
    constant1848: burn::module::Param<Tensor<1>>,
    constant1849: burn::module::Param<Tensor<1>>,
    constant1891: burn::module::Param<Tensor<1, Int>>,
    constant1894: burn::module::Param<Tensor<1, Int>>,
    constant1906: burn::module::Param<Tensor<1, Int>>,
    constant1907: burn::module::Param<Tensor<1, Int>>,
    linear151: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule9 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1831: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1832: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1833: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1834: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear145 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear146 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear147 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1838: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1839: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1840: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1841: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear148 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear149 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear150 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1847: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1848: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1849: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1891: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1894: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1906: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1907: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear151 = LinearConfig::new(896, 896).with_bias(false).init(device);
        Self {
            constant1831,
            constant1832,
            constant1833,
            constant1834,
            linear145,
            linear146,
            linear147,
            constant1838,
            constant1839,
            constant1840,
            constant1841,
            linear148,
            linear149,
            linear150,
            constant1847,
            constant1848,
            constant1849,
            constant1891,
            constant1894,
            constant1906,
            constant1907,
            linear151,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add234_out1: Tensor<3>,
        past_key_values_21_value: Tensor<4>,
        gather24_out1: i64,
        constant108_out1: Tensor<2>,
        constant112_out1: Tensor<2>,
        position_ids: Tensor<2, Int>,
        past_key_values_21_key: Tensor<4>,
        scatternd1_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<4>, Tensor<4>) {
        let constant1831_out1 = self.constant1831.val();
        let pow42_out1 = add234_out1
            .clone()
            .powf((constant1831_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean42_out1 = { pow42_out1.mean_dim(2usize) };
        let constant1832_out1 = self.constant1832.val();
        let add235_out1 = reducemean42_out1
            .add((constant1832_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt42_out1 = add235_out1.sqrt();
        let constant1833_out1 = self.constant1833.val();
        let div42_out1 = (constant1833_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt42_out1);
        let mul250_out1 = add234_out1.clone().mul(div42_out1);
        let constant1834_out1 = self.constant1834.val();
        let mul251_out1 = (constant1834_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul250_out1);
        let linear145_out1 = self.linear145.forward(mul251_out1.clone());
        let linear146_out1 = self.linear146.forward(mul251_out1);
        let sigmoid21_out1 = burn::tensor::activation::sigmoid(linear145_out1.clone());
        let mul252_out1 = linear145_out1.mul(sigmoid21_out1);
        let mul253_out1 = mul252_out1.mul(linear146_out1);
        let linear147_out1 = self.linear147.forward(mul253_out1);
        let add236_out1 = add234_out1.add(linear147_out1);
        let constant1838_out1 = self.constant1838.val();
        let pow43_out1 = add236_out1
            .clone()
            .powf((constant1838_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean43_out1 = { pow43_out1.mean_dim(2usize) };
        let constant1839_out1 = self.constant1839.val();
        let add237_out1 = reducemean43_out1
            .add((constant1839_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt43_out1 = add237_out1.sqrt();
        let constant1840_out1 = self.constant1840.val();
        let div43_out1 = (constant1840_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt43_out1);
        let mul254_out1 = add236_out1.clone().mul(div43_out1);
        let constant1841_out1 = self.constant1841.val();
        let mul255_out1 = (constant1841_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul254_out1);
        let shape135_out1: [i64; 3] = {
            let axes = &mul255_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear148_out1 = self.linear148.forward(mul255_out1.clone());
        let linear149_out1 = self.linear149.forward(mul255_out1.clone());
        let linear150_out1 = self.linear150.forward(mul255_out1);
        let gather242_out1 = shape135_out1[0] as i64;
        let gather243_out1 = shape135_out1[1] as i64;
        let constant1847_out1 = self.constant1847.val();
        let add238_out1 = (constant1847_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear148_out1);
        let constant1848_out1 = self.constant1848.val();
        let add239_out1 = (constant1848_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear149_out1);
        let constant1849_out1 = self.constant1849.val();
        let add240_out1 = (constant1849_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear150_out1);
        let unsqueeze264_out1 = [gather242_out1 as i64];
        let unsqueeze265_out1 = [gather243_out1 as i64];
        let constant1852_out1: [i64; 1] = [14i64];
        let constant1853_out1: [i64; 1] = [64i64];
        let concat235_out1: [i64; 4usize] = [
            &unsqueeze264_out1[..],
            &unsqueeze265_out1[..],
            &constant1852_out1[..],
            &constant1853_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1854_out1: [i64; 1] = [2i64];
        let constant1855_out1: [i64; 1] = [64i64];
        let concat236_out1: [i64; 4usize] = [
            &unsqueeze264_out1[..],
            &unsqueeze265_out1[..],
            &constant1854_out1[..],
            &constant1855_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1856_out1: [i64; 1] = [896i64];
        let concat237_out1: [i64; 3usize] = [
            &unsqueeze264_out1[..],
            &unsqueeze265_out1[..],
            &constant1856_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape131_out1 = add238_out1.reshape(concat235_out1);
        let reshape132_out1 = add239_out1.reshape(concat236_out1);
        let reshape133_out1 = add240_out1.reshape(concat236_out1);
        let transpose107_out1 = reshape131_out1.permute([0, 2, 1, 3]);
        let transpose108_out1 = reshape132_out1.permute([0, 2, 1, 3]);
        let transpose109_out1 = reshape133_out1.permute([0, 2, 1, 3]);
        let shape136_out1: [i64; 4] = {
            let axes = &transpose108_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat238_out1 = burn::tensor::Tensor::cat(
            [past_key_values_21_value, transpose109_out1].into(),
            2,
        );
        let slice150_out1 = transpose107_out1.clone().slice(s![.., .., .., 0..32]);
        let slice151_out1 = transpose107_out1.clone().slice(s![.., .., .., 32..]);
        let slice152_out1 = transpose108_out1.clone().slice(s![.., .., .., 0..32]);
        let slice153_out1 = transpose108_out1.clone().slice(s![.., .., .., 32..]);
        let gather244_out1 = shape136_out1[2] as i64;
        let shape137_out1: [i64; 4] = {
            let axes = &concat238_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze266_out1: Tensor<5> = concat238_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg43_out1 = slice151_out1.neg();
        let neg44_out1 = slice153_out1.neg();
        let add241_out1 = gather244_out1 + gather24_out1;
        let gather245_out1 = shape137_out1[0] as i64;
        let gather246_out1 = shape137_out1[2] as i64;
        let concat239_out1 = burn::tensor::Tensor::cat(
            [neg43_out1, slice150_out1].into(),
            3,
        );
        let concat240_out1 = burn::tensor::Tensor::cat(
            [neg44_out1, slice152_out1].into(),
            3,
        );
        let unsqueeze267_out1 = [add241_out1 as i64];
        let unsqueeze268_out1 = [gather245_out1 as i64];
        let unsqueeze269_out1 = [gather246_out1 as i64];
        let slice154_out1 = constant108_out1.slice(s![0..unsqueeze267_out1[0], ..]);
        let slice155_out1 = constant112_out1.slice(s![0..unsqueeze267_out1[0], ..]);
        let constant1886_out1: [i64; 1] = [2i64];
        let constant1887_out1: [i64; 1] = [7i64];
        let constant1888_out1: [i64; 1] = [64i64];
        let concat241_out1: [i64; 5usize] = [
            &unsqueeze268_out1[..],
            &constant1886_out1[..],
            &constant1887_out1[..],
            &unsqueeze269_out1[..],
            &constant1888_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1889_out1: [i64; 1] = [14i64];
        let constant1890_out1: [i64; 1] = [64i64];
        let concat242_out1: [i64; 4usize] = [
            &unsqueeze268_out1[..],
            &constant1889_out1[..],
            &unsqueeze269_out1[..],
            &constant1890_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather247_out1 = slice154_out1.take::<2, 3>(0, position_ids.clone());
        let gather248_out1 = slice155_out1.take::<2, 3>(0, position_ids);
        let constant1891_out1 = self.constant1891.val();
        let equal46_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat241_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1891_out1)
        };
        let unsqueeze270_out1: Tensor<4> = gather247_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze271_out1: Tensor<4> = gather248_out1.unsqueeze_dims::<4>(&[1]);
        let constant1894_out1 = self.constant1894.val();
        let where46_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat241_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal46_out1, constant1894_out1);
        let mul256_out1 = transpose107_out1.mul(unsqueeze270_out1.clone());
        let mul257_out1 = transpose108_out1.mul(unsqueeze270_out1);
        let mul258_out1 = concat239_out1.mul(unsqueeze271_out1.clone());
        let mul259_out1 = concat240_out1.mul(unsqueeze271_out1);
        let expand49_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where46_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze266_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze266_out1.expand(shape)
        };
        let add242_out1 = mul256_out1.add(mul258_out1);
        let add243_out1 = mul257_out1.add(mul259_out1);
        let reshape134_out1 = expand49_out1.reshape(concat242_out1);
        let concat243_out1 = burn::tensor::Tensor::cat(
            [past_key_values_21_key, add243_out1].into(),
            2,
        );
        let shape138_out1: [i64; 4] = {
            let axes = &concat243_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze272_out1: Tensor<5> = concat243_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather249_out1 = shape138_out1[0] as i64;
        let gather250_out1 = shape138_out1[2] as i64;
        let unsqueeze273_out1 = [gather249_out1 as i64];
        let unsqueeze274_out1 = [gather250_out1 as i64];
        let constant1901_out1: [i64; 1] = [2i64];
        let constant1902_out1: [i64; 1] = [7i64];
        let constant1903_out1: [i64; 1] = [64i64];
        let concat244_out1: [i64; 5usize] = [
            &unsqueeze273_out1[..],
            &constant1901_out1[..],
            &constant1902_out1[..],
            &unsqueeze274_out1[..],
            &constant1903_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1904_out1: [i64; 1] = [14i64];
        let constant1905_out1: [i64; 1] = [64i64];
        let concat245_out1: [i64; 4usize] = [
            &unsqueeze273_out1[..],
            &constant1904_out1[..],
            &unsqueeze274_out1[..],
            &constant1905_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1906_out1 = self.constant1906.val();
        let equal47_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat244_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1906_out1)
        };
        let constant1907_out1 = self.constant1907.val();
        let where47_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat244_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal47_out1, constant1907_out1);
        let expand50_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where47_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze272_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze272_out1.expand(shape)
        };
        let reshape135_out1 = expand50_out1.reshape(concat245_out1);
        let shape139_out1: [i64; 4] = {
            let axes = &reshape135_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather251_out1 = shape139_out1[2] as i64;
        let unsqueeze275_out1 = [gather251_out1 as i64];
        let slice156_out1 = scatternd1_out1
            .slice(s![.., .., .., 0..unsqueeze275_out1[0]]);
        let (matmul194_out1,) = {
            let q = add242_out1;
            let k = reshape135_out1;
            let v = reshape134_out1;
            let matmul194_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice156_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul194_out1,)
        };
        let transpose111_out1 = matmul194_out1.permute([0, 2, 1, 3]);
        let reshape136_out1 = transpose111_out1.reshape(concat237_out1);
        let linear151_out1 = self.linear151.forward(reshape136_out1);
        let add245_out1 = add236_out1.add(linear151_out1);
        (add245_out1, concat243_out1, concat238_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule10 {
    constant1915: burn::module::Param<Tensor<1>>,
    constant1916: burn::module::Param<Tensor<1>>,
    constant1917: burn::module::Param<Tensor<1>>,
    constant1918: burn::module::Param<Tensor<1>>,
    linear152: Linear,
    linear153: Linear,
    linear154: Linear,
    constant1922: burn::module::Param<Tensor<1>>,
    constant1923: burn::module::Param<Tensor<1>>,
    constant1924: burn::module::Param<Tensor<1>>,
    constant1925: burn::module::Param<Tensor<1>>,
    linear155: Linear,
    linear156: Linear,
    linear157: Linear,
    constant1931: burn::module::Param<Tensor<1>>,
    constant1932: burn::module::Param<Tensor<1>>,
    constant1933: burn::module::Param<Tensor<1>>,
    constant1975: burn::module::Param<Tensor<1, Int>>,
    constant1978: burn::module::Param<Tensor<1, Int>>,
    constant1990: burn::module::Param<Tensor<1, Int>>,
    constant1991: burn::module::Param<Tensor<1, Int>>,
    linear158: Linear,
    #[module(skip)]
    device: Device,
}
impl Submodule10 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1915: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1916: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1917: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1918: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear152 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear153 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear154 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant1922: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1923: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant1924: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant1925: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear155 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear156 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear157 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant1931: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant1932: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1933: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant1975: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1978: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1990: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant1991: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear158 = LinearConfig::new(896, 896).with_bias(false).init(device);
        Self {
            constant1915,
            constant1916,
            constant1917,
            constant1918,
            linear152,
            linear153,
            linear154,
            constant1922,
            constant1923,
            constant1924,
            constant1925,
            linear155,
            linear156,
            linear157,
            constant1931,
            constant1932,
            constant1933,
            constant1975,
            constant1978,
            constant1990,
            constant1991,
            linear158,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add245_out1: Tensor<3>,
        past_key_values_22_value: Tensor<4>,
        gather25_out1: i64,
        constant108_out1: Tensor<2>,
        constant112_out1: Tensor<2>,
        position_ids: Tensor<2, Int>,
        past_key_values_22_key: Tensor<4>,
        scatternd1_out1: Tensor<4>,
    ) -> (Tensor<3>, Tensor<4>, Tensor<4>) {
        let constant1915_out1 = self.constant1915.val();
        let pow44_out1 = add245_out1
            .clone()
            .powf((constant1915_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean44_out1 = { pow44_out1.mean_dim(2usize) };
        let constant1916_out1 = self.constant1916.val();
        let add246_out1 = reducemean44_out1
            .add((constant1916_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt44_out1 = add246_out1.sqrt();
        let constant1917_out1 = self.constant1917.val();
        let div44_out1 = (constant1917_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt44_out1);
        let mul262_out1 = add245_out1.clone().mul(div44_out1);
        let constant1918_out1 = self.constant1918.val();
        let mul263_out1 = (constant1918_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul262_out1);
        let linear152_out1 = self.linear152.forward(mul263_out1.clone());
        let linear153_out1 = self.linear153.forward(mul263_out1);
        let sigmoid22_out1 = burn::tensor::activation::sigmoid(linear152_out1.clone());
        let mul264_out1 = linear152_out1.mul(sigmoid22_out1);
        let mul265_out1 = mul264_out1.mul(linear153_out1);
        let linear154_out1 = self.linear154.forward(mul265_out1);
        let add247_out1 = add245_out1.add(linear154_out1);
        let constant1922_out1 = self.constant1922.val();
        let pow45_out1 = add247_out1
            .clone()
            .powf((constant1922_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean45_out1 = { pow45_out1.mean_dim(2usize) };
        let constant1923_out1 = self.constant1923.val();
        let add248_out1 = reducemean45_out1
            .add((constant1923_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt45_out1 = add248_out1.sqrt();
        let constant1924_out1 = self.constant1924.val();
        let div45_out1 = (constant1924_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt45_out1);
        let mul266_out1 = add247_out1.clone().mul(div45_out1);
        let constant1925_out1 = self.constant1925.val();
        let mul267_out1 = (constant1925_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul266_out1);
        let shape140_out1: [i64; 3] = {
            let axes = &mul267_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear155_out1 = self.linear155.forward(mul267_out1.clone());
        let linear156_out1 = self.linear156.forward(mul267_out1.clone());
        let linear157_out1 = self.linear157.forward(mul267_out1);
        let gather252_out1 = shape140_out1[0] as i64;
        let gather253_out1 = shape140_out1[1] as i64;
        let constant1931_out1 = self.constant1931.val();
        let add249_out1 = (constant1931_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear155_out1);
        let constant1932_out1 = self.constant1932.val();
        let add250_out1 = (constant1932_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear156_out1);
        let constant1933_out1 = self.constant1933.val();
        let add251_out1 = (constant1933_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear157_out1);
        let unsqueeze276_out1 = [gather252_out1 as i64];
        let unsqueeze277_out1 = [gather253_out1 as i64];
        let constant1936_out1: [i64; 1] = [14i64];
        let constant1937_out1: [i64; 1] = [64i64];
        let concat246_out1: [i64; 4usize] = [
            &unsqueeze276_out1[..],
            &unsqueeze277_out1[..],
            &constant1936_out1[..],
            &constant1937_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1938_out1: [i64; 1] = [2i64];
        let constant1939_out1: [i64; 1] = [64i64];
        let concat247_out1: [i64; 4usize] = [
            &unsqueeze276_out1[..],
            &unsqueeze277_out1[..],
            &constant1938_out1[..],
            &constant1939_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1940_out1: [i64; 1] = [896i64];
        let concat248_out1: [i64; 3usize] = [
            &unsqueeze276_out1[..],
            &unsqueeze277_out1[..],
            &constant1940_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape137_out1 = add249_out1.reshape(concat246_out1);
        let reshape138_out1 = add250_out1.reshape(concat247_out1);
        let reshape139_out1 = add251_out1.reshape(concat247_out1);
        let transpose112_out1 = reshape137_out1.permute([0, 2, 1, 3]);
        let transpose113_out1 = reshape138_out1.permute([0, 2, 1, 3]);
        let transpose114_out1 = reshape139_out1.permute([0, 2, 1, 3]);
        let shape141_out1: [i64; 4] = {
            let axes = &transpose113_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat249_out1 = burn::tensor::Tensor::cat(
            [past_key_values_22_value, transpose114_out1].into(),
            2,
        );
        let slice157_out1 = transpose112_out1.clone().slice(s![.., .., .., 0..32]);
        let slice158_out1 = transpose112_out1.clone().slice(s![.., .., .., 32..]);
        let slice159_out1 = transpose113_out1.clone().slice(s![.., .., .., 0..32]);
        let slice160_out1 = transpose113_out1.clone().slice(s![.., .., .., 32..]);
        let gather254_out1 = shape141_out1[2] as i64;
        let shape142_out1: [i64; 4] = {
            let axes = &concat249_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze278_out1: Tensor<5> = concat249_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg45_out1 = slice158_out1.neg();
        let neg46_out1 = slice160_out1.neg();
        let add252_out1 = gather254_out1 + gather25_out1;
        let gather255_out1 = shape142_out1[0] as i64;
        let gather256_out1 = shape142_out1[2] as i64;
        let concat250_out1 = burn::tensor::Tensor::cat(
            [neg45_out1, slice157_out1].into(),
            3,
        );
        let concat251_out1 = burn::tensor::Tensor::cat(
            [neg46_out1, slice159_out1].into(),
            3,
        );
        let unsqueeze279_out1 = [add252_out1 as i64];
        let unsqueeze280_out1 = [gather255_out1 as i64];
        let unsqueeze281_out1 = [gather256_out1 as i64];
        let slice161_out1 = constant108_out1.slice(s![0..unsqueeze279_out1[0], ..]);
        let slice162_out1 = constant112_out1.slice(s![0..unsqueeze279_out1[0], ..]);
        let constant1970_out1: [i64; 1] = [2i64];
        let constant1971_out1: [i64; 1] = [7i64];
        let constant1972_out1: [i64; 1] = [64i64];
        let concat252_out1: [i64; 5usize] = [
            &unsqueeze280_out1[..],
            &constant1970_out1[..],
            &constant1971_out1[..],
            &unsqueeze281_out1[..],
            &constant1972_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1973_out1: [i64; 1] = [14i64];
        let constant1974_out1: [i64; 1] = [64i64];
        let concat253_out1: [i64; 4usize] = [
            &unsqueeze280_out1[..],
            &constant1973_out1[..],
            &unsqueeze281_out1[..],
            &constant1974_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather257_out1 = slice161_out1.take::<2, 3>(0, position_ids.clone());
        let gather258_out1 = slice162_out1.take::<2, 3>(0, position_ids);
        let constant1975_out1 = self.constant1975.val();
        let equal48_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat252_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1975_out1)
        };
        let unsqueeze282_out1: Tensor<4> = gather257_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze283_out1: Tensor<4> = gather258_out1.unsqueeze_dims::<4>(&[1]);
        let constant1978_out1 = self.constant1978.val();
        let where48_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat252_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal48_out1, constant1978_out1);
        let mul268_out1 = transpose112_out1.mul(unsqueeze282_out1.clone());
        let mul269_out1 = transpose113_out1.mul(unsqueeze282_out1);
        let mul270_out1 = concat250_out1.mul(unsqueeze283_out1.clone());
        let mul271_out1 = concat251_out1.mul(unsqueeze283_out1);
        let expand51_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where48_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze278_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze278_out1.expand(shape)
        };
        let add253_out1 = mul268_out1.add(mul270_out1);
        let add254_out1 = mul269_out1.add(mul271_out1);
        let reshape140_out1 = expand51_out1.reshape(concat253_out1);
        let concat254_out1 = burn::tensor::Tensor::cat(
            [past_key_values_22_key, add254_out1].into(),
            2,
        );
        let shape143_out1: [i64; 4] = {
            let axes = &concat254_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze284_out1: Tensor<5> = concat254_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather259_out1 = shape143_out1[0] as i64;
        let gather260_out1 = shape143_out1[2] as i64;
        let unsqueeze285_out1 = [gather259_out1 as i64];
        let unsqueeze286_out1 = [gather260_out1 as i64];
        let constant1985_out1: [i64; 1] = [2i64];
        let constant1986_out1: [i64; 1] = [7i64];
        let constant1987_out1: [i64; 1] = [64i64];
        let concat255_out1: [i64; 5usize] = [
            &unsqueeze285_out1[..],
            &constant1985_out1[..],
            &constant1986_out1[..],
            &unsqueeze286_out1[..],
            &constant1987_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1988_out1: [i64; 1] = [14i64];
        let constant1989_out1: [i64; 1] = [64i64];
        let concat256_out1: [i64; 4usize] = [
            &unsqueeze285_out1[..],
            &constant1988_out1[..],
            &unsqueeze286_out1[..],
            &constant1989_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant1990_out1 = self.constant1990.val();
        let equal49_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat255_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant1990_out1)
        };
        let constant1991_out1 = self.constant1991.val();
        let where49_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat255_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal49_out1, constant1991_out1);
        let expand52_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where49_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze284_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze284_out1.expand(shape)
        };
        let reshape141_out1 = expand52_out1.reshape(concat256_out1);
        let shape144_out1: [i64; 4] = {
            let axes = &reshape141_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather261_out1 = shape144_out1[2] as i64;
        let unsqueeze287_out1 = [gather261_out1 as i64];
        let slice163_out1 = scatternd1_out1
            .slice(s![.., .., .., 0..unsqueeze287_out1[0]]);
        let (matmul203_out1,) = {
            let q = add253_out1;
            let k = reshape141_out1;
            let v = reshape140_out1;
            let matmul203_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice163_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul203_out1,)
        };
        let transpose116_out1 = matmul203_out1.permute([0, 2, 1, 3]);
        let reshape142_out1 = transpose116_out1.reshape(concat248_out1);
        let linear158_out1 = self.linear158.forward(reshape142_out1);
        let add256_out1 = add247_out1.add(linear158_out1);
        (add256_out1, concat254_out1, concat249_out1)
    }
}
#[derive(Module, Debug)]
pub struct Submodule11 {
    constant1999: burn::module::Param<Tensor<1>>,
    constant2000: burn::module::Param<Tensor<1>>,
    constant2001: burn::module::Param<Tensor<1>>,
    constant2002: burn::module::Param<Tensor<1>>,
    linear159: Linear,
    linear160: Linear,
    linear161: Linear,
    constant2006: burn::module::Param<Tensor<1>>,
    constant2007: burn::module::Param<Tensor<1>>,
    constant2008: burn::module::Param<Tensor<1>>,
    constant2009: burn::module::Param<Tensor<1>>,
    linear162: Linear,
    linear163: Linear,
    linear164: Linear,
    constant2015: burn::module::Param<Tensor<1>>,
    constant2016: burn::module::Param<Tensor<1>>,
    constant2017: burn::module::Param<Tensor<1>>,
    constant2059: burn::module::Param<Tensor<1, Int>>,
    constant2062: burn::module::Param<Tensor<1, Int>>,
    #[module(skip)]
    device: Device,
}
impl Submodule11 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant1999: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant2000: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant2001: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant2002: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear159 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear160 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear161 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant2006: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant2007: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant2008: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant2009: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear162 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let linear163 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let linear164 = LinearConfig::new(896, 128).with_bias(false).init(device);
        let constant2015: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let constant2016: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant2017: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([128], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [128].into(),
        );
        let constant2059: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant2062: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        Self {
            constant1999,
            constant2000,
            constant2001,
            constant2002,
            linear159,
            linear160,
            linear161,
            constant2006,
            constant2007,
            constant2008,
            constant2009,
            linear162,
            linear163,
            linear164,
            constant2015,
            constant2016,
            constant2017,
            constant2059,
            constant2062,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        add256_out1: Tensor<3>,
        past_key_values_23_value: Tensor<4>,
        gather26_out1: i64,
        constant108_out1: Tensor<2>,
        constant112_out1: Tensor<2>,
        position_ids: Tensor<2, Int>,
    ) -> (
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<5>,
        Tensor<1, Int>,
        Tensor<4>,
        Tensor<4>,
        [i64; 4],
        [i64; 3],
        Tensor<3>,
        Tensor<4>,
    ) {
        let constant1999_out1 = self.constant1999.val();
        let pow46_out1 = add256_out1
            .clone()
            .powf((constant1999_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean46_out1 = { pow46_out1.mean_dim(2usize) };
        let constant2000_out1 = self.constant2000.val();
        let add257_out1 = reducemean46_out1
            .add((constant2000_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt46_out1 = add257_out1.sqrt();
        let constant2001_out1 = self.constant2001.val();
        let div46_out1 = (constant2001_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt46_out1);
        let mul274_out1 = add256_out1.clone().mul(div46_out1);
        let constant2002_out1 = self.constant2002.val();
        let mul275_out1 = (constant2002_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul274_out1);
        let linear159_out1 = self.linear159.forward(mul275_out1.clone());
        let linear160_out1 = self.linear160.forward(mul275_out1);
        let sigmoid23_out1 = burn::tensor::activation::sigmoid(linear159_out1.clone());
        let mul276_out1 = linear159_out1.mul(sigmoid23_out1);
        let mul277_out1 = mul276_out1.mul(linear160_out1);
        let linear161_out1 = self.linear161.forward(mul277_out1);
        let add258_out1 = add256_out1.add(linear161_out1);
        let constant2006_out1 = self.constant2006.val();
        let pow47_out1 = add258_out1
            .clone()
            .powf((constant2006_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean47_out1 = { pow47_out1.mean_dim(2usize) };
        let constant2007_out1 = self.constant2007.val();
        let add259_out1 = reducemean47_out1
            .add((constant2007_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt47_out1 = add259_out1.sqrt();
        let constant2008_out1 = self.constant2008.val();
        let div47_out1 = (constant2008_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt47_out1);
        let mul278_out1 = add258_out1.clone().mul(div47_out1);
        let constant2009_out1 = self.constant2009.val();
        let mul279_out1 = (constant2009_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul278_out1);
        let shape145_out1: [i64; 3] = {
            let axes = &mul279_out1.clone().dims()[0..3];
            let mut output = [0i64; 3];
            for i in 0..3 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let linear162_out1 = self.linear162.forward(mul279_out1.clone());
        let linear163_out1 = self.linear163.forward(mul279_out1.clone());
        let linear164_out1 = self.linear164.forward(mul279_out1);
        let gather262_out1 = shape145_out1[0] as i64;
        let gather263_out1 = shape145_out1[1] as i64;
        let constant2015_out1 = self.constant2015.val();
        let add260_out1 = (constant2015_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear162_out1);
        let constant2016_out1 = self.constant2016.val();
        let add261_out1 = (constant2016_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear163_out1);
        let constant2017_out1 = self.constant2017.val();
        let add262_out1 = (constant2017_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .add(linear164_out1);
        let unsqueeze288_out1 = [gather262_out1 as i64];
        let unsqueeze289_out1 = [gather263_out1 as i64];
        let constant2020_out1: [i64; 1] = [14i64];
        let constant2021_out1: [i64; 1] = [64i64];
        let concat257_out1: [i64; 4usize] = [
            &unsqueeze288_out1[..],
            &unsqueeze289_out1[..],
            &constant2020_out1[..],
            &constant2021_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant2022_out1: [i64; 1] = [2i64];
        let constant2023_out1: [i64; 1] = [64i64];
        let concat258_out1: [i64; 4usize] = [
            &unsqueeze288_out1[..],
            &unsqueeze289_out1[..],
            &constant2022_out1[..],
            &constant2023_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant2024_out1: [i64; 1] = [896i64];
        let concat259_out1: [i64; 3usize] = [
            &unsqueeze288_out1[..],
            &unsqueeze289_out1[..],
            &constant2024_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let reshape143_out1 = add260_out1.reshape(concat257_out1);
        let reshape144_out1 = add261_out1.reshape(concat258_out1);
        let reshape145_out1 = add262_out1.reshape(concat258_out1);
        let transpose117_out1 = reshape143_out1.permute([0, 2, 1, 3]);
        let transpose118_out1 = reshape144_out1.permute([0, 2, 1, 3]);
        let transpose119_out1 = reshape145_out1.permute([0, 2, 1, 3]);
        let shape146_out1: [i64; 4] = {
            let axes = &transpose118_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let concat260_out1 = burn::tensor::Tensor::cat(
            [past_key_values_23_value, transpose119_out1].into(),
            2,
        );
        let slice164_out1 = transpose117_out1.clone().slice(s![.., .., .., 0..32]);
        let slice165_out1 = transpose117_out1.clone().slice(s![.., .., .., 32..]);
        let slice166_out1 = transpose118_out1.clone().slice(s![.., .., .., 0..32]);
        let slice167_out1 = transpose118_out1.clone().slice(s![.., .., .., 32..]);
        let gather264_out1 = shape146_out1[2] as i64;
        let shape147_out1: [i64; 4] = {
            let axes = &concat260_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze290_out1: Tensor<5> = concat260_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let neg47_out1 = slice165_out1.neg();
        let neg48_out1 = slice167_out1.neg();
        let add263_out1 = gather264_out1 + gather26_out1;
        let gather265_out1 = shape147_out1[0] as i64;
        let gather266_out1 = shape147_out1[2] as i64;
        let concat261_out1 = burn::tensor::Tensor::cat(
            [neg47_out1, slice164_out1].into(),
            3,
        );
        let concat262_out1 = burn::tensor::Tensor::cat(
            [neg48_out1, slice166_out1].into(),
            3,
        );
        let unsqueeze291_out1 = [add263_out1 as i64];
        let unsqueeze292_out1 = [gather265_out1 as i64];
        let unsqueeze293_out1 = [gather266_out1 as i64];
        let slice168_out1 = constant108_out1.slice(s![0..unsqueeze291_out1[0], ..]);
        let slice169_out1 = constant112_out1.slice(s![0..unsqueeze291_out1[0], ..]);
        let constant2054_out1: [i64; 1] = [2i64];
        let constant2055_out1: [i64; 1] = [7i64];
        let constant2056_out1: [i64; 1] = [64i64];
        let concat263_out1: [i64; 5usize] = [
            &unsqueeze292_out1[..],
            &constant2054_out1[..],
            &constant2055_out1[..],
            &unsqueeze293_out1[..],
            &constant2056_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant2057_out1: [i64; 1] = [14i64];
        let constant2058_out1: [i64; 1] = [64i64];
        let concat264_out1: [i64; 4usize] = [
            &unsqueeze292_out1[..],
            &constant2057_out1[..],
            &unsqueeze293_out1[..],
            &constant2058_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let gather267_out1 = slice168_out1.take::<2, 3>(0, position_ids.clone());
        let gather268_out1 = slice169_out1.take::<2, 3>(0, position_ids);
        let constant2059_out1 = self.constant2059.val();
        let equal50_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat263_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant2059_out1)
        };
        let unsqueeze294_out1: Tensor<4> = gather267_out1.unsqueeze_dims::<4>(&[1]);
        let unsqueeze295_out1: Tensor<4> = gather268_out1.unsqueeze_dims::<4>(&[1]);
        let constant2062_out1 = self.constant2062.val();
        let where50_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat263_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal50_out1, constant2062_out1);
        let mul280_out1 = transpose117_out1.mul(unsqueeze294_out1.clone());
        let mul281_out1 = transpose118_out1.mul(unsqueeze294_out1);
        (
            concat261_out1,
            unsqueeze295_out1,
            concat262_out1,
            unsqueeze290_out1,
            where50_out1,
            mul280_out1,
            mul281_out1,
            concat264_out1,
            concat259_out1,
            add258_out1,
            concat260_out1,
        )
    }
}
#[derive(Module, Debug)]
pub struct Submodule12 {
    constant2074: burn::module::Param<Tensor<1, Int>>,
    constant2075: burn::module::Param<Tensor<1, Int>>,
    linear165: Linear,
    constant2083: burn::module::Param<Tensor<1>>,
    constant2084: burn::module::Param<Tensor<1>>,
    constant2085: burn::module::Param<Tensor<1>>,
    constant2086: burn::module::Param<Tensor<1>>,
    linear166: Linear,
    linear167: Linear,
    linear168: Linear,
    constant2090: burn::module::Param<Tensor<1>>,
    constant2091: burn::module::Param<Tensor<1>>,
    constant2092: burn::module::Param<Tensor<1>>,
    constant2093: burn::module::Param<Tensor<1>>,
    #[module(skip)]
    device: Device,
}
impl Submodule12 {
    #[allow(unused_variables)]
    pub fn new(device: &Device) -> Self {
        let constant2074: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let constant2075: burn::module::Param<Tensor<1, Int>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
                Int,
            >::zeros([5], (device, burn::tensor::DType::I64)),
            device.clone(),
            false,
            [5].into(),
        );
        let linear165 = LinearConfig::new(896, 896).with_bias(false).init(device);
        let constant2083: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant2084: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant2085: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant2086: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        let linear166 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear167 = LinearConfig::new(896, 4864).with_bias(false).init(device);
        let linear168 = LinearConfig::new(4864, 896).with_bias(false).init(device);
        let constant2090: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant2091: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::from_data(
                burn::tensor::TensorData::from([0.0000009999999974752427f64]),
                (device, burn::tensor::DType::F32),
            ),
            device.clone(),
            false,
            [1].into(),
        );
        let constant2092: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
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
        let constant2093: burn::module::Param<Tensor<1>> = burn::module::Param::uninitialized(
            burn::module::ParamId::new(),
            move |device, _require_grad| Tensor::<
                1,
            >::zeros([896], (device, burn::tensor::DType::F32)),
            device.clone(),
            false,
            [896].into(),
        );
        Self {
            constant2074,
            constant2075,
            linear165,
            constant2083,
            constant2084,
            constant2085,
            constant2086,
            linear166,
            linear167,
            linear168,
            constant2090,
            constant2091,
            constant2092,
            constant2093,
            device: device.clone(),
        }
    }
    #[allow(clippy::let_and_return, clippy::approx_constant)]
    pub fn forward(
        &self,
        concat261_out1: Tensor<4>,
        unsqueeze295_out1: Tensor<4>,
        concat262_out1: Tensor<4>,
        unsqueeze290_out1: Tensor<5>,
        where50_out1: Tensor<1, Int>,
        mul280_out1: Tensor<4>,
        mul281_out1: Tensor<4>,
        concat264_out1: [i64; 4],
        past_key_values_23_key: Tensor<4>,
        scatternd1_out1: Tensor<4>,
        concat259_out1: [i64; 3],
        add258_out1: Tensor<3>,
        transpose1_out1: Tensor<2>,
    ) -> (Tensor<3>, Tensor<4>) {
        let mul282_out1 = concat261_out1.mul(unsqueeze295_out1.clone());
        let mul283_out1 = concat262_out1.mul(unsqueeze295_out1);
        let expand53_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where50_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze290_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze290_out1.expand(shape)
        };
        let add264_out1 = mul280_out1.add(mul282_out1);
        let add265_out1 = mul281_out1.add(mul283_out1);
        let reshape146_out1 = expand53_out1.reshape(concat264_out1);
        let concat265_out1 = burn::tensor::Tensor::cat(
            [past_key_values_23_key, add265_out1].into(),
            2,
        );
        let shape148_out1: [i64; 4] = {
            let axes = &concat265_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let unsqueeze296_out1: Tensor<5> = concat265_out1
            .clone()
            .unsqueeze_dims::<5>(&[2]);
        let gather269_out1 = shape148_out1[0] as i64;
        let gather270_out1 = shape148_out1[2] as i64;
        let unsqueeze297_out1 = [gather269_out1 as i64];
        let unsqueeze298_out1 = [gather270_out1 as i64];
        let constant2069_out1: [i64; 1] = [2i64];
        let constant2070_out1: [i64; 1] = [7i64];
        let constant2071_out1: [i64; 1] = [64i64];
        let concat266_out1: [i64; 5usize] = [
            &unsqueeze297_out1[..],
            &constant2069_out1[..],
            &constant2070_out1[..],
            &unsqueeze298_out1[..],
            &constant2071_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant2072_out1: [i64; 1] = [14i64];
        let constant2073_out1: [i64; 1] = [64i64];
        let concat267_out1: [i64; 4usize] = [
            &unsqueeze297_out1[..],
            &constant2072_out1[..],
            &unsqueeze298_out1[..],
            &constant2073_out1[..],
        ]
            .concat()
            .try_into()
            .unwrap();
        let constant2074_out1 = self.constant2074.val();
        let equal51_out1 = {
            let shape_tensor = Tensor::<
                1,
                Int,
            >::from_data(
                burn::tensor::TensorData::from(concat266_out1.as_slice()),
                (&self.device, burn::tensor::DType::I64),
            );
            shape_tensor.equal(constant2074_out1)
        };
        let constant2075_out1 = self.constant2075.val();
        let where51_out1 = Tensor::<
            1,
            burn::tensor::Int,
        >::from_data(
                burn::tensor::TensorData::from(&concat266_out1 as &[i64]),
                (&self.device, burn::tensor::DType::I64),
            )
            .mask_where(equal51_out1, constant2075_out1);
        let expand54_out1 = {
            let onnx_shape: [i64; 5usize] = TryInto::<
                [i64; 5usize],
            >::try_into(where51_out1.to_data().convert::<i64>().as_slice().unwrap())
                .unwrap();
            let input_dims = unsqueeze296_out1.dims();
            let mut shape = onnx_shape;
            #[allow(clippy::needless_range_loop)]
            for i in 0..5usize {
                let dim_offset = 5usize - 5usize + i;
                if shape[dim_offset] == 1 && input_dims[i] > 1 {
                    shape[dim_offset] = input_dims[i] as i64;
                }
            }
            unsqueeze296_out1.expand(shape)
        };
        let reshape147_out1 = expand54_out1.reshape(concat267_out1);
        let shape149_out1: [i64; 4] = {
            let axes = &reshape147_out1.clone().dims()[0..4];
            let mut output = [0i64; 4];
            for i in 0..4 {
                output[i] = axes[i] as i64;
            }
            output
        };
        let gather271_out1 = shape149_out1[2] as i64;
        let unsqueeze299_out1 = [gather271_out1 as i64];
        let slice170_out1 = scatternd1_out1
            .slice(s![.., .., .., 0..unsqueeze299_out1[0]]);
        let (matmul212_out1,) = {
            let q = add264_out1;
            let k = reshape147_out1;
            let v = reshape146_out1;
            let matmul212_out1 = burn::tensor::module::attention(
                q,
                k,
                v,
                None,
                Some(slice170_out1),
                burn::tensor::ops::AttentionModuleOptions {
                    scale: Some(0.1249999925494194f64),
                    softcap: None,
                    is_causal: false,
                },
            );
            (matmul212_out1,)
        };
        let transpose121_out1 = matmul212_out1.permute([0, 2, 1, 3]);
        let reshape148_out1 = transpose121_out1.reshape(concat259_out1);
        let linear165_out1 = self.linear165.forward(reshape148_out1);
        let add267_out1 = add258_out1.add(linear165_out1);
        let constant2083_out1 = self.constant2083.val();
        let pow48_out1 = add267_out1
            .clone()
            .powf((constant2083_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean48_out1 = { pow48_out1.mean_dim(2usize) };
        let constant2084_out1 = self.constant2084.val();
        let add268_out1 = reducemean48_out1
            .add((constant2084_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt48_out1 = add268_out1.sqrt();
        let constant2085_out1 = self.constant2085.val();
        let div48_out1 = (constant2085_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt48_out1);
        let mul286_out1 = add267_out1.clone().mul(div48_out1);
        let constant2086_out1 = self.constant2086.val();
        let mul287_out1 = (constant2086_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul286_out1);
        let linear166_out1 = self.linear166.forward(mul287_out1.clone());
        let linear167_out1 = self.linear167.forward(mul287_out1);
        let sigmoid24_out1 = burn::tensor::activation::sigmoid(linear166_out1.clone());
        let mul288_out1 = linear166_out1.mul(sigmoid24_out1);
        let mul289_out1 = mul288_out1.mul(linear167_out1);
        let linear168_out1 = self.linear168.forward(mul289_out1);
        let add269_out1 = add267_out1.add(linear168_out1);
        let constant2090_out1 = self.constant2090.val();
        let pow49_out1 = add269_out1
            .clone()
            .powf((constant2090_out1).unsqueeze_dims(&[0isize, 1isize]));
        let reducemean49_out1 = { pow49_out1.mean_dim(2usize) };
        let constant2091_out1 = self.constant2091.val();
        let add270_out1 = reducemean49_out1
            .add((constant2091_out1).unsqueeze_dims(&[0isize, 1isize]));
        let sqrt49_out1 = add270_out1.sqrt();
        let constant2092_out1 = self.constant2092.val();
        let div49_out1 = (constant2092_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .div(sqrt49_out1);
        let mul290_out1 = add269_out1.mul(div49_out1);
        let constant2093_out1 = self.constant2093.val();
        let mul291_out1 = (constant2093_out1)
            .unsqueeze_dims(&[0isize, 1isize])
            .mul(mul290_out1);
        let matmul217_out1 = mul291_out1.matmul(transpose1_out1.unsqueeze::<3usize>());
        (matmul217_out1, concat265_out1)
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


extern crate std;

impl Default for Model {
    fn default() -> Self {
        Self::from_file("out_q32/model.bpk", &Default::default())
    }
}

impl Model {
    /// Load model weights from a burnpack file.
    pub fn from_file<P: AsRef<std::path::Path>>(file: P, device: &Device) -> Self {
        let mut model = Self::new(device);
        let mut store = BurnpackStore::from_file(file);
        model.load_from(&mut store).expect("Failed to load burnpack file");
        model
    }

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
        position_ids: Tensor<2, Int>,
        past_key_values_0_key: Tensor<4>,
        past_key_values_0_value: Tensor<4>,
        past_key_values_1_key: Tensor<4>,
        past_key_values_1_value: Tensor<4>,
        past_key_values_2_key: Tensor<4>,
        past_key_values_2_value: Tensor<4>,
        past_key_values_3_key: Tensor<4>,
        past_key_values_3_value: Tensor<4>,
        past_key_values_4_key: Tensor<4>,
        past_key_values_4_value: Tensor<4>,
        past_key_values_5_key: Tensor<4>,
        past_key_values_5_value: Tensor<4>,
        past_key_values_6_key: Tensor<4>,
        past_key_values_6_value: Tensor<4>,
        past_key_values_7_key: Tensor<4>,
        past_key_values_7_value: Tensor<4>,
        past_key_values_8_key: Tensor<4>,
        past_key_values_8_value: Tensor<4>,
        past_key_values_9_key: Tensor<4>,
        past_key_values_9_value: Tensor<4>,
        past_key_values_10_key: Tensor<4>,
        past_key_values_10_value: Tensor<4>,
        past_key_values_11_key: Tensor<4>,
        past_key_values_11_value: Tensor<4>,
        past_key_values_12_key: Tensor<4>,
        past_key_values_12_value: Tensor<4>,
        past_key_values_13_key: Tensor<4>,
        past_key_values_13_value: Tensor<4>,
        past_key_values_14_key: Tensor<4>,
        past_key_values_14_value: Tensor<4>,
        past_key_values_15_key: Tensor<4>,
        past_key_values_15_value: Tensor<4>,
        past_key_values_16_key: Tensor<4>,
        past_key_values_16_value: Tensor<4>,
        past_key_values_17_key: Tensor<4>,
        past_key_values_17_value: Tensor<4>,
        past_key_values_18_key: Tensor<4>,
        past_key_values_18_value: Tensor<4>,
        past_key_values_19_key: Tensor<4>,
        past_key_values_19_value: Tensor<4>,
        past_key_values_20_key: Tensor<4>,
        past_key_values_20_value: Tensor<4>,
        past_key_values_21_key: Tensor<4>,
        past_key_values_21_value: Tensor<4>,
        past_key_values_22_key: Tensor<4>,
        past_key_values_22_value: Tensor<4>,
        past_key_values_23_key: Tensor<4>,
        past_key_values_23_value: Tensor<4>,
    ) -> (
        Tensor<3>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
        Tensor<4>,
    ) {
        let (
            add157_out1,
            gather17_out1,
            constant108_out1,
            constant112_out1,
            scatternd1_out1,
            gather18_out1,
            gather19_out1,
            gather20_out1,
            gather21_out1,
            gather22_out1,
            gather23_out1,
            gather24_out1,
            gather25_out1,
            gather26_out1,
            transpose1_out1,
            concat11_out1,
            concat6_out1,
            concat23_out1,
            concat18_out1,
            concat34_out1,
            concat29_out1,
            concat45_out1,
            concat40_out1,
            concat56_out1,
            concat51_out1,
            concat67_out1,
            concat62_out1,
            concat78_out1,
            concat73_out1,
            concat89_out1,
            concat84_out1,
            concat100_out1,
            concat95_out1,
            concat111_out1,
            concat106_out1,
            concat122_out1,
            concat117_out1,
            concat133_out1,
            concat128_out1,
            concat144_out1,
            concat139_out1,
            concat155_out1,
            concat150_out1,
        ) = self
            .submodule1
            .forward(
                input_ids,
                past_key_values_0_key,
                attention_mask,
                past_key_values_1_key,
                past_key_values_2_key,
                past_key_values_3_key,
                past_key_values_4_key,
                past_key_values_5_key,
                past_key_values_6_key,
                past_key_values_7_key,
                past_key_values_8_key,
                past_key_values_9_key,
                past_key_values_10_key,
                past_key_values_11_key,
                past_key_values_12_key,
                past_key_values_13_key,
                past_key_values_14_key.clone(),
                past_key_values_15_key.clone(),
                past_key_values_16_key.clone(),
                past_key_values_17_key.clone(),
                past_key_values_18_key.clone(),
                past_key_values_19_key.clone(),
                past_key_values_20_key.clone(),
                past_key_values_21_key.clone(),
                past_key_values_22_key.clone(),
                past_key_values_23_key.clone(),
                past_key_values_0_value,
                position_ids.clone(),
                past_key_values_1_value,
                past_key_values_2_value,
                past_key_values_3_value,
                past_key_values_4_value,
                past_key_values_5_value,
                past_key_values_6_value,
                past_key_values_7_value,
                past_key_values_8_value,
                past_key_values_9_value,
                past_key_values_10_value,
                past_key_values_11_value,
                past_key_values_12_value,
                past_key_values_13_value,
            );
        let (add168_out1, concat166_out1, concat161_out1) = self
            .submodule2
            .forward(
                add157_out1,
                past_key_values_14_value,
                gather17_out1,
                constant108_out1.clone(),
                constant112_out1.clone(),
                position_ids.clone(),
                past_key_values_14_key,
                scatternd1_out1.clone(),
            );
        let (add179_out1, concat177_out1, concat172_out1) = self
            .submodule3
            .forward(
                add168_out1,
                past_key_values_15_value,
                gather18_out1,
                constant108_out1.clone(),
                constant112_out1.clone(),
                position_ids.clone(),
                past_key_values_15_key,
                scatternd1_out1.clone(),
            );
        let (add190_out1, concat188_out1, concat183_out1) = self
            .submodule4
            .forward(
                add179_out1,
                past_key_values_16_value,
                gather19_out1,
                constant108_out1.clone(),
                constant112_out1.clone(),
                position_ids.clone(),
                past_key_values_16_key,
                scatternd1_out1.clone(),
            );
        let (add201_out1, concat199_out1, concat194_out1) = self
            .submodule5
            .forward(
                add190_out1,
                past_key_values_17_value,
                gather20_out1,
                constant108_out1.clone(),
                constant112_out1.clone(),
                position_ids.clone(),
                past_key_values_17_key,
                scatternd1_out1.clone(),
            );
        let (add212_out1, concat210_out1, concat205_out1) = self
            .submodule6
            .forward(
                add201_out1,
                past_key_values_18_value,
                gather21_out1,
                constant108_out1.clone(),
                constant112_out1.clone(),
                position_ids.clone(),
                past_key_values_18_key,
                scatternd1_out1.clone(),
            );
        let (add223_out1, concat221_out1, concat216_out1) = self
            .submodule7
            .forward(
                add212_out1,
                past_key_values_19_value,
                gather22_out1,
                constant108_out1.clone(),
                constant112_out1.clone(),
                position_ids.clone(),
                past_key_values_19_key,
                scatternd1_out1.clone(),
            );
        let (add234_out1, concat232_out1, concat227_out1) = self
            .submodule8
            .forward(
                add223_out1,
                past_key_values_20_value,
                gather23_out1,
                constant108_out1.clone(),
                constant112_out1.clone(),
                position_ids.clone(),
                past_key_values_20_key,
                scatternd1_out1.clone(),
            );
        let (add245_out1, concat243_out1, concat238_out1) = self
            .submodule9
            .forward(
                add234_out1,
                past_key_values_21_value,
                gather24_out1,
                constant108_out1.clone(),
                constant112_out1.clone(),
                position_ids.clone(),
                past_key_values_21_key,
                scatternd1_out1.clone(),
            );
        let (add256_out1, concat254_out1, concat249_out1) = self
            .submodule10
            .forward(
                add245_out1,
                past_key_values_22_value,
                gather25_out1,
                constant108_out1.clone(),
                constant112_out1.clone(),
                position_ids.clone(),
                past_key_values_22_key,
                scatternd1_out1.clone(),
            );
        let (
            concat261_out1,
            unsqueeze295_out1,
            concat262_out1,
            unsqueeze290_out1,
            where50_out1,
            mul280_out1,
            mul281_out1,
            concat264_out1,
            concat259_out1,
            add258_out1,
            concat260_out1,
        ) = self
            .submodule11
            .forward(
                add256_out1,
                past_key_values_23_value,
                gather26_out1,
                constant108_out1,
                constant112_out1,
                position_ids,
            );
        let (matmul217_out1, concat265_out1) = self
            .submodule12
            .forward(
                concat261_out1,
                unsqueeze295_out1,
                concat262_out1,
                unsqueeze290_out1,
                where50_out1,
                mul280_out1,
                mul281_out1,
                concat264_out1,
                past_key_values_23_key,
                scatternd1_out1,
                concat259_out1,
                add258_out1,
                transpose1_out1,
            );
        (
            matmul217_out1,
            concat11_out1,
            concat6_out1,
            concat23_out1,
            concat18_out1,
            concat34_out1,
            concat29_out1,
            concat45_out1,
            concat40_out1,
            concat56_out1,
            concat51_out1,
            concat67_out1,
            concat62_out1,
            concat78_out1,
            concat73_out1,
            concat89_out1,
            concat84_out1,
            concat100_out1,
            concat95_out1,
            concat111_out1,
            concat106_out1,
            concat122_out1,
            concat117_out1,
            concat133_out1,
            concat128_out1,
            concat144_out1,
            concat139_out1,
            concat155_out1,
            concat150_out1,
            concat166_out1,
            concat161_out1,
            concat177_out1,
            concat172_out1,
            concat188_out1,
            concat183_out1,
            concat199_out1,
            concat194_out1,
            concat210_out1,
            concat205_out1,
            concat221_out1,
            concat216_out1,
            concat232_out1,
            concat227_out1,
            concat243_out1,
            concat238_out1,
            concat254_out1,
            concat249_out1,
            concat265_out1,
            concat260_out1,
        )
    }
}
