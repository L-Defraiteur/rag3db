//! Choix du périphérique burn (wgpu), partagé par les embedders, les rerankers
//! et l'OCR. Compilé dès qu'une feature burn est active (`burn-embedder` ou
//! `burn-ocr`) ; les modules historiques le ré-exportent sous leur ancien chemin
//! (`crate::burn_bge_m3_embedder::BurnDevice`).

use burn::prelude::*;

/// Which GPU burn should run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BurnDevice {
    /// Best available device (discrete GPU if present).
    #[default]
    Default,
    /// Nth discrete GPU — useful for sharding across several cards.
    DiscreteGpu(usize),
    /// Integrated GPU.
    IntegratedGpu(usize),
    /// CPU fallback. Correct but slow; handy for reproducible reference output.
    Cpu,
}

impl BurnDevice {
    /// Shared with the other burn embedders in this crate.
    pub(crate) fn resolve(self) -> Device {
        match self {
            BurnDevice::Default => Device::default(),
            BurnDevice::DiscreteGpu(i) => Device::wgpu(DeviceKind::DiscreteGpu(i)),
            BurnDevice::IntegratedGpu(i) => Device::wgpu(DeviceKind::IntegratedGpu(i)),
            BurnDevice::Cpu => Device::wgpu(DeviceKind::Cpu),
        }
    }
}
