import Foundation

struct SkillEffectivenessCount: Codable, Hashable {
    let state: SkillEffectivenessState
    let count: Int
}

struct SkillInstanceEffectivenessRecord: Decodable, Hashable, Identifiable {
    let instanceID: String
    let agent: ProductAgentID?
    let scope: ProductScope
    let sourceIdentity: String
    let runtimeIdentity: String
    let installed: Bool
    let linked: Bool
    let enabled: Bool
    let precedenceProven: Bool
    let state: SkillEffectivenessState
    let coverage: SourceCoverage
    let evidenceRefs: [String]
    let actionIDs: [String]

    var id: String { instanceID }

    enum CodingKeys: String, CodingKey {
        case instanceID = "instance_id"
        case agent
        case scope
        case sourceIdentity = "source_identity"
        case runtimeIdentity = "runtime_identity"
        case installed
        case linked
        case enabled
        case precedenceProven = "precedence_proven"
        case state
        case coverage
        case evidenceRefs = "evidence_refs"
        case actionIDs = "action_ids"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        instanceID = try container.decode(String.self, forKey: .instanceID)
        agent = try container.decodeIfPresent(ProductAgentID.self, forKey: .agent)
        scope = try container.decode(ProductScope.self, forKey: .scope)
        sourceIdentity = try container.decode(String.self, forKey: .sourceIdentity)
        runtimeIdentity = try container.decode(String.self, forKey: .runtimeIdentity)
        installed = try container.decode(Bool.self, forKey: .installed)
        linked = try container.decode(Bool.self, forKey: .linked)
        enabled = try container.decode(Bool.self, forKey: .enabled)
        precedenceProven = try container.decode(Bool.self, forKey: .precedenceProven)
        state = try container.decode(SkillEffectivenessState.self, forKey: .state)
        coverage = try container.decode(SourceCoverage.self, forKey: .coverage)
        evidenceRefs = try container.decode([String].self, forKey: .evidenceRefs)
        actionIDs = try container.decodeIfPresent([String].self, forKey: .actionIDs) ?? []
    }

    @discardableResult
    fileprivate func validated(
        evidenceIDs: Set<String>,
        actionIDs knownActionIDs: Set<String>
    ) throws -> SkillInstanceEffectivenessRecord {
        try requireProductText(instanceID, "skill instance id is empty")
        try requireSafeProductIdentity(sourceIdentity, "skill source identity is empty")
        try requireSafeProductIdentity(runtimeIdentity, "skill runtime identity is empty")
        try coverage.validated()
        try requireKnownProductIDs(
            evidenceRefs,
            known: evidenceIDs,
            allowEmpty: false,
            label: "skill instance evidence"
        )
        try requireKnownProductIDs(
            actionIDs,
            known: knownActionIDs,
            label: "skill instance actions"
        )
        switch state {
        case .effective:
            guard installed, linked, enabled, precedenceProven, coverage.isComplete else {
                throw ProductProjectionValidationError.invalid(
                    "effective skill lacks proven enablement"
                )
            }
        case .disabled:
            guard installed, linked, !enabled, coverage.isComplete else {
                throw ProductProjectionValidationError.invalid(
                    "disabled skill state is inconsistent"
                )
            }
        case .shadowed:
            guard installed, linked, enabled, precedenceProven, coverage.isComplete else {
                throw ProductProjectionValidationError.invalid(
                    "shadowed skill lacks precedence evidence"
                )
            }
        case .installedUnlinked:
            guard installed, !linked, coverage.isComplete else {
                throw ProductProjectionValidationError.invalid(
                    "installed-unlinked skill state is inconsistent"
                )
            }
        case .broken:
            guard installed, coverage.isComplete else {
                throw ProductProjectionValidationError.invalid(
                    "broken skill state is inconsistent"
                )
            }
        case .unavailable:
            guard !installed
                    || !coverage.isComplete
                    || (linked && enabled && !precedenceProven) else {
                throw ProductProjectionValidationError.invalid(
                    "unavailable skill has complete classifiable evidence"
                )
            }
        }
        return self
    }
}

struct SkillAggregateRecord: Decodable, Hashable, Identifiable {
    let id: String
    let definitionID: String
    let definitionFingerprint: String?
    let canonicalName: String
    let displayName: String
    let description: String
    let publisher: String?
    let packageName: String?
    let packageVersion: String?
    let sourceKind: String
    let sourceIdentity: String
    let runtimeIdentity: String
    let readOnlyReason: String?
    let instanceIDs: [String]
    let agents: [ProductAgentID]
    let scopes: [ProductScope]
    let installedInstanceCount: Int
    let enabledInstanceCount: Int
    let effectiveInstanceCount: Int
    let primaryEffectiveness: SkillEffectivenessState
    let effectivenessCounts: [SkillEffectivenessCount]
    let instanceEffectiveness: [SkillInstanceEffectivenessRecord]
    let findingCount: Int
    let conflictCount: Int
    let sourceRevision: String
    let coverage: SourceCoverage
    let evidence: [EvidenceRef]
    let actions: [ActionDescriptorWire]

