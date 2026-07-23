// swift-tools-version: 5.9
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
            resources: [.process("Resources")],
            linkerSettings: [.linkedFramework("Security")]
        ),
        .target(
            name: "SkillsCopilotTestHarness",
            path: "Tests/SkillsCopilotTestHarness",
            publicHeadersPath: "."
        ),
        .testTarget(
            name: "SkillsCopilotTests",
            dependencies: ["SkillsCopilot", "SkillsCopilotTestHarness"],
            path: "Tests/SkillsCopilotTests"
        )
    ]
)
