import Foundation

private enum LLMPromptRequestTimeouts {
    static let standardSendMS = 600_000
    static let taskCockpitSendMS = 300_000
}

extension ServiceClient {
    func llmStatus() async throws -> LLMStatus {
        try await call(method: "llm.status", params: EmptyParams())
    }

    func aiProviderStatus() async throws -> AIProviderStatus {
        try await call(method: "llm.listProviderProfiles", params: EmptyParams())
    }

    func previewSaveAIProviderSettings(
        draft: AIProviderSettingsDraft
    ) async throws -> AIProviderActionPreview {
        let preview: AIProviderActionPreview = try await call(
            method: "llm.previewSaveProviderProfile",
            params: makeSaveAIProviderProfileParams(draft: draft, confirmation: nil)
        )
        try validateProviderActionPreview(
            preview,
            previewMethod: "llm.previewSaveProviderProfile",
            applyMethod: "llm.saveProviderProfile",
            network: "none",
            operation: "save",
            profileID: draft.kind.rawValue
        )
        return preview
    }

    func saveAIProviderSettings(
        draft: AIProviderSettingsDraft,
        preview: AIProviderActionPreview
    ) async throws -> AIProviderStatus {
        let params = makeSaveAIProviderProfileParams(
            draft: draft,
            confirmation: preview.confirmation
        )
        let result: AIProviderSaveResult = try await call(
            method: "llm.saveProviderProfile",
            params: params
        )
        if let outcome = result.outcome, !outcome.isVerified {
            throw ClientError.actionOutcome(outcome.userFacingFailure)
        }
        try requireProviderReadback(
            result.readback,
            action: preview.action,
            operation: "Provider save"
        )
        return try await aiProviderStatus()
    }

    func previewDeleteAIProviderSettings(
        profileID: String,
        deleteCredential: Bool
    ) async throws -> AIProviderActionPreview {
        let preview: AIProviderActionPreview = try await call(
            method: "llm.previewDeleteProviderProfile",
            params: DeleteAIProviderProfileParams(
                profileID: profileID,
                deleteCredential: deleteCredential,
                actionConfirmation: nil
            )
        )
        try validateProviderActionPreview(
            preview,
            previewMethod: "llm.previewDeleteProviderProfile",
            applyMethod: "llm.deleteProviderProfile",
            network: "none",
            operation: "delete",
            profileID: profileID
        )
        return preview
    }

    func deleteAIProviderSettings(
        preview: AIProviderActionPreview,
        deleteCredential: Bool
    ) async throws -> AIProviderStatus {
        let result: AIProviderDeleteResult = try await call(
            method: "llm.deleteProviderProfile",
            params: DeleteAIProviderProfileParams(
                profileID: preview.profileID,
                deleteCredential: deleteCredential,
                actionConfirmation: preview.confirmation
            )
        )
        if let outcome = result.outcome, !outcome.isVerified {
            throw ClientError.actionOutcome(outcome.userFacingFailure)
        }
        try requireProviderReadback(
            result.readback,
            action: preview.action,
            operation: "Provider delete"
        )
        return try await aiProviderStatus()
    }

    func previewAIProviderConnectionTest(
        profileID: String
    ) async throws -> AIProviderActionPreview {
        let preview: AIProviderActionPreview = try await call(
            method: "llm.previewProviderConnectionTest",
            params: TestAIProviderConnectionParams(
                profileID: profileID,
                timeoutMS: 4_000,
                actionConfirmation: nil
            )
        )
        try validateProviderActionPreview(
            preview,
            previewMethod: "llm.previewProviderConnectionTest",
            applyMethod: "llm.testProviderConnection",
            network: "required",
            operation: "test",
            profileID: profileID
        )
        return preview
    }

    func testAIProviderConnection(
        preview: AIProviderActionPreview
    ) async throws -> AIProviderTestResult {
        let result: AIProviderTestResult = try await call(
            method: "llm.testProviderConnection",
            params: TestAIProviderConnectionParams(
                profileID: preview.profileID,
                timeoutMS: 4_000,
                actionConfirmation: preview.confirmation
            )
        )
        if let outcome = result.outcome, !outcome.isVerified {
            throw ClientError.actionOutcome(outcome.userFacingFailure)
        }
        try requireProviderReadback(
            result.readback,
            action: preview.action,
            operation: "Provider test"
        )
        return result
    }

