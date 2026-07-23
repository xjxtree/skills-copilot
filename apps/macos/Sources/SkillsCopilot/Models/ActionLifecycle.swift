import Foundation

struct ActionTargetWire: Codable, Hashable {
    let kind: String
    let id: String
    let agent: String?
    let scope: String?
}

struct ActionDescriptorWire: Codable, Hashable {
    static let configMutationAgents: Set<String> = [
        "claude-code",
        "codex",
        "hermes",
        "openclaw",
        "opencode",
        "pi",
    ]
    static let configMutationAgentScopes: Set<String> = [
        "claude-code:agent-global",
        "claude-code:agent-project",
        "codex:agent-global",
        "hermes:agent-global",
        "openclaw:agent-global",
        "opencode:agent-global",
        "opencode:agent-project",
        "pi:agent-global",
        "pi:agent-project",
    ]

    let id: String
    let kind: String
    let intent: String
    let target: ActionTargetWire
    let projectID: String?
    let impacts: [String]
    let previewMethod: String
    let applyMethod: String?
    let sourceRevision: String
    let confirmationRequired: Bool
    let network: String
    let readback: [String]
    let evidenceRefs: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case intent
        case target
        case projectID = "project_id"
        case impacts
        case previewMethod = "preview_method"
        case applyMethod = "apply_method"
        case sourceRevision = "source_revision"
        case confirmationRequired = "confirmation_required"
        case network
        case readback
        case evidenceRefs = "evidence_refs"
    }

    var reference: ActionReferenceWire {
        ActionReferenceWire(
            actionID: id,
            sourceRevision: sourceRevision,
            projectID: projectID,
            target: target
        )
    }

    var confirmationSummary: ActionConfirmationSummary {
        ActionConfirmationSummary(
            target: target.id,
            agent: target.agent,
            scope: target.scope,
            projectID: projectID,
            impacts: impacts,
            network: network,
            sourceRevision: sourceRevision,
            readback: readback,
            evidenceRefs: evidenceRefs
        )
    }

    @discardableResult
    func validatedProjection(
        sourceRevision expectedSourceRevision: String,
        evidenceIDs: Set<String>
    ) throws -> ActionDescriptorWire {
        let targetKinds: Set<String> = [
            "project",
            "agent",
            "skill",
            "session",
            "config",
            "package",
            "provider_profile",
            "app_data",
        ]
        let impactKinds: Set<String> = [
            "read_only",
            "app_local_data",
            "credential_store",
            "agent_config",
            "skill_files",
            "external_manager",
        ]
        let networkPostures: Set<String> = ["none", "conditional", "required"]
        let readbackDomains: Set<String> = [
            "project_context",
            "catalog_skills",
            "skill_aggregates",
            "skill_files",
            "agent_config",
            "config_snapshots",
            "manager_inventory",
            "session_continuation",
            "blocked_attempt_audit",
            "provider_profiles",
            "provider_credentials",
            "provider_activity",
            "prompt_runs",
            "private_content",
        ]
        let knownAgents = Self.configMutationAgents.union(["tool-global"])
        let knownScopes = Set(["tool-global", "agent-global", "agent-project"])
        let validMethod: (String) -> Bool = { value in
            let parts = value.split(separator: ".", omittingEmptySubsequences: false)
            return parts.count == 2
                && parts.allSatisfy({ part in
                    !part.isEmpty && part.allSatisfy(\.isASCIIServiceIdentifierCharacter)
                })
        }
        let readOnly = impacts.allSatisfy { $0 == "read_only" }
        guard !id.trimmedForProjectionValidation.isEmpty,
              !kind.trimmedForProjectionValidation.isEmpty,
              !intent.trimmedForProjectionValidation.isEmpty,
              targetKinds.contains(target.kind),
              !target.id.trimmedForProjectionValidation.isEmpty,
              target.agent.map(knownAgents.contains) ?? true,
              target.scope.map(knownScopes.contains) ?? true,
              target.scope != "agent-project"
                  || projectID?.trimmedForProjectionValidation.isEmpty == false,
              target.kind != "project" || projectID == target.id,
              projectID?.trimmedForProjectionValidation.isEmpty != true,
              !impacts.isEmpty,
              Set(impacts).count == impacts.count,
              impacts.allSatisfy(impactKinds.contains),
              validMethod(previewMethod),
              sourceRevision == expectedSourceRevision,
              !sourceRevision.trimmedForProjectionValidation.isEmpty,
              networkPostures.contains(network),
              !readback.isEmpty,
              Set(readback).count == readback.count,
              readback.allSatisfy(readbackDomains.contains),
              !evidenceRefs.isEmpty,
              Set(evidenceRefs).count == evidenceRefs.count,
              evidenceRefs.allSatisfy({
                  !$0.trimmedForProjectionValidation.isEmpty && evidenceIDs.contains($0)
              }) else {
            throw ActionDescriptorValidationError.invalidLifecycle
        }
        if let applyMethod {
            guard validMethod(applyMethod), !readOnly, confirmationRequired else {
                throw ActionDescriptorValidationError.invalidLifecycle
            }
        } else if !readOnly {
            throw ActionDescriptorValidationError.invalidLifecycle
        }
        return self
    }

    @discardableResult
    func validated(
        previewMethod expectedPreviewMethod: String,
        applyMethod expectedApplyMethod: String,
        network expectedNetwork: String,
        expectation: ActionDescriptorExpectation
    ) throws -> ActionDescriptorWire {
        guard !id.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !kind.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !intent.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !target.kind.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !target.id.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !sourceRevision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              confirmationRequired,
              previewMethod == expectedPreviewMethod,
              applyMethod == expectedApplyMethod,
              network == expectedNetwork,
              kind == expectation.kind,
              intent == expectation.intent,
              target.kind == expectation.targetKind,
              expectation.targetID.matches(target.id),
              expectation.targetAgent.matches(target.agent),
              expectation.targetScope.matches(target.scope),
              expectation.projectID.matches(projectID),
              impacts == expectation.impacts,
              Set(impacts).count == impacts.count,
              readback == expectation.readback,
              Set(readback).count == readback.count,
              !evidenceRefs.isEmpty,
              Set(evidenceRefs).count == evidenceRefs.count,
              evidenceRefs.allSatisfy({
                  !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
              }) else {
            throw ActionDescriptorValidationError.invalidLifecycle
        }
        return self
    }

    @discardableResult
    func validatedOptionalProjectContextScope() throws -> ActionDescriptorWire {
        let hasProject = projectID?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
        let hasProjectScope = target.scope == "agent-project"
        guard hasProject == hasProjectScope,
              target.scope == nil || hasProjectScope else {
            throw ActionDescriptorValidationError.invalidLifecycle
        }
        return self
    }

    @discardableResult
    func validatedConfigMutationBinding() throws -> ActionDescriptorWire {
        let project = projectID?.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let agent = target.agent,
              Self.configMutationAgents.contains(agent),
              let scope = target.scope,
              ["agent-global", "agent-project"].contains(scope),
              Self.configMutationAgentScopes.contains("\(agent):\(scope)"),
              project?.isEmpty != true,
              scope != "agent-project" || project?.isEmpty == false else {
            throw ActionDescriptorValidationError.invalidLifecycle
        }
        return self
    }

    @discardableResult
    func validatedBatchToggleBinding() throws -> ActionDescriptorWire {
        let project = projectID?.trimmingCharacters(in: .whitespacesAndNewlines)
        let agent = target.agent?.trimmingCharacters(in: .whitespacesAndNewlines)
        let scope = target.scope?.trimmingCharacters(in: .whitespacesAndNewlines)
        guard agent?.isEmpty != true,
              scope?.isEmpty != true,
              project?.isEmpty != true,
              agent == nil || Self.configMutationAgents.contains(agent!),
              scope == nil || ["agent-global", "agent-project"].contains(scope!),
              agent == nil
                  || scope == nil
                  || Self.configMutationAgentScopes.contains("\(agent!):\(scope!)"),
              scope != "agent-project" || project?.isEmpty == false else {
            throw ActionDescriptorValidationError.invalidLifecycle
        }
        return self
    }
}

