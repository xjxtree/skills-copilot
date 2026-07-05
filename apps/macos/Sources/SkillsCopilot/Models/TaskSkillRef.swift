import Foundation

struct TaskSkillRef: Codable, Hashable, Identifiable {
    let instanceID: String?
    let name: String
    let agent: String
    let definitionID: String?

    var id: String { instanceID ?? "\(agent)-\(name)" }

    enum CodingKeys: String, CodingKey {
        case instanceID = "instance_id"
        case instanceId = "instanceId"
        case id
        case name
        case skillName = "skill_name"
        case title
        case agent
        case definitionID = "definition_id"
        case definitionId = "definitionId"
    }

    init(instanceID: String?, name: String, agent: String, definitionID: String? = nil) {
        self.instanceID = instanceID
        self.name = name
        self.agent = agent
        self.definitionID = definitionID
    }

    init(skill: SkillRecord) {
        self.init(instanceID: skill.id, name: skill.name, agent: skill.agent, definitionID: skill.definitionId)
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encodeIfPresent(instanceID, forKey: .instanceID)
        try container.encode(name, forKey: .name)
        try container.encode(agent, forKey: .agent)
        try container.encodeIfPresent(definitionID, forKey: .definitionID)
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            instanceID = value.isEmpty ? nil : value
            name = value.isEmpty ? UIStrings.unknown : value
            agent = UIStrings.unknown
            definitionID = nil
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        instanceID = try container.decodeIfPresent(String.self, forKey: .instanceID)
            ?? container.decodeIfPresent(String.self, forKey: .instanceId)
            ?? container.decodeIfPresent(String.self, forKey: .id)
        name = try container.decodeIfPresent(String.self, forKey: .name)
            ?? container.decodeIfPresent(String.self, forKey: .skillName)
            ?? container.decodeIfPresent(String.self, forKey: .title)
            ?? instanceID
            ?? UIStrings.unknown
        agent = try container.decodeIfPresent(String.self, forKey: .agent) ?? UIStrings.unknown
        definitionID = try container.decodeIfPresent(String.self, forKey: .definitionID)
            ?? container.decodeIfPresent(String.self, forKey: .definitionId)
    }
}
