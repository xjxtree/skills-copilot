import Foundation

enum ContextualIntelligenceKind: String, Hashable {
    case projectHealth = "project_health"
    case skillChangeReview = "skill_change_review"
    case sessionDigest = "session_digest"
    case semanticSearch = "semantic_search"

    var title: String {
        switch self {
        case .projectHealth:
            return UIStrings.text("intelligence.project.title", "Project explanation")
        case .skillChangeReview:
            return UIStrings.text("intelligence.skill.title", "Contextual skill review")
        case .sessionDigest:
            return UIStrings.text("intelligence.session.title", "Session digest")
        case .semanticSearch:
            return UIStrings.text("intelligence.search.title", "Semantic rerank")
        }
    }
}

enum ContextualIntelligencePhase: Hashable {
    case idle
    case previewing
    case awaitingConfirmation
    case sending
    case complete
    case failed
}

struct ContextualIntelligenceFlow: Identifiable, Hashable {
    let id: String
    let kind: ContextualIntelligenceKind
    let subjectID: String
    let sourceRevision: String
    var phase: ContextualIntelligencePhase
    var preview: LLMPromptPreview?
    var envelope: AIResponseEnvelopeWire?
    var output: ContextualIntelligenceOutput?
    var errorMessage: String?

    static func idle(
        key: String,
        kind: ContextualIntelligenceKind,
        subjectID: String,
        sourceRevision: String
    ) -> ContextualIntelligenceFlow {
        ContextualIntelligenceFlow(
            id: key,
            kind: kind,
            subjectID: subjectID,
            sourceRevision: sourceRevision,
            phase: .idle
        )
    }

    func isStale(currentSourceRevision: String?) -> Bool {
        guard let currentSourceRevision, !currentSourceRevision.isEmpty else { return true }
        return sourceRevision != currentSourceRevision
    }

    var citations: [EvidenceRef] {
        guard let preview, let envelope else { return [] }
        let accepted = Set(envelope.evidenceRefs)
        return preview.responseContract?.evidence.filter { accepted.contains($0.id) } ?? []
    }
}

@MainActor
final class ContextualIntelligenceStore: ObservableObject {
    @Published private(set) var flows: [String: ContextualIntelligenceFlow] = [:]

    private let service: ServiceClient
    private var generations: [String: UUID] = [:]

    init(service: ServiceClient) {
        self.service = service
    }

    func flow(for key: String) -> ContextualIntelligenceFlow? {
        flows[key]
    }

    func clear(_ key: String) {
        generations[key] = UUID()
        flows.removeValue(forKey: key)
    }

    func previewProjectHealth(_ record: ProjectReadinessRecord) async {
        let key = Self.projectKey(record.projectID)
        await preview(
            key: key,
            kind: .projectHealth,
            subjectID: record.projectID,
            sourceRevision: record.sourceRevision
        ) {
            try await self.service.previewPromptForProjectHealth(
                sourceRevision: record.sourceRevision
            )
        }
    }

    func sendProjectHealth(_ record: ProjectReadinessRecord) async {
        let key = Self.projectKey(record.projectID)
        await send(key: key, currentSourceRevision: record.sourceRevision) { preview in
            try await self.service.confirmPromptAndSendForProjectHealth(
                preview: preview,
                sourceRevision: record.sourceRevision
            )
        }
    }

    func previewSkillReview(
        aggregate: SkillAggregateRecord,
        productSourceRevision: String
    ) async {
        let key = Self.skillKey(aggregate.id)
        await preview(
            key: key,
            kind: .skillChangeReview,
            subjectID: aggregate.id,
            sourceRevision: productSourceRevision
        ) {
            try await self.service.previewPromptForSkillChangeReview(
                aggregate: aggregate,
                sourceRevision: productSourceRevision
            )
        }
    }

    func sendSkillReview(
        aggregate: SkillAggregateRecord,
        productSourceRevision: String
    ) async {
        let key = Self.skillKey(aggregate.id)
        await send(key: key, currentSourceRevision: productSourceRevision) { preview in
            try await self.service.confirmPromptAndSendForSkillChangeReview(
                preview: preview,
                aggregate: aggregate,
                sourceRevision: productSourceRevision
            )
        }
    }

    func previewSessionDigest(
        authorizedRoots: [String],
        project: ProjectContext,
        session: SessionContinuationRecord,
        productSourceRevision: String
    ) async {
        let key = Self.sessionKey(session.id)
        await preview(
            key: key,
            kind: .sessionDigest,
            subjectID: session.id,
            sourceRevision: productSourceRevision
        ) {
            try await self.service.previewPromptForSessionDigest(
                authorizedRoots: authorizedRoots,
                project: project,
                session: session,
                productSourceRevision: productSourceRevision
            )
        }
    }

    func sendSessionDigest(
        authorizedRoots: [String],
        project: ProjectContext,
        session: SessionContinuationRecord,
        productSourceRevision: String
    ) async {
        let key = Self.sessionKey(session.id)
        await send(key: key, currentSourceRevision: productSourceRevision) { preview in
            try await self.service.confirmPromptAndSendForSessionDigest(
                preview: preview,
                authorizedRoots: authorizedRoots,
                project: project,
                session: session,
                productSourceRevision: productSourceRevision
            )
        }
    }

