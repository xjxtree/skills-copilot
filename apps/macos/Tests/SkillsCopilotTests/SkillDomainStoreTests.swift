import Combine
import Testing
@testable import SkillsCopilot

@Suite("SkillDomainStoreTests")
@MainActor
struct SkillDomainStoreTests {
    @Test("domain notifications remain isolated")
    func domainNotificationsRemainIsolated() {
        let sessions = SessionStore()
        let providers = ProviderStore()
        let manager = SkillManagerStore()
        var sessionNotifications = 0
        var providerNotifications = 0
        var managerNotifications = 0
        let sessionObservation = sessions.objectWillChange.sink {
            sessionNotifications += 1
        }
        let providerObservation = providers.objectWillChange.sink {
            providerNotifications += 1
        }
        let managerObservation = manager.objectWillChange.sink {
            managerNotifications += 1
        }

        sessions.isPreviewingLocalSessions = true
        #expect(sessionNotifications == 1)
        #expect(providerNotifications == 0)
        #expect(managerNotifications == 0)

        providers.isLoadingProviderObservability = true
        #expect(sessionNotifications == 1)
        #expect(providerNotifications == 1)
        #expect(managerNotifications == 0)

        manager.isLoadingSkillManagerTools = true
        #expect(sessionNotifications == 1)
        #expect(providerNotifications == 1)
        #expect(managerNotifications == 1)

        withExtendedLifetime([
            sessionObservation,
            providerObservation,
            managerObservation,
        ]) {}
    }

    @Test("domain criteria callbacks stay inside their coordinator lane")
    func domainCriteriaCallbacksStayInsideTheirCoordinatorLane() {
        let sessions = SessionStore()
        let providers = ProviderStore()
        let manager = SkillManagerStore()
        var sessionSearchChanges = 0
        var providerCriteriaChanges = 0
        var managerSearchChanges = 0
        var managerMutationChanges = 0
        sessions.onSearchChanged = { sessionSearchChanges += 1 }
        providers.onObservabilityCriteriaChanged = { providerCriteriaChanges += 1 }
        manager.onSearchCriteriaChanged = { managerSearchChanges += 1 }
        manager.onMutationCriteriaChanged = { managerMutationChanges += 1 }

        sessions.localSessionSearchText = "review"
        #expect(sessionSearchChanges == 1)
        #expect(providerCriteriaChanges == 0)
        #expect(managerSearchChanges == 0)
        #expect(managerMutationChanges == 0)

        providers.providerObservabilityDateRange = .last7Days
        #expect(sessionSearchChanges == 1)
        #expect(providerCriteriaChanges == 1)
        #expect(managerSearchChanges == 0)
        #expect(managerMutationChanges == 0)

        manager.skillManagerNetworkAllowed = false
        #expect(sessionSearchChanges == 1)
        #expect(providerCriteriaChanges == 1)
        #expect(managerSearchChanges == 1)
        #expect(managerMutationChanges == 1)
    }
}
