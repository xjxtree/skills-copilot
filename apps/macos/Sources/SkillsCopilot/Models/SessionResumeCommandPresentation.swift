import Foundation

struct SessionResumeCommandPresentation: Equatable {
    let command: String

    init?(session: SessionContinuationRecord) {
        guard session.resume.state == .supported,
              session.resume.copyOnly,
              !session.resume.argv.isEmpty else {
            return nil
        }
        command = session.resume.argv.map(Self.shellQuoted).joined(separator: " ")
    }

    private static func shellQuoted(_ argument: String) -> String {
        let safe = CharacterSet.alphanumerics.union(
            CharacterSet(charactersIn: "_-./:@%+=,")
        )
        if !argument.isEmpty,
           argument.unicodeScalars.allSatisfy({ safe.contains($0) }) {
            return argument
        }
        return "'" + argument.replacingOccurrences(of: "'", with: "'\"'\"'") + "'"
    }
}
