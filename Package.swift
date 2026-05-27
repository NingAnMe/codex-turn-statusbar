// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "CodexTurnStatusBar",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .executable(name: "CodexTurnStatusBar", targets: ["CodexTurnStatusBar"])
    ],
    targets: [
        .target(name: "CodexTurnStatusCore"),
        .executableTarget(
            name: "CodexTurnStatusBar",
            dependencies: ["CodexTurnStatusCore"]
        ),
        .executableTarget(
            name: "CodexTurnStatusCoreSmokeTests",
            dependencies: ["CodexTurnStatusCore"]
        )
    ]
)
