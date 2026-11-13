//! Publication of complete DECLARATION snapshots on explicitly new owned catalogs.
//! Revisions identify publication events, not content and not an applied index format.
pub mod layers;

use super::facts::{self, DeclaredSchemaFacts, DeclaredSchemaInput, FactsError};
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "rag3db-native", test))]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "rag3db-native")]
mod native;
#[cfg(feature = "rag3db-native")]
pub use native::OwnedPublicationCatalog;

const STATE_DOMAIN: &str = "declared-publication-state:v1:";
const INTENT_DOMAIN: &str = "declared-publication-intent:v1:";
const PROTOCOL: u32 = 1;

#[derive(Debug)]
pub enum PublicationError {
    Facts(FactsError),
    Invalid(String),
    Corrupt(String),
    UnknownContract,
    ExistingPath,
    NotOwned,
    OriginMismatch,
    PublicationMoved,
    OperationConflict,
    NeedsReopen,
    Storage(String),
}
impl std::fmt::Display for PublicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PublicationError {}
impl From<FactsError> for PublicationError {
    fn from(e: FactsError) -> Self {
        Self::Facts(e)
    }
}
type Result<T> = std::result::Result<T, PublicationError>;

fn fresh_id(kind: &str) -> String {
    format!("{kind}:{}", crate::refs::generate_temp_uuid())
}
fn validate_id(value: &str, kind: &str) -> Result<()> {
    let uuid = value
        .strip_prefix(&format!("{kind}:"))
        .ok_or_else(|| PublicationError::Corrupt(format!("invalid {kind} identity")))?;
    if uuid.len() != 36
        || !uuid.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
    {
        return Err(PublicationError::Corrupt(format!(
            "invalid {kind} identity"
        )));
    }
    Ok(())
}
fn operation(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(PublicationError::Invalid("empty operation identity".into()));
    }
    Ok(())
}

/// A one-use creation seed. Save `identifier()` before creation to resolve a lost
/// acknowledgement. No Clone/Deserialize: an expected existing ID is not a new seed.
#[derive(Debug)]
pub struct NewPublicationOrigin {
    identifier: String,
}
impl NewPublicationOrigin {
    pub fn fresh() -> Self {
        Self {
            identifier: fresh_id("declaration-origin-v1"),
        }
    }
    pub fn identifier(&self) -> &str {
        &self.identifier
    }
}

