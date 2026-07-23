import Foundation
@testable import SkillsCopilot

struct LLMModelTests {
    func run() throws {
        try statusDecodesSnakeCasePayload()
        try statusDecodesRealServicePayload()
        try prepareResultDecodesEstimatePayload()
        try promptPreviewDecodesV242Payload()
        try promptPreviewDecodesServiceArrayScopePayload()
        try promptSendResultDecodesCopyOnlyAuditPayload()
        try promptSendResultDecodesServiceDraftOutput()
        try promptSendResultUsesAuditErrorMessage()
        try promptRunListIgnoresDeprecatedBodyFields()
        try longTextReviewBlockDefaultsToMarkdown()
        try markdownRenderDocumentParsesModelOutputBlocks()
        try markdownRenderDocumentUnwrapsWholeMarkdownFence()
        try markdownRenderDocumentNormalizesCollapsedModelMarkdown()
        try markdownTableDisplayModelKeepsReadableColumns()
        try markdownWideTableDisplayModelUsesCardLayout()
        try markdownThreeColumnQualityTableUsesCardLayoutWhenCellsAreLong()
    }

    private struct ServiceEnvelope<ResultPayload: Decodable>: Decodable {
        let id: String?
        let ok: Bool
        let result: ResultPayload?
    }

    private func statusDecodesSnakeCasePayload() throws {
        let data = Data(
            """
            {
              "enabled": true,
              "provider": "openai",
              "model": "gpt-5",
              "disabled_reason": null,
              "supported_actions": ["analyze", "recommend", "explain_conflict", "draft_frontmatter"]
            }
            """.utf8
        )

        let status = try JSONDecoder().decode(LLMStatus.self, from: data)

        try expectEqual(status.enabled, true, "LLM status should decode enabled.")
        try expectEqual(status.provider, "openai", "LLM status should decode provider.")
        try expectEqual(status.model, "gpt-5", "LLM status should decode model.")
        try expectEqual(status.supportedActions, LLMAction.allCases, "LLM status should decode supported actions.")
    }

    private func statusDecodesRealServicePayload() throws {
        let data = Data(
            """
            {
              "id": "fixture-llm-status",
              "ok": true,
              "result": {
                "enabled": false,
                "configured": false,
                "provider": null,
                "model": null,
                "reason": "LLM actions are disabled by default; no local provider is configured.",
                "single_request_token_limit": 8000,
                "monthly_budget_usd": 0.0,
                "credentials_storage": "none",
                "credential_persistence_allowed": false
              }
            }
            """.utf8
        )

        let envelope = try JSONDecoder().decode(ServiceEnvelope<LLMStatus>.self, from: data)
        guard let status = envelope.result else {
            throw NativeModelTestFailure(description: "LLM status service envelope should include a result.")
        }

        try expectEqual(envelope.ok, true, "LLM status service envelope should decode ok.")
        try expectEqual(status.enabled, false, "Real service LLM status should decode enabled.")
        try expectEqual(status.provider, nil, "Real service LLM status should decode provider.")
        try expectEqual(status.model, nil, "Real service LLM status should decode model.")
        try expectEqual(
            status.disabledReason,
            "LLM actions are disabled by default; no local provider is configured.",
            "Real service LLM status should decode reason as disabled reason."
        )
        try expectEqual(
            status.supportedActions,
            LLMAction.allCases,
            "Real service LLM status should default supported actions when omitted."
        )
    }

    private func prepareResultDecodesEstimatePayload() throws {
        let data = Data(
            """
            {
              "action": "draft_frontmatter",
              "allowed": true,
              "disabled_reason": null,
              "provider": "anthropic",
              "model": "claude-sonnet-4",
              "estimated_input_tokens": 320,
              "estimated_output_tokens": 180,
              "estimated_total_tokens": 500,
              "estimated_cost_usd": 0.0125,
              "requires_confirmation": true
            }
            """.utf8
        )

        let result = try JSONDecoder().decode(LLMPrepareResult.self, from: data)

        try expectEqual(result.action, .draftFrontmatter, "LLM prepare result should decode action.")
        try expectEqual(result.enabled, true, "LLM prepare result should decode enabled.")
        try expectEqual(result.provider, "anthropic", "LLM prepare result should decode provider.")
        try expectEqual(result.model, "claude-sonnet-4", "LLM prepare result should decode model.")
        try expectEqual(result.estimate?.inputTokens, 320, "LLM prepare result should decode input tokens.")
        try expectEqual(result.estimate?.outputTokens, 180, "LLM prepare result should decode output tokens.")
        try expectEqual(result.estimate?.totalTokens, 500, "LLM prepare result should decode total tokens.")
        try expectEqual(result.estimate?.estimatedCostUSD, 0.0125, "LLM prepare result should decode estimated cost.")
        try expectEqual(result.confirmationRequired, true, "LLM prepare result should decode confirmation requirement.")
    }

