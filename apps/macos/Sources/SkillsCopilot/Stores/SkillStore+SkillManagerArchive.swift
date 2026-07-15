import Foundation

@MainActor
extension SkillStore {
    func previewSkillManagerLocalArchiveImport(archivePath: String) async {
        let path = archivePath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !path.isEmpty else {
            setSkillManagerError(UIStrings.text("skillManager.localArchive.required", "Choose a ZIP archive."))
            return
        }
        let generation = beginSkillManagerLocalArchiveImport(
            for: .localArchiveImport(archivePath: path)
        )
        clearSkillManagerFeedback()
        let service = service
        let task = Task { @MainActor [weak self, service] in
            do {
                let result = try await service.previewSkillManagerLocalArchiveImport(
                    archivePath: path
                )
                guard let self else { return }
                defer { self.finishSkillManagerLocalArchiveImport(generation) }
                guard self.currentSkillManagerLocalArchiveImportGeneration == generation else { return }
                self.skillManagerLocalArchiveImportConfirmation = .init(
                    generation: generation,
                    archivePath: path,
                    result: result
                )
            } catch {
                guard let self else { return }
                defer { self.finishSkillManagerLocalArchiveImport(generation) }
                guard self.currentSkillManagerLocalArchiveImportGeneration == generation else { return }
                guard !(error is CancellationError), !Task.isCancelled else { return }
                self.setSkillManagerError(error.localizedDescription)
                self.skillManagerLocalArchiveImportConfirmation = nil
            }
        }
        let handle = SkillManagerRequestTaskHandle(task: task)
        skillManagerLocalArchiveImportTask = handle
        await handle.wait()
        if Task.isCancelled, currentSkillManagerLocalArchiveImportGeneration == generation {
            invalidateSkillManagerLocalArchiveImportPreview()
        }
    }

    func applySkillManagerLocalArchiveImport(
        confirmation: SkillManagerLocalArchiveImportConfirmation
    ) async {
        guard skillManagerLocalArchiveImportConfirmation == confirmation else { return }
        await runSkillManagerConfirmedWrite { [self] in
            do {
                let result = try await service.applySkillManagerLocalArchiveImport(
                    preview: confirmation.result,
                    archivePath: confirmation.archivePath
                )
                retireSkillManagerLocalArchiveImportConfirmation(confirmation)
                if let importedSkill = result.importedSkill {
                    invalidateDetailCaches(for: [importedSkill.id])
                }
                try await refreshCollections()
                await loadSkillManagerInventory()
                skillManagerMessage = UIStrings.text(
                    "skillManager.localArchive.imported",
                    "Local ZIP imported. Select the local skill in the inventory to install it for agents."
                )
                recordLocalRefresh(message: UIStrings.refreshAfterWrite)
            } catch {
                setSkillManagerError(error.localizedDescription)
            }
        }
    }

    func invalidateSkillManagerLocalArchiveImportPreview() {
        skillManagerLocalArchiveImportTask?.cancel()
        skillManagerLocalArchiveImportTask = nil
        skillManagerLocalArchiveImportGenerationValue &+= 1
        currentSkillManagerLocalArchiveImportGeneration = nil
        skillManagerLocalArchiveImportConfirmation = nil
        isPreviewingSkillManagerLocalArchiveImport = false
    }

    private func retireSkillManagerLocalArchiveImportConfirmation(
        _ confirmation: SkillManagerLocalArchiveImportConfirmation
    ) {
        guard skillManagerLocalArchiveImportConfirmation?.generation == confirmation.generation else { return }
        invalidateSkillManagerLocalArchiveImportPreview()
    }

    private func beginSkillManagerLocalArchiveImport(
        for key: SkillManagerRequestKey
    ) -> SkillManagerRequestGeneration {
        skillManagerLocalArchiveImportTask?.cancel()
        skillManagerLocalArchiveImportTask = nil
        skillManagerLocalArchiveImportGenerationValue &+= 1
        let generation = SkillManagerRequestGeneration(
            value: skillManagerLocalArchiveImportGenerationValue,
            key: key
        )
        currentSkillManagerLocalArchiveImportGeneration = generation
        skillManagerLocalArchiveImportConfirmation = nil
        isPreviewingSkillManagerLocalArchiveImport = true
        return generation
    }

    private func finishSkillManagerLocalArchiveImport(_ generation: SkillManagerRequestGeneration) {
        guard currentSkillManagerLocalArchiveImportGeneration == generation else { return }
        skillManagerLocalArchiveImportTask = nil
        isPreviewingSkillManagerLocalArchiveImport = false
    }

