//! CPU admission for pinned occurrences (2b-A), not a physical view loader.
//! No public constructor, deserialization or tool parameter grants this context.
//! The only constructor today is explicitly a test fixture. Physical B and
//! reachable tool graphs C require their own qualified provider; fixture contexts
//! are refused at that boundary instead of pretending to certify an index.
#[cfg(test)]
use super::layers::PinnedLayers;
use super::layers::{LayerError, LayerGeneration, OccurrenceKey};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub(crate) const SERVICE: &str = "pinned_read_admission";
type Result<T> = std::result::Result<T, AdmissionError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AdmissionError {
    Layer(LayerError),
    Expired,
    MissingSnapshot,
    ForeignCell,
    MissingCell,
    ConflictingContexts,
    MissingBinding(String),
    StorageChanged,
    IncapableFilter,
    ScanWithoutAdmission,
    FixtureOnly,
    UnsupportedRead,
    LeakedOccurrence(OccurrenceKey),
    MissingSourceIdentity,
}
impl std::fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pinned admission: {self:?}")
    }
}
impl std::error::Error for AdmissionError {}

#[derive(Debug, Clone)]
pub(crate) struct StorageAddress {
    cell: crate::scope::Scope,
    generation: LayerGeneration,
    model: String,
    offset: u64,
}

/// Immutable visibility and winning provenance plus one irreversible validity
/// latch for the call. H and payload coordinates remain in the resolved rows;
/// they are never used as occurrence identities or rewritten here.
#[derive(Debug)]
pub(crate) struct ReadAdmission {
    cell: crate::scope::Scope,
    selected: LayerGeneration,
    entities: BTreeSet<String>,
    winners: BTreeMap<OccurrenceKey, LayerGeneration>,
    bindings: BTreeMap<OccurrenceKey, StorageAddress>,
    deadline: Instant,
    invalid: Mutex<Option<AdmissionError>>,
}
impl ReadAdmission {
    #[cfg(test)]
    pub(crate) fn from_fixture<T>(
        layers: &PinnedLayers<T>,
        name: &str,
        cell: crate::scope::Scope,
        entities: BTreeSet<String>,
        bindings: BTreeMap<OccurrenceKey, StorageAddress>,
        deadline: Instant,
    ) -> Result<Self> {
        let selected = layers.select(name).map_err(AdmissionError::Layer)?;
        let resolved = layers.resolve(selected).map_err(AdmissionError::Layer)?;
        let mut winners = BTreeMap::new();
        for (key, row) in resolved.occurrences() {
            if !entities.contains(key.entity()) {
                return Err(AdmissionError::MissingBinding(key.entity().into()));
            }
            let address = bindings
                .get(key)
                .ok_or_else(|| AdmissionError::MissingBinding(key.uuid().into()))?;
            if address.generation != *row.provenance() || address.model.trim().is_empty() {
                return Err(AdmissionError::StorageChanged);
            }
            winners.insert(key.clone(), row.provenance().clone());
        }
        Ok(Self {
            cell,
            selected: selected.clone(),
            entities,
            winners,
            bindings,
            deadline,
            invalid: Mutex::new(None),
        })
    }