    private func promptPreviewDecodesV242Payload() throws {
        let data = Data(
            """
            {
              "preview_id": "preview-1",
              "request_kind": "action",
              "action": "analyze",
              "scope": "selected",
              "prompt_scope": "Selected skill analysis",
              "provider": "openai-compatible",
              "model": "gpt-5",
              "destination_host": "llm.example.com",
              "included_fields": [{"name":"skill.name","label":"Skill name"}, "findings.summary"],
              "excluded_fields": [{"name":"api_key","reason":"credential"}],
              "redaction": {
                "status": "redacted",
                "summary": "Secrets and local paths removed.",
                "redacted_fields": ["api_key"],
                "placeholders": ["<project-root>"]
              },
              "estimate": {
                "input_tokens": 600,
                "output_tokens": 300,
                "total_tokens": 900,
                "estimated_cost_usd": 0.012
              },
              "confirmation_required": true,
              "raw_prompt_persisted": false,
              "raw_response_persisted": false,
              "draft_copy_only": true,
              "redacted_prompt_preview": "Analyze Beta without paths."
            }
            """.utf8
        )

        let preview = try JSONDecoder().decode(LLMPromptPreview.self, from: data)

        try expectEqual(preview.previewID, "preview-1", "Prompt preview should decode preview id.")
        try expectEqual(preview.action, .analyze, "Prompt preview should decode action kind.")
        try expectEqual(preview.analysisKind, nil, "Action prompt preview should not require an analysis kind.")
        try expectEqual(preview.promptScope, "Selected skill analysis", "Prompt preview should decode prompt scope.")
        try expectEqual(preview.destinationHost, "llm.example.com", "Prompt preview should decode destination host.")
        try expectEqual(preview.includedFields.count, 2, "Prompt preview should decode flexible included fields.")
        try expectEqual(preview.excludedFields.first?.reason, "credential", "Prompt preview should decode excluded field reason.")
        try expectEqual(preview.redaction.redactedFields, ["api_key"], "Prompt preview should decode redaction fields.")
        try expectEqual(preview.estimate?.totalTokens, 900, "Prompt preview should decode token estimate.")
        try expectFalse(preview.rawPromptPersisted, "Prompt preview should keep raw prompt persistence false.")
        try expectFalse(preview.rawResponsePersisted, "Prompt preview should keep raw response persistence false.")
        try expectEqual(preview.promptPreview, "Analyze Beta without paths.", "Prompt preview should decode redacted prompt text.")
    }

    private func promptPreviewDecodesServiceArrayScopePayload() throws {
        let data = Data(
            """
            {
              "preview_id": "prompt-preview-action",
              "status": "blocked",
              "allowed": false,
              "reason": "No enabled provider profile is configured; no provider request can be sent.",
              "action": "analyze",
              "provider": null,
              "model": null,
              "destination_host": null,
              "prompt_scope": [
                "operation metadata",
                "selected skill analysis",
                "safety flags"
              ],
              "included_fields": ["skill id", "catalog evidence"],
              "excluded_fields": ["raw skill body", "provider API key"],
              "redaction": {
                "status": "redacted-preview-confirmed-required",
                "redacted_fields": ["local paths"],
                "placeholders": ["$HOME", "<redacted>"]
              },
              "prompt_preview": "Analysis evidence with redacted local paths.",
              "estimated_input_tokens": 480,
              "estimated_output_tokens": 650,
              "estimated_total_tokens": 1130,
              "estimated_cost_usd": 0.0,
              "requires_confirmation": true,
              "draft_requires_user_copy": true,
              "raw_prompt_persisted": false,
              "raw_response_persisted": false
            }
            """.utf8
        )

        let preview = try JSONDecoder().decode(LLMPromptPreview.self, from: data)

        try expectEqual(preview.previewID, "prompt-preview-action", "Prompt preview should decode service preview id.")
        try expectEqual(preview.enabled, false, "Blocked preview should not be sendable.")
        try expectContains(preview.promptScope, "selected skill analysis", "Prompt scope should decode array labels.")
        try expectEqual(preview.includedFields.map(\.name), ["skill id", "catalog evidence"], "Prompt preview should decode string field arrays.")
        try expectEqual(preview.estimate?.totalTokens, 1130, "Prompt preview should decode top-level token estimates.")
        try expectFalse(preview.rawPromptPersisted, "Prompt preview must not persist raw prompt.")
        try expectFalse(preview.rawResponsePersisted, "Prompt preview must not persist raw response.")
    }

