// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "MyLib",
    platforms: [.iOS(.v16), .macOS(.v14)],
    products: [
        .library(name: "MyLib", targets: ["MyLib"])
    ],
    targets: [
        .target(name: "MyLib"),
        .testTarget(name: "MyLibTests", dependencies: ["MyLib"])
    ]
)
