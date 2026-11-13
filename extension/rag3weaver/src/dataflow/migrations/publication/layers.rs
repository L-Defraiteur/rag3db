//! Pure, immutable layer resolution over existing publication identities (lot 2a).
//! Payloads retain occurrence fields/content H; this module never computes H,
//! opens storage, grants a writer permit, or claims to filter an index top-k.
use super::PublishedDeclaration;
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, LayerError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerError {
    InvalidIdentity,
    DuplicateGeneration(LayerGeneration),
    UnknownGeneration(LayerGeneration),
    UnknownLayer(String),
    AmbiguousLayer(String),
    MissingParent(LayerGeneration),
    ForeignOrigin,
    Cycle(LayerGeneration),
    DuplicateOccurrence(OccurrenceKey),
    ConflictingTombstone(OccurrenceKey),
}
impl std::fmt::Display for LayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for LayerError {}
fn nonempty(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(LayerError::InvalidIdentity)
    } else {
        Ok(())
    }
}

/// The exact P of one layer, within one owned origin; never a content H.
/// A reference is not a qualification of the layer's physical contents.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LayerGeneration {
    layer: String,
    origin: String,
    publication: String,
}
impl LayerGeneration {
    fn new(layer: &str, origin: &str, publication: &str) -> Result<Self> {
        nonempty(layer)?;
        Ok(Self {
            layer: layer.into(),
            origin: origin.into(),
            publication: publication.into(),
        })
    }
    pub fn from_publication(layer: &str, publication: &PublishedDeclaration) -> Result<Self> {
        Self::new(layer, publication.origin(), publication.publication())
    }
    /// Binds to the existing qualified new-index state, without creating a DB
    /// or treating a declaration receipt as a physical snapshot certificate.
    #[cfg(feature = "rag3db-native")]
    pub fn from_physical(
        layer: &str,
        state: &super::super::physical_index::QualifiedPhysicalState,
    ) -> Result<Self> {
        Self::new(layer, state.origin(), state.publication())
    }
    pub fn layer(&self) -> &str {
        &self.layer
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn publication(&self) -> &str {
        &self.publication
    }
}

/// Caller-supplied stable entity/occurrence identity, distinct from content H.
/// For Scope this identifies a Scope UUID, not a tenant (org, project) cell.
/// The caller must supply occurrence identities unique in the selected origin.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OccurrenceKey {
    entity: String,
    uuid: String,
}
impl OccurrenceKey {
    pub fn new(entity: &str, uuid: &str) -> Result<Self> {
        nonempty(entity)?;
        nonempty(uuid)?;
        Ok(Self {
            entity: entity.into(),
            uuid: uuid.into(),
        })
    }
    pub fn entity(&self) -> &str {
        &self.entity
    }
    pub fn uuid(&self) -> &str {
        &self.uuid
    }
}
/// Logical file identity supplied by the same occurrence projection, not a
/// filesystem path to resolve or a content digest to deduplicate.
/// The caller must include its domain/cell namespace when needed: a bare
/// "a.rs" shared by two cells would intentionally name the same file here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileKey(String);
impl FileKey {
    pub fn new(value: &str) -> Result<Self> {
        nonempty(value)?;
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// T contains the actual public fields/content reference. Resolution preserves
/// it by borrowing, without rendering, relocating coordinates or merging H.
#[derive(Debug)]
pub struct Occurrence<T> {
    key: OccurrenceKey,
    file: FileKey,
    value: T,
}
impl<T> Occurrence<T> {
    pub fn new(key: OccurrenceKey, file: FileKey, value: T) -> Self {
        Self { key, file, value }
    }
    pub fn key(&self) -> &OccurrenceKey {
        &self.key
    }
    pub fn file(&self) -> &FileKey {
        &self.file
    }
    pub fn value(&self) -> &T {
        &self.value
    }
}

/// Local delta in one exact publication. A file tombstone hides ancestors;
/// explicit values in this same revision can describe a replacement file.
#[derive(Debug)]
pub struct LayerRevision<T> {
    generation: LayerGeneration,
    parent: Option<LayerGeneration>,
    occurrences: BTreeMap<OccurrenceKey, Occurrence<T>>,
    removed_occurrences: BTreeSet<OccurrenceKey>,
    removed_files: BTreeSet<FileKey>,
}
impl<T> LayerRevision<T> {
    pub fn new(
        generation: LayerGeneration,
        parent: Option<LayerGeneration>,
        occurrences: Vec<Occurrence<T>>,
        removed_occurrences: BTreeSet<OccurrenceKey>,
        removed_files: BTreeSet<FileKey>,
    ) -> Result<Self> {
        let mut rows = BTreeMap::new();
        for row in occurrences {
            let key = row.key.clone();
            if removed_occurrences.contains(&key) {
                return Err(LayerError::ConflictingTombstone(key));
            }
            if rows.insert(key.clone(), row).is_some() {
                return Err(LayerError::DuplicateOccurrence(key));
            }
        }
        Ok(Self {
            generation,
            parent,
            occurrences: rows,
            removed_occurrences,
            removed_files,
        })
    }
    pub fn generation(&self) -> &LayerGeneration {
        &self.generation
    }
    pub fn parent(&self) -> Option<&LayerGeneration> {
        self.parent.as_ref()
    }
}

/// One frozen set of supplied layer deltas. No mutable head lookup occurs
/// during resolution. This is not the storage/index snapshot used by L6 yet.
#[derive(Debug)]
pub struct PinnedLayers<T> {
    revisions: BTreeMap<LayerGeneration, LayerRevision<T>>,
}
impl<T> PinnedLayers<T> {
    pub fn new(revisions: Vec<LayerRevision<T>>) -> Result<Self> {
        let mut map = BTreeMap::new();
        for revision in revisions {
            let key = revision.generation.clone();
            if map.insert(key.clone(), revision).is_some() {
                return Err(LayerError::DuplicateGeneration(key));
            }
        }
        // Validate the complete supplied forest, including unselected views.
        // Iterative traversal cannot overflow the call stack on a deep chain.
        for (key, revision) in &map {
            if let Some(parent) = &revision.parent {
                if parent.origin != key.origin {
                    return Err(LayerError::ForeignOrigin);
                }
                if !map.contains_key(parent) {
                    return Err(LayerError::MissingParent(parent.clone()));
                }
            }
        }
        for key in map.keys() {
            let mut seen = BTreeSet::new();
            let mut cursor = Some(key);
            while let Some(current) = cursor {
                if !seen.insert(current) {
                    return Err(LayerError::Cycle(current.clone()));
                }
                cursor = map[current].parent.as_ref();
            }
        }
        Ok(Self { revisions: map })
    }
    /// Name-only selection refuses multiple Ps, rather than guessing "latest"
    /// or following an anchor that advanced after the view was pinned.
    pub fn select(&self, layer: &str) -> Result<&LayerGeneration> {
        let mut keys = self.revisions.keys().filter(|key| key.layer == layer);
        let key = keys
            .next()
            .ok_or_else(|| LayerError::UnknownLayer(layer.into()))?;
        if keys.next().is_some() {
            return Err(LayerError::AmbiguousLayer(layer.into()));
        }
        Ok(key)
    }
    pub fn resolve(&self, generation: &LayerGeneration) -> Result<ResolvedLayer<'_, T>> {
        let (key, _) = self
            .revisions
            .get_key_value(generation)
            .ok_or_else(|| LayerError::UnknownGeneration(generation.clone()))?;
        let mut chain = Vec::new();
        let mut cursor = Some(key);
        while let Some(current) = cursor {
            let revision = &self.revisions[current];
            chain.push(revision);
            cursor = revision.parent.as_ref();
        }
        let mut visible: BTreeMap<OccurrenceKey, VisibleOccurrence<'_, T>> = BTreeMap::new();
        for revision in chain.into_iter().rev() {
            visible.retain(|id, row| {
                !revision.removed_occurrences.contains(id)
                    && !revision.removed_files.contains(&row.occurrence.file)
            });
            for (id, occurrence) in &revision.occurrences {
                visible.insert(
                    id.clone(),
                    VisibleOccurrence {
                        occurrence,
                        provenance: &revision.generation,
                    },
                );
            }
        }
        Ok(ResolvedLayer {
            generation: key,
            occurrences: visible,
        })
    }
}
#[derive(Debug)]
pub struct VisibleOccurrence<'a, T> {
    occurrence: &'a Occurrence<T>,
    provenance: &'a LayerGeneration,
}
impl<'a, T> VisibleOccurrence<'a, T> {
    pub fn occurrence(&self) -> &'a Occurrence<T> {
        self.occurrence
    }
    pub fn provenance(&self) -> &'a LayerGeneration {
        self.provenance
    }
}
#[derive(Debug)]
pub struct ResolvedLayer<'a, T> {
    generation: &'a LayerGeneration,
    occurrences: BTreeMap<OccurrenceKey, VisibleOccurrence<'a, T>>,
}
impl<'a, T> ResolvedLayer<'a, T> {
    pub fn generation(&self) -> &LayerGeneration {
        self.generation
    }
    pub fn occurrences(&self) -> &BTreeMap<OccurrenceKey, VisibleOccurrence<'a, T>> {
        &self.occurrences
    }
}

#[cfg(test)]
mod tests;