    private func promptSendResultDecodesCopyOnlyAuditPayload() throws {
        let data = Data(
            """
            {
              "preview_id": "preview-1",
              "status": "succeeded",
              "message": "Done.",
              "output_text": "Read-only recommendation.",
              "draft_copy_only": true,
              "raw_prompt_persisted": false,
              "raw_response_persisted": false,
              "write_back_allowed": false,
              "script_execution_allowed": false,
              "audit_metadata": {
                "request_id": "audit-42",
                "status": "succeeded",
                "provider": "openai-compatible",
                "model": "gpt-5",
                "destination_host": "llm.example.com",
                "redaction_applied": true,
                "raw_prompt_persisted": false,
                "raw_response_persisted": false,
                "input_tokens": 600,
                "output_tokens": 120
              }
            }
            """.utf8
        )

        let result = try JSONDecoder().decode(LLMPromptSendResult.self, from: data)

        try expectEqual(result.success, true, "Prompt send should decode succeeded status.")
        try expectEqual(result.outputText, "Read-only recommendation.", "Prompt send should decode copy-only output.")
        try expectEqual(result.draftCopyOnly, true, "Prompt send should remain copy-only.")
        try expectFalse(result.rawPromptPersisted, "Prompt send should keep raw prompt persistence false.")
        try expectFalse(result.rawResponsePersisted, "Prompt send should keep raw response persistence false.")
        try expectFalse(result.writeBackAllowed, "Prompt send must not allow write-back.")
        try expectFalse(result.scriptExecutionAllowed, "Prompt send must not allow script execution.")
        try expectEqual(result.audit?.auditID, "audit-42", "Prompt send should decode audit metadata.")
    }

    private func promptSendResultUsesAuditErrorMessage() throws {
        let data = Data(
            """
            {
              "preview_id": "preview-timeout",
              "status": "failed",
              "draft_copy_only": true,
              "raw_prompt_persisted": false,
              "raw_response_persisted": false,
              "write_back_allowed": false,
              "script_execution_allowed": false,
              "audit": {
                "status": "failed",
                "provider_type": "openai-compatible",
                "model": "deepseek-v4-flash",
                "destination_host": "llm.example.com",
                "redaction_status": "redacted-preview-confirmed-required",
                "raw_prompt_persisted": false,
                "raw_response_persisted": false,
                "estimated_input_tokens": 1325,
                "estimated_output_tokens": 650,
                "error_code": "network_error",
                "error_message": "timed out reading response"
              }
            }
            """.utf8
        )

        let result = try JSONDecoder().decode(LLMPromptSendResult.self, from: data)

        try expectFalse(result.success, "Failed prompt send should not decode as success.")
        try expectEqual(result.message, "network_error: timed out reading response", "Prompt send should surface audit error details.")
        try expectEqual(result.audit?.errorCode, "network_error", "Prompt send should decode audit error code.")
        try expectEqual(result.audit?.errorMessage, "timed out reading response", "Prompt send should decode audit error message.")
    }

    private func promptSendResultDecodesServiceDraftOutput() throws {
        let data = Data(
            """
            {
              "preview_id": "preview-draft-output",
              "status": "succeeded",
              "output_text": "",
              "draft_output": "Copy-only provider analysis.",
              "draft_copy_only": true,
              "raw_prompt_persisted": false,
              "raw_response_persisted": false,
              "write_back_allowed": false,
              "script_execution_allowed": false
            }
            """.utf8
        )

        let result = try JSONDecoder().decode(LLMPromptSendResult.self, from: data)

        try expectEqual(result.success, true, "Prompt send should decode service success status.")
        try expectEqual(
            result.outputText,
            "Copy-only provider analysis.",
            "Prompt send should expose service draft_output as visible copy-only output."
        )
        try expectEqual(result.draftCopyOnly, true, "Prompt send draft_output should remain copy-only.")
        try expectFalse(result.rawPromptPersisted, "Prompt send must not persist raw prompts.")
        try expectFalse(result.rawResponsePersisted, "Prompt send must not persist raw responses.")
    }