    enum CodingKeys: String, CodingKey {
        case id
        case definitionID = "definition_id"
        case definitionFingerprint = "definition_fingerprint"
        case canonicalName = "canonical_name"
        case displayName = "display_name"
        case description
        case publisher
        case packageName = "package_name"
        case packageVersion = "package_version"
        case sourceKind = "source_kind"
        case sourceIdentity = "source_identity"
        case runtimeIdentity = "runtime_identity"
        case readOnlyReason = "read_only_reason"
        case instanceIDs = "instance_ids"
        case agents
        case scopes
        case installedInstanceCount = "installed_instance_count"
        case enabledInstanceCount = "enabled_instance_count"
        case effectiveInstanceCount = "effective_instance_count"
        case primaryEffectiveness = "primary_effectiveness"
        case effectivenessCounts = "effectiveness_counts"
        case instanceEffectiveness = "instance_effectiveness"
        case findingCount = "finding_count"
        case conflictCount = "conflict_count"
        case sourceRevision = "source_revision"
        case coverage
        case evidence
        case actions
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        definitionID = try container.decode(String.self, forKey: .definitionID)
        definitionFingerprint = try container.decodeIfPresent(
            String.self,
            forKey: .definitionFingerprint
        )
        canonicalName = try container.decode(String.self, forKey: .canonicalName)
        displayName = try container.decode(String.self, forKey: .displayName)
        description = try container.decode(String.self, forKey: .description)
        publisher = try container.decodeIfPresent(String.self, forKey: .publisher)
        packageName = try container.decodeIfPresent(String.self, forKey: .packageName)
        packageVersion = try container.decodeIfPresent(String.self, forKey: .packageVersion)
        sourceKind = try container.decode(String.self, forKey: .sourceKind)
        sourceIdentity = try container.decode(String.self, forKey: .sourceIdentity)
        runtimeIdentity = try container.decode(String.self, forKey: .runtimeIdentity)
        readOnlyReason = try container.decodeIfPresent(String.self, forKey: .readOnlyReason)
        instanceIDs = try container.decode([String].self, forKey: .instanceIDs)
        agents = try container.decode([ProductAgentID].self, forKey: .agents)
        scopes = try container.decode([ProductScope].self, forKey: .scopes)
        installedInstanceCount = try container.decode(Int.self, forKey: .installedInstanceCount)
        enabledInstanceCount = try container.decode(Int.self, forKey: .enabledInstanceCount)
        effectiveInstanceCount = try container.decode(Int.self, forKey: .effectiveInstanceCount)
        primaryEffectiveness = try container.decode(
            SkillEffectivenessState.self,
            forKey: .primaryEffectiveness
        )
        effectivenessCounts = try container.decode(
            [SkillEffectivenessCount].self,
            forKey: .effectivenessCounts
        )
        instanceEffectiveness = try container.decode(
            [SkillInstanceEffectivenessRecord].self,
            forKey: .instanceEffectiveness
        )
        findingCount = try container.decode(Int.self, forKey: .findingCount)
        conflictCount = try container.decode(Int.self, forKey: .conflictCount)
        sourceRevision = try container.decode(String.self, forKey: .sourceRevision)
        coverage = try container.decode(SourceCoverage.self, forKey: .coverage)
        evidence = try container.decodeIfPresent([EvidenceRef].self, forKey: .evidence) ?? []
        actions = try container.decodeIfPresent([ActionDescriptorWire].self, forKey: .actions) ?? []
        try validate()
    }

