//! Sparse vector type used by sparse embedders and the sparse_vector extension.

/// A sparse vector: parallel arrays of token IDs and weights.
///
/// Invariant: `indices.len() == values.len()`.
/// Indices should be sorted for efficient dot product with another sparse vector.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SparseVector {
    pub indices: Vec<u32>,
    pub values: Vec<f32>,
}

impl SparseVector {
    pub fn new(indices: Vec<u32>, values: Vec<f32>) -> Self {
        assert_eq!(
            indices.len(),
            values.len(),
            "indices and values must have same length"
        );
        Self { indices, values }
    }

    pub fn nnz(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// **Le poids d'une position du texte**, avant que le vecteur creux ne fonde
/// les positions d'un même jeton au maximum : de quoi surligner en creux sans
/// charger le modèle. `start..end` est une plage d'octets du texte reçu ; les
/// jetons spéciaux n'en ont pas.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SparseSpan {
    pub start: usize,
    pub end: usize,
    pub weight: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_vector_basics() {
        let v = SparseVector::new(vec![1, 3, 5], vec![0.5, 0.3, 0.2]);
        assert_eq!(v.nnz(), 3);
        assert!(!v.is_empty());

        let empty = SparseVector::new(vec![], vec![]);
        assert!(empty.is_empty());
    }

    #[test]
    #[should_panic(expected = "indices and values must have same length")]
    fn sparse_vector_mismatched_lengths() {
        SparseVector::new(vec![1, 2], vec![0.5]);
    }
}
