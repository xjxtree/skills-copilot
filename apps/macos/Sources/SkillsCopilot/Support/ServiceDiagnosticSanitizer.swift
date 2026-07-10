import Foundation

enum ServiceDiagnosticSanitizer {
    static let maximumDisplayCharacters = 512
    private static let credentialKeys = ["API_KEY", "TOKEN", "SECRET", "PASSWORD"]
        .map { Array($0.utf8) }
    private static let redactedCredentialValue = Array("<redacted>".utf8)

    private struct CredentialAssignment {
        let valueStart: Int
    }

    static func displayMessage(_ raw: String) -> String {
        var sanitized = raw
            .replacingOccurrences(of: "\r\n", with: "\n")
            .replacingOccurrences(of: "\r", with: "\n")
        // Redact credential chains before the line-oriented config pass can insert a
        // placeholder between an assignment and its newline-delimited value.
        sanitized = redactingCredentialAssignments(in: sanitized)
        sanitized = ConfigContentRedactor.redactedForDisplay(sanitized)
        sanitized = normalizingConfigCredentialPlaceholders(in: sanitized)
        sanitized = replacing(
            pattern: #"(?i)\bsk-[A-Za-z0-9_-]{20,}"#,
            in: sanitized,
            with: "<redacted-token>"
        )
        for pathComponents in [
            ["Users"],
            ["home"],
            ["private", "var"],
            ["var", "folders"]
        ] {
            sanitized = replacing(
                pattern: absolutePathPattern(pathComponents),
                in: sanitized,
                with: "<redacted-path>"
            )
        }

        let collapsed = sanitized
            .components(separatedBy: .whitespacesAndNewlines)
            .filter { !$0.isEmpty }
            .joined(separator: " ")
        guard !collapsed.isEmpty else {
            return UIStrings.text(
                "service.error.sidecarDiagnosticUnavailable",
                "The service exited without a safe diagnostic."
            )
        }
        return String(collapsed.prefix(maximumDisplayCharacters))
    }

    private static func redactingCredentialAssignments(in value: String) -> String {
        let bytes = Array(value.utf8)
        var output: [UInt8] = []
        output.reserveCapacity(bytes.count)
        var copiedThrough = 0
        var searchStart = 0

        while let assignment = nextCredentialAssignment(in: bytes, from: searchStart) {
            output.append(contentsOf: bytes[copiedThrough..<assignment.valueStart])
            output.append(contentsOf: redactedCredentialValue)

            let valueEnd = credentialValueEnd(in: bytes, afterEquals: assignment.valueStart)
            copiedThrough = valueEnd
            searchStart = valueEnd
        }

        guard copiedThrough > 0 else { return value }
        output.append(contentsOf: bytes[copiedThrough...])
        return String(decoding: output, as: UTF8.self)
    }

    private static func nextCredentialAssignment(
        in bytes: [UInt8],
        from searchStart: Int
    ) -> CredentialAssignment? {
        var candidate = searchStart
        while candidate < bytes.count {
            if let assignment = credentialAssignment(in: bytes, at: candidate) {
                return assignment
            }
            candidate += 1
        }
        return nil
    }

    private static func credentialAssignment(
        in bytes: [UInt8],
        at start: Int
    ) -> CredentialAssignment? {
        guard start == 0 || !isASCIIWordByte(bytes[start - 1]) else {
            return nil
        }

        for key in credentialKeys where start + key.count < bytes.count {
            var matches = true
            for offset in key.indices where asciiUppercased(bytes[start + offset]) != key[offset] {
                matches = false
                break
            }
            let equalsIndex = start + key.count
            if matches, bytes[equalsIndex] == 0x3D {
                return CredentialAssignment(valueStart: equalsIndex + 1)
            }
        }
        return nil
    }

