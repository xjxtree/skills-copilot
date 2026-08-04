import Foundation

private enum LLMPromptRequestTimeouts {
    static let standardSendMS = 600_000
    static let taskCockpitSendMS = 600_000
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

    func saveAIProviderSettings(draft: AIProviderSettingsDraft) async throws -> AIProviderStatus {
        let params = SaveAIProviderProfileParams(
            id: draft.kind.rawValue,
            displayName: draft.kind.title,
            providerType: draft.kind.rawValue,
            baseURL: draft.trimmedEndpoint,
            model: draft.trimmedModel,
            enabled: true,
            apiVersion: draft.trimmedAPIVersion,
            apiKey: draft.trimmedAPIKey,
            singleRequestTokenLimit: draft.parsedSingleRequestTokenLimit,
            monthlyBudgetUSD: draft.parsedMonthlyBudgetUSD
        )
        let _: AIProviderSaveResult = try await call(method: "llm.saveProviderProfile", params: params)
        return try await aiProviderStatus()
    }

    func testAIProviderConnection(draft: AIProviderSettingsDraft) async throws -> AIProviderTestResult {
        let params = TestAIProviderConnectionParams(
            profileID: draft.kind.rawValue,
            confirmationID: "settings-test-\(UUID().uuidString)",
            timeoutMS: 4_000
        )
        do {
            return try await call(method: "llm.testProviderConnection", params: params)
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable()
        }
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

    func confirmPromptAndSendForLLMAction(previewID: String, action: LLMAction, skill: SkillRecord) async throws -> LLMPromptSendResult {
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
        return try await confirmPromptAndSend(previewID: previewID, request: request)
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
        previewID: String,
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
            previewID: previewID,
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
        previewID: String,
        request: PreviewLLMPromptParams,
        timeoutMS: Int = LLMPromptRequestTimeouts.standardSendMS
    ) async throws -> LLMPromptSendResult {
        let params = ConfirmLLMPromptParams(
            previewID: previewID,
            confirmationID: "prompt-confirm-\(UUID().uuidString)",
            request: request,
            timeoutMS: timeoutMS
        )
        do {
            return try await call(method: "llm.confirmPromptAndSend", params: params, timeoutMS: timeoutMS)
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable(previewID: previewID, reason: UIStrings.llmPromptUnavailable)
        }
    }
}
