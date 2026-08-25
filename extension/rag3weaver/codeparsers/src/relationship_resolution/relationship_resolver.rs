use crate::cached_regex;
use crate::import_resolution::types::BaseImportResolver;
use crate::parallel::parser_worker::SupportedLanguage;
use crate::relationship_resolution::types::EnrichedFileAnalysis;
use crate::relationship_resolution::types::SupportedLanguage as ResSupportedLanguage;
use crate::relationship_resolution::types::EnrichedScope;
use crate::relationship_resolution::types::ExternalLibraryInfo;
use crate::relationship_resolution::types::FileInfo;
use crate::relationship_resolution::types::GlobalScopeMapping;
use crate::relationship_resolution::types::ParsedFilesMap;
use crate::relationship_resolution::types::RelationshipResolutionResult;
use crate::relationship_resolution::types::RelationshipResolverOptions;
use crate::relationship_resolution::types::RelationshipType;
use crate::relationship_resolution::types::RelationshipMetadata;
use crate::relationship_resolution::types::ResolutionStats;
use crate::relationship_resolution::types::ResolvedRelationship;
use crate::relationship_resolution::types::ScopeMappingEntry;
use crate::relationship_resolution::types::UnresolvedReference;
use crate::relationship_resolution::types::UuidToScopeMapping;
use crate::scope_extraction::types::IdentifierReferenceKind;
use crate::scope_extraction::types::ScopeFileAnalysis;
use crate::scope_extraction::types::ScopeInfo;
use crate::scope_extraction::types::ScopeInfoType;

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

lazy_static::lazy_static! {
    pub static ref EXTENSION_TO_LANGUAGE: HashMap<&'static str, SupportedLanguage> = {
        let mut m = HashMap::new();
        m.insert(".ts", SupportedLanguage::Typescript);
        m.insert(".tsx", SupportedLanguage::Typescript);
        m.insert(".js", SupportedLanguage::Javascript);
        m.insert(".jsx", SupportedLanguage::Javascript);
        m.insert(".mjs", SupportedLanguage::Javascript);
        m.insert(".cjs", SupportedLanguage::Javascript);
        m.insert(".py", SupportedLanguage::Python);
        m.insert(".rs", SupportedLanguage::Rust);
        m.insert(".go", SupportedLanguage::Go);
        m.insert(".c", SupportedLanguage::C);
        m.insert(".h", SupportedLanguage::C);
        m.insert(".cpp", SupportedLanguage::Cpp);
        m.insert(".cc", SupportedLanguage::Cpp);
        m.insert(".cxx", SupportedLanguage::Cpp);
        m.insert(".hpp", SupportedLanguage::Cpp);
        m.insert(".hxx", SupportedLanguage::Cpp);
        m.insert(".cs", SupportedLanguage::Csharp);
        m
    };
}

use crate::utils::hash::blake3_uuid;

/// Value types for candidate prioritization.
const VALUE_TYPES: &[&str] = &[
    "class", "interface", "struct", "trait", "enum",
    "function", "constant", "method", "variable", "namespace",
];

fn is_value_type(t: &str) -> bool {
    VALUE_TYPES.contains(&t)
}

fn scope_type_str(t: &ScopeInfoType) -> &'static str {
    match t {
        ScopeInfoType::Class => "class",
        ScopeInfoType::Interface => "interface",
        ScopeInfoType::Function => "function",
        ScopeInfoType::Method => "method",
        ScopeInfoType::Enum => "enum",
        ScopeInfoType::TypeAlias => "type_alias",
        ScopeInfoType::Namespace => "namespace",
        ScopeInfoType::Module => "module",
        ScopeInfoType::Variable => "variable",
        ScopeInfoType::Lambda => "lambda",
        ScopeInfoType::Constant => "constant",
        ScopeInfoType::Block => "block",
    }
}

#[derive(Default)]
pub struct RelationshipResolver {
    options: RelationshipResolverOptions,
    import_resolvers: HashMap<SupportedLanguage, Box<dyn BaseImportResolver>>,
    scope_mapping: GlobalScopeMapping,
    uuid_mapping: UuidToScopeMapping,
    files_map: HashMap<String, FileInfo>,
    external_libraries_map: HashMap<String, ExternalLibraryInfo>,
}

impl RelationshipResolver {
    pub fn new(options: RelationshipResolverOptions) -> Self {
        Self {
            options,
            import_resolvers: HashMap::new(),
            scope_mapping: GlobalScopeMapping::default(),
            uuid_mapping: UuidToScopeMapping::default(),
            files_map: HashMap::new(),
            external_libraries_map: HashMap::new(),
        }
    }

    /// Initialize import resolvers for detected languages.
    /// Currently a no-op — resolvers are not yet implemented in Rust.
    /// Cross-file resolution uses fallback (global scope mapping) instead.
    fn initialize_resolvers(&mut self, _languages: HashSet<SupportedLanguage>) {
        // TODO: When language-specific import resolvers are implemented,
        // instantiate them here based on the detected languages.
        // For now, cross-file resolution works via fallback (global scope mapping).
    }

