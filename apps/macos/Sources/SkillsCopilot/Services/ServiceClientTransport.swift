import Foundation

struct ServiceRequest<Params: Encodable>: Encodable {
    let id: String
    let method: String
    let params: Params
}

struct ServiceEnvelope<ResultPayload: Decodable>: Decodable {
    let id: String?
    let ok: Bool
    let result: ResultPayload?
    let error: ServiceErrorPayload?
}

extension ServiceClient {
    func call<ResultPayload: Decodable, Params: Encodable>(
        method: String,
        params: Params,
        timeoutMS: Int? = nil
    ) async throws -> ResultPayload {
        let request = ServiceRequest(
            id: UUID().uuidString,
            method: method,
            params: params
        )
        let input = try JSONEncoder().encode(request)
        let output = try await runService(input: input, timeoutMS: timeoutMS)
        let envelope: ServiceEnvelope<ResultPayload>
        do {
            envelope = try JSONDecoder().decode(ServiceEnvelope<ResultPayload>.self, from: output)
        } catch {
            throw ClientError.invalidOutput(
                invalidServiceOutputMessage(
                    byteCount: output.count,
                    category: serviceOutputDecodeCategory(error)
                )
            )
        }
        if envelope.ok, let result = envelope.result {
            return result
        }
        if let error = envelope.error {
            throw ClientError.service(error)
        }
        throw ClientError.invalidOutput(
            invalidServiceOutputMessage(
                byteCount: output.count,
                category: "invalid_envelope"
            )
        )
    }

    private func runService(input: Data, timeoutMS: Int?) async throws -> Data {
        let timeoutNanoseconds = timeoutMS.map { UInt64(max($0, 50)) * 1_000_000 }
        return try await processRunner.run(
            executableURL: resolveServiceURL(),
            input: input,
            timeoutNanoseconds: timeoutNanoseconds
        )
    }

    private func resolveServiceURL() throws -> URL {
        if let serviceURLOverride {
            return serviceURLOverride
        }
        if let url = Bundle.main.url(forResource: "skills-copilot-service", withExtension: nil) {
            return url
        }
        throw ClientError.missingBinary
    }
}

private func serviceOutputDecodeCategory(_ error: Error) -> String {
    switch error {
    case DecodingError.typeMismatch:
        return "type_mismatch"
    case DecodingError.valueNotFound:
        return "value_not_found"
    case DecodingError.keyNotFound:
        return "key_not_found"
    case DecodingError.dataCorrupted:
        return "data_corrupted"
    default:
        return "unknown"
    }
}

private func invalidServiceOutputMessage(byteCount: Int, category: String) -> String {
    let message = UIStrings.text(
        "service.error.invalidOutputMetadata",
        "Service response decode failed"
    )
    return "\(message) (response bytes=\(byteCount), category=\(category))."
}
