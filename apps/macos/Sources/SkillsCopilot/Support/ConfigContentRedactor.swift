import Foundation

enum ConfigContentRedactor {
    static let redactedValue = "[REDACTED]"
    static let redactedPathValue = "<local-path>"
    static let redactedFileURLValue = "<local-file-url>"

    static func redactedForDisplay(_ content: String) -> String {
        guard !content.isEmpty else { return content }
        if let json = redactedJSON(content) {
            return json
        }
        return content
            .split(separator: "\n", omittingEmptySubsequences: false)
            .map { redactLocalPaths(in: redactSimpleSecretLine(String($0))) }
            .joined(separator: "\n")
    }

    static func containsSensitiveKey(_ key: String) -> Bool {
        let normalized = key
            .unicodeScalars
            .filter { scalar in
                scalar.isASCII
                    && CharacterSet.alphanumerics.contains(scalar)
            }
            .map { Character($0).lowercased() }
            .joined()
        return [
            "apikey",
            "token",
            "accesstoken",
            "refreshtoken",
            "secret",
            "clientsecret",
            "password",
            "passwd"
        ].contains(normalized)
            || normalized.hasSuffix("apikey")
            || normalized.hasSuffix("token")
            || normalized.hasSuffix("secret")
            || normalized.hasSuffix("password")
    }

    private static func redactedJSON(_ content: String) -> String? {
        guard let data = content.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) else {
            return nil
        }
        let (redacted, changed) = redactJSONValue(json)
        guard changed,
              JSONSerialization.isValidJSONObject(redacted),
              let renderedData = try? JSONSerialization.data(
                withJSONObject: redacted,
                options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
              ),
              let rendered = String(data: renderedData, encoding: .utf8) else {
            return changed ? content : nil
        }
        return rendered + (content.hasSuffix("\n") ? "\n" : "")
    }

    private static func redactJSONValue(_ value: Any) -> (Any, Bool) {
        if let dictionary = value as? [String: Any] {
            var changed = false
            var redacted: [String: Any] = [:]
            for (key, child) in dictionary {
                if containsSensitiveKey(key) {
                    redacted[key] = redactedValue
                    if (child as? String) != redactedValue {
                        changed = true
                    }
                } else {
                    let (redactedChild, childChanged) = redactJSONValue(child)
                    redacted[key] = redactedChild
                    changed = changed || childChanged
                }
            }
            return (redacted, changed)
        }
        if let array = value as? [Any] {
            var changed = false
            let redacted = array.map { child in
                let (redactedChild, childChanged) = redactJSONValue(child)
                changed = changed || childChanged
                return redactedChild
            }
            return (redacted, changed)
        }
        if let string = value as? String {
            let redacted = redactLocalPaths(in: string)
            return (redacted, redacted != string)
        }
        return (value, false)
    }

    private static func redactLocalPaths(in value: String) -> String {
        var redacted = DisplayText.redactLocalPath(value)
        redacted = replacingMatches(
            in: redacted,
            pattern: #"<temp>(?:/[^\s"',;\]\}\)]+)+"#,
            template: "<temp>"
        )
        redacted = replacingMatches(
            in: redacted,
            pattern: #"file://[^\s"',;\]\}]+"#,
            template: redactedFileURLValue
        )
        redacted = replacingMatches(
            in: redacted,
            pattern: #"(^|[\s=:\[\]\{\}\(\),;"'])(/(?!/)[^\s"',;\]\}\)]+)"#,
            template: "$1\(redactedPathValue)"
        )
        redacted = replacingMatches(
            in: redacted,
            pattern: #"(^|[\s=:\[\]\{\}\(\),;"'])([A-Za-z]:[\\/][^\s"',;\]\}\)]+|\\\\[^\s"',;\]\}\)]+)"#,
            template: "$1\(redactedPathValue)"
        )
        return redacted
    }

    private static func replacingMatches(
        in value: String,
        pattern: String,
        template: String
    ) -> String {
        guard let expression = try? NSRegularExpression(pattern: pattern) else {
            return value
        }
        let range = NSRange(value.startIndex..<value.endIndex, in: value)
        return expression.stringByReplacingMatches(
            in: value,
            range: range,
            withTemplate: template
        )
    }

    private static func redactSimpleSecretLine(_ line: String) -> String {
        guard let separatorIndex = line.firstIndex(where: { $0 == "=" || $0 == ":" }) else {
            return line
        }
        let keyPart = String(line[..<separatorIndex])
        let separator = String(line[separatorIndex])
        let valuePart = String(line[line.index(after: separatorIndex)...])
        let key = keyPart.trimmingCharacters(in: .whitespacesAndNewlines)
            .trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
        guard containsSensitiveKey(key) else { return line }

        let trimmedValue = valuePart.trimmingCharacters(in: .whitespaces)
        let suffix = trimmedValue.hasSuffix(",") ? "," : ""
        return "\(keyPart)\(separator) \"\(redactedValue)\"\(suffix)"
    }
}
