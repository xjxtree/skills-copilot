import Testing
import Foundation
@testable import SkillsCopilot

@Suite("AIProviderModelTests")
struct AIProviderModelTests {
    @Test("AIProviderModelTests")
    func run() throws {
        try providerStatusDecodesProfilesAndBudget()
        try providerStatusDecodesFlatBudgetAndAliases()
        try testResultDecodesAuditMetadata()
        try draftDoesNotCarryStoredAPIKey()
    }

    private struct ServiceEnvelope<ResultPayload: Decodable>: Decodable {
        let id: String?
        let ok: Bool
        let result: ResultPayload?
    }

    private func providerStatusDecodesProfilesAndBudget() throws {
        let data = Data(
            """
            {
              "id": "provider-status",
              "ok": true,
              "result": {
                "service_available": true,
                "enabled": false,
                "configured": false,
                "reason": "No provider profile is configured.",
                "active_profile_id": "openai-compatible",
                "credential_storage": "keychain",
                "credential_persistence_allowed": true,
                "budget": {
                  "single_request_token_limit": 8000,
                  "monthly_budget_usd": 25.0,
                  "spent_this_month_usd": 1.5
                },
                "profiles": [
                  {
                    "id": "openai-compatible",
                    "name": "Work gateway",
                    "provider_kind": "openai_compatible",
                    "base_url": "https://llm.example.com/v1",
                    "model": "gpt-5",
                    "api_version": "2026-01-01",
                    "enabled": false,
                    "configured": false,
                    "has_api_key": false,
                    "credential_storage": "keychain"
                  },
                  {
                    "id": "claude-compatible",
                    "provider_type": "anthropic",
                    "endpoint": "https://claude.example.com",
                    "model": "claude-sonnet-4.5",
                    "enabled": true,
                    "configured": true,
                    "key_configured": true
                  }
                ]
              }
            }
            """.utf8
        )

        let envelope = try JSONDecoder().decode(ServiceEnvelope<AIProviderStatus>.self, from: data)
        guard let status = envelope.result else {
            throw NativeModelTestFailure(description: "Provider status envelope should include a result.")
        }

        try expectEqual(envelope.ok, true, "Provider status envelope should decode ok.")
        try expectEqual(status.serviceAvailable, true, "Provider status should decode service availability.")
        try expectEqual(status.configured, false, "Provider status should decode unconfigured state.")
        try expectEqual(status.disabledReason, "No provider profile is configured.", "Provider status should decode reason alias.")
        try expectEqual(status.credentialStorage, "keychain", "Provider status should decode credential storage.")
        try expectEqual(status.credentialPersistenceAllowed, true, "Provider status should decode credential persistence allowance.")
        try expectEqual(status.budget.singleRequestTokenLimit, 8000, "Provider status should decode nested token limit.")
        try expectEqual(status.budget.monthlyBudgetUSD, 25.0, "Provider status should decode nested monthly budget.")
        try expectEqual(status.profiles.count, 2, "Provider status should decode profiles.")
        try expectEqual(status.profiles[0].kind, .openAICompatible, "OpenAI-compatible kind aliases should normalize.")
        try expectEqual(status.profiles[0].endpoint, "https://llm.example.com/v1", "Profile should decode base_url as endpoint.")
        try expectEqual(status.profiles[1].kind, .claudeCompatible, "Claude-compatible provider aliases should normalize.")
        try expectEqual(status.profiles[1].hasAPIKey, true, "Profile should decode key_configured alias.")
    }

    private func providerStatusDecodesFlatBudgetAndAliases() throws {
        let data = Data(
            """
            {
              "enabled": true,
              "configured": true,
              "active_profile": "claude",
              "single_request_token_limit": 12000,
              "monthly_budget_usd": 30.5,
              "provider_profiles": [
                {
                  "provider": "claude",
                  "endpoint": "https://api.anthropic-compatible.example",
                  "model": "claude-opus-4",
                  "enabled": true,
                  "configured": true,
                  "has_api_key": true
                }
              ],
              "last_test": {
                "success": true,
                "status": "ok",
                "message": "Connected."
              }
            }
            """.utf8
        )

        let status = try JSONDecoder().decode(AIProviderStatus.self, from: data)

        try expectEqual(status.enabled, true, "Flat provider status should decode enabled.")
        try expectEqual(status.activeProfileID, "claude", "Provider status should decode active_profile alias.")
        try expectEqual(status.budget.singleRequestTokenLimit, 12000, "Flat provider status should decode token limit.")
        try expectEqual(status.budget.monthlyBudgetUSD, 30.5, "Flat provider status should decode monthly budget.")
        try expectEqual(status.profiles.first?.id, "claude-compatible", "Profile without ID should synthesize from kind.")
        try expectEqual(status.lastTest?.success, true, "Provider status should decode last test result.")
    }

    private func testResultDecodesAuditMetadata() throws {
        let data = Data(
            """
            {
              "status": "failed",
              "message": "Unauthorized.",
              "audit_metadata": {
                "request_id": "audit-1",
                "status": "failed",
                "provider": "openai-compatible",
                "model": "gpt-5",
                "endpoint": "https://llm.example.com/v1",
                "duration_ms": 342,
                "redaction_applied": true,
                "prompt_stored": false,
                "response_stored": false,
                "input_tokens": 12,
                "output_tokens": 0,
                "estimated_cost_usd": 0.0001,
                "error_code": "unauthorized"
              }
            }
            """.utf8
        )

        let result = try JSONDecoder().decode(AIProviderTestResult.self, from: data)

        try expectEqual(result.success, false, "Failed status should decode as unsuccessful.")
        try expectEqual(result.message, "Unauthorized.", "Test result should decode message.")
        try expectEqual(result.audit?.auditID, "audit-1", "Audit metadata should decode request_id alias.")
        try expectEqual(result.audit?.durationMS, 342, "Audit metadata should decode duration.")
        try expectEqual(result.audit?.promptStored, false, "Audit metadata should keep prompt storage false.")
        try expectEqual(result.audit?.responseStored, false, "Audit metadata should keep response storage false.")
        try expectEqual(result.audit?.errorCode, "unauthorized", "Audit metadata should decode error code.")
    }

    private func draftDoesNotCarryStoredAPIKey() throws {
        let status = try JSONDecoder().decode(
            AIProviderStatus.self,
            from: Data(
                """
                {
                  "enabled": true,
                  "configured": true,
                  "profiles": [
                    {
                      "id": "openai-compatible",
                      "kind": "openai-compatible",
                      "endpoint": "https://llm.example.com/v1",
                      "model": "gpt-5",
                      "has_api_key": true
                    }
                  ]
                }
                """.utf8
            )
        )

        let draft = AIProviderSettingsDraft(status: status)

        try expectEqual(draft.endpoint, "https://llm.example.com/v1", "Draft should prefill endpoint.")
        try expectEqual(draft.model, "gpt-5", "Draft should prefill model.")
        try expectEqual(draft.apiKey, "", "Draft must not expose stored API keys.")
    }
}
