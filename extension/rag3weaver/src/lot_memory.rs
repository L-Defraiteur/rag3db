//! **The memory of a batch, bounded before the call** (20 November 2026). A batch the
//! service accepted asked cubecl for one buffer of 4 357 506 816 bytes on the 7879 : over
//! radv's single allocation limit (`maxMemoryAllocationSize`, 0xfffffffc bytes), though the
//! card had 12 GiB free. A refused allocation poisons cubecl's server for good (« Parent
//! device is lost », then every call panics) : a batch cannot be retried smaller after the
//! fact. So each batch is estimated **before** the call, against two bounds — its largest
//! buffer under the adapter's allocation limit, its total under half the card's free memory
//! — and cut in halves until it fits ; a text alone that does not fit is refused, named.
//!
//! The largest buffer is the attention's : batch × heads × T² × 4 bytes, T the longest text
//! of the batch in tokens. Measured exactly : 32 texts of 1 704 tokens on bge-m3 (16 heads)
//! asked for 5 946 605 568 bytes.

/// What the memory of a batch depends on, as the model declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    pub heads: usize,
    pub hidden: usize,
}

/// The card's bounds, read now ; `None` : not known, not bounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bounds {
    /// The largest single allocation the adapter takes (wgpu `max_buffer_size`).
    pub max_allocation: Option<u64>,
    /// The card's free memory (amdgpu sysfs).
    pub free: Option<u64>,
}

/// The share of the allocation limit a batch's largest buffer may take.
pub const ALLOCATION_SHARE: f64 = 0.9;
/// The share of the free memory a batch may take : the card has other tenants.
pub const FREE_SHARE: f64 = 0.5;

/// The memory a batch asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estimate {
    pub largest: u64,
    pub total: u64,
}

/// Why a batch does not fit, if it does not.
pub fn over(e: Estimate, bounds: Bounds) -> Option<String> {
    if let Some(limit) = bounds.max_allocation {
        if e.largest as f64 > limit as f64 * ALLOCATION_SHARE {
            return Some(format!("un tampon de {} octets estimés, au-delà de {} % de la limite d'allocation de la carte ({limit})", e.largest, (ALLOCATION_SHARE * 100.0) as u32));
        }
    }
    if let Some(free) = bounds.free {
        if e.total as f64 > free as f64 * FREE_SHARE {
            return Some(format!("{} octets estimés, au-delà de la moitié de la mémoire libre de la carte ({free})", e.total));
        }
    }
    None
}

/// **The free memory of the card at this PCI address** : `mem_info_vram_total −
/// mem_info_vram_used` under `drm/cardN/device`, the card found by its `PCI_SLOT_NAME`.
pub fn free_memory(pci_bus_id: &str, drm: &std::path::Path) -> Option<u64> {
    let wanted = pci_bus_id.trim().to_ascii_lowercase();
    for entry in std::fs::read_dir(drm).ok()?.flatten() {
        let device = entry.path().join("device");
        let uevent = std::fs::read_to_string(device.join("uevent")).unwrap_or_default();
        let slot = uevent.lines().find_map(|l| l.strip_prefix("PCI_SLOT_NAME=")).unwrap_or("").to_ascii_lowercase();
        if slot.is_empty() || slot != wanted {
            continue;
        }
        let read = |name: &str| std::fs::read_to_string(device.join(name)).ok()?.trim().parse::<u64>().ok();
        if let (Some(total), Some(used)) = (read("mem_info_vram_total"), read("mem_info_vram_used")) {
            return Some(total.saturating_sub(used));
        }
    }
    None
}

/// The memory a batch of these token counts asks for : the attention's buffer
/// (batch × heads × T² × 4 bytes), and a total of three such buffers (scores, mask,
/// softmax) plus the activations (batch × T × hidden × 4 bytes, 32 times over the layers'
/// live tensors).
pub fn estimate(shape: Shape, tokens: &[usize]) -> Estimate {
    let batch = tokens.len() as u64;
    let t = tokens.iter().copied().max().unwrap_or(0) as u64;
    let largest = batch * shape.heads as u64 * t * t * 4;
    Estimate { largest, total: 3 * largest + batch * t * shape.hidden as u64 * 4 * 32 }
}

