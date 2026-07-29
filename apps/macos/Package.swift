// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "SkillsCopilotMac",
    defaultLocalization: "en",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "SkillsCopilot", targets: ["SkillsCopilot"])
    ],
    targets: [
        .executableTarget(
            name: "SkillsCopilot",
            path: "Sources/SkillsCopilot",
            resources: [.process("Resources")]
        ),
        .testTarget(
            name: "SkillsCopilotTests",
            dependencies: ["SkillsCopilot"],
            path: "Tests/SkillsCopilotTests"
        )
    ],
    swiftLanguageModes: [.v5]
)
