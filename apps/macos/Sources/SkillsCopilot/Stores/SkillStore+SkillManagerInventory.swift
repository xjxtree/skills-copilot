import Foundation

@MainActor
extension SkillStore {
    func refreshSkillManagerData() async {
        clearSkillManagerFeedback()
        await loadSkillManagerTools()
        guard !Task.isCancelled else { return }
        await loadSkillManagerInventory()
    }

    func listSkillManagerInstalled() async {
        await loadSkillManagerInventory()
    }

    func loadSkillManagerInventory(preservingVerifiedResult: Bool = false) async {
        let generation = beginSkillManagerInstalledList(for: .installedInventory)
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
                }
            }
            if let firstError,
               self.currentSkillManagerInstalledGeneration == generation {
                if preservingVerifiedResult {
                    let warning = UIStrings.text(
                        "actionLifecycle.appliedRefreshFailed",
                        "The action was verified, but the cached view could not refresh."
                    ) + " \(firstError.localizedDescription)"
                    self.skillManagerMessage = [self.skillManagerMessage, warning]
                        .compactMap { $0 }
                        .joined(separator: " ")
                } else {
                    self.setSkillManagerError(firstError.localizedDescription)
                }
            }
        }
        let handle = SkillManagerRequestTaskHandle(task: task)
        skillManagerInstalledTask = handle
        await handle.wait()
        if Task.isCancelled, currentSkillManagerInstalledGeneration == generation {
            invalidateSkillManagerInstalledList()
        }
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