    func previewSemanticSearch(
        query: String,
        candidates: [AppSearchItem],
        sourceRevision: String
    ) async {
        let key = Self.semanticSearchKey(query: query, candidates: candidates)
        await preview(
            key: key,
            kind: .semanticSearch,
            subjectID: query,
            sourceRevision: sourceRevision
        ) {
            try await self.service.previewPromptForSemanticSearch(
                query: query,
                candidates: candidates,
                sourceRevision: sourceRevision
            )
        }
    }

    func sendSemanticSearch(
        query: String,
        candidates: [AppSearchItem],
        sourceRevision: String
    ) async {
        let key = Self.semanticSearchKey(query: query, candidates: candidates)
        await send(key: key, currentSourceRevision: sourceRevision) { preview in
            try await self.service.confirmPromptAndSendForSemanticSearch(
                preview: preview,
                query: query,
                candidates: candidates,
                sourceRevision: sourceRevision
            )
        }
    }

    func rankedSearchItems(
        query: String,
        candidates: [AppSearchItem],
        currentSourceRevision: String?
    ) -> [AppSearchItem] {
        let key = Self.semanticSearchKey(query: query, candidates: candidates)
        guard let flow = flows[key],
              flow.phase == .complete,
              !flow.isStale(currentSourceRevision: currentSourceRevision),
              let output = flow.output,
              let contract = flow.preview?.responseContract else {
            return candidates
        }
        let targetByEvidenceID = Dictionary(
            uniqueKeysWithValues: contract.evidence.compactMap { reference in
                reference.targetID.map { (reference.id, $0) }
            }
        )
        let itemByID = Dictionary(uniqueKeysWithValues: candidates.map { ($0.id, $0) })
        var seen = Set<String>()
        let ranked = output.rankedEvidenceIDs.compactMap { evidenceID -> AppSearchItem? in
            guard let targetID = targetByEvidenceID[evidenceID],
                  let item = itemByID[targetID],
                  seen.insert(item.id).inserted else {
                return nil
            }
            return item
        }
        return ranked + candidates.filter { seen.insert($0.id).inserted }
    }

    nonisolated static func projectKey(_ projectID: String) -> String {
        "project:\(projectID)"
    }

    nonisolated static func skillKey(_ aggregateID: String) -> String {
        "skill:\(aggregateID)"
    }

    nonisolated static func sessionKey(_ sessionID: String) -> String {
        "session:\(sessionID)"
    }

    nonisolated static func semanticSearchKey(
        query: String,
        candidates: [AppSearchItem]
    ) -> String {
        let ids = candidates.prefix(18).map(\.id).joined(separator: "\u{1f}")
        return "search:\(query)\u{1e}\(ids)"
    }

    private func preview(
        key: String,
        kind: ContextualIntelligenceKind,
        subjectID: String,
        sourceRevision: String,
        operation: @escaping () async throws -> LLMPromptPreview
    ) async {
        let generation = UUID()
        generations[key] = generation
        flows[key] = .idle(
            key: key,
            kind: kind,
            subjectID: subjectID,
            sourceRevision: sourceRevision
        )
        flows[key]?.phase = .previewing
        do {
            let prompt = try await operation()
            guard generations[key] == generation else { return }
            guard prompt.enabled,
                  prompt.confirmationRequired,
                  prompt.responseContract?.sourceRevision == sourceRevision else {
                flows[key]?.phase = .failed
                flows[key]?.errorMessage = UIStrings.localizedServiceMessage(
                    prompt.disabledReason
                        ?? UIStrings.text(
                            "intelligence.unavailable",
                            "Contextual intelligence is unavailable."
                        )
                )
                return
            }
            flows[key]?.preview = prompt
            flows[key]?.phase = .awaitingConfirmation
        } catch {
            guard generations[key] == generation else { return }
            flows[key]?.phase = .failed
            flows[key]?.errorMessage = UIStrings.localizedServiceMessage(
                error.localizedDescription
            )
        }
    }

    private func send(
        key: String,
        currentSourceRevision: String,
        operation: @escaping (LLMPromptPreview) async throws -> LLMPromptSendResult
    ) async {
        guard var flow = flows[key],
              let preview = flow.preview,
              !flow.isStale(currentSourceRevision: currentSourceRevision) else {
            flows[key]?.phase = .failed
            flows[key]?.errorMessage = UIStrings.text(
                "intelligence.stale.preview",
                "The evidence changed. Preview the provider request again."
            )
            return
        }
        let generation = UUID()
        generations[key] = generation
        flow.phase = .sending
        flow.errorMessage = nil
        flows[key] = flow
        do {
            let result = try await operation(preview)
            guard generations[key] == generation else { return }
            guard result.success,
                  let envelope = result.responseEnvelope,
                  let output = ContextualIntelligenceOutput.parse(envelope) else {
                flows[key]?.phase = .failed
                flows[key]?.errorMessage = UIStrings.localizedServiceMessage(result.message)
                return
            }
            flows[key]?.envelope = envelope
            flows[key]?.output = output
            flows[key]?.phase = .complete
        } catch {
            guard generations[key] == generation else { return }
            flows[key]?.phase = .failed
            flows[key]?.errorMessage = UIStrings.localizedServiceMessage(
                error.localizedDescription
            )
        }
    }
}
