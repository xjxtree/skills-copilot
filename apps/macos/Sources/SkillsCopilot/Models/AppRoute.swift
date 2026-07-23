import Foundation

enum AppRoute: String, CaseIterable, Codable, Hashable, Identifiable {
    case overview
    case skills
    case sessions
    case advanced

    static let defaultRoute: AppRoute = .overview

    var id: String { rawValue }

    var restorationValue: String { rawValue }

    init?(restorationValue: String) {
        self.init(rawValue: restorationValue)
    }

    func encodedForRestoration() throws -> Data {
        try JSONEncoder().encode(self)
    }

    static func restored(from data: Data) throws -> AppRoute {
        try JSONDecoder().decode(AppRoute.self, from: data)
    }
}
