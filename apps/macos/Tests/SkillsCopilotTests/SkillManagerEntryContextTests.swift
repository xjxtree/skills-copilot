import Foundation
@testable import SkillsCopilot

struct SkillManagerEntryContextTests {
    func run() throws {
        try defaultEntryPreservesLegacyState()
        try addEntryCanonicalizesPresentationInputs()
        try packageEntriesResolveWorkflowActionAndScope()
        try utilityEntriesRemainPreviewOnly()
        try targetResolutionRequiresUniqueBestCandidate()
    }

    private func defaultEntryPreservesLegacyState() throws {
        let presentation = SkillManagerEntryContext.default.presentation
        try expectEqual(presentation.workflow, .searchInstall, "Default workflow")
        try expectNil(presentation.preferredAction, "Default action")
        try expectEqual(presentation.scope, .project, "Default scope")
        try expectNil(presentation.agentIDs, "Default agents remain derived")
        try expectEqual(presentation.inventoryQuery, "", "Default inventory query")
        try expectNil(presentation.searchQuery, "Default search query")
        try expectNil(presentation.focusedInput, "Default focus")
        try expectFalse(presentation.requestsImportArchive, "Default import request")
    }

    private func addEntryCanonicalizesPresentationInputs() throws {
        let presentation = SkillManagerEntryContext.add(
            query: "  lint tools  ",
            scope: .global,
            agentIDs: ["codex", "pi", "codex", "unsupported", " "]
        ).presentation
        try expectEqual(presentation.workflow, .searchInstall, "Add workflow")
        try expectEqual(presentation.scope, .global, "Add scope")
        try expectEqual(presentation.agentIDs, Set(["codex", "pi"]), "Add agents")
        try expectEqual(presentation.searchQuery, "lint tools", "Add query")
        try expectEqual(presentation.focusedInput, .search, "Add focus")
        try expectNil(presentation.preferredAction, "Add awaits a search result")
        try expectEqual(
            SkillManagerEntryContext.managerAgentIDs(
                for: [.hermes, .toolGlobal, .claudeCode]
            ),
            ["hermes-agent", "claude-code"],
            "Product agents map to manager identifiers without tool-global"
        )
    }

    private func packageEntriesResolveWorkflowActionAndScope() throws {
        let target = SkillManagerPackageTarget(
            name: "Formatter",
            instanceIDs: ["formatter-project"],
            scope: .project
        )
        let detail = SkillManagerEntryContext.packageDetail(target: target).presentation
        try expectEqual(detail.workflow, .installedUpdates, "Package detail workflow")
        try expectNil(detail.preferredAction, "Package detail action")
        try expectEqual(detail.scope, .project, "Package detail scope")
        try expectEqual(detail.inventoryQuery, "Formatter", "Package detail query")

        let updateContext = SkillManagerEntryContext.update(
            target: target,
            scope: .global
        )
        let update = updateContext.presentation
        try expectEqual(update.workflow, .installedUpdates, "Update workflow")
        try expectEqual(update.preferredAction, .update, "Update action")
        try expectEqual(update.scope, .global, "Explicit update scope")
        try expectEqual(
            updateContext.target?.scope,
            .global,
            "Explicit scope also binds target resolution"
        )
        try expectEqual(
            update.resolvedAction(available: [.update, .remove]),
            .update,
            "Available update action"
        )

        let remove = SkillManagerEntryContext.remove(
            target: target,
            agentIDs: ["codex"]
        ).presentation
        try expectEqual(remove.preferredAction, .remove, "Remove action")
        try expectEqual(remove.agentIDs, Set(["codex"]), "Remove agents")
        try expectEqual(
            remove.resolvedAction(available: [.install, .deleteSource]),
            .deleteSource,
            "Unlinked local removal becomes source deletion"
        )
    }

    private func utilityEntriesRemainPreviewOnly() throws {
        let localCreate = SkillManagerEntryContext.localCreate(
            suggestedName: "  my-skill  "
        ).presentation
        try expectEqual(localCreate.workflow, .searchInstall, "Local create workflow")
        try expectEqual(
            localCreate.suggestedLocalSkillName,
            "my-skill",
            "Local create suggested name"
        )
        try expectEqual(localCreate.focusedInput, .localCreate, "Local create focus")
        try expectFalse(localCreate.requestsImportArchive, "Local create import request")

        let importArchive = SkillManagerEntryContext.importArchive.presentation
        try expectEqual(importArchive.workflow, .searchInstall, "Import workflow")
        try expectEqual(importArchive.requestsImportArchive, true, "Import picker request")
        try expectNil(importArchive.preferredAction, "Import has no apply action")
    }

    private func targetResolutionRequiresUniqueBestCandidate() throws {
        let first = inventoryItem(
            name: "Formatter",
            source: "https://example.test/one",
            instanceID: nil
        )
        let second = inventoryItem(
            name: "Formatter",
            source: "https://example.test/two",
            instanceID: nil
        )
        let ambiguous = SkillManagerPackageTarget(
            name: "formatter",
            scope: .project
        )
        try expectNil(
            ambiguous.uniqueBestMatch(in: [first, second]),
            "Same-name packages stay unselected"
        )

        let exact = SkillManagerPackageTarget(
            inventoryItemID: second.id,
            name: "Formatter",
            scope: .project
        )
        try expectEqual(
            exact.uniqueBestMatch(in: [first, second])?.id,
            second.id,
            "Exact inventory identity wins over name fallbacks"
        )

        let local = inventoryItem(
            name: "Different display name",
            source: "/redacted/local",
            instanceID: "formatter-project",
            origin: .local
        )
        let byInstance = SkillManagerPackageTarget(
            name: "Formatter",
            instanceIDs: ["formatter-project"],
            scope: .project
        )
        try expectEqual(
            byInstance.uniqueBestMatch(in: [first, local])?.id,
            local.id,
            "Catalog instance identity wins over a name fallback"
        )

        let wrongScope = SkillManagerPackageTarget(
            inventoryItemID: first.id,
            name: first.name,
            scope: .global
        )
        try expectNil(
            wrongScope.uniqueBestMatch(in: [first]),
            "Package target never crosses scope"
        )
    }

    private func inventoryItem(
        name: String,
        source: String,
        instanceID: String?,
        origin: SkillManagerInventoryItem.Origin = .manager
    ) -> SkillManagerInventoryItem {
        SkillManagerInventoryItem(
            name: name,
            source: source,
            scope: .project,
            agents: ["codex"],
            origin: origin,
            localOwnership: origin == .local ? .appOwned : nil,
            localInstanceID: instanceID,
            localPath: origin == .local ? source : nil
        )
    }
}

#if canImport(XCTest)
import XCTest

final class SkillManagerEntryContextXCTests: XCTestCase {
    func testEntryContextContract() throws {
        try SkillManagerEntryContextTests().run()
    }
}
#endif
