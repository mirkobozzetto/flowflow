// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "RecordingWidget",
    platforms: [.iOS(.v16)],
    products: [
        .library(name: "RecordingWidget", targets: ["RecordingWidget"])
    ],
    targets: [
        .target(name: "RecordingWidget", path: "Sources")
    ]
)
