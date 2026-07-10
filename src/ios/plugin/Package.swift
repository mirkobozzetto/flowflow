// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "RecordingPlugin",
    platforms: [.iOS("16.2")],
    products: [
        .library(name: "RecordingPlugin", type: .static, targets: ["RecordingPlugin"])
    ],
    targets: [
        .target(
            name: "RecordingPlugin",
            path: "Sources",
            linkerSettings: [
                .linkedFramework("ActivityKit"),
                .linkedFramework("AppIntents"),
                .linkedFramework("Foundation"),
            ]
        )
    ]
)