    private func makeSaveAIProviderProfileParams(
        draft: AIProviderSettingsDraft,
        confirmation: ActionConfirmationWire?
    ) -> SaveAIProviderProfileParams {
        SaveAIProviderProfileParams(
            id: draft.kind.rawValue,
            displayName: draft.kind.title,
            providerType: draft.kind.rawValue,
            baseURL: draft.trimmedEndpoint,
            model: draft.trimmedModel,
            enabled: true,
            apiVersion: draft.trimmedAPIVersion,
            apiKey: draft.trimmedAPIKey,
            singleRequestTokenLimit: draft.parsedSingleRequestTokenLimit,
            monthlyBudgetUSD: draft.parsedMonthlyBudgetUSD,
            actionConfirmation: confirmation
        )
    }

    private func validateProviderActionPreview(
        _ preview: AIProviderActionPreview,
        previewMethod: String,
        applyMethod: String,
        network: String,
        operation: String,
        profileID: String
    ) throws {
        do {
            let isConnectionTest = operation == "test"
            try preview.action.validated(
                previewMethod: previewMethod,
                applyMethod: applyMethod,
                network: network,
                expectation: ActionDescriptorExpectation(
                    kind: isConnectionTest ? "provider_connection_test" : "provider_profile",
                    intent: "\(operation)_provider_\(isConnectionTest ? "connection" : "profile")",
                    targetKind: "provider_profile",
                    targetID: .exact(profileID),
                    targetAgent: .absent,
                    targetScope: .absent,
                    projectID: .absent,
                    impacts: isConnectionTest
                        ? ["app_local_data"]
                        : ["app_local_data", "credential_store"],
                    readback: isConnectionTest
                        ? ["provider_profiles", "provider_activity"]
                        : ["provider_profiles", "provider_credentials"]
                )
            )
            try preview.preconditions.validated(
                kinds: ["provider_profile", "prompt_context"]
            )
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
        guard !preview.previewToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              preview.operation == operation,
              preview.profileID == profileID,
              preview.action.target.kind == "provider_profile",
              preview.action.target.id == profileID,
              !preview.expectedRevision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !preview.rawSecretReturned else {
            throw ClientError.invalidOutput(
                "Provider preview does not match the requested service-owned action."
            )
        }
    }

    private func requireProviderReadback(
        _ readback: ActionReadbackRecordWire?,
        action: ActionDescriptorWire,
        operation: String
    ) throws {
        guard let readback else {
            throw ClientError.invalidOutput("\(operation) did not return typed read-back.")
        }
        do {
            try readback.validated(for: action)
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
    }

    func prepareLLMAction(action: LLMAction, skill: SkillRecord) async throws -> LLMPrepareResult {
        try await call(
            method: "llm.prepareAction",
            params: PrepareLLMActionParams(
                action: action,
                instanceId: skill.id,
                definitionId: skill.definitionId,
                agent: skill.agent
            )
        )
    }

    func previewPromptForLLMAction(action: LLMAction, skill: SkillRecord) async throws -> LLMPromptPreview {
        let params = PreviewLLMPromptParams(
            action: action.rawValue,
            requestKind: action.rawValue,
            scope: "selected",
            instanceIDs: nil,
            instanceId: skill.id,
            definitionId: skill.definitionId,
            agent: skill.agent,
            agents: nil,
            taskText: nil,
            userIntent: nil,
            candidateInstanceIDs: nil
        )
        let preview: LLMPromptPreview = try await call(method: "llm.previewPrompt", params: params)
        try validatePromptPreview(preview, request: params)
        return preview
    }

    func confirmPromptAndSendForLLMAction(
        preview: LLMPromptPreview,
        action: LLMAction,
        skill: SkillRecord
    ) async throws -> LLMPromptSendResult {
        let request = PreviewLLMPromptParams(
            action: action.rawValue,
            requestKind: action.rawValue,
            scope: "selected",
            instanceIDs: nil,
            instanceId: skill.id,
            definitionId: skill.definitionId,
            agent: skill.agent,
            agents: nil,
            taskText: nil,
            userIntent: nil,
            candidateInstanceIDs: nil
        )
        return try await confirmPromptAndSend(preview: preview, request: request)
    }

    func previewPromptForTaskCockpit(
        taskText: String,
        agents: [String],
        instanceIDs: [String],
        sourceRevision: String? = nil
    ) async throws -> LLMPromptPreview {
        let params = PreviewLLMPromptParams(
            action: "task_cockpit",
            requestKind: "task_cockpit",
            scope: "agents",
            instanceIDs: instanceIDs,
            instanceId: nil,
            definitionId: nil,
            agent: nil,
            agents: agents,
            taskText: taskText,
            userIntent: taskText,
            candidateInstanceIDs: instanceIDs,
            sourceRevision: sourceRevision
        )
        let preview: LLMPromptPreview = try await call(
            method: "llm.previewPrompt",
            params: params,
            timeoutMS: LLMPromptRequestTimeouts.taskCockpitSendMS
        )
        try validatePromptPreview(preview, request: params)
        return preview
    }

    func confirmPromptAndSendForTaskCockpit(
        preview: LLMPromptPreview,
        taskText: String,
        agents: [String],
        instanceIDs: [String]
    ) async throws -> LLMPromptSendResult {
        let request = PreviewLLMPromptParams(
            action: "task_cockpit",
            requestKind: "task_cockpit",
            scope: "agents",
            instanceIDs: instanceIDs,
            instanceId: nil,
            definitionId: nil,
            agent: nil,
            agents: agents,
            taskText: taskText,
            userIntent: taskText,
            candidateInstanceIDs: instanceIDs,
            sourceRevision: preview.responseContract?.sourceRevision
        )
        return try await confirmPromptAndSend(
            preview: preview,
            request: request,
            timeoutMS: LLMPromptRequestTimeouts.taskCockpitSendMS
        )
    }

    func previewPromptForSessionDigest(
        authorizedRoots: [String],
        project: ProjectContext,
        session: SessionContinuationRecord,
        productSourceRevision: String
    ) async throws -> LLMPromptPreview {
        let request = sessionDigestPromptRequest(
            authorizedRoots: authorizedRoots,
            project: project,
            session: session,
            productSourceRevision: productSourceRevision
        )
        let preview: LLMPromptPreview = try await call(
            method: "llm.previewPrompt",
            params: request
        )
        try validatePromptPreview(preview, request: request)
        return preview
    }

    func confirmPromptAndSendForSessionDigest(
        preview: LLMPromptPreview,
        authorizedRoots: [String],
        project: ProjectContext,
        session: SessionContinuationRecord,
        productSourceRevision: String
    ) async throws -> LLMPromptSendResult {
        try await confirmPromptAndSend(
            preview: preview,
            request: sessionDigestPromptRequest(
                authorizedRoots: authorizedRoots,
                project: project,
                session: session,
                productSourceRevision: productSourceRevision
            )
        )
    }

    func previewPromptForProjectHealth(
        sourceRevision: String
    ) async throws -> LLMPromptPreview {
        let request = projectHealthPromptRequest(sourceRevision: sourceRevision)
        let preview: LLMPromptPreview = try await call(
            method: "llm.previewPrompt",
            params: request
        )
        try validatePromptPreview(preview, request: request)
        return preview
    }

    func confirmPromptAndSendForProjectHealth(
        preview: LLMPromptPreview,
        sourceRevision: String
    ) async throws -> LLMPromptSendResult {
        try await confirmPromptAndSend(
            preview: preview,
            request: projectHealthPromptRequest(sourceRevision: sourceRevision)
        )
    }

    func previewPromptForSkillChangeReview(
        aggregate: SkillAggregateRecord,
        sourceRevision: String
    ) async throws -> LLMPromptPreview {
        let request = skillChangeReviewPromptRequest(
            aggregate: aggregate,
            sourceRevision: sourceRevision
        )
        let preview: LLMPromptPreview = try await call(
            method: "llm.previewPrompt",
            params: request
        )
        try validatePromptPreview(preview, request: request)
        return preview
    }

    func confirmPromptAndSendForSkillChangeReview(
        preview: LLMPromptPreview,
        aggregate: SkillAggregateRecord,
        sourceRevision: String
    ) async throws -> LLMPromptSendResult {
        try await confirmPromptAndSend(
            preview: preview,
            request: skillChangeReviewPromptRequest(
                aggregate: aggregate,
                sourceRevision: sourceRevision
            )
        )
    }

    func previewPromptForSemanticSearch(
        query: String,
        candidates: [AppSearchItem],
        sourceRevision: String
    ) async throws -> LLMPromptPreview {
        let request = semanticSearchPromptRequest(
            query: query,
            candidates: candidates,
            sourceRevision: sourceRevision
        )
        let preview: LLMPromptPreview = try await call(
            method: "llm.previewPrompt",
            params: request
        )
        try validatePromptPreview(preview, request: request)
        return preview
    }

    func confirmPromptAndSendForSemanticSearch(
        preview: LLMPromptPreview,
        query: String,
        candidates: [AppSearchItem],
        sourceRevision: String
    ) async throws -> LLMPromptSendResult {
        try await confirmPromptAndSend(
            preview: preview,
            request: semanticSearchPromptRequest(
                query: query,
                candidates: candidates,
                sourceRevision: sourceRevision
            )
        )
    }

    func listLLMPromptRuns(skill: SkillRecord? = nil, limit: Int? = nil) async throws -> LLMPromptRunListResult {
        let params = ListLLMPromptRunsParams(
            instanceId: skill?.id,
            action: nil,
            requestKind: nil,
            limit: limit
        )
        return try await call(method: "llm.listPromptRuns", params: params)
    }

    func providerObservability(
        windowDays: Int? = 30,
        startAt: Int? = nil,
        endAt: Int? = nil,
        limit: Int = 30,
        includeHistory: Bool = true,
        includeBudgetHints: Bool = true,
        includeRetentionRecommendations: Bool = true,
        includeEvidence: Bool = true
    ) async throws -> ProviderObservabilityResult {
        let params = ProviderObservabilityParams(
            windowDays: windowDays,
            startAt: startAt,
            endAt: endAt,
            limit: limit,
            includeHistory: includeHistory,
            includeBudgetHints: includeBudgetHints,
            includeRetentionRecommendations: includeRetentionRecommendations,
            includeEvidence: includeEvidence
        )
        return try await call(method: "llm.providerObservability", params: params)
    }

    func listProviderActivity(
        provider: String? = nil,
        model: String? = nil,
        action: String? = nil,
        windowDays: Int? = nil,
        startAt: Int? = nil,
        endAt: Int? = nil,
        limit: Int = 50,
        cursor: String? = nil,
        sourceRevision: String? = nil
    ) async throws -> ProviderActivityPageResult {
        try await call(
            method: "llm.listProviderActivity",
            params: ListProviderActivityParams(
                provider: provider,
                model: model,
                action: action,
                windowDays: windowDays,
                startAt: startAt,
                endAt: endAt,
                limit: limit,
                cursor: cursor,
                sourceRevision: sourceRevision
            )
        )
    }

    private func confirmPromptAndSend(
        preview: LLMPromptPreview,
        request: PreviewLLMPromptParams,
        timeoutMS: Int = LLMPromptRequestTimeouts.standardSendMS
    ) async throws -> LLMPromptSendResult {
        guard let action = preview.actionDescriptor,
              let actionConfirmation = preview.actionConfirmation else {
            throw ClientError.invalidOutput(
                "The provider prompt preview is missing its typed action confirmation."
            )
        }
        let params = ConfirmLLMPromptParams(
            actionConfirmation: actionConfirmation,
            request: request,
            timeoutMS: timeoutMS
        )
        let result: LLMPromptSendResult = try await call(
            method: "llm.confirmPromptAndSend",
            params: params,
            timeoutMS: timeoutMS
        )
        if result.partialOutcome == nil {
            guard let readback = result.readback else {
                throw ClientError.invalidOutput(
                    "Provider prompt send did not return verified typed read-back."
                )
            }
            do {
                try readback.validated(for: action)
            } catch {
                throw ClientError.invalidOutput(error.localizedDescription)
            }
        }
        if result.partialOutcome != nil, result.status.lowercased() != "partial" {
            throw ClientError.invalidOutput(
                "Provider prompt partial outcome must use the explicit partial status."
            )
        }
        if result.success {
            guard let contract = preview.responseContract,
                  let envelope = result.responseEnvelope else {
                throw ClientError.invalidOutput(
                    "Successful provider output omitted its evidence-bound response envelope."
                )
            }
            do {
                try envelope.validated(against: contract)
            } catch {
                throw ClientError.invalidOutput(error.localizedDescription)
            }
        }
        return result
    }

    private func sessionDigestPromptRequest(
        authorizedRoots: [String],
        project: ProjectContext,
        session: SessionContinuationRecord,
        productSourceRevision: String
    ) -> PreviewLLMPromptParams {
        PreviewLLMPromptParams(
            action: "session_digest",
            requestKind: "session_digest",
            scope: "selected_session",
            instanceIDs: nil,
            instanceId: nil,
            definitionId: nil,
            agent: session.agent.rawValue,
            agents: [session.agent.rawValue],
            taskText: nil,
            userIntent: nil,
            candidateInstanceIDs: nil,
            sourceRevision: productSourceRevision,
            session: LLMSessionEvidenceParams(
                authorizedRoots: authorizedRoots,
                autoDiscover: authorizedRoots.isEmpty,
                agent: session.agent.rawValue,
                projectRoot: project.rootPath,
                currentCWD: project.currentCWD ?? project.rootPath,
                sessionID: session.id,
                sourceRevision: session.sourceRevision,
                snapshotRevision: session.snapshotRevision
            )
        )
    }

    private func projectHealthPromptRequest(
        sourceRevision: String
    ) -> PreviewLLMPromptParams {
        PreviewLLMPromptParams(
            action: "project_health",
            requestKind: "project_health",
            scope: "active_project",
            instanceIDs: nil,
            instanceId: nil,
            definitionId: nil,
            agent: nil,
            agents: nil,
            taskText: nil,
            userIntent: nil,
            candidateInstanceIDs: nil,
            sourceRevision: sourceRevision
        )
    }

    private func skillChangeReviewPromptRequest(
        aggregate: SkillAggregateRecord,
        sourceRevision: String
    ) -> PreviewLLMPromptParams {
        PreviewLLMPromptParams(
            action: "skill_change_review",
            requestKind: "skill_change_review",
            scope: "selected_skill",
            instanceIDs: aggregate.instanceIDs,
            instanceId: aggregate.instanceIDs.first,
            definitionId: aggregate.definitionID,
            agent: aggregate.agents.first?.rawValue,
            agents: aggregate.agents.map(\.rawValue),
            taskText: nil,
            userIntent: nil,
            candidateInstanceIDs: nil,
            sourceRevision: sourceRevision
        )
    }

    private func semanticSearchPromptRequest(
        query: String,
        candidates: [AppSearchItem],
        sourceRevision: String
    ) -> PreviewLLMPromptParams {
        PreviewLLMPromptParams(
            action: "semantic_search",
            requestKind: "semantic_search",
            scope: "returned_candidates",
            instanceIDs: nil,
            instanceId: nil,
            definitionId: nil,
            agent: nil,
            agents: nil,
            taskText: query,
            userIntent: query,
            candidateInstanceIDs: nil,
            sourceRevision: sourceRevision,
            searchCandidates: Array(candidates.prefix(18)).map {
                LLMSearchCandidateParams(
                    id: $0.id,
                    kind: $0.kind.rawValue,
                    title: $0.title,
                    subtitle: $0.subtitle
                )
            }
        )
    }

    private func validatePromptPreview(
        _ preview: LLMPromptPreview,
        request: PreviewLLMPromptParams
    ) throws {
        guard preview.enabled else { return }
        guard let action = preview.actionDescriptor,
              let previewToken = preview.previewToken,
              let responseContract = preview.responseContract,
              !previewToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            throw ClientError.invalidOutput(
                "Enabled provider prompt preview omitted its service-owned action."
            )
        }
        do {
            try action.validated(
                previewMethod: "llm.previewPrompt",
                applyMethod: "llm.confirmPromptAndSend",
                network: "required",
                expectation: ActionDescriptorExpectation(
                    kind: "provider_prompt",
                    intent: "send_provider_prompt",
                    targetKind: "provider_profile",
                    targetID: .present,
                    targetAgent: .absent,
                    targetScope: .absent,
                    projectID: .present,
                    impacts: ["app_local_data"],
                    readback: ["provider_activity", "prompt_runs"]
                )
            )
            try preview.preconditions.validated(
                kinds: ["provider_profile", "prompt_context"]
            )
            try responseContract.validated(
                requestKind: request.action,
                projectID: action.projectID
            )
        } catch {
            throw ClientError.invalidOutput(error.localizedDescription)
        }
        guard action.kind == "provider_prompt",
              action.intent == "send_provider_prompt",
              action.target.kind == "provider_profile",
              preview.confirmationRequired,
              preview.requestKind == request.action,
              preview.preconditions.contains(where: {
                  $0.targetID == "product-evidence"
                      && $0.expectedRevision == responseContract.sourceRevision
              }),
              preview.rawPromptPersisted == false,
              preview.rawResponsePersisted == false,
              preview.draftCopyOnly else {
            throw ClientError.invalidOutput(
                "Provider prompt preview violated its confirmed copy-only lifecycle."
            )
        }
    }
}