private extension Character {
    var isASCIIServiceIdentifierCharacter: Bool {
        isASCII && (isLetter || isNumber)
    }
}

private extension String {
    var trimmedForProjectionValidation: String {
        trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

enum ActionStringExpectation: Hashable {
    case absent
    case optional
    case present
    case exact(String)
    case oneOf(Set<String>)

    func matches(_ value: String?) -> Bool {
        let normalized = value?.trimmingCharacters(in: .whitespacesAndNewlines)
        switch self {
        case .absent:
            return normalized == nil
        case .optional:
            return normalized?.isEmpty != true
        case .present:
            return normalized?.isEmpty == false
        case .exact(let expected):
            return normalized == expected
        case .oneOf(let expected):
            guard let normalized, !normalized.isEmpty else { return false }
            return expected.contains(normalized)
        }
    }
}

struct ActionDescriptorExpectation: Hashable {
    let kind: String
    let intent: String
    let targetKind: String
    let targetID: ActionStringExpectation
    let targetAgent: ActionStringExpectation
    let targetScope: ActionStringExpectation
    let projectID: ActionStringExpectation
    let impacts: [String]
    let readback: [String]
}

enum ActionDescriptorValidationError: LocalizedError, Equatable {
    case invalidLifecycle

    var errorDescription: String? {
        "The action preview does not match the required service-owned lifecycle."
    }
}

struct ActionReferenceWire: Codable, Hashable {
    let actionID: String
    let sourceRevision: String
    let projectID: String?
    let target: ActionTargetWire

    enum CodingKeys: String, CodingKey {
        case actionID = "action_id"
        case sourceRevision = "source_revision"
        case projectID = "project_id"
        case target
    }
}

struct ActionConfirmationWire: Codable, Hashable {
    let reference: ActionReferenceWire
    let previewToken: String
    let confirmed: Bool

    enum CodingKeys: String, CodingKey {
        case reference
        case previewToken = "preview_token"
        case confirmed
    }

    init(action: ActionDescriptorWire, previewToken: String) {
        reference = action.reference
        self.previewToken = previewToken
        confirmed = true
    }
}

struct ActionConfirmationSummary: Hashable {
    let target: String
    let agent: String?
    let scope: String?
    let projectID: String?
    let impacts: [String]
    let network: String
    let sourceRevision: String
    let readback: [String]
    let evidenceRefs: [String]

    var disclosureLines: [String] {
        var lines = [
            "\(UIStrings.text("actionConfirmation.target", "Target")): \(target)"
        ]
        if let agent, !agent.isEmpty {
            lines.append(
                "\(UIStrings.text("actionConfirmation.agent", "Agent")): \(DisplayText.agent(agent))"
            )
        }
        if let scope, !scope.isEmpty {
            lines.append(
                "\(UIStrings.text("actionConfirmation.scope", "Scope")): \(scope)"
            )
        }
        if let projectID, !projectID.isEmpty {
            lines.append(
                "\(UIStrings.text("actionConfirmation.project", "Project")): \(projectID)"
            )
        }
        lines.append(
            "\(UIStrings.text("actionConfirmation.impacts", "Impacts")): \(impacts.joined(separator: ", "))"
        )
        lines.append(
            "\(UIStrings.text("actionConfirmation.network", "Network")): \(network)"
        )
        lines.append(
            "\(UIStrings.text("actionConfirmation.revision", "Reviewed revision")): \(sourceRevision)"
        )
        lines.append(
            "\(UIStrings.text("actionConfirmation.readback", "Read-back")): \(readback.joined(separator: ", "))"
        )
        lines.append(
            "\(UIStrings.text("actionConfirmation.evidence", "Evidence")): \(evidenceRefs.joined(separator: ", "))"
        )
        return lines
    }

    var disclosureText: String {
        disclosureLines.joined(separator: "\n")
    }
}

struct ActionPreconditionWire: Codable, Hashable {
    let kind: String
    let targetID: String
    let expectedRevision: String

    enum CodingKeys: String, CodingKey {
        case kind
        case targetID = "target_id"
        case expectedRevision = "expected_revision"
    }
}

extension Array where Element == ActionPreconditionWire {
    @discardableResult
    func validated(kinds expectedKinds: Set<String>) throws -> [ActionPreconditionWire] {
        guard !isEmpty,
              Set(map(\.kind)) == expectedKinds,
              allSatisfy({
                  !$0.kind.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                      && !$0.targetID.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                      && !$0.expectedRevision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
              }) else {
            throw ActionDescriptorValidationError.invalidLifecycle
        }
        return self
    }
}

struct ActionReadbackObservationWire: Codable, Hashable {
    let domain: String
    let targetID: String
    let revision: String

    enum CodingKeys: String, CodingKey {
        case domain
        case targetID = "target_id"
        case revision
    }
}

struct ActionReadbackWire: Codable, Hashable {
    let actionID: String
    let sourceRevision: String
    let projectID: String?
    let domains: [String]
    let targetIDs: [String]
    let observations: [ActionReadbackObservationWire]
    let verified: Bool

    enum CodingKeys: String, CodingKey {
        case actionID = "action_id"
        case sourceRevision = "source_revision"
        case projectID = "project_id"
        case domains
        case targetIDs = "target_ids"
        case observations
        case verified
    }

    @discardableResult
    func validated(for action: ActionDescriptorWire) throws -> ActionReadbackWire {
        guard verified else {
            throw ActionReadbackValidationError.unverified
        }
        guard actionID == action.id else {
            throw ActionReadbackValidationError.actionMismatch
        }
        guard projectID == action.projectID else {
            throw ActionReadbackValidationError.projectMismatch
        }
        guard !sourceRevision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            throw ActionReadbackValidationError.missingRevision
        }

        let declaredDomains = Set(action.readback)
        let observedDomains = Set(observations.map(\.domain))
        guard !declaredDomains.isEmpty,
              declaredDomains.count == action.readback.count,
              Set(domains).count == domains.count,
              Set(domains) == declaredDomains,
              observedDomains == declaredDomains else {
            throw ActionReadbackValidationError.domainMismatch
        }

        let observationKeys = Set(observations.map { "\($0.domain)\u{0}\($0.targetID)" })
        guard !observations.isEmpty,
              observationKeys.count == observations.count,
              observations.allSatisfy({
                  !$0.targetID.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                      && !$0.revision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
              }) else {
            throw ActionReadbackValidationError.invalidObservation
        }

        let observedTargetIDs = Set(observations.map(\.targetID))
        guard !observedTargetIDs.isEmpty,
              Set(targetIDs).count == targetIDs.count,
              Set(targetIDs) == observedTargetIDs else {
            throw ActionReadbackValidationError.targetMismatch
        }
        return self
    }
}

enum ActionReadbackValidationError: LocalizedError, Equatable {
    case unverified
    case actionMismatch
    case projectMismatch
    case missingRevision
    case domainMismatch
    case invalidObservation
    case targetMismatch

