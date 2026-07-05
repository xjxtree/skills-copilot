enum DetailSection: String, CaseIterable, Identifiable {
    case overview
    case findings
    case history
    case metadata

    var id: String { rawValue }

    static var visibleCases: [DetailSection] {
        [.overview, .findings, .history, .metadata]
    }

    static var primaryWorkCases: [DetailSection] {
        []
    }

    var visibleSkillDetailSection: DetailSection {
        self
    }

    var requiresSelectedSkill: Bool {
        true
    }

    var title: String {
        switch self {
        case .overview:
            return UIStrings.overview
        case .findings:
            return UIStrings.findings
        case .history:
            return UIStrings.text("detail.history", "History")
        case .metadata:
            return UIStrings.text("detail.metadata", "Metadata")
        }
    }

    var systemImage: String {
        switch self {
        case .overview:
            return "chart.pie"
        case .findings:
            return "exclamationmark.triangle"
        case .history:
            return "clock.arrow.circlepath"
        case .metadata:
            return "info.circle"
        }
    }

    var summary: String {
        switch self {
        case .overview:
            return UIStrings.text("detail.section.overview.summary", "Inspect the selected skill metadata, permissions, provenance, and raw catalog details.")
        case .findings:
            return UIStrings.text("detail.section.findings.summary", "Explain selected-skill issues with rules, suggestions, and evidence.")
        case .history:
            return UIStrings.text("detail.section.history.summary", "Review selected-skill toggle and config history.")
        case .metadata:
            return UIStrings.text("detail.section.metadata.summary", "Inspect raw catalog metadata, frontmatter, body excerpts, and adapter capability details.")
        }
    }

    var isAgentWorkspaceSurface: Bool {
        false
    }
}
