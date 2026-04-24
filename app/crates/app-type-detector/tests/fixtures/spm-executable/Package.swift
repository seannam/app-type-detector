// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "mycli",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "mycli", targets: ["mycli"])
    ],
    targets: [
        .executableTarget(name: "mycli")
    ]
)