    pub(crate) fn invalidate(&self, reason: AdmissionError) -> AdmissionError {
        self.invalid
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_or_insert(reason)
            .clone()
    }
    pub(crate) fn check_at(&self, now: Instant) -> Result<()> {
        let mut invalid = self.invalid.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(error) = &*invalid {
            return Err(error.clone());
        }
        // Evaluating the effective deadline is essential after the last DB
        // read: no subsequent ConnDInstantane::avant may have latched expiry.
        if now >= self.deadline {
            *invalid = Some(AdmissionError::Expired);
            return Err(AdmissionError::Expired);
        }
        Ok(())
    }
    pub(crate) fn check(&self) -> Result<()> {
        self.check_at(Instant::now())
    }
    pub(crate) fn selected(&self) -> &LayerGeneration {
        &self.selected
    }
    pub(crate) fn finish_at(&self, now: Instant) -> Result<()> {
        self.check_at(now)
    }
    pub(crate) fn require_filter(&self, capable: bool) -> Result<()> {
        self.check()?;
        if !capable {
            return Err(self.invalidate(AdmissionError::IncapableFilter));
        }
        Ok(())
    }
    pub(crate) fn require_physical_tool_access(&self) -> Result<()> {
        self.check()?;
        Err(self.invalidate(AdmissionError::FixtureOnly))
    }
    pub(crate) fn check_cell(&self, cell: &crate::scope::Scope) -> Result<()> {
        self.check()?;
        if cell != &self.cell {
            return Err(self.invalidate(AdmissionError::ForeignCell));
        }
        Ok(())
    }
    pub(crate) fn mask(
        &self,
        entity: &str,
        admitted: Option<&HashSet<String>>,
    ) -> Result<HashSet<String>> {
        self.check()?;
        if !self.entities.contains(entity) {
            return Err(self.invalidate(AdmissionError::MissingBinding(entity.into())));
        }
        Ok(self
            .winners
            .keys()
            .filter(|k| {
                k.entity() == entity
                    && self.bindings.get(*k).is_some_and(|b| b.cell == self.cell)
                    && admitted.is_none_or(|ids| ids.contains(k.uuid()))
            })
            .map(|k| k.uuid().to_string())
            .collect())
    }
    pub(crate) fn admits(&self, entity: &str, uuid: &str) -> Result<bool> {
        self.check()?;
        if !self.entities.contains(entity) {
            return Err(self.invalidate(AdmissionError::MissingBinding(entity.into())));
        }
        let key = OccurrenceKey::new(entity, uuid).map_err(AdmissionError::Layer)?;
        Ok(self.winners.contains_key(&key)
            && self.bindings.get(&key).is_some_and(|b| b.cell == self.cell))
    }
    pub(crate) fn assert_results<'a>(
        &self,
        entity: &str,
        uuids: impl IntoIterator<Item = &'a str>,
    ) -> Result<()> {
        for uuid in uuids {
            if !self.admits(entity, uuid)? {
                let key = OccurrenceKey::new(entity, uuid).map_err(AdmissionError::Layer)?;
                return Err(self.invalidate(AdmissionError::LeakedOccurrence(key)));
            }
        }
        self.check()
    }
    pub(crate) fn provenance(&self, key: &OccurrenceKey) -> Result<&LayerGeneration> {
        self.check()?;
        if !self.admits(key.entity(), key.uuid())? {
            return Err(self.invalidate(AdmissionError::LeakedOccurrence(key.clone())));
        }
        self.winners
            .get(key)
            .ok_or_else(|| self.invalidate(AdmissionError::LeakedOccurrence(key.clone())))
    }
    pub(crate) fn provenances(
        &self,
        keys: impl IntoIterator<Item = OccurrenceKey>,
        requested: bool,
    ) -> Result<Option<BTreeMap<OccurrenceKey, LayerGeneration>>> {
        self.check()?;
        if !requested {
            return Ok(None);
        }
        let mut values = BTreeMap::new();
        for key in keys {
            values.insert(key.clone(), self.provenance(&key)?.clone());
        }
        self.check()?;
        Ok(Some(values))
    }
    pub(crate) fn validate_storage(
        &self,
        key: &OccurrenceKey,
        generation: &LayerGeneration,
        model: &str,
        offset: u64,
    ) -> Result<()> {
        self.check()?;
        if !self.admits(key.entity(), key.uuid())? {
            return Err(self.invalidate(AdmissionError::StorageChanged));
        }
        let Some(address) = self.bindings.get(key) else {
            return Err(self.invalidate(AdmissionError::StorageChanged));
        };
        if address.generation != *generation || address.model != model || address.offset != offset {
            return Err(self.invalidate(AdmissionError::StorageChanged));
        }
        Ok(())
    }
    /// Source identities in untyped relation queries must retain their entity.
    /// UUID alone cannot disambiguate occurrences from different entity tables.
    pub(crate) fn admit_sources(
        &self,
        uuids: &[String],
        sources: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        self.check()?;
        let mut result = Vec::new();
        for uuid in uuids {
            let entity = sources
                .get(uuid)
                .ok_or_else(|| self.invalidate(AdmissionError::MissingSourceIdentity))?;
            if self.admits(entity, uuid)? {
                result.push(uuid.clone());
            }
        }
        Ok(result)
    }
}

/// The same Arc must travel through direct node services and ReadView copies.
/// Two different selections cannot be silently resolved by service precedence.
pub(crate) fn for_node(ctx: &crate::dataflow::NodeContext) -> Result<Option<Arc<ReadAdmission>>> {
    let direct = ctx.service::<Arc<ReadAdmission>>(SERVICE).cloned();
    let view = ctx
        .service::<Arc<crate::read_view::ReadViewCell>>(crate::read_view::READ_VIEW_SERVICE)
        .and_then(|cell| cell.current());
    let from_view = view.as_ref().and_then(|view| view.admission().cloned());
    if let (Some(view), Some(a)) = (&view, &from_view) {
        a.check_cell(view.scope())?;
    }
    if let (Some(a), Some(b)) = (&direct, &from_view) {
        if !Arc::ptr_eq(a, b) {
            let error = a.invalidate(AdmissionError::ConflictingContexts);
            b.invalidate(error.clone());
            return Err(error);
        }
    }
    let selected = direct.or(from_view);
    if let Some(a) = &selected {
        a.check()?;
        if let Some(cell) = ctx.service::<crate::scope::Scope>("scope") {
            a.check_cell(cell)?;
        } else if let Some(view) = &view {
            a.check_cell(view.scope())?;
        } else {
            return Err(a.invalidate(AdmissionError::MissingCell));
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests;