    func previewSkillManagerLocalArchiveUpdate(instanceID: String, archivePath: String) async {
        let path = archivePath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !path.isEmpty else {
            setSkillManagerError(UIStrings.text("skillManager.localArchive.required", "Choose a ZIP archive."))
            return
        }
        let generation = beginSkillManagerLocalArchiveUpdate(
            for: .localArchiveUpdate(instanceID: instanceID, archivePath: path)
        )
        clearSkillManagerFeedback()
        let service = service
        let task = Task { @MainActor [weak self, service] in
            do {
                let result = try await service.previewSkillManagerLocalArchiveUpdate(
                    instanceID: instanceID,
                    archivePath: path
                )
                guard let self else { return }
                defer { self.finishSkillManagerLocalArchiveUpdate(generation) }
                guard self.currentSkillManagerLocalArchiveUpdateGeneration == generation else { return }
                self.skillManagerLocalArchiveUpdateConfirmation = .init(
                    generation: generation,
                    instanceID: instanceID,
                    archivePath: path,
                    result: result
                )
            } catch {
                guard let self else { return }
                defer { self.finishSkillManagerLocalArchiveUpdate(generation) }
                guard self.currentSkillManagerLocalArchiveUpdateGeneration == generation else { return }
                guard !(error is CancellationError), !Task.isCancelled else { return }
                self.setSkillManagerError(error.localizedDescription)
                self.skillManagerLocalArchiveUpdateConfirmation = nil
            }
        }
        let handle = SkillManagerRequestTaskHandle(task: task)
        skillManagerLocalArchiveUpdateTask = handle
        await handle.wait()
        if Task.isCancelled, currentSkillManagerLocalArchiveUpdateGeneration == generation {
            invalidateSkillManagerLocalArchiveUpdatePreview()
        }
    }

    func applySkillManagerLocalArchiveUpdate(
        confirmation: SkillManagerLocalArchiveUpdateConfirmation
    ) async {
        guard skillManagerLocalArchiveUpdateConfirmation == confirmation else { return }
        await runSkillManagerConfirmedWrite { [self] in
            do {
                let result = try await service.applySkillManagerLocalArchiveUpdate(
                    preview: confirmation.result,
                    instanceID: confirmation.instanceID,
                    archivePath: confirmation.archivePath
                )
                retireSkillManagerLocalArchiveUpdateConfirmation(confirmation)
                if let updatedSkill = result.updatedSkill {
                    invalidateDetailCaches(for: [updatedSkill.id])
                }
                try await refreshCollections()
                await loadSkillManagerInventory()
                skillManagerMessage = UIStrings.text(
                    "skillManager.localArchive.applied",
                    "Local skill package updated from the ZIP archive."
                )
                recordLocalRefresh(message: UIStrings.refreshAfterWrite)
                await loadSelectedDetail()
            } catch {
                setSkillManagerError(error.localizedDescription)
            }
        }
    }

    func invalidateSkillManagerLocalArchiveUpdatePreview() {
        skillManagerLocalArchiveUpdateTask?.cancel()
        skillManagerLocalArchiveUpdateTask = nil
        skillManagerLocalArchiveUpdateGenerationValue &+= 1
        currentSkillManagerLocalArchiveUpdateGeneration = nil
        skillManagerLocalArchiveUpdateConfirmation = nil
        isPreviewingSkillManagerLocalArchiveUpdate = false
    }

    private func retireSkillManagerLocalArchiveUpdateConfirmation(
        _ confirmation: SkillManagerLocalArchiveUpdateConfirmation
    ) {
        guard skillManagerLocalArchiveUpdateConfirmation?.generation == confirmation.generation else { return }
        invalidateSkillManagerLocalArchiveUpdatePreview()
    }

    private func beginSkillManagerLocalArchiveUpdate(
        for key: SkillManagerRequestKey
    ) -> SkillManagerRequestGeneration {
        skillManagerLocalArchiveUpdateTask?.cancel()
        skillManagerLocalArchiveUpdateTask = nil
        skillManagerLocalArchiveUpdateGenerationValue &+= 1
        let generation = SkillManagerRequestGeneration(
            value: skillManagerLocalArchiveUpdateGenerationValue,
            key: key
        )
        currentSkillManagerLocalArchiveUpdateGeneration = generation
        skillManagerLocalArchiveUpdateConfirmation = nil
        isPreviewingSkillManagerLocalArchiveUpdate = true
        return generation
    }

    private func finishSkillManagerLocalArchiveUpdate(_ generation: SkillManagerRequestGeneration) {
        guard currentSkillManagerLocalArchiveUpdateGeneration == generation else { return }
        skillManagerLocalArchiveUpdateTask = nil
        isPreviewingSkillManagerLocalArchiveUpdate = false
    }
}