    var errorDescription: String? {
        switch self {
        case .unverified:
            return "The action completed without a verified read-back."
        case .actionMismatch:
            return "The action read-back belongs to a different action."
        case .projectMismatch:
            return "The action read-back belongs to a different project."
        case .missingRevision:
            return "The action read-back is missing its source revision."
        case .domainMismatch:
            return "The action read-back does not cover every declared domain."
        case .invalidObservation:
            return "The action read-back contains an invalid observation."
        case .targetMismatch:
            return "The action read-back target set is inconsistent."
        }
    }
}

extension ActionReadbackWire {
    func verifies(
        action: ActionDescriptorWire,
        document: ConfigDocumentRecord,
        snapshotID: String? = nil
    ) -> Bool {
        guard (try? validated(for: action)) != nil,
              targetIDs.contains(document.target),
              let documentRevision = document.revision,
              observations.contains(where: {
                  $0.domain == "agent_config"
                      && $0.targetID == document.target
                      && $0.revision == documentRevision
              }) else {
            return false
        }
        guard action.target.id == document.target,
              action.target.agent == document.agent,
              action.target.scope == document.scope else {
            return false
        }
        if action.readback.contains("config_snapshots") {
            guard let snapshotID,
                  targetIDs.contains(snapshotID),
                  observations.contains(where: {
                      $0.domain == "config_snapshots"
                          && $0.targetID == snapshotID
                          && !$0.revision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                  }) else {
                return false
            }
        }
        return true
    }
}

typealias ActionReadbackRecordWire = ActionReadbackWire
