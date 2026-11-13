// swift-tools-version:5.9
// Lets an application add Compono by this repository's URL. It builds the same
// sources as native/apple/Compono, which stays the package for local work.
import PackageDescription

let package = Package(
    name: "Compono",
    platforms: [.iOS(.v17), .macOS(.v14), .tvOS(.v17), .visionOS(.v1)],
    products: [.library(name: "Compono", targets: ["Compono"])],
    targets: [
        .target(name: "Compono", path: "native/apple/Compono/Sources/Compono"),
    ]
)
