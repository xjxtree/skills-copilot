import AppKit
import SwiftUI

struct LocalSkillMapSelectedContext: View {
    let skill: SkillRecord
    let selectedSkill: CapabilityTaxonomySkill?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.localSkillMapSelectedContext)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            DetailMetricGrid(minColumnWidth: 150, spacing: 8) {
                SummaryChip(title: UIStrings.text("metadata.name", "Name"), value: selectedSkill?.skillName ?? skill.name, systemImage: "target")
                SummaryChip(title: UIStrings.agent, value: DisplayText.agent(selectedSkill?.agent ?? skill.agent), systemImage: "person.crop.circle")
                SummaryChip(title: UIStrings.scope, value: selectedSkill?.scope ?? DisplayText.scope(for: skill), systemImage: "folder")
                SummaryChip(title: UIStrings.definition, value: selectedSkill?.definitionID ?? skill.definitionId, systemImage: "number")
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }
}

struct LocalSkillMapResultView: View {
    let result: LocalSkillMapResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.localSkillMapNodes, value: "\(nodeCount)", systemImage: "circle.grid.cross")
                SummaryChip(title: UIStrings.localSkillMapEdges, value: "\(edgeCount)", systemImage: "arrow.triangle.branch")
                SummaryChip(title: UIStrings.localSkillMapClusters, value: "\(clusterCount)", systemImage: "square.grid.3x3")
                SummaryChip(title: UIStrings.agent, value: "\(agentCount)", systemImage: "person.3")
                SummaryChip(title: UIStrings.knowledgeGapNotes, value: "\(gapCount)", systemImage: "puzzlepiece.extension")
                SummaryChip(title: UIStrings.knowledgeBlockerNotes, value: "\(blockerCount)", systemImage: "exclamationmark.octagon")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: agentFilterLabel)
                if let selectedSkillID = result.filters.selectedSkillID, !selectedSkillID.isEmpty {
                    MetadataRow(label: UIStrings.localSkillMapSelectedContext, value: result.filters.selectedSkillName ?? selectedSkillID)
                } else if let selectedSkillContext = result.summary.selectedSkillContext, !selectedSkillContext.isEmpty {
                    MetadataRow(label: UIStrings.localSkillMapSelectedContext, value: selectedSkillContext)
                }
                if let projectRoot = result.filters.projectRoot, !projectRoot.isEmpty {
                    PrivacyPathRow(label: UIStrings.text("projectContext.root", "Project root"), path: projectRoot)
                }
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                if let promptRequest = result.promptRequest {
                    MetadataRow(label: UIStrings.routingAccuracyPromptRequest, value: promptRequestLabel(promptRequest))
                }
            }

            if !result.summary.summaryText.isEmpty {
                Text(result.summary.summaryText)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            LocalSkillMapNodeList(nodes: result.nodes)
            LocalSkillMapEdgeList(edges: result.edges)
            LocalSkillMapClusterList(clusters: result.clusters)
            LocalSkillMapIssueList(title: UIStrings.knowledgeGapNotes, rows: result.gapRows, empty: UIStrings.routingAccuracyNoGaps, systemImage: "puzzlepiece.extension")
            LocalSkillMapIssueList(title: UIStrings.knowledgeBlockerNotes, rows: result.blockerRows, empty: UIStrings.routingAccuracyNoBlockers, systemImage: "exclamationmark.octagon")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var nodeCount: Int {
        result.summary.nodeCount > 0 ? result.summary.nodeCount : result.nodes.count
    }

    private var edgeCount: Int {
        result.summary.edgeCount > 0 ? result.summary.edgeCount : result.edges.count
    }

    private var clusterCount: Int {
        result.summary.clusterCount > 0 ? result.summary.clusterCount : result.clusters.count
    }

    private var agentCount: Int {
        result.summary.agentCount > 0 ? result.summary.agentCount : Set(result.nodes.compactMap(\.agent)).count
    }

    private var gapCount: Int {
        result.summary.gapCount > 0 ? result.summary.gapCount : result.gapRows.count
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerRows.count
    }

    private var agentFilterLabel: String {
        if !result.filters.agents.isEmpty {
            return result.filters.agents.map(DisplayText.agent).joined(separator: ", ")
        }
        return result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")
    }

    private func promptRequestLabel(_ promptRequest: LocalSkillMapPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct LocalSkillMapNodeList: View {
    let nodes: [LocalSkillMapNode]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.localSkillMapNodes)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if nodes.isEmpty {
                Text(UIStrings.localSkillMapNoNodes)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 280), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(nodes.prefix(8)) { node in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Label(node.label, systemImage: iconName(for: node.kind))
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text(node.kind)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }

                            if !node.summary.isEmpty {
                                PrivacyEvidenceText(value: node.summary, font: .caption, lineLimit: nil)
                            }

                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                                MetadataRow(label: UIStrings.agent, value: node.agent.map(DisplayText.agent) ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.scope, value: node.scope ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.state, value: node.statusLabel)
                                if let riskLevel = node.riskLevel, !riskLevel.isEmpty {
                                    MetadataRow(label: UIStrings.text("quality.riskLevel", "Risk level"), value: riskLevel)
                                }
                                if let domain = node.domain, !domain.isEmpty {
                                    MetadataRow(label: UIStrings.capabilityTaxonomyDomain, value: domain)
                                }
                                if let weight = node.weight {
                                    MetadataRow(label: UIStrings.localSkillMapStrength, value: RoutingAccuracySummary.confidenceLabel(weight))
                                }
                            }

                            KnowledgeTokenFlow(title: UIStrings.text("knowledge.tags", "Tags"), values: node.tags)
                            RoutingInlineList(title: UIStrings.routingConfidenceMatchReasons, empty: UIStrings.routingConfidenceNoReasons, values: node.reasons, systemImage: "text.bubble")
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: node.evidenceRefs, systemImage: "checklist")
                            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: node.safetyFlags, systemImage: "checkmark.shield")
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }

    private func iconName(for kind: String) -> String {
        let value = kind.lowercased()
        if value.contains("domain") { return "square.grid.3x3.topleft.filled" }
        if value.contains("agent") { return "person.crop.circle" }
        if value.contains("capability") { return "tag" }
        return "doc.text"
    }
}