    private func promptRunListIgnoresDeprecatedBodyFields() throws {
        let data = Data(
            """
            {
              "count": 1,
              "total_count": 3,
              "returned_count": 1,
              "limit": 1,
              "truncated": true,
              "runs": [
                {
                  "run_id": "run-1",
                  "preview_id": "preview-1",
                  "confirmation_id": "confirm-1",
                  "action": "task_cockpit",
                  "request_kind": "task_cockpit",
                  "analysis_kind": null,
                  "scope": "single-skill",
                  "instance_id": "skill-1",
                  "instance_ids": ["skill-1"],
                  "task": "Review release readiness",
                  "provider": "openai-compatible",
                  "model": "gpt-5",
                  "destination_host": "llm.example.com",
                  "status": "succeeded",
                  "message": "Provider request completed.",
                  "error_code": null,
                  "error_message": null,
                  "duration_ms": 1234,
                  "input_tokens": 600,
                  "output_tokens": 120,
                  "estimated_cost_usd": 0.012,
                  "draft_output": "Copy-only persisted explanation.",
                  "draft_copy_only": true,
                  "raw_prompt_persisted": false,
                  "raw_response_persisted": false,
                  "raw_secret_returned": false,
                  "redaction": {
                    "status": "redacted",
                    "redacted_fields": ["local paths"],
                    "placeholders": ["<project-root>"]
                  },
                  "safety_flags": {
                    "provider_request_sent": true,
                    "write_back_allowed": false,
                    "script_execution_allowed": false,
                    "config_mutation_allowed": false,
                    "snapshot_created": false,
                    "triage_mutation_allowed": false,
                    "credential_accessed": false,
                    "raw_prompt_persisted": false,
                    "raw_response_persisted": false,
                    "raw_secret_returned": false,
                    "cloud_sync_enabled": false,
                    "telemetry_enabled": false
                  },
                  "created_at": 1781260000000,
                  "completed_at": 1781260001234
                }
              ],
              "provider_request_sent": false,
              "raw_prompt_persisted": false,
              "raw_response_persisted": false,
              "raw_secret_returned": false
            }
            """.utf8
        )

        let list = try JSONDecoder().decode(LLMPromptRunListResult.self, from: data)
        guard let run = list.runs.first else {
            throw NativeModelTestFailure(description: "Prompt run list should decode a run.")
        }
        let sendResult = run.sendResult

        try expectEqual(list.runs.count, 1, "Prompt run list should decode runs.")
        try expectEqual(list.count, 1, "Prompt run list should decode returned count.")
        try expectEqual(list.totalCount, 3, "Prompt run list should decode total count.")
        try expectEqual(list.returnedCount, 1, "Prompt run list should decode explicit returned count.")
        try expectEqual(list.limit, 1, "Prompt run list should decode applied limit.")
        try expectEqual(list.truncated, true, "Prompt run list should decode truncation.")
        try expectEqual(run.requestKind, "task_cockpit", "Prompt run should decode request kind.")
        try expectEqual(run.task, nil, "Prompt-run metadata must ignore deprecated task text.")
        try expectEqual(sendResult.outputText, nil, "Prompt-run metadata must not rehydrate provider output.")
        try expectFalse(
            run.draftRequiresUserCopy,
            "Prompt-run metadata must normalize deprecated copy-only state to false."
        )
        try expectFalse(sendResult.rawPromptPersisted, "Prompt run must not persist raw prompts.")
        try expectFalse(sendResult.rawResponsePersisted, "Prompt run must not persist raw responses.")
        try expectFalse(run.rawSecretReturned, "Prompt run must not return raw secrets.")
    }

