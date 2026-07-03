enum AppAccessibilityID {
    static let mainWindow = "skills-copilot.main-window"
    static let mainContent = "skills-copilot.main-content"
    static let taskCockpitPanel = "skills-copilot.task-cockpit.panel"
    static let taskCockpitInput = "skills-copilot.task-cockpit.input"
    static let taskCockpitInputStatus = "skills-copilot.task-cockpit.input.status"
    static let taskCockpitBuildButton = "skills-copilot.task-cockpit.build"
    static let taskCockpitStatus = "skills-copilot.task-cockpit.status"
    static let taskCockpitCancelButton = "skills-copilot.task-cockpit.cancel"
    static let taskCockpitRetryButton = "skills-copilot.task-cockpit.retry"
    static let taskCockpitStageProgress = "skills-copilot.task-cockpit.stage-progress"
    static let taskCockpitResult = "skills-copilot.task-cockpit.result"
}

enum MainWindowModel {
    static let windowIdentifierRawValue = AppAccessibilityID.mainWindow
    static let autosaveName = "SkillsCopilot.MainWindow"
    static let minimumWidth = 1349
    static let minimumHeight = 600

    static func mainWindowScore(identifierRawValue: String?, title: String, canBecomeMain: Bool) -> Int {
        var score = 0
        if identifierRawValue == windowIdentifierRawValue {
            score += 100
        }
        if title == UIStrings.appWindowTitle {
            score += 50
        }
        if canBecomeMain {
            score += 10
        }
        return score
    }
}
