// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "dwv-macos-bridge-probe",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "dwv-macos-bridge-probe", targets: ["DwvMacOSBridgeProbe"])
    ],
    targets: [
        .executableTarget(name: "DwvMacOSBridgeProbe")
    ]
)
