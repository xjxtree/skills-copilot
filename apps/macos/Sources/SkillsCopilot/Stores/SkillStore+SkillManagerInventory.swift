import Foundation

@MainActor
private final class SkillManagerInventoryLoadOutcome {
    var succeeded = false
}

@MainActor
extension SkillStore {
    func refreshSkillManagerData() async {
        clearSkillManagerFeedback()
        await loadSkillManagerTools()
        guard !Task.isCancelled else { return }
        _ = await loadSkillManagerInventory()
    }

    func listSkillManagerInstalled() async {
        _ = await loadSkillManagerInventory()
    }

    @discardableResult
    func loadSkillManagerInventory(
        invalidateFailedScopes: Bool = false
    ) async -> Bool {
        let generation = beginSkillManagerInstalledList(for: .installedInventory)
        let outcome = SkillManagerInventoryLoadOutcome()
        clearSkillManagerFeedback()
        let service = service
        let task = Task { @MainActor [weak self, service] in
            guard let self else { return }
            defer { self.finishSkillManagerInstalledList(generation) }
            var next = self.skillManagerInstalledByScope
            var firstError: Error?
            for scope in [SkillManagerScope.project, .global] {
                guard self.currentSkillManagerInstalledGeneration == generation,
                      !Task.isCancelled else { return }
                do {
                    let result = try await service.listSkillManagerInstalled(scope: scope)
                    guard self.currentSkillManagerInstalledGeneration == generation,
                          !Task.isCancelled else { return }
                    next[scope] = result
                    self.skillManagerInstalledByScope = next
                } catch {
                    guard !(error is CancellationError), !Task.isCancelled else { return }
                    firstError = firstError ?? error
                    if invalidateFailedScopes {
                        next.removeValue(forKey: scope)
                        self.skillManagerInstalledByScope = next
                    }
                }
            }
            if let firstError,
               self.currentSkillManagerInstalledGeneration == generation {
                self.setSkillManagerError(firstError.localizedDescription)
            } else if self.currentSkillManagerInstalledGeneration == generation {
                outcome.succeeded = true
            }
        }
        let handle = SkillManagerRequestTaskHandle(task: task)
        skillManagerInstalledTask = handle
        await handle.wait()
        if Task.isCancelled, currentSkillManagerInstalledGeneration == generation {
            invalidateSkillManagerInstalledList()
        }
        return outcome.succeeded
            && currentSkillManagerInstalledGeneration == generation
            && !Task.isCancelled
    }

    func ensureSkillManagerInventoryRefreshedAfterWrite() async -> Bool {
        guard await loadSkillManagerInventory(invalidateFailedScopes: true) else {
            let detail = skillManagerErrorMessage
                ?? UIStrings.text(
                    "skillManager.inventory.reloadUnknown",
                    "The installed inventory refresh did not complete."
                )
            setSkillManagerError(String(
                format: UIStrings.text(
                    "skillManager.inventory.reloadAfterWriteFailed",
                    "The operation was applied, but the installed inventory could not be reloaded: %@"
                ),
                detail
            ))
            return false
        }
        return true
    }

    func invalidateSkillManagerInstalledList() {
        skillManagerInstalledTask?.cancel()
        skillManagerInstalledTask = nil
        skillManagerInstalledGenerationValue &+= 1
        currentSkillManagerInstalledGeneration = nil
        isListingSkillManagerInstalled = false
    }

    private func beginSkillManagerInstalledList(
        for key: SkillManagerRequestKey
    ) -> SkillManagerRequestGeneration {
        skillManagerInstalledTask?.cancel()
        skillManagerInstalledTask = nil
        skillManagerInstalledGenerationValue &+= 1
        let generation = SkillManagerRequestGeneration(
            value: skillManagerInstalledGenerationValue,
            key: key
        )
        currentSkillManagerInstalledGeneration = generation
        isListingSkillManagerInstalled = true
        return generation
    }

    private func finishSkillManagerInstalledList(_ generation: SkillManagerRequestGeneration) {
        guard currentSkillManagerInstalledGeneration == generation else { return }
        skillManagerInstalledTask = nil
        isListingSkillManagerInstalled = false
    }
}