/// **The batches, cut so that each fits** : a batch over a bound is cut in two, then
/// again, in order ; a text alone that does not fit is `Err((its index, why))`.
pub fn cut(tokens: &[usize], batches: Vec<std::ops::Range<usize>>, shape: Shape, bounds: Bounds) -> Result<Vec<std::ops::Range<usize>>, (usize, String)> {
    let mut out = Vec::new();
    let mut stack: Vec<std::ops::Range<usize>> = batches.into_iter().rev().collect();
    while let Some(batch) = stack.pop() {
        match over(estimate(shape, &tokens[batch.clone()]), bounds) {
            None => out.push(batch),
            Some(why) if batch.len() == 1 => return Err((batch.start, why)),
            Some(_) => {
                let mid = batch.start + batch.len() / 2;
                stack.push(mid..batch.end);
                stack.push(batch.start..mid);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BGE: Shape = Shape { heads: 16, hidden: 1024 };
    const RADV: Bounds = Bounds { max_allocation: Some(0xffff_fffc), free: None };

    /// **The estimate meets the buffer cubecl asked for, and a batch over the adapter's
    /// limit is cut until it fits** ; a text alone over the limit is refused, named
    /// (20 November 2026 : 32 × 1 704 tokens asked 5 946 605 568 bytes on bge-m3).
    #[test]
    fn a_batch_over_the_allocation_limit_is_cut_before_the_call() {
        assert_eq!(estimate(BGE, &[1704; 32]).largest, 5_946_605_568);
        let parts = cut(&[1704; 32], vec![0..32], BGE, RADV).expect("cut, not refused");
        assert!(parts.len() > 1, "{parts:?}");
        assert_eq!(parts.first().unwrap().start, 0);
        assert_eq!(parts.last().unwrap().end, 32);
        assert!(parts.windows(2).all(|w| w[0].end == w[1].start), "in order, contiguous : {parts:?}");
        assert!(parts.iter().all(|p| over(estimate(BGE, &[1704; 32][p.clone()]), RADV).is_none()));
        let (at, why) = cut(&[100, 8192, 100], vec![0..3], BGE, RADV).expect_err("a text of 8 192 tokens asks 4 294 967 296 bytes");
        assert_eq!(at, 1);
        assert!(why.contains("4294967296"), "{why}");
        let free = Bounds { max_allocation: None, free: Some(4 << 30) };
        assert!(cut(&[1704; 32], vec![0..32], BGE, free).unwrap().len() > 1, "half the free memory bounds the total too");
        assert_eq!(cut(&[1704; 32], vec![0..32], BGE, Bounds::default()).unwrap(), vec![0..32], "unknown bounds : unchanged");
    }

    /// The free memory of a card, found by its PCI address in a drm tree.
    #[test]
    fn the_free_memory_is_read_by_the_pci_address() {
        let drm = tempfile::tempdir().unwrap();
        for (card, slot, total, used) in [("card0", "0000:03:00.0", 100, 40), ("card2", "0000:0c:00.0", 1000, 250)] {
            let d = drm.path().join(card).join("device");
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("uevent"), format!("DRIVER=amdgpu\nPCI_SLOT_NAME={slot}\n")).unwrap();
            std::fs::write(d.join("mem_info_vram_total"), format!("{total}\n")).unwrap();
            std::fs::write(d.join("mem_info_vram_used"), format!("{used}\n")).unwrap();
        }
        assert_eq!(free_memory("0000:0C:00.0", drm.path()), Some(750));
        assert_eq!(free_memory("0000:99:00.0", drm.path()), None);
    }
}