    private func markdownRenderDocumentParsesModelOutputBlocks() throws {
        let document = MarkdownRenderDocument(
            text: """
            ## Result

            - First finding
            > Keep this copy-only.

            | Field | Value |
            | --- | --- |
            | Score | **High** |

            ```text
            raw-id
            ```
            """,
            maxBlocks: nil
        )

        try expectFalse(document.isTruncated, "Full Markdown details should not be truncated.")
        try expectEqual(
            document.blocks.contains { block in
                if case let .heading(level, value) = block {
                    return level == 2 && value == "Result"
                }
                return false
            },
            true,
            "Markdown renderer should parse model headings."
        )
        try expectEqual(
            document.blocks.contains { block in
                if case let .bullet(value) = block {
                    return value == "First finding"
                }
                return false
            },
            true,
            "Markdown renderer should parse model bullet lists."
        )
        try expectEqual(
            document.blocks.contains { block in
                if case let .quote(value) = block {
                    return value == "Keep this copy-only."
                }
                return false
            },
            true,
            "Markdown renderer should parse model block quotes."
        )
        try expectEqual(
            document.blocks.contains { block in
                if case let .table(rows) = block {
                    return rows.first == ["Field", "Value"] && rows.last == ["Score", "**High**"]
                }
                return false
            },
            true,
            "Markdown renderer should parse model tables."
        )
        try expectEqual(
            document.blocks.contains { block in
                if case let .code(value) = block {
                    return value == "raw-id"
                }
                return false
            },
            true,
            "Markdown renderer should parse model fenced code blocks."
        )
    }

    private func longTextReviewBlockDefaultsToMarkdown() throws {
        let isMarkdown: Bool
        if case .markdown = LongTextReviewPresentation.defaultRenderMode {
            isMarkdown = true
        } else {
            isMarkdown = false
        }
        try expectEqual(isMarkdown, true, "Model long-text previews should render Markdown by default.")
    }

    private func markdownRenderDocumentUnwrapsWholeMarkdownFence() throws {
        let output = """
        ```markdown
        ## 结论
        ce:compound 当前质量评分 51/100。

        | 组件 | 得分 | 关键问题 | 证据 |
        | --- | --- | --- | --- |
        | 权限元数据 | 5/20 | 权限元数据为空。 | finding:permissions |
        ```
        """

        let document = MarkdownRenderDocument(text: output, maxBlocks: nil)

        try expectEqual(
            MarkdownRenderDocument.renderableText(from: output).contains("```markdown"),
            false,
            "Whole-response Markdown fences should be removed before rendering provider output."
        )
        try expectEqual(
            document.blocks.contains { block in
                if case let .heading(_, value) = block {
                    return value == "结论"
                }
                return false
            },
            true,
            "Unwrapped provider Markdown should render headings instead of one giant code block."
        )
        try expectEqual(
            document.blocks.contains { block in
                if case let .table(rows) = block {
                    return rows.first == ["组件", "得分", "关键问题", "证据"]
                }
                return false
            },
            true,
            "Unwrapped provider Markdown tables should remain parseable for card fallback."
        )

        let code = """
        ```swift
        print("keep as code")
        ```
        """
        let codeDocument = MarkdownRenderDocument(text: code, maxBlocks: nil)
        try expectEqual(
            codeDocument.blocks.contains { block in
                if case let .code(value) = block {
                    return value.contains("print")
                }
                return false
            },
            true,
            "Real language code fences should stay code blocks."
        )
    }

    private func markdownRenderDocumentNormalizesCollapsedModelMarkdown() throws {
        let output = "# 技能质量评估草稿指导 ## 概述 技能 `ce:compound` 当前质量评分 **51 / 100**。 ## 组件分析 | 组件 | 得分 | 关键问题 | |------|------|----------| | 元数据完整性 | 25/25 | 本地名称、描述、frontmatter 和正文指导均符合预期，无扣分项。 | | 权限清晰度 | 5/20 | 权限元数据为空或不可用；未找到显式工具允许列表；网络访问意图未知。 | ## 证据说明 - **发现 `name.canonical-case`**：技能名称不是规范形式。 - **适配器诊断**：状态 verified。"

        let document = MarkdownRenderDocument(text: output, maxBlocks: nil)

        try expectEqual(
            document.blocks.contains { block in
                if case let .heading(level, value) = block {
                    return level == 1 && value == "技能质量评估草稿指导"
                }
                return false
            },
            true,
            "Collapsed model Markdown should recover the top-level heading."
        )
        try expectEqual(
            document.blocks.contains { block in
                if case let .heading(level, value) = block {
                    return level == 2 && value == "组件分析"
                }
                return false
            },
            true,
            "Collapsed model Markdown should split inline section headings before tables."
        )
        try expectEqual(
            document.blocks.contains { block in
                if case let .table(rows) = block {
                    return rows.count == 3
                        && rows.first == ["组件", "得分", "关键问题"]
                        && rows.dropFirst().contains(["权限清晰度", "5/20", "权限元数据为空或不可用；未找到显式工具允许列表；网络访问意图未知。"])
                        && !rows.contains(["------", "------", "----------"])
                }
                return false
            },
            true,
            "Collapsed model Markdown tables should recover row boundaries."
        )
        try expectEqual(
            document.blocks.contains { block in
                if case let .bullet(value) = block {
                    return value.contains("name.canonical-case")
                }
                return false
            },
            true,
            "Collapsed model Markdown should recover bullets after a table."
        )
    }

