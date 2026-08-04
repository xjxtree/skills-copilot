import Foundation

struct SkillManagerInspectLocalSourceParams: Encodable {
    let sourcePath: String

    enum CodingKeys: String, CodingKey {
        case sourcePath = "source_path"
    }
}

struct SkillManagerLocalSourceSkillRecord: Codable, Hashable, Identifiable {
    let name: String
    let description: String
    let relativePath: String

    var id: String { "\(name.lowercased())|\(relativePath.lowercased())" }

    enum CodingKeys: String, CodingKey {
        case name
        case description
        case relativePath = "relative_path"
    }
}

struct SkillManagerLocalSourceInspectionRecord: Codable, Hashable {
    let preview: SkillManagerCommandPreview
    let output: SkillManagerCommandOutput
    let sourcePath: String
    let sourceRevision: String
    let skills: [SkillManagerLocalSourceSkillRecord]

    enum CodingKeys: String, CodingKey {
        case preview
        case output
        case sourcePath = "source_path"
        case sourceRevision = "source_revision"
        case skills
    }
}

struct SkillManagerDirectLocalSourceCandidate: Hashable, Identifiable {
    let name: String
    let description: String
    let relativePath: String
    let sourcePath: String
    let displayPath: String
    let sourceRevision: String

    var id: String {
        [sourceRevision, name.lowercased(), relativePath.lowercased()].joined(separator: "|")
    }
}

struct SkillManagerDirectLocalSource: Hashable {
    let sourcePath: String
    let inspection: SkillManagerLocalSourceInspectionRecord

    var displayPath: String { inspection.sourcePath }

    var candidates: [SkillManagerDirectLocalSourceCandidate] {
        inspection.skills.map { skill in
            SkillManagerDirectLocalSourceCandidate(
                name: skill.name,
                description: skill.description,
                relativePath: skill.relativePath,
                sourcePath: sourcePath,
                displayPath: inspection.sourcePath,
                sourceRevision: inspection.sourceRevision
            )
        }
    }
}
