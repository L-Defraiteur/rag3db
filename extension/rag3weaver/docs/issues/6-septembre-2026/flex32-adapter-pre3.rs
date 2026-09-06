impl burn_store::ModuleAdapter for Flex32Adapter {
    fn adapt(
        &self,
        tensor: burn_store::burn_pack::Tensor,
        _ctx: burn_store::ModuleContext<'_>,
    ) -> burn_store::burn_pack::Tensor {
        use burn::tensor::DType;
        if tensor.dtype != DType::F32 {
            return tensor;
        }
        let (name, shape) = (tensor.name.clone(), tensor.shape.clone());
        burn_store::bridge::map_data(tensor, name, DType::Flex32, shape, |d| {
            burn::tensor::TensorData::from_bytes(d.bytes, d.shape, DType::Flex32)
        })
    }

    fn clone_box(&self) -> Box<dyn burn_store::ModuleAdapter> {
        Box::new(Flex32Adapter)
    }
}
