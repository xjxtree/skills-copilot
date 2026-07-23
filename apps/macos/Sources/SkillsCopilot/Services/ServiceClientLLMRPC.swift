import Foundation

private enum LLMPromptRequestTimeouts {
    static let standardSendMS = 600_000
    static let taskCockpitSendMS = 300_000
}

extension ServiceClient {
    func llmStatus() async throws -> LLMStatus {
        do {
            return try await call(method: "llm.status", params: EmptyParams())
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .disabledFallback()
        }
    }

    func aiProviderStatus() async throws -> AIProviderStatus {
        do {
            return try await call(method: "llm.listProviderProfiles", params: EmptyParams())
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable()
        }
    }

    func previewSaveAIProviderSettings(
        draft: AIProviderSettingsDraft
    ) async throws -> AIProviderActionPreview {
        try await call(
            method: "llm.previewSaveProviderProfile",
            params: makeSaveAIProviderProfileParams(draft: draft, confirmation: nil)
        )
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
        guard result.readback?.verified == true else {
            throw ClientError.invalidOutput("Provider save did not return verified typed read-back.")
        }
        return try await aiProviderStatus()
    }

    func previewDeleteAIProviderSettings(
        profileID: String,
        deleteCredential: Bool
    ) async throws -> AIProviderActionPreview {
        try await call(
            method: "llm.previewDeleteProviderProfile",
            params: DeleteAIProviderProfileParams(
                profileID: profileID,
                deleteCredential: deleteCredential,
                actionConfirmation: nil
            )
        )
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
        guard result.readback?.verified == true else {
            throw ClientError.invalidOutput("Provider delete did not return verified typed read-back.")
        }
        return try await aiProviderStatus()
    }

    func previewAIProviderConnectionTest(
        profileID: String
    ) async throws -> AIProviderActionPreview {
        try await call(
            method: "llm.previewProviderConnectionTest",
            params: TestAIProviderConnectionParams(
                profileID: profileID,
                timeoutMS: 4_000,
                actionConfirmation: nil
            )
        )
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
        if let outcome = result.outcome, outcome.isPartial {
            return result
        }
        if let outcome = result.outcome, !outcome.isVerified {
            throw ClientError.actionOutcome(outcome.userFacingFailure)
        }
        guard result.readback?.verified == true else {
            throw ClientError.invalidOutput("Provider test did not return verified typed read-back.")
        }
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

    func prepareLLMAction(action: LLMAction, skill: SkillRecord) async throws -> LLMPrepareResult {
        do {
            return try await call(
                method: "llm.prepareAction",
                params: PrepareLLMActionParams(
                    action: action,
                    instanceId: skill.id,
                    definitionId: skill.definitionId,
                    agent: skill.agent
                )
            )
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .disabledFallback(action: action)
        }
    }

    func previewPromptForLLMAction(action: LLMAction, skill: SkillRecord) async throws -> LLMPromptPreview {
        let params = PreviewLLMPromptParams(
            action: action.rawValue,
            requestKind: "action",
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
        do {
            return try await call(method: "llm.previewPrompt", params: params)
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable(reason: UIStrings.llmPromptUnavailable)
        }
    }

    func confirmPromptAndSendForLLMAction(
        preview: LLMPromptPreview,
        action: LLMAction,
        skill: SkillRecord
    ) async throws -> LLMPromptSendResult {
        let request = PreviewLLMPromptParams(
            action: action.rawValue,
            requestKind: "action",
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
        instanceIDs: [String]
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
            candidateInstanceIDs: instanceIDs
        )
        do {
            return try await call(
                method: "llm.previewPrompt",
                params: params,
                timeoutMS: LLMPromptRequestTimeouts.taskCockpitSendMS
            )
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable(reason: UIStrings.taskCockpitUnavailable)
        }
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
            candidateInstanceIDs: instanceIDs
        )
        return try await confirmPromptAndSend(
            preview: preview,
            request: request,
            timeoutMS: LLMPromptRequestTimeouts.taskCockpitSendMS
        )
    }

    func listLLMPromptRuns(skill: SkillRecord? = nil, limit: Int? = nil) async throws -> LLMPromptRunListResult {
        let params = ListLLMPromptRunsParams(
            instanceId: skill?.id,
            action: nil,
            requestKind: nil,
            limit: limit
        )
        do {
            return try await call(method: "llm.listPromptRuns", params: params)
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable()
        }
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
        do {
            return try await call(method: "llm.providerObservability", params: params)
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable()
        }
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
        guard let actionConfirmation = preview.actionConfirmation else {
            throw ClientError.invalidOutput(
                "The provider prompt preview is missing its typed action confirmation."
            )
        }
        let params = ConfirmLLMPromptParams(
            actionConfirmation: actionConfirmation,
            request: request,
            timeoutMS: timeoutMS
        )
        do {
            let result: LLMPromptSendResult = try await call(
                method: "llm.confirmPromptAndSend",
                params: params,
                timeoutMS: timeoutMS
            )
            if result.partialOutcome == nil, result.readback?.verified != true {
                throw ClientError.invalidOutput(
                    "Provider prompt send did not return verified typed read-back."
                )
            }
            if result.partialOutcome != nil, result.status.lowercased() != "partial" {
                throw ClientError.invalidOutput(
                    "Provider prompt partial outcome must use the explicit partial status."
                )
            }
            return result
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable(previewID: preview.previewID, reason: UIStrings.llmPromptUnavailable)
        }
    }
}