    private func markdownTableDisplayModelKeepsReadableColumns() throws {
        let rows = [
            ["Field", "Value"],
            ["Score", "**High**"],
            ["Reason", "Local evidence is clear."],
        ]

        let model = MarkdownTableDisplayModel(rows: rows, maxVisibleRows: nil)

        try expectEqual(model.usesCardLayout, false, "Compact two-column AI output tables should keep the normal table layout.")
        try expectEqual(model.displayRows.count, rows.count, "Two-column AI output tables should preserve visible rows.")
        try expectFalse(
            model.columnWidth(at: 0) < MarkdownTableDisplayModel.minimumColumnWidth,
            "AI output table columns should keep a readable minimum width instead of collapsing into vertical text."
        )
    }

    private func markdownWideTableDisplayModelUsesCardLayout() throws {
        let rows = [
            ["组件", "得分", "关键问题", "元数据完整性", "权限清晰度", "风险发现"],
            ["整体", "51/100", "主要集中在权限声明缺失、风险发现未充分处理以及跨代理重复问题。", "25/25", "5/20", "4/25"],
            ["本地名称", "25/25", "描述、frontmatter 和文档导向符合预期。", "25/25", "5/20", "4/25"],
            ["权限元数据", "5/20", "权限元数据为空或不可用。", "25/25", "5/20", "4/25"],
            ["风险发现", "4/25", "存在 2 条相关发现，本地文本信号进一步降低风险得分。", "25/25", "5/20", "4/25"],
            ["适配器状态", "12/15", "适配器状态为 verified，读写、安装均正常。", "25/25", "5/20", "4/25"],
        ]

        let compact = MarkdownTableDisplayModel(rows: rows, maxVisibleRows: 4)
        let full = MarkdownTableDisplayModel(rows: rows, maxVisibleRows: nil)

        try expectEqual(compact.usesCardLayout, true, "Wide AI output tables should render as readable cards instead of a horizontal grid.")
        try expectEqual(compact.bodyRowCount, 5, "AI output table summaries should count body rows separately from the header.")
        try expectEqual(compact.displayCardRows.count, 3, "Compact AI output table cards should reserve one row for headers and show bounded body rows.")
        try expectEqual(compact.hiddenRowCount, 2, "Compact AI output tables should report hidden rows for the details sheet.")
        try expectEqual(compact.columnCount, 6, "Table layout should preserve all model-returned columns.")
        try expectEqual(full.displayCardRows.count, rows.count - 1, "Full AI output details should keep every table body row as cards.")
        try expectEqual(full.hiddenRowCount, 0, "Full AI output details should not hide rows.")
    }

    private func markdownThreeColumnQualityTableUsesCardLayoutWhenCellsAreLong() throws {
        let rows = [
            ["组件", "得分", "关键问题"],
            ["权限清晰度", "5/20", "权限元数据为空或不可用；未找到显式工具允许列表；网络访问意图未知。"],
            ["风险发现", "4/25", "存在 2 条相关发现，本地文本信号进一步降低了风险得分。"],
        ]

        let model = MarkdownTableDisplayModel(rows: rows, maxVisibleRows: nil)

        try expectEqual(model.usesCardLayout, true, "Three-column quality-score tables with long issue text should render as cards.")
        try expectEqual(model.displayCardRows.count, 2, "Quality-score table cards should include all body rows in details.")
    }

}
