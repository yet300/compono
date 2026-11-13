// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "Compono",
    platforms: [.iOS(.v17), .macOS(.v14), .tvOS(.v17), .visionOS(.v1)],
    products: [.library(name: "Compono", targets: ["Compono"])],
    targets: [
        .target(name: "Compono"),
        .testTarget(name: "ComponoTests", dependencies: ["Compono"]),
    ]
)
