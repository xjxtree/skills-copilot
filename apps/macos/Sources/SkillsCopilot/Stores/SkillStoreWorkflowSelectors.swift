import Foundation

@MainActor
extension SkillStore {
    var selectedTaskCockpitInput: String {
        let trimmedCockpit = taskCockpitText.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmedCockpit.isEmpty {
            return taskCockpitText
        }
        return ""
    }

    func scriptExecutionPreview(for skill: SkillRecord) -> ScriptExecutionPreview? {
        scriptExecutionPreviews[skill.id]
    }

    func isPreviewingScriptExecution(for skill: SkillRecord) -> Bool {
        previewingScriptExecutionSkillIDs.contains(skill.id)
    }
}