    /// Detect language from file extension.
    fn detect_language(&self, file_path: &str) -> SupportedLanguage {
        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_lowercase()));

        let from_ext = ext.as_deref()
            .and_then(|e| EXTENSION_TO_LANGUAGE.get(e).cloned());

        if let Some(lang) = from_ext {
            return lang;
        }

        // Map from options.default_language (relationship_resolution::types::SupportedLanguage)
        // to parallel::parser_worker::SupportedLanguage
        match &self.options.default_language {
            Some(ResSupportedLanguage::Typescript) => SupportedLanguage::Typescript,
            Some(ResSupportedLanguage::Javascript) => SupportedLanguage::Javascript,
            Some(ResSupportedLanguage::Python) => SupportedLanguage::Python,
            Some(ResSupportedLanguage::Rust) => SupportedLanguage::Rust,
            Some(ResSupportedLanguage::Go) => SupportedLanguage::Go,
            Some(ResSupportedLanguage::C) => SupportedLanguage::C,
            Some(ResSupportedLanguage::Cpp) => SupportedLanguage::Cpp,
            Some(ResSupportedLanguage::Csharp) => SupportedLanguage::Csharp,
            None => SupportedLanguage::Typescript,
        }
    }

    /// Get import resolver for a file.
    fn get_resolver(&self, file_path: &str) -> Option<&dyn BaseImportResolver> {
        let lang = self.detect_language(file_path);
        self.import_resolvers.get(&lang).map(|r| r.as_ref())
    }

    /// Main entry point: resolve all relationships from parsed files.
    pub fn resolve_relationships(&mut self, parsed_files: &ParsedFilesMap) -> RelationshipResolutionResult {
        let start = Instant::now();

        // Detect languages and initialize resolvers
        let mut languages = HashSet::new();
        for file_path in parsed_files.keys() {
            languages.insert(self.detect_language(file_path));
        }
        self.initialize_resolvers(languages);

        // Build global scope mapping
        self.build_global_scope_mapping(parsed_files);

        // Un FileInfo par fichier parsé — c'est ce dont un consommateur a
        // besoin pour créer les entités File (la map restait vide, 25 août 2026).
        self.files_map.clear();
        for file_path in parsed_files.keys() {
            let relative_path = self.get_relative_path(file_path);
            let absolute_path = if Path::new(file_path).is_absolute() {
                file_path.clone()
            } else {
                Path::new(&self.options.project_root).join(file_path).to_string_lossy().to_string()
            };
            self.files_map.insert(relative_path.clone(), FileInfo {
                uuid: self.generate_file_uuid(&relative_path),
                path: relative_path,
                absolute_path,
            });
        }

        // Resolve relationships
        let mut relationships: Vec<ResolvedRelationship> = Vec::new();
        let mut unresolved_references: Vec<UnresolvedReference> = Vec::new();

        for (file_path, analysis) in parsed_files {
            for scope in &analysis.scopes {
                // 1. Resolve local scope references (same file)
                let local_refs = self.resolve_local_scope_references(scope, file_path);
                relationships.extend(local_refs);

                // 2. Resolve import references (cross-file)
                if self.options.resolve_cross_file.unwrap_or(true) {
                    let import_result = self.resolve_import_references(scope, file_path, Some(analysis));
                    relationships.extend(import_result.0);
                    unresolved_references.extend(import_result.1);

                    // 2b. Fallback: resolve unknown references
                    let unknown_refs = self.resolve_unknown_references(scope, file_path, Some(analysis));
                    relationships.extend(unknown_refs);
                }

                // 3. Resolve PARENT_OF relationships
                if self.options.include_contains.unwrap_or(true) {
                    if scope.parent.is_some() {
                        if let Some(contains_ref) = self.resolve_contains_relation(scope, file_path) {
                            relationships.push(contains_ref);
                        }
                    }
                }

                // 3b. Resolve heritage clauses (extends/implements) directly
                let heritage_refs = self.resolve_heritage_relations(scope, file_path);
                relationships.extend(heritage_refs);

                // 4. Resolve DECORATES relationships
                if self.options.include_decorators.unwrap_or(true) {
                    let has_decorators = scope.decorators.as_ref().map_or(false, |d| !d.is_empty())
                        || scope.decorator_details.as_ref().map_or(false, |d| !d.is_empty());
                    if has_decorators {
                        let decorator_refs = self.resolve_decorator_relations(scope, file_path);
                        relationships.extend(decorator_refs);
                    }
                }

                // 5. Resolve DEFINED_IN relationship
                if self.options.include_defined_in.unwrap_or(true) {
                    if let Some(defined_in_ref) = self.resolve_defined_in_relation(scope, file_path) {
                        relationships.push(defined_in_ref);
                    }
                }

                // 6. Resolve USES_LIBRARY relationships
                if self.options.include_uses_library.unwrap_or(true) {
                    let library_refs = self.resolve_uses_library_relations(scope, file_path);
                    relationships.extend(library_refs);
                }
            }
        }

        // Une ExternalLibraryInfo par librairie vue dans USES_LIBRARY, avec les
        // symboles importés (même remarque : la map restait vide).
        self.external_libraries_map.clear();
        for rel in relationships.iter().filter(|r| r.r#type == RelationshipType::USESLIBRARY) {
            let entry = self.external_libraries_map
                .entry(rel.to_name.clone())
                .or_insert_with(|| ExternalLibraryInfo {
                    uuid: rel.to_uuid.clone(),
                    name: rel.to_name.clone(),
                    symbols: Vec::new(),
                });
            if let Some(sym) = rel.metadata.as_ref().and_then(|m| m.symbol.clone()) {
                if !sym.is_empty() && !entry.symbols.contains(&sym) {
                    entry.symbols.push(sym);
                }
            }
        }

        // Generate inverse relationships
        if self.options.include_inverse.unwrap_or(true) {
            let inverse_rels = self.generate_inverse_relationships(&relationships);
            relationships.extend(inverse_rels);
        }

        // Calculate stats
        let time_ms = start.elapsed().as_millis() as f64;
        let stats = self.calculate_stats(&relationships, &unresolved_references, parsed_files.len(), time_ms);

        RelationshipResolutionResult {
            relationships,
            scope_mapping: self.scope_mapping.clone(),
            uuid_mapping: self.uuid_mapping.clone(),
            files: self.files_map.clone(),
            external_libraries: self.external_libraries_map.clone(),
            stats,
            unresolved_references,
        }
    }

    /// Build global scope mapping: name → [{uuid, file, type, ...}]
    fn build_global_scope_mapping(&mut self, parsed_files: &ParsedFilesMap) {
        self.scope_mapping.clear();
        self.uuid_mapping.clear();

        for (file_path, analysis) in parsed_files {
            let relative_path = self.get_relative_path(file_path);

            for scope in &analysis.scopes {
                let uuid = self.generate_uuid(scope, &relative_path);

                let entry = ScopeMappingEntry {
                    uuid: uuid.clone(),
                    file: relative_path.clone(),
                    r#type: scope_type_str(&scope.r#type).to_string(),
                    name: scope.name.clone(),
                    signature: Some(scope.signature.clone()),
                    parent: scope.parent.clone(),
                    start_line: scope.scope_start_line,
                    end_line: scope.scope_end_line,
                };

                // Add to name mapping
                self.scope_mapping
                    .entry(scope.name.clone())
                    .or_default()
                    .push(entry.clone());

                // Add to UUID mapping
                self.uuid_mapping.insert(uuid, entry);
            }
        }
    }

    /// Resolve local scope references (same file).
    fn resolve_local_scope_references(&self, scope: &ScopeInfo, file_path: &str) -> Vec<ResolvedRelationship> {
        let mut relationships = Vec::new();
        let relative_path = self.get_relative_path(file_path);
        let source_uuid = self.generate_uuid(scope, &relative_path);

        for r in &scope.identifier_references {
            // Only process local_scope references
            let is_local = r.kind.as_ref().map_or(false, |k| *k == IdentifierReferenceKind::LocalScope);
            if !is_local {
                continue;
            }

            let candidates = self.scope_mapping.get(&r.identifier).cloned().unwrap_or_default();
            // Filter to same file
            let matched = candidates.iter().find(|c| c.file == relative_path);

            if let Some(target) = matched {
                // Skip if target is a child of the source scope
                if target.parent.as_deref() == Some(&scope.name) {
                    continue;
                }

                let rel_type = self.detect_relationship_type(scope, target, r.context.as_deref());

                relationships.push(ResolvedRelationship {
                    r#type: rel_type,
                    from_uuid: source_uuid.clone(),
                    to_uuid: target.uuid.clone(),
                    from_file: relative_path.clone(),
                    to_file: target.file.clone(),
                    from_name: scope.name.clone(),
                    to_name: target.name.clone(),
                    from_type: scope_type_str(&scope.r#type).to_string(),
                    to_type: target.r#type.clone(),
                    metadata: Some(RelationshipMetadata {
                        context: r.context.clone(),
                        via_import: Some(false),
                        ..Default::default()
                    }),
                });
            }
        }

        relationships
    }

    /// Resolve unknown references as relationships (both same-file and cross-file).
    fn resolve_unknown_references(&self, scope: &ScopeInfo, file_path: &str, file_analysis: Option<&ScopeFileAnalysis>) -> Vec<ResolvedRelationship> {
        let mut relationships = Vec::new();
        let relative_path = self.get_relative_path(file_path);

        // Collect identifier references from the scope
        let mut refs: Vec<&crate::scope_extraction::types::IdentifierReference> =
            scope.identifier_references.iter().collect();

        // For non-module scopes, also include file-level identifier references
        if let Some(analysis) = file_analysis {
            if scope.r#type != ScopeInfoType::Module && self.options.include_file_level_refs.unwrap_or(true) {
                if let Some(file_scope) = analysis.scopes.iter().find(|s| {
                    s.r#type == ScopeInfoType::Module || s.name.starts_with("file_scope")
                }) {
                    refs.extend(file_scope.identifier_references.iter());
                }
            }

            // For class scopes, also include child method/constructor references
            if scope.r#type == ScopeInfoType::Class && self.options.include_child_refs.unwrap_or(true) {
                for child in analysis.scopes.iter().filter(|s| {
                    s.parent.as_deref() == Some(&scope.name)
                        && (s.r#type == ScopeInfoType::Method || s.r#type == ScopeInfoType::Function)
                }) {
                    refs.extend(child.identifier_references.iter());
                }
            }
        }

        if refs.is_empty() {
            return relationships;
        }

        let source_uuid = self.generate_uuid(scope, &relative_path);
        let mut seen_targets: HashSet<String> = HashSet::new();

        for r in &refs {
            // Only process unknown references
            let is_unknown = r.kind.as_ref().map_or(false, |k| *k == IdentifierReferenceKind::Unknown);
            if !is_unknown {
                continue;
            }

            let candidates = match self.scope_mapping.get(&r.identifier) {
                Some(c) if !c.is_empty() => c,
                _ => continue,
            };

            // If reference has a qualifier, verify it's a known scope.
            // "this" is always an instance keyword (JS/TS/C#/C++/Java).
            // "self" is an instance keyword only in Python/Rust files.
            if let Some(ref qualifier) = r.qualifier {
                let is_instance_qual = qualifier == "this"
                    || (qualifier == "self" && (file_path.ends_with(".py") || file_path.ends_with(".rs")));
                if !is_instance_qual {
                    let qual_candidates = self.scope_mapping.get(qualifier);
                    if qual_candidates.map_or(true, |c| c.is_empty()) {
                        continue;
                    }
                }
            }

            // Don't reference ourselves
            let valid: Vec<&ScopeMappingEntry> = candidates.iter()
                .filter(|c| c.uuid != source_uuid)
                .collect();

            if valid.is_empty() {
                continue;
            }

            // Pick the best candidate
            let same_file: Vec<&ScopeMappingEntry> = valid.iter().copied().filter(|c| c.file == relative_path).collect();
            let cross_file: Vec<&ScopeMappingEntry> = valid.iter().copied().filter(|c| c.file != relative_path).collect();

            let target_entry = if !same_file.is_empty() {
                if same_file.len() == 1 {
                    Some(same_file[0])
                } else {
                    same_file.iter().find(|c| is_value_type(&c.r#type)).copied().or(Some(same_file[0]))
                }
            } else if !cross_file.is_empty() {
                if cross_file.len() == 1 {
                    Some(cross_file[0])
                } else {
                    cross_file.iter().find(|c| is_value_type(&c.r#type)).copied().or(Some(cross_file[0]))
                }
            } else {
                None
            };

            if let Some(target) = target_entry {
                if seen_targets.contains(&target.uuid) {
                    continue;
                }
                // Skip if target is a child of the source scope
                if target.parent.as_deref() == Some(&scope.name) {
                    continue;
                }
                seen_targets.insert(target.uuid.clone());

                let rel_type = self.detect_relationship_type(scope, target, r.context.as_deref());
                let is_cross_file = target.file != relative_path;

                relationships.push(ResolvedRelationship {
                    r#type: rel_type,
                    from_uuid: source_uuid.clone(),
                    to_uuid: target.uuid.clone(),
                    from_file: relative_path.clone(),
                    to_file: target.file.clone(),
                    from_name: scope.name.clone(),
                    to_name: target.name.clone(),
                    from_type: scope_type_str(&scope.r#type).to_string(),
                    to_type: target.r#type.clone(),
                    metadata: Some(RelationshipMetadata {
                        context: r.context.clone(),
                        via_import: Some(false),
                        fallback_resolution: if is_cross_file { Some(true) } else { None },
                        ..Default::default()
                    }),
                });
            }
        }

        relationships
    }

    /// Resolve import references (cross-file).
    /// Returns (resolved, unresolved).
    fn resolve_import_references(
        &self,
        scope: &ScopeInfo,
        file_path: &str,
        file_analysis: Option<&ScopeFileAnalysis>,
    ) -> (Vec<ResolvedRelationship>, Vec<UnresolvedReference>) {
        let mut resolved = Vec::new();
        let mut unresolved = Vec::new();
        let relative_path = self.get_relative_path(file_path);

        let mut import_refs: Vec<&crate::scope_extraction::types::ImportReference> =
            scope.import_references.iter().collect();
        let mut identifier_refs: Vec<&crate::scope_extraction::types::IdentifierReference> =
            scope.identifier_references.iter().collect();

        // For class scopes, also include child method/constructor references
        if let Some(analysis) = file_analysis {
            if scope.r#type == ScopeInfoType::Class && self.options.include_child_refs.unwrap_or(true) {
                for child in analysis.scopes.iter().filter(|s| {
                    s.parent.as_deref() == Some(&scope.name)
                        && (s.r#type == ScopeInfoType::Method || s.r#type == ScopeInfoType::Function)
                }) {
                    import_refs.extend(child.import_references.iter());
                    identifier_refs.extend(child.identifier_references.iter());
                }
            }
        }

        if import_refs.is_empty() || identifier_refs.is_empty() {
            return (resolved, unresolved);
        }

        let source_uuid = self.generate_uuid(scope, &relative_path);

        for imp in &import_refs {
            // Find identifier references that use this import
            let used_name = imp.alias.as_deref().unwrap_or(&imp.imported);
            let matching_refs: Vec<&&crate::scope_extraction::types::IdentifierReference> = identifier_refs.iter()
                .filter(|r| {
                    let is_import = r.kind.as_ref().map_or(false, |k| *k == IdentifierReferenceKind::Import);
                    let source_matches = r.source.as_deref() == Some(&imp.source);
                    let name_matches = r.identifier == used_name || r.qualifier.as_deref() == Some(used_name);
                    is_import && source_matches && name_matches
                })
                .collect();

            for r in &matching_refs {
                // Find target scope in global mapping
                let candidates = self.scope_mapping.get(&imp.imported).cloned().unwrap_or_default();
                let mut target_entry: Option<&ScopeMappingEntry> = None;

                // No resolver-based resolution for now — use fallback
                if !candidates.is_empty() {
                    // Filter to different file (cross-file only)
                    let cross_file: Vec<&ScopeMappingEntry> = candidates.iter()
                        .filter(|c| c.file != relative_path)
                        .collect();

                    if cross_file.len() == 1 {
                        target_entry = Some(cross_file[0]);
                    } else if cross_file.len() > 1 {
                        target_entry = cross_file.iter()
                            .find(|c| is_value_type(&c.r#type))
                            .copied()
                            .or(Some(cross_file[0]));
                    }
                }

                if let Some(target) = target_entry {
                    let rel_type = self.detect_relationship_type(scope, target, r.context.as_deref());

                    resolved.push(ResolvedRelationship {
                        r#type: rel_type,
                        from_uuid: source_uuid.clone(),
                        to_uuid: target.uuid.clone(),
                        from_file: relative_path.clone(),
                        to_file: target.file.clone(),
                        from_name: scope.name.clone(),
                        to_name: target.name.clone(),
                        from_type: scope_type_str(&scope.r#type).to_string(),
                        to_type: target.r#type.clone(),
                        metadata: Some(RelationshipMetadata {
                            context: r.context.clone(),
                            via_import: Some(true),
                            import_path: Some(imp.source.clone()),
                            ..Default::default()
                        }),
                    });
                } else {
                    unresolved.push(UnresolvedReference {
                        from_scope: scope.name.clone(),
                        from_type: scope_type_str(&scope.r#type).to_string(),
                        from_file: relative_path.clone(),
                        identifier: imp.imported.clone(),
                        kind: "import".to_string(),
                        reason: if candidates.is_empty() {
                            "No scope found with this name".to_string()
                        } else {
                            format!("Multiple candidates ({}) but no file match", candidates.len())
                        },
                        candidates: None,
                    });
                }
            }
        }

        (resolved, unresolved)
    }

    /// Resolve PARENT_OF relationship (parent -> child).
    fn resolve_contains_relation(&self, scope: &ScopeInfo, file_path: &str) -> Option<ResolvedRelationship> {
        let parent_name = scope.parent.as_ref()?;
        let relative_path = self.get_relative_path(file_path);
        let candidates = self.scope_mapping.get(parent_name)?;

        // Find parent in same file
        let parent_entry = candidates.iter().find(|c| c.file == relative_path)?;
        let child_uuid = self.generate_uuid(scope, &relative_path);

        Some(ResolvedRelationship {
            r#type: RelationshipType::PARENTOF,
            from_uuid: parent_entry.uuid.clone(),
            to_uuid: child_uuid,
            from_file: relative_path.clone(),
            to_file: relative_path.clone(),
            from_name: parent_entry.name.clone(),
            to_name: scope.name.clone(),
            from_type: parent_entry.r#type.clone(),
            to_type: scope_type_str(&scope.r#type).to_string(),
            metadata: None,
        })
    }

    /// Resolve DECORATES relationships for decorators/attributes.
    fn resolve_decorator_relations(&self, scope: &ScopeInfo, file_path: &str) -> Vec<ResolvedRelationship> {
        let mut relationships = Vec::new();
        let relative_path = self.get_relative_path(file_path);
        let target_uuid = self.generate_uuid(scope, &relative_path);

        // Collect decorator names/arguments
        let mut decorator_infos: Vec<(String, Option<String>)> = Vec::new();

        if let Some(ref details) = scope.decorator_details {
            for d in details {
                decorator_infos.push((d.name.clone(), d.arguments.clone()));
            }
        } else if let Some(ref decorators) = scope.decorators {
            for d in decorators {
                let name = d.trim_start_matches('@').split('(').next().unwrap_or(d).to_string();
                decorator_infos.push((name, None));
            }
        }

        for (name, args) in &decorator_infos {
            let decorator_name = name.trim_start_matches('@');
            let candidates = match self.scope_mapping.get(decorator_name) {
                Some(c) if !c.is_empty() => c,
                _ => continue,
            };

            // Prefer decorator in same file, then any
            let decorator_entry = candidates.iter()
                .find(|c| c.file == relative_path)
                .unwrap_or(&candidates[0]);

            relationships.push(ResolvedRelationship {
                r#type: RelationshipType::DECORATES,
                from_uuid: decorator_entry.uuid.clone(),
                to_uuid: target_uuid.clone(),
                from_file: decorator_entry.file.clone(),
                to_file: relative_path.clone(),
                from_name: decorator_entry.name.clone(),
                to_name: scope.name.clone(),
                from_type: decorator_entry.r#type.clone(),
                to_type: scope_type_str(&scope.r#type).to_string(),
                metadata: Some(RelationshipMetadata {
                    decorator_args: args.clone(),
                    ..Default::default()
                }),
            });
        }

        relationships
    }

    /// Resolve DEFINED_IN relationship (Scope -> File).
    fn resolve_defined_in_relation(&self, scope: &ScopeInfo, file_path: &str) -> Option<ResolvedRelationship> {
        let relative_path = self.get_relative_path(file_path);
        let absolute_path = if Path::new(file_path).is_absolute() {
            file_path.to_string()
        } else {
            Path::new(&self.options.project_root).join(file_path).to_string_lossy().to_string()
        };

        let file_uuid = self.generate_file_uuid(&relative_path);

        let scope_uuid = self.generate_uuid(scope, &relative_path);

        Some(ResolvedRelationship {
            r#type: RelationshipType::DEFINEDIN,
            from_uuid: scope_uuid,
            to_uuid: file_uuid,
            from_file: relative_path.clone(),
            to_file: relative_path.clone(),
            from_name: scope.name.clone(),
            to_name: relative_path,
            from_type: scope_type_str(&scope.r#type).to_string(),
            to_type: "file".to_string(),
            metadata: None,
        })
    }

    /// Resolve USES_LIBRARY relationships (Scope -> ExternalLibrary).
    fn resolve_uses_library_relations(&self, scope: &ScopeInfo, file_path: &str) -> Vec<ResolvedRelationship> {
        let mut relationships = Vec::new();
        let relative_path = self.get_relative_path(file_path);

        if scope.import_references.is_empty() {
            return relationships;
        }

        let scope_uuid = self.generate_uuid(scope, &relative_path);
        let mut seen_libraries: HashSet<String> = HashSet::new();

        for imp in &scope.import_references {
            // Skip local imports
            if imp.is_local {
                continue;
            }

            if seen_libraries.contains(&imp.source) {
                continue;
            }
            seen_libraries.insert(imp.source.clone());

            let library_uuid = self.generate_external_library_uuid(&imp.source);

            relationships.push(ResolvedRelationship {
                r#type: RelationshipType::USESLIBRARY,
                from_uuid: scope_uuid.clone(),
                to_uuid: library_uuid,
                from_file: relative_path.clone(),
                to_file: imp.source.clone(),
                from_name: scope.name.clone(),
                to_name: imp.source.clone(),
                from_type: scope_type_str(&scope.r#type).to_string(),
                to_type: "external_library".to_string(),
                metadata: Some(RelationshipMetadata {
                    symbol: Some(imp.imported.clone()),
                    import_path: Some(imp.source.clone()),
                    ..Default::default()
                }),
            });
        }

        relationships
    }

    /// Generate deterministic UUID for a file.
    fn generate_file_uuid(&self, file_path: &str) -> String {
        blake3_uuid(&format!("file:{file_path}"))
    }

    /// Generate deterministic UUID for an external library.
    fn generate_external_library_uuid(&self, library_name: &str) -> String {
        blake3_uuid(&format!("extlib:{library_name}"))
    }

    /// Detect relationship type: CONSUMES, INHERITS_FROM, or IMPLEMENTS.
    fn detect_relationship_type(&self, source: &ScopeInfo, target: &ScopeMappingEntry, context: Option<&str>) -> RelationshipType {
        // Check context for "extends" / "implements" (all languages)
        if let Some(ctx) = context {
            if ctx.contains("extends") {
                return RelationshipType::INHERITSFROM;
            }
            if ctx.contains("implements") {
                return RelationshipType::IMPLEMENTS;
            }
        }

        let file = &source.file_path;
        let sig = &source.signature;

        if !sig.is_empty() && sig.contains(&target.name) {
            // TypeScript/JavaScript/Java: class X extends Y
            if sig.contains("extends") {
                return RelationshipType::INHERITSFROM;
            }

            // TypeScript/JavaScript/Java: class X implements Y
            if sig.contains("implements") {
                return RelationshipType::IMPLEMENTS;
            }

            // Rust only: impl Trait for Struct
            if file.ends_with(".rs")
                && cached_regex!(r"impl\s+\w+\s+for").is_match(sig)
            {
                return RelationshipType::IMPLEMENTS;
            }

            // C++/C# only: class X : public Y or class X : Y
            if (file.ends_with(".cpp") || file.ends_with(".cc") || file.ends_with(".h")
                || file.ends_with(".hpp") || file.ends_with(".cs") || file.ends_with(".c"))
                && cached_regex!(r":\s*(public|private|protected)?\s*").is_match(sig)
            {
                if target.r#type == "interface" {
                    return RelationshipType::IMPLEMENTS;
                }
                return RelationshipType::INHERITSFROM;
            }

            // Python only: class X(Y):
            if file.ends_with(".py")
                && source.r#type == ScopeInfoType::Class
                && sig.contains(&format!("({}", target.name))
            {
                return RelationshipType::INHERITSFROM;
            }

            // Go only: embedding (type A struct { B })
            if file.ends_with(".go") && sig.contains("struct") {
                let content = &source.content;
                if content.contains(&format!("\t{}\n", target.name))
                    || content.contains(&format!(" {}\n", target.name))
                {
                    return RelationshipType::INHERITSFROM;
                }
            }
        }

        // Check heritage clauses if available (TypeScript/JavaScript)
        if let Some(ref clauses) = source.heritage_clauses {
            for clause in clauses {
                if clause.types.iter().any(|t| t == &target.name) {
                    return if clause.clause == crate::scope_extraction::types::HeritageClauseClause::Implements {
                        RelationshipType::IMPLEMENTS
                    } else {
                        RelationshipType::INHERITSFROM
                    };
                }
            }
        }

        // Default
        RelationshipType::CONSUMES
    }

    /// Resolve heritage clauses (extends/implements) directly as relationships.
    /// This handles cases where the parent/interface type is NOT in identifier_references
    /// but IS listed in the scope's heritage_clauses.
    fn resolve_heritage_relations(&self, scope: &ScopeInfo, file_path: &str) -> Vec<ResolvedRelationship> {
        let mut relationships = Vec::new();
        let clauses = match scope.heritage_clauses {
            Some(ref c) if !c.is_empty() => c,
            _ => return relationships,
        };

        let relative_path = self.get_relative_path(file_path);
        let source_uuid = self.generate_uuid(scope, &relative_path);

        for clause in clauses {
            let rel_type = if clause.clause == crate::scope_extraction::types::HeritageClauseClause::Implements {
                RelationshipType::IMPLEMENTS
            } else {
                RelationshipType::INHERITSFROM
            };

            for type_name in &clause.types {
                let candidates = match self.scope_mapping.get(type_name) {
                    Some(c) if !c.is_empty() => c,
                    _ => continue,
                };

                // Pick best candidate (same file preferred)
                let target = candidates.iter()
                    .find(|c| c.file == relative_path && c.uuid != source_uuid)
                    .or_else(|| candidates.iter().find(|c| c.uuid != source_uuid));

                if let Some(target) = target {
                    relationships.push(ResolvedRelationship {
                        r#type: rel_type.clone(),
                        from_uuid: source_uuid.clone(),
                        to_uuid: target.uuid.clone(),
                        from_file: relative_path.clone(),
                        to_file: target.file.clone(),
                        from_name: scope.name.clone(),
                        to_name: target.name.clone(),
                        from_type: format!("{:?}", scope.r#type),
                        to_type: target.r#type.clone(),
                        metadata: None,
                    });
                }
            }
        }

        relationships
    }

    /// Generate inverse relationships.
    fn generate_inverse_relationships(&self, relationships: &[ResolvedRelationship]) -> Vec<ResolvedRelationship> {
        let mut inverse_rels = Vec::new();

        for rel in relationships {
            let inverse_type = match rel.r#type {
                RelationshipType::CONSUMES => Some(RelationshipType::CONSUMEDBY),
                RelationshipType::PARENTOF => Some(RelationshipType::HASPARENT),
                RelationshipType::DECORATES => Some(RelationshipType::DECORATEDBY),
                _ => None,
            };

            if let Some(inv_type) = inverse_type {
                inverse_rels.push(ResolvedRelationship {
                    r#type: inv_type,
                    from_uuid: rel.to_uuid.clone(),
                    to_uuid: rel.from_uuid.clone(),
                    from_file: rel.to_file.clone(),
                    to_file: rel.from_file.clone(),
                    from_name: rel.to_name.clone(),
                    to_name: rel.from_name.clone(),
                    from_type: rel.to_type.clone(),
                    to_type: rel.from_type.clone(),
                    metadata: rel.metadata.clone(),
                });
            }
        }

        inverse_rels
    }

    /// Generate deterministic UUID for a scope.
    /// Uses blake3 (same as rag3weaver) for consistency.
    fn generate_uuid(&self, scope: &ScopeInfo, file_path: &str) -> String {
        let signature_hash = self.get_signature_hash(scope);
        let type_str = scope_type_str(&scope.r#type);
        let input = format!("{file_path}:{name}:{type_str}:{signature_hash}", name = scope.name);
        blake3_uuid(&input)
    }

    /// Get signature hash for UUID stability.
    /// Same scope = same hash even if line numbers change.
    fn get_signature_hash(&self, scope: &ScopeInfo) -> String {
        let parent_prefix = scope.parent.as_deref().map(|p| format!("{p}.")).unwrap_or_default();

        let base_input = if !scope.signature.is_empty() {
            scope.signature.clone()
        } else {
            let type_str = scope_type_str(&scope.r#type);
            let content = if !scope.content_dedented.is_empty() {
                &scope.content_dedented
            } else {
                &scope.content
            };
            format!("{}:{}:{}", scope.name, type_str, content)
        };

        let mut hash_input = format!("{parent_prefix}{base_input}");

        // For variables/constants: include line number to differentiate same-name vars
        if scope.r#type == ScopeInfoType::Variable || scope.r#type == ScopeInfoType::Constant {
            hash_input.push_str(&format!(":line{}", scope.scope_start_line));
        }

        let hash = blake3::hash(hash_input.as_bytes()).to_hex().to_string();
        hash[..8].to_string()
    }

    /// Get relative path from project root.
    fn get_relative_path(&self, file_path: &str) -> String {
        if Path::new(file_path).is_absolute() {
            pathdiff::diff_paths(file_path, &self.options.project_root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| file_path.to_string())
        } else {
            file_path.to_string()
        }
    }

    /// Calculate resolution statistics.
    fn calculate_stats(
        &self,
        relationships: &[ResolvedRelationship],
        unresolved_references: &[UnresolvedReference],
        files_count: usize,
        time_ms: f64,
    ) -> ResolutionStats {
        let mut by_type: HashMap<RelationshipType, f64> = HashMap::new();

        for rel in relationships {
            *by_type.entry(rel.r#type.clone()).or_insert(0.0) += 1.0;
        }

        ResolutionStats {
            total_scopes: self.uuid_mapping.len(),
            total_relationships: relationships.len(),
            by_type,
            unresolved_count: unresolved_references.len() as f64,
            files_analyzed: files_count as f64,
            resolution_time_ms: Some(time_ms),
        }
    }

    /// Enrich parsed files with resolved relationships.
    pub fn enrich_parsed_files(
        &self,
        parsed_files: &ParsedFilesMap,
        result: &RelationshipResolutionResult,
    ) -> HashMap<String, EnrichedFileAnalysis> {
        let mut enriched_files = HashMap::new();

        // Build UUID -> relationships index
        let mut rels_by_source: HashMap<String, Vec<&ResolvedRelationship>> = HashMap::new();
        let mut rels_by_target: HashMap<String, Vec<&ResolvedRelationship>> = HashMap::new();

        for rel in &result.relationships {
            rels_by_source.entry(rel.from_uuid.clone()).or_default().push(rel);
            rels_by_target.entry(rel.to_uuid.clone()).or_default().push(rel);
        }

        for (file_path, analysis) in parsed_files {
            let relative_path = self.get_relative_path(file_path);
            let mut enriched_scopes = Vec::new();

            for scope in &analysis.scopes {
                let uuid = self.generate_uuid(scope, &relative_path);
                let outgoing = rels_by_source.get(&uuid).cloned().unwrap_or_default();
                let incoming = rels_by_target.get(&uuid).cloned().unwrap_or_default();

                let enriched = EnrichedScope {
                    uuid: uuid.clone(),
                    consumes: outgoing.iter()
                        .filter(|r| r.r#type == RelationshipType::CONSUMES)
                        .map(|r| r.to_uuid.clone())
                        .collect(),
                    consumed_by: incoming.iter()
                        .filter(|r| r.r#type == RelationshipType::CONSUMES)
                        .map(|r| r.from_uuid.clone())
                        .collect(),
                    inherits_from: outgoing.iter()
                        .filter(|r| r.r#type == RelationshipType::INHERITSFROM || r.r#type == RelationshipType::IMPLEMENTS)
                        .map(|r| r.to_uuid.clone())
                        .collect(),
                    inherited_by: incoming.iter()
                        .filter(|r| r.r#type == RelationshipType::INHERITSFROM || r.r#type == RelationshipType::IMPLEMENTS)
                        .map(|r| r.from_uuid.clone())
                        .collect(),
                    parent_of: outgoing.iter()
                        .filter(|r| r.r#type == RelationshipType::PARENTOF)
                        .map(|r| r.to_uuid.clone())
                        .collect(),
                    has_parent: incoming.iter()
                        .find(|r| r.r#type == RelationshipType::PARENTOF)
                        .map(|r| r.from_uuid.clone()),
                    decorated_by: incoming.iter()
                        .filter(|r| r.r#type == RelationshipType::DECORATES)
                        .map(|r| r.from_uuid.clone())
                        .collect(),
                    defined_in: outgoing.iter()
                        .find(|r| r.r#type == RelationshipType::DEFINEDIN)
                        .map(|r| r.to_uuid.clone()),
                    uses_libraries: outgoing.iter()
                        .filter(|r| r.r#type == RelationshipType::USESLIBRARY)
                        .map(|r| r.to_uuid.clone())
                        .collect(),
                };

                enriched_scopes.push(enriched);
            }

            enriched_files.insert(relative_path.clone(), EnrichedFileAnalysis {
                file_path: relative_path,
                scopes: enriched_scopes,
                imports: analysis.import_references.iter().map(|i| i.source.clone()).collect(),
            });
        }

        enriched_files
    }

    /// Get scope by UUID.
    pub fn get_scope_by_uuid(&self, uuid: &str) -> Option<&ScopeMappingEntry> {
        self.uuid_mapping.get(uuid)
    }

    /// Get all scopes by name.
    pub fn get_scopes_by_name(&self, name: &str) -> Vec<&ScopeMappingEntry> {
        self.scope_mapping.get(name)
            .map(|entries| entries.iter().collect())
            .unwrap_or_default()
    }

    /// Find consumers of a scope (what uses this scope).
    pub fn find_consumers(&self, uuid: &str, result: &RelationshipResolutionResult) -> Vec<&ScopeMappingEntry> {
        result.relationships.iter()
            .filter(|rel| rel.to_uuid == uuid && rel.r#type == RelationshipType::CONSUMES)
            .filter_map(|rel| self.uuid_mapping.get(&rel.from_uuid))
            .collect()
    }

    /// Find dependencies of a scope (what this scope uses).
    pub fn find_dependencies(&self, uuid: &str, result: &RelationshipResolutionResult) -> Vec<&ScopeMappingEntry> {
        result.relationships.iter()
            .filter(|rel| rel.from_uuid == uuid && rel.r#type == RelationshipType::CONSUMES)
            .filter_map(|rel| self.uuid_mapping.get(&rel.to_uuid))
            .collect()
    }
}
