import Foundation

struct ProjectContextState: Codable, Hashable {
    let revision: String
    let active: ProjectContext?
    let recent: [ProjectContext]

    init(revision: String = "", active: ProjectContext?, recent: [ProjectContext]) {
        self.revision = revision
        self.active = active
        self.recent = recent
    }
}

struct ProjectContext: Codable, Identifiable, Hashable {
    let id: String
    let name: String
    let rootPath: String
    let currentCWD: String?
    let lastUsedAt: String?
    let isActive: Bool
    let validationError: String?

    enum CodingKeys: String, CodingKey {
        case id
        case name
        case rootPath = "root_path"
        case currentCWD = "current_cwd"
        case lastUsedAt = "last_used_at"
        case isActive = "is_active"
        case validationError = "validation_error"
    }

    init(
        id: String,
        name: String,
        rootPath: String,
        currentCWD: String?,
        lastUsedAt: String?,
        isActive: Bool,
        validationError: String?
    ) {
        self.id = id
        self.name = name
        self.rootPath = rootPath
        self.currentCWD = currentCWD
        self.lastUsedAt = lastUsedAt
        self.isActive = isActive
        self.validationError = validationError
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        name = try container.decode(String.self, forKey: .name)
        rootPath = try container.decode(String.self, forKey: .rootPath)
        currentCWD = try container.decodeIfPresent(String.self, forKey: .currentCWD)
        isActive = try container.decode(Bool.self, forKey: .isActive)
        validationError = try container.decodeIfPresent(String.self, forKey: .validationError)

        if let value = try? container.decodeIfPresent(String.self, forKey: .lastUsedAt) {
            lastUsedAt = value
        } else if let value = try? container.decodeIfPresent(Int64.self, forKey: .lastUsedAt) {
            lastUsedAt = String(value)
        } else {
            lastUsedAt = nil
        }
    }
}

struct ProjectContextActionPreview: Codable, Hashable {
    let action: ActionDescriptorWire
    let preconditions: [ActionPreconditionWire]
    let previewToken: String
    let current: ProjectContextState
    let candidate: ProjectContextState
    let affectedCount: Int

    enum CodingKeys: String, CodingKey {
        case action
        case preconditions
        case previewToken = "preview_token"
        case current
        case candidate
        case affectedCount = "affected_count"
    }

    var confirmation: ActionConfirmationWire {
        ActionConfirmationWire(action: action, previewToken: previewToken)
    }
}

struct ProjectContextApplyResult: Codable, Hashable {
    let action: ActionDescriptorWire
    let previewToken: String
    let state: ProjectContextState
    let affectedCount: Int
    let readback: ActionReadbackWire

    enum CodingKeys: String, CodingKey {
        case action
        case previewToken = "preview_token"
        case state
        case affectedCount = "affected_count"
        case readback
    }
}

enum ProjectContextPendingAction: Hashable, Identifiable {
    case clearActive(ProjectContextActionPreview)
    case clearRecent(ProjectContextActionPreview)

    var id: String { preview.action.id }

    var preview: ProjectContextActionPreview {
        switch self {
        case .clearActive(let preview), .clearRecent(let preview):
            return preview
        }
    }
}
