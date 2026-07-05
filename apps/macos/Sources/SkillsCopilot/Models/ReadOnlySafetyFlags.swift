struct ReadOnlySafetyFlags: Decodable, Hashable {
    let providerRequestSent: Bool
    let writeBackAllowed: Bool
    let writeActionsAvailable: Bool
    let scriptExecutionAllowed: Bool
    let executionActionsAvailable: Bool
    let configMutationAllowed: Bool
    let snapshotCreated: Bool
    let triageMutationAllowed: Bool
    let credentialAccessed: Bool
    let rawPromptPersisted: Bool
    let rawResponsePersisted: Bool
    let rawTracePersisted: Bool
    let cloudSyncEnabled: Bool
    let telemetryEnabled: Bool
    let rawSecretReturned: Bool
    let notes: [String]

    var allReadOnlyFlagsClear: Bool {
        !providerRequestSent
            && !writeBackAllowed
            && !writeActionsAvailable
            && !scriptExecutionAllowed
            && !executionActionsAvailable
            && !configMutationAllowed
            && !snapshotCreated
            && !triageMutationAllowed
            && !credentialAccessed
            && !rawPromptPersisted
            && !rawResponsePersisted
            && !rawTracePersisted
            && !cloudSyncEnabled
            && !telemetryEnabled
            && !rawSecretReturned
    }

    enum CodingKeys: String, CodingKey {
        case providerRequestSent = "provider_request_sent"
        case providerCallSent = "provider_call_sent"
        case writeBackAllowed = "write_back_allowed"
        case writeActionsAvailable = "write_actions_available"
        case writesAllowed = "writes_allowed"
        case scriptExecutionAllowed = "script_execution_allowed"
        case executionActionsAvailable = "execution_actions_available"
        case configMutationAllowed = "config_mutation_allowed"
        case snapshotCreated = "snapshot_created"
        case triageMutationAllowed = "triage_mutation_allowed"
        case credentialAccessed = "credential_accessed"
        case rawPromptPersisted = "raw_prompt_persisted"
        case rawResponsePersisted = "raw_response_persisted"
        case rawTracePersisted = "raw_trace_persisted"
        case rawTraceStored = "raw_trace_stored"
        case cloudSyncEnabled = "cloud_sync_enabled"
        case cloudSyncPerformed = "cloud_sync_performed"
        case cloudSync = "cloud_sync"
        case telemetryEnabled = "telemetry_enabled"
        case telemetryEmitted = "telemetry_emitted"
        case telemetry
        case rawSecretReturned = "raw_secret_returned"
        case notes
        case flags
    }

    init(
        providerRequestSent: Bool = false,
        writeBackAllowed: Bool = false,
        writeActionsAvailable: Bool = false,
        scriptExecutionAllowed: Bool = false,
        executionActionsAvailable: Bool = false,
        configMutationAllowed: Bool = false,
        snapshotCreated: Bool = false,
        triageMutationAllowed: Bool = false,
        credentialAccessed: Bool = false,
        rawPromptPersisted: Bool = false,
        rawResponsePersisted: Bool = false,
        rawTracePersisted: Bool = false,
        cloudSyncEnabled: Bool = false,
        telemetryEnabled: Bool = false,
        rawSecretReturned: Bool = false,
        notes: [String] = []
    ) {
        self.providerRequestSent = providerRequestSent
        self.writeBackAllowed = writeBackAllowed
        self.writeActionsAvailable = writeActionsAvailable
        self.scriptExecutionAllowed = scriptExecutionAllowed
        self.executionActionsAvailable = executionActionsAvailable
        self.configMutationAllowed = configMutationAllowed
        self.snapshotCreated = snapshotCreated
        self.triageMutationAllowed = triageMutationAllowed
        self.credentialAccessed = credentialAccessed
        self.rawPromptPersisted = rawPromptPersisted
        self.rawResponsePersisted = rawResponsePersisted
        self.rawTracePersisted = rawTracePersisted
        self.cloudSyncEnabled = cloudSyncEnabled
        self.telemetryEnabled = telemetryEnabled
        self.rawSecretReturned = rawSecretReturned
        self.notes = notes
    }

    init(from decoder: Decoder) throws {
        if let values = try? decoder.singleValueContainer().decode([String].self) {
            self.init(notes: values)
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            providerRequestSent: try container.decodeIfPresent(Bool.self, forKey: .providerRequestSent)
                ?? container.decodeIfPresent(Bool.self, forKey: .providerCallSent)
                ?? false,
            writeBackAllowed: try container.decodeIfPresent(Bool.self, forKey: .writeBackAllowed) ?? false,
            writeActionsAvailable: try container.decodeIfPresent(Bool.self, forKey: .writeActionsAvailable)
                ?? container.decodeIfPresent(Bool.self, forKey: .writesAllowed)
                ?? false,
            scriptExecutionAllowed: try container.decodeIfPresent(Bool.self, forKey: .scriptExecutionAllowed) ?? false,
            executionActionsAvailable: try container.decodeIfPresent(Bool.self, forKey: .executionActionsAvailable) ?? false,
            configMutationAllowed: try container.decodeIfPresent(Bool.self, forKey: .configMutationAllowed) ?? false,
            snapshotCreated: try container.decodeIfPresent(Bool.self, forKey: .snapshotCreated) ?? false,
            triageMutationAllowed: try container.decodeIfPresent(Bool.self, forKey: .triageMutationAllowed) ?? false,
            credentialAccessed: try container.decodeIfPresent(Bool.self, forKey: .credentialAccessed) ?? false,
            rawPromptPersisted: try container.decodeIfPresent(Bool.self, forKey: .rawPromptPersisted) ?? false,
            rawResponsePersisted: try container.decodeIfPresent(Bool.self, forKey: .rawResponsePersisted) ?? false,
            rawTracePersisted: try container.decodeIfPresent(Bool.self, forKey: .rawTracePersisted)
                ?? container.decodeIfPresent(Bool.self, forKey: .rawTraceStored)
                ?? false,
            cloudSyncEnabled: try container.decodeIfPresent(Bool.self, forKey: .cloudSyncEnabled)
                ?? container.decodeIfPresent(Bool.self, forKey: .cloudSyncPerformed)
                ?? container.decodeIfPresent(Bool.self, forKey: .cloudSync)
                ?? false,
            telemetryEnabled: try container.decodeIfPresent(Bool.self, forKey: .telemetryEnabled)
                ?? container.decodeIfPresent(Bool.self, forKey: .telemetryEmitted)
                ?? container.decodeIfPresent(Bool.self, forKey: .telemetry)
                ?? false,
            rawSecretReturned: try container.decodeIfPresent(Bool.self, forKey: .rawSecretReturned) ?? false,
            notes: try container.decodeFlexibleReadOnlySafetyStringArray(keys: [.notes, .flags])
        )
    }
}

private extension KeyedDecodingContainer {
    func decodeFlexibleReadOnlySafetyStringArray(keys: [Key]) throws -> [String] {
        for key in keys {
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values
            }
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                return value.isEmpty ? [] : [value]
            }
        }
        return []
    }
}
