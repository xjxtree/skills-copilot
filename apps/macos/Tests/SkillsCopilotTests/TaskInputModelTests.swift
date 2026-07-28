import Testing
import Foundation
@testable import SkillsCopilot

@Suite("TaskInputModelTests")
struct TaskInputModelTests {
    @Test("TaskInputModelTests")
    func run() throws {
        try preservesOriginalTextWhileClassifyingSubmittableInput()
        try rejectsBlankAndWhitespaceOnlyInput()
        try tracksMultilineInputWithoutCollapsingText()
        try keepsPasteAndEmojiTextIntact()
        try exposesStableAutomationIdentifiers()
    }

    private func preservesOriginalTextWhileClassifyingSubmittableInput() throws {
        let raw = "  用中文输入本地技能审计  "
        let model = TaskInputModel(rawText: raw)

        try expectEqual(model.rawText, raw, "Task input model should preserve Chinese raw text.")
        try expectEqual(model.trimmedText, "用中文输入本地技能审计", "Task input model should trim only for submission decisions.")
        try expectEqual(model.submissionText, raw, "Task input model should preserve raw non-blank text for submission.")
        try expectEqual(model.canSubmit, true, "Chinese input should be submittable after trimming.")
    }

    private func rejectsBlankAndWhitespaceOnlyInput() throws {
        for raw in ["", "   ", "\n\t  \n"] {
            let model = TaskInputModel(rawText: raw)

            try expectEqual(model.rawText, raw, "Blank task input should keep the raw value.")
            try expectEqual(model.trimmedText, "", "Blank task input should trim to empty.")
            try expectNil(model.submissionText, "Blank task input should not expose submission text.")
            try expectFalse(model.canSubmit, "Blank task input should not be submittable.")
        }
    }

    private func tracksMultilineInputWithoutCollapsingText() throws {
        let raw = "第一行\nsecond line\nthird line"
        let model = TaskInputModel(rawText: raw)

        try expectEqual(model.rawText, raw, "Multiline task input should preserve newline boundaries.")
        try expectEqual(model.trimmedText, raw, "Multiline task input should not collapse internal newlines.")
        try expectEqual(model.submissionText, raw, "Multiline task input should preserve raw submission text.")
        try expectEqual(model.lineCount, 3, "Multiline task input should count newline-delimited lines.")
        try expectEqual(model.isMultiline, true, "Multiline task input should be classified as multiline.")
        try expectEqual(model.canSubmit, true, "Multiline task input should be submittable.")
    }

    private func keepsPasteAndEmojiTextIntact() throws {
        let raw = "  Paste release notes 🚀\r\n保留 emoji 和中文\r\n  "
        let model = TaskInputModel(rawText: raw)

        try expectEqual(model.rawText, raw, "Pasted task input should preserve raw pasted text.")
        try expectEqual(model.trimmedText, "Paste release notes 🚀\r\n保留 emoji 和中文", "Pasted task input should trim edges only.")
        try expectEqual(model.submissionText, raw, "Pasted task input should preserve raw submission text.")
        try expectEqual(model.lineCount, 3, "Pasted task input should retain multiline shape.")
        try expectEqual(model.canSubmit, true, "Pasted emoji and Chinese text should be submittable.")
    }

    private func exposesStableAutomationIdentifiers() throws {
        try expectEqual(AppAccessibilityID.taskCockpitInput, "skills-copilot.task-cockpit.input", "Task input AX identifier should stay stable.")
        try expectEqual(AppAccessibilityID.taskCockpitInputStatus, "skills-copilot.task-cockpit.input.status", "Task input status AX identifier should stay stable.")
    }
}
