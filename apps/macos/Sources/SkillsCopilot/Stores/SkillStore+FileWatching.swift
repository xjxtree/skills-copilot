import Foundation

@MainActor
extension SkillStore {
    func updateAuthorizedFileWatcher(
        with plan: AuthorizedFileWatchPlan,
        forceRestart: Bool = false,
        clearPendingChanges: Bool = false
    ) {
        authorizedFileWatchPlan = plan
        guard let fileSystemWatcher else {
            activeAuthorizedFileWatchRoots = []
            watcherStatusMessage = UIStrings.refreshWatcherManual
            return
        }

        let sanitizedRoots = AuthorizedWatchRootSanitizer.sanitizedPaths(from: plan.roots)
        let rootsChanged = sanitizedRoots != activeAuthorizedFileWatchRoots
        if clearPendingChanges {
            hasPendingFileSystemChanges = false
            fileWatcherRequiresDeepScan = false
        }

        guard !sanitizedRoots.isEmpty else {
            fileSystemWatcher.stop()
            activeAuthorizedFileWatchRoots = []
            watcherStatusMessage = UIStrings.refreshWatcherNoRoots
            return
        }

        if forceRestart || rootsChanged {
            fileSystemWatcher.stop()
            activeAuthorizedFileWatchRoots = []
            let started = fileSystemWatcher.start(paths: sanitizedRoots) { [weak self] summary in
                Task { @MainActor [weak self] in
                    self?.recordAuthorizedFileSystemChange(summary)
                }
            }
            guard started else {
                watcherStatusMessage = UIStrings.refreshWatcherUnavailable
                return
            }
            activeAuthorizedFileWatchRoots = sanitizedRoots
        }

        guard !hasPendingFileSystemChanges else {
            updatePendingWatcherStatus()
            return
        }
        if plan.truncated || sanitizedRoots.count < plan.roots.count {
            watcherStatusMessage = UIStrings.refreshWatcherLimited(
                sanitizedRoots.count,
                plan.totalCount
            )
        } else {
            watcherStatusMessage = UIStrings.refreshWatcherActive(sanitizedRoots.count)
        }
    }

    func reconcileAuthorizedFileWatcherAfterDeepScan() {
        updateAuthorizedFileWatcher(
            with: authorizedFileWatchPlan,
            forceRestart: true,
            clearPendingChanges: true
        )
    }

    private func recordAuthorizedFileSystemChange(_ summary: FileSystemChangeSummary) {
        guard summary.eventCount > 0 else { return }
        hasPendingFileSystemChanges = true
        fileWatcherRequiresDeepScan = fileWatcherRequiresDeepScan || summary.requiresDeepScan
        updatePendingWatcherStatus()
    }

    private func updatePendingWatcherStatus() {
        watcherStatusMessage = fileWatcherRequiresDeepScan
            ? UIStrings.refreshWatcherPendingDeepScan
            : UIStrings.refreshWatcherPending
    }
}