    private static func credentialValueEnd(in bytes: [UInt8], afterEquals: Int) -> Int {
        var tokenStart = afterEquals
        while tokenStart < bytes.count, isASCIIWhitespace(bytes[tokenStart]) {
            tokenStart += 1
        }

        guard tokenStart < bytes.count else { return bytes.count }
        if credentialAssignment(in: bytes, at: tokenStart) != nil {
            // Preserve the separating whitespace and let the next scan redact the
            // following assignment independently.
            return afterEquals
        }
        if let escapedQuotedValueEnd = escapedOuterQuotedValueEnd(
            in: bytes,
            tokenStart: tokenStart
        ) {
            return escapedQuotedValueEnd
        }

        var cursor = tokenStart
        var activeQuote: UInt8?
        while cursor < bytes.count {
            let byte = bytes[cursor]
            if byte == 0x0A || byte == 0x0D {
                // An unterminated quoted value is untrusted through the end of its
                // diagnostic line, but must not hide a later ordinary line.
                return cursor
            }
            if let quote = activeQuote {
                if byte == 0x5C,
                   cursor + 1 < bytes.count,
                   !isLineBreak(bytes[cursor + 1]) {
                    cursor += 2
                } else {
                    cursor += 1
                    if byte == quote {
                        activeQuote = nil
                    }
                }
            } else if cursor > tokenStart,
                      credentialAssignment(in: bytes, at: cursor) != nil {
                // An exact key immediately after a completed quoted or
                // placeholder value begins the next assignment in the chain.
                return cursor
            } else if isASCIIWhitespace(byte) {
                return cursor
            } else if byte == 0x22 || byte == 0x27 {
                activeQuote = byte
                cursor += 1
            } else if byte == 0x5C,
                      cursor + 1 < bytes.count,
                      !isLineBreak(bytes[cursor + 1]) {
                cursor += 2
            } else {
                cursor += 1
            }
        }
        return cursor
    }

    private static func escapedOuterQuotedValueEnd(
        in bytes: [UInt8],
        tokenStart: Int
    ) -> Int? {
        var openingQuoteIndex = tokenStart
        while openingQuoteIndex < bytes.count, bytes[openingQuoteIndex] == 0x5C {
            openingQuoteIndex += 1
        }
        let openingSlashCount = openingQuoteIndex - tokenStart
        guard openingSlashCount > 0,
              openingQuoteIndex < bytes.count,
              bytes[openingQuoteIndex] == 0x22 || bytes[openingQuoteIndex] == 0x27 else {
            return nil
        }

        let quote = bytes[openingQuoteIndex]
        var cursor = openingQuoteIndex + 1
        while cursor < bytes.count, !isLineBreak(bytes[cursor]) {
            guard bytes[cursor] == quote else {
                cursor += 1
                continue
            }

            var slashStart = cursor
            while slashStart > openingQuoteIndex + 1, bytes[slashStart - 1] == 0x5C {
                slashStart -= 1
            }
            if cursor - slashStart == openingSlashCount {
                return cursor + 1
            }
            cursor += 1
        }

        // The wrapper is malformed or truncated. Treat the rest of this
        // diagnostic line as untrusted, while preserving following lines.
        return cursor
    }

    private static func asciiUppercased(_ byte: UInt8) -> UInt8 {
        if byte >= 0x61, byte <= 0x7A {
            return byte - 32
        }
        return byte
    }

    private static func isASCIIWordByte(_ byte: UInt8) -> Bool {
        (byte >= 0x61 && byte <= 0x7A)
            || (byte >= 0x41 && byte <= 0x5A)
            || (byte >= 0x30 && byte <= 0x39)
            || byte == 0x5F
    }

    private static func isASCIIWhitespace(_ byte: UInt8) -> Bool {
        byte == 0x20
            || byte == 0x09
            || byte == 0x0A
            || byte == 0x0D
            || byte == 0x0B
            || byte == 0x0C
    }

    private static func isLineBreak(_ byte: UInt8) -> Bool {
        byte == 0x0A || byte == 0x0D
    }

    private static func normalizingConfigCredentialPlaceholders(in value: String) -> String {
        replacing(
            pattern: #"(?i)\b(API_KEY|TOKEN|SECRET|PASSWORD)=\s*"\[REDACTED\]""#,
            in: value,
            with: "$1=<redacted>"
        )
    }

    private static func replacing(
        pattern: String,
        in value: String,
        with replacement: String
    ) -> String {
        guard let expression = try? NSRegularExpression(pattern: pattern) else {
            return value
        }
        let range = NSRange(value.startIndex..<value.endIndex, in: value)
        return expression.stringByReplacingMatches(
            in: value,
            range: range,
            withTemplate: replacement
        )
    }

    private static func absolutePathPattern(_ components: [String]) -> String {
        "/" + components.joined(separator: "/") + #"/[^\s]+"#
    }
}