struct LocalSkillMapEdgeList: View {
    let edges: [LocalSkillMapEdge]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.localSkillMapEdges)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if edges.isEmpty {
                Text(UIStrings.localSkillMapNoEdges)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 280), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(edges.prefix(8)) { edge in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Label(edge.label, systemImage: "arrow.triangle.branch")
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text(edge.relation)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }

                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                                MetadataRow(label: UIStrings.localSkillMapRelation, value: relationText(edge))
                                if let strength = edge.strength {
                                    MetadataRow(label: UIStrings.localSkillMapStrength, value: RoutingAccuracySummary.confidenceLabel(strength))
                                }
                                if let direction = edge.direction, !direction.isEmpty {
                                    MetadataRow(label: UIStrings.localSkillMapDirection, value: direction)
                                }
                            }

                            RoutingInlineList(title: UIStrings.routingConfidenceMatchReasons, empty: UIStrings.routingConfidenceNoReasons, values: edge.reasons, systemImage: "text.bubble")
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: edge.evidenceRefs, systemImage: "checklist")
                            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: edge.safetyFlags, systemImage: "checkmark.shield")
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }

    private func relationText(_ edge: LocalSkillMapEdge) -> String {
        let endpoints = [edge.sourceID, edge.targetID].compactMap { $0 }.joined(separator: " -> ")
        return endpoints.isEmpty ? edge.relation : "\(endpoints) · \(edge.relation)"
    }
}

struct LocalSkillMapClusterList: View {
    let clusters: [LocalSkillMapCluster]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.localSkillMapClusters)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if clusters.isEmpty {
                Text(UIStrings.localSkillMapNoClusters)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 300), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(clusters.prefix(6)) { cluster in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Label(cluster.title, systemImage: "square.grid.3x3")
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text(cluster.kind)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }

                            if !cluster.summary.isEmpty {
                                PrivacyEvidenceText(value: cluster.summary, font: .caption, lineLimit: nil)
                            }

                            KnowledgeTokenFlow(title: UIStrings.localSkillMapNodeIDs, values: cluster.nodeIDs)
                            KnowledgeTokenFlow(title: UIStrings.agent, values: cluster.agents.map(DisplayText.agent))
                            KnowledgeTokenFlow(title: UIStrings.knowledgeCapabilities, values: cluster.capabilities)
                            CapabilitySkillList(skills: cluster.representativeSkills)
                            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: cluster.gapNotes, systemImage: "puzzlepiece.extension")
                            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: cluster.blockerNotes, systemImage: "exclamationmark.octagon")
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: cluster.evidenceRefs, systemImage: "checklist")
                            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: cluster.safetyFlags, systemImage: "checkmark.shield")
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }
}

struct LocalSkillMapIssueList: View {
    let title: String
    let rows: [LocalSkillMapIssueRow]
    let empty: String
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(empty)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(rows.prefix(8)) { row in
                    VStack(alignment: .leading, spacing: 4) {
                        Label(row.title, systemImage: systemImage)
                            .font(.callout)
                        PrivacyEvidenceText(value: row.detail, font: .caption, lineLimit: nil)
                        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 3) {
                            if let severity = row.severity, !severity.isEmpty {
                                MetadataRow(label: UIStrings.findingSeverityFilter, value: severity)
                            }
                            if let agent = row.agent, !agent.isEmpty {
                                MetadataRow(label: UIStrings.agent, value: DisplayText.agent(agent))
                            }
                            if let source = row.source, !source.isEmpty {
                                PrivacyPathRow(label: UIStrings.source, path: source)
                            }
                        }
                        RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                    }
                }
            }
        }
    }
}