/// Complete supplied declarations validated by A; neither physical data nor an
/// observed model/schema certificate. Its fingerprint is computed, never supplied.
#[derive(Debug, Clone, Serialize)]
pub struct DeclarationSnapshot {
    input: DeclaredSchemaInput,
    facts: DeclaredSchemaFacts,
    fingerprint: String,
}
impl DeclarationSnapshot {
    pub fn from_complete_input(input: DeclaredSchemaInput) -> Result<Self> {
        let facts = DeclaredSchemaFacts::from_complete_input(input.clone())?;
        let encoded = canonical(&input)?;
        let decoded: DeclaredSchemaInput = serde_json::from_str(&encoded)
            .map_err(|e| PublicationError::Invalid(format!("declaration codec: {e}")))?;
        // ModelSource has typed Eq, including the exact Addresses vector. A
        // codec normalization must not be hidden by a representation alias.
        if input.recipe.model_sources != decoded.recipe.model_sources
            || encoded != canonical(&decoded)?
        {
            return Err(PublicationError::Invalid(
                "declarations do not round-trip through the v1 codec".into(),
            ));
        }
        let fingerprint = facts::digest(STATE_DOMAIN, &facts)?;
        Ok(Self {
            input,
            facts,
            fingerprint,
        })
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    #[cfg(feature = "rag3db-native")]
    pub(super) fn complete_input(&self) -> &DeclaredSchemaInput { &self.input }
    pub fn supplied_facts(&self) -> &DeclaredSchemaFacts {
        &self.facts
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum SourceRevision {
    Genesis,
    Publication {
        publication: String,
        fingerprint: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntentBody {
    version: u32,
    origin: String,
    operation: String,
    source: SourceRevision,
    target_publication: String,
    target: DeclaredSchemaInput,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredIntent {
    body: IntentBody,
    identity: String,
}

/// An immutable publication intent, distinct from a Mermaid migration/execution.
/// Encode/decode use a strict canonical v1 wire contract; no public Deserialize.
#[derive(Debug, Clone)]
pub struct PublicationIntent {
    stored: StoredIntent,
    target: DeclarationSnapshot,
}
impl PublicationIntent {
    fn build(body: IntentBody) -> Result<Self> {
        if body.version != PROTOCOL {
            return Err(PublicationError::UnknownContract);
        }
        validate_id(&body.origin, "declaration-origin-v1")?;
        validate_id(&body.target_publication, "declaration-publication-v1")?;
        operation(&body.operation)?;
        if let SourceRevision::Publication {
            publication,
            fingerprint,
        } = &body.source
        {
            validate_id(publication, "declaration-publication-v1")?;
            if !fingerprint.starts_with(STATE_DOMAIN) {
                return Err(PublicationError::UnknownContract);
            }
            if publication == &body.target_publication {
                return Err(PublicationError::Invalid(
                    "source equals target publication".into(),
                ));
            }
        }
        let target = DeclarationSnapshot::from_complete_input(body.target.clone())?;
        let identity = facts::digest(INTENT_DOMAIN, &body)?;
        Ok(Self {
            stored: StoredIntent { body, identity },
            target,
        })
    }
    fn restore(stored: StoredIntent) -> Result<Self> {
        let rebuilt = Self::build(stored.body)?;
        if rebuilt.identity() != stored.identity {
            return Err(PublicationError::Corrupt("intent identity mismatch".into()));
        }
        Ok(rebuilt)
    }
    pub fn identity(&self) -> &str {
        &self.stored.identity
    }
    pub fn operation(&self) -> &str {
        &self.stored.body.operation
    }
    pub fn target_publication(&self) -> &str {
        &self.stored.body.target_publication
    }
    pub fn target(&self) -> &DeclarationSnapshot {
        &self.target
    }
    pub fn encode(&self) -> Result<String> {
        canonical(&self.stored)
    }
    pub fn decode(encoded: &str) -> Result<Self> {
        let stored: StoredIntent =
            serde_json::from_str(encoded).map_err(|e| PublicationError::Corrupt(e.to_string()))?;
        if canonical(&stored)? != encoded {
            return Err(PublicationError::Corrupt(
                "noncanonical intent encoding".into(),
            ));
        }
        Self::restore(stored)
    }
}
fn canonical<T: Serialize>(value: &T) -> Result<String> {
    facts::canonical_json(value).map_err(PublicationError::Invalid)
}

/// A committed value read from the dedicated registry. This does not establish
/// a storage generation for an index, grant a writer lease, or qualify a migration.
#[derive(Debug, Clone, Serialize)]
pub struct PublishedDeclaration {
    origin: String,
    publication: String,
    operation: String,
    intent_identity: String,
    snapshot: DeclarationSnapshot,
}
impl PublishedDeclaration {
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn publication(&self) -> &str {
        &self.publication
    }
    pub fn operation(&self) -> &str {
        &self.operation
    }
    pub fn intent_identity(&self) -> &str {
        &self.intent_identity
    }
    pub fn snapshot(&self) -> &DeclarationSnapshot {
        &self.snapshot
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PublicationDisposition {
    Committed,
    AlreadyCommittedAtHead,
    AlreadyCommittedHistorical,
}
#[derive(Debug, Clone, Serialize)]
pub struct PublicationOutcome {
    disposition: PublicationDisposition,
    receipt: PublishedDeclaration,
    current: PublishedDeclaration,
}
impl PublicationOutcome {
    pub fn disposition(&self) -> PublicationDisposition {
        self.disposition
    }
    pub fn receipt(&self) -> &PublishedDeclaration {
        &self.receipt
    }
    pub fn current(&self) -> &PublishedDeclaration {
        &self.current
    }
}

#[cfg(any(feature = "rag3db-native", test))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Registry {
    version: u32,
    origin: String,
    head: String,
    publications: BTreeMap<String, StoredIntent>,
}
#[cfg(any(feature = "rag3db-native", test))]
impl Registry {
    #[cfg(feature = "rag3db-native")]
    pub(super) fn origin(&self) -> &str { &self.origin }
    #[cfg(feature = "rag3db-native")]
    pub(super) fn head(&self) -> &str { &self.head }
    #[cfg(feature = "rag3db-native")]
    pub(super) fn is_genesis_publication(&self, id: &str) -> bool {
        self.publications.get(id).is_some_and(|intent| matches!(intent.body.source, SourceRevision::Genesis))
    }
    #[cfg(feature = "rag3db-native")]
    pub(super) fn is_unique_genesis_head(&self) -> bool {
        self.publications.len() == 1 && self.is_genesis_publication(&self.head)
    }

    pub(super) fn genesis(
        origin: NewPublicationOrigin,
        op: &str,
        target: DeclarationSnapshot,
    ) -> Result<Self> {
        let intent = PublicationIntent::build(IntentBody {
            version: PROTOCOL,
            origin: origin.identifier,
            operation: op.into(),
            source: SourceRevision::Genesis,
            target_publication: fresh_id("declaration-publication-v1"),
            target: target.input,
        })?;
        let id = intent.target_publication().to_owned();
        Ok(Self {
            version: PROTOCOL,
            origin: intent.stored.body.origin.clone(),
            head: id.clone(),
            publications: BTreeMap::from([(id, intent.stored)]),
        })
    }
    pub(super) fn decode(encoded: &str) -> Result<Self> {
        let record: Self =
            serde_json::from_str(encoded).map_err(|e| PublicationError::Corrupt(e.to_string()))?;
        // The writer emits exactly this canonical wire form. This also refuses
        // hidden/duplicate JSON fields or silently ignored nested input fields.
        if canonical(&record)? != encoded {
            return Err(PublicationError::Corrupt(
                "noncanonical registry encoding".into(),
            ));
        }
        record.validate()?;
        Ok(record)
    }
    pub(super) fn validate(&self) -> Result<()> {
        if self.version != PROTOCOL {
            return Err(PublicationError::UnknownContract);
        }
        validate_id(&self.origin, "declaration-origin-v1")?;
        if self.publications.is_empty() || !self.publications.contains_key(&self.head) {
            return Err(PublicationError::Corrupt(
                "missing current publication".into(),
            ));
        }
        let mut operations = BTreeSet::new();
        let mut intents = BTreeMap::new();
        for (id, stored) in &self.publications {
            let intent = PublicationIntent::restore(stored.clone())?;
            if id != intent.target_publication()
                || intent.stored.body.origin != self.origin
                || !operations.insert(intent.operation().to_owned())
            {
                return Err(PublicationError::Corrupt(
                    "publication or operation identity mismatch".into(),
                ));
            }
            intents.insert(id.clone(), intent);
        }
        let mut seen = BTreeSet::new();
        let mut cursor = self.head.as_str();
        loop {
            if !seen.insert(cursor) {
                return Err(PublicationError::Corrupt("publication cycle".into()));
            }
            let intent = intents
                .get(cursor)
                .ok_or_else(|| PublicationError::Corrupt("missing parent publication".into()))?;
            match &intent.stored.body.source {
                SourceRevision::Genesis => break,
                SourceRevision::Publication {
                    publication,
                    fingerprint,
                } => {
                    let parent = intents.get(publication).ok_or_else(|| {
                        PublicationError::Corrupt("missing parent publication".into())
                    })?;
                    if parent.target.fingerprint() != fingerprint {
                        return Err(PublicationError::Corrupt(
                            "source fingerprint mismatch".into(),
                        ));
                    }
                    cursor = publication;
                }
            }
        }
        if seen.len() != self.publications.len() {
            return Err(PublicationError::Corrupt("unreachable publication".into()));
        }
        Ok(())
    }
    pub(super) fn published(&self, id: &str) -> Result<PublishedDeclaration> {
        let intent = PublicationIntent::restore(
            self.publications
                .get(id)
                .ok_or_else(|| PublicationError::Corrupt("publication absent".into()))?
                .clone(),
        )?;
        Ok(PublishedDeclaration {
            origin: self.origin.clone(),
            publication: id.into(),
            operation: intent.operation().into(),
            intent_identity: intent.identity().into(),
            snapshot: intent.target,
        })
    }
    fn known_operation(&self, op: &str) -> Result<Option<PublicationIntent>> {
        self.publications
            .values()
            .find(|i| i.body.operation == op)
            .cloned()
            .map(PublicationIntent::restore)
            .transpose()
    }
    pub(super) fn prepare(&self, op: &str, target: DeclarationSnapshot) -> Result<PublicationIntent> {
        operation(op)?;
        if let Some(known) = self.known_operation(op)? {
            if canonical(&known.target.facts)? != canonical(&target.facts)? {
                return Err(PublicationError::OperationConflict);
            }
            return Ok(known);
        }
        let current = self.published(&self.head)?;
        PublicationIntent::build(IntentBody {
            version: PROTOCOL,
            origin: self.origin.clone(),
            operation: op.into(),
            source: SourceRevision::Publication {
                publication: self.head.clone(),
                fingerprint: current.snapshot.fingerprint().into(),
            },
            target_publication: fresh_id("declaration-publication-v1"),
            target: target.input,
        })
    }
    pub(super) fn apply(&mut self, intent: &PublicationIntent) -> Result<PublicationOutcome> {
        if intent.stored.body.origin != self.origin {
            return Err(PublicationError::OriginMismatch);
        }
        if let Some(known) = self.known_operation(intent.operation())? {
            if known.encode()? != intent.encode()? {
                return Err(PublicationError::OperationConflict);
            }
            let receipt = self.published(known.target_publication())?;
            let current = self.published(&self.head)?;
            return Ok(PublicationOutcome {
                disposition: if receipt.publication == current.publication {
                    PublicationDisposition::AlreadyCommittedAtHead
                } else {
                    PublicationDisposition::AlreadyCommittedHistorical
                },
                receipt,
                current,
            });
        }
        let current = self.published(&self.head)?;
        let expected = SourceRevision::Publication {
            publication: self.head.clone(),
            fingerprint: current.snapshot.fingerprint().into(),
        };
        if intent.stored.body.source != expected {
            return Err(PublicationError::PublicationMoved);
        }
        if self.publications.contains_key(intent.target_publication()) {
            return Err(PublicationError::Corrupt(
                "target publication already exists".into(),
            ));
        }
        self.publications
            .insert(intent.target_publication().into(), intent.stored.clone());
        self.head = intent.target_publication().into();
        self.validate()?;
        let published = self.published(&self.head)?;
        Ok(PublicationOutcome {
            disposition: PublicationDisposition::Committed,
            receipt: published.clone(),
            current: published,
        })
    }
}

#[cfg(test)]
mod tests;