    private func validate() throws {
        try requireProductText(id, "skill aggregate id is empty")
        try requireProductText(definitionID, "skill definition id is empty")
        try requireProductText(canonicalName, "skill canonical name is empty")
        try requireProductText(sourceKind, "skill source kind is empty")
        try requireSafeProductIdentity(sourceIdentity, "skill source identity is empty")
        try requireSafeProductIdentity(runtimeIdentity, "skill runtime identity is empty")
        try requireProductText(sourceRevision, "skill source revision is empty")
        try coverage.validated()
        guard !instanceIDs.isEmpty,
              Set(instanceIDs).count == instanceIDs.count,
              installedInstanceCount >= 0,
              enabledInstanceCount >= 0,
              effectiveInstanceCount >= 0,
              findingCount >= 0,
              conflictCount >= 0,
              installedInstanceCount <= instanceIDs.count,
              enabledInstanceCount <= installedInstanceCount,
              effectiveInstanceCount <= enabledInstanceCount else {
            throw ProductProjectionValidationError.invalid(
                "skill aggregate counts or instances are inconsistent"
            )
        }
        try validateProductProjectionLinks(
            evidence: evidence,
            evidenceRevision: sourceRevision,
            actions: actions,
            actionRevision: sourceRevision
        )
        let evidenceIDs = Set(evidence.map(\.id))
        let actionIDs = Set(actions.map(\.id))
        guard instanceEffectiveness.count == instanceIDs.count,
              Set(instanceEffectiveness.map(\.instanceID)) == Set(instanceIDs),
              Set(agents).count == agents.count,
              Set(scopes).count == scopes.count else {
            throw ProductProjectionValidationError.invalid(
                "skill aggregate membership is inconsistent"
            )
        }
        for record in instanceEffectiveness {
            guard record.sourceIdentity == sourceIdentity,
                  record.runtimeIdentity == runtimeIdentity else {
                throw ProductProjectionValidationError.invalid(
                    "skill instance identity differs from its aggregate"
                )
            }
            try record.validated(evidenceIDs: evidenceIDs, actionIDs: actionIDs)
        }
        let actualAgents = Set(instanceEffectiveness.compactMap(\.agent))
        let actualScopes = Set(instanceEffectiveness.map(\.scope))
        guard actualAgents == Set(agents), actualScopes == Set(scopes) else {
            throw ProductProjectionValidationError.invalid(
                "skill aggregate agent or scope membership is inconsistent"
            )
        }
        let stateCounts = Dictionary(
            grouping: instanceEffectiveness,
            by: \.state
        ).mapValues(\.count)
        guard !effectivenessCounts.isEmpty,
              Set(effectivenessCounts.map(\.state)).count == effectivenessCounts.count,
              effectivenessCounts.allSatisfy({ $0.count > 0 }),
              effectivenessCounts.allSatisfy({ stateCounts[$0.state] == $0.count }),
              effectivenessCounts.reduce(0, { $0 + $1.count }) == instanceIDs.count,
              Set(effectivenessCounts.map(\.state)) == Set(stateCounts.keys),
              primaryEffectiveness
                  == instanceEffectiveness.map(\.state).min(by: {
                      $0.severityRank < $1.severityRank
                  }) else {
            throw ProductProjectionValidationError.invalid(
                "skill effectiveness summary is inconsistent"
            )
        }
        let mergedCoverage = try SourceCoverage.merged(instanceEffectiveness.map(\.coverage))
        guard mergedCoverage == coverage,
              installedInstanceCount == instanceEffectiveness.filter(\.installed).count,
              enabledInstanceCount
                  == instanceEffectiveness.filter({ $0.installed && $0.enabled }).count,
              effectiveInstanceCount
                  == instanceEffectiveness.filter({ $0.state == .effective }).count else {
            throw ProductProjectionValidationError.invalid(
                "skill aggregate projection does not match its instance rows"
            )
        }
    }
}

struct ProductListPageMetadata: Codable, Hashable {
    let returnedCount: Int
    let totalCount: Int?
    let hasMore: Bool
    let nextCursor: String?
    let sourceCompleteness: ListSourceCompleteness
    let incompleteReason: ListIncompleteReason?

    enum CodingKeys: String, CodingKey {
        case returnedCount = "returned_count"
        case totalCount = "total_count"
        case hasMore = "has_more"
        case nextCursor = "next_cursor"
        case sourceCompleteness = "source_completeness"
        case incompleteReason = "incomplete_reason"
    }

    fileprivate func validated(returnedCount actualCount: Int) throws {
        guard returnedCount == actualCount,
              returnedCount >= 0,
              totalCount.map({ $0 >= returnedCount }) != false,
              !hasMore || nextCursor?.isEmpty == false,
              hasMore || nextCursor == nil,
              sourceCompleteness != .enumerable || incompleteReason == nil,
              sourceCompleteness == .enumerable || incompleteReason != nil else {
            throw ProductProjectionValidationError.invalid(
                "skill aggregate page metadata is inconsistent"
            )
        }
    }
}

struct SkillAggregateListResult: Decodable, Hashable {
    let sourceRevision: String
    let coverage: SourceCoverage
    let page: ProductListPageMetadata
    let aggregates: [SkillAggregateRecord]

    enum CodingKeys: String, CodingKey {
        case sourceRevision = "source_revision"
        case coverage
        case page
        case aggregates
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        sourceRevision = try container.decode(String.self, forKey: .sourceRevision)
        coverage = try container.decode(SourceCoverage.self, forKey: .coverage)
        page = try container.decode(ProductListPageMetadata.self, forKey: .page)
        aggregates = try container.decode([SkillAggregateRecord].self, forKey: .aggregates)
        try requireProductText(sourceRevision, "aggregate list source revision is empty")
        try coverage.validated()
        try page.validated(returnedCount: aggregates.count)
        guard page.sourceCompleteness == coverage.completeness,
              page.incompleteReason == coverage.incompleteReason,
              aggregates.allSatisfy({ $0.sourceRevision == sourceRevision }) else {
            throw ProductProjectionValidationError.invalid(
                "aggregate page source binding is inconsistent"
            )
        }
    }
}
