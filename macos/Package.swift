// swift-tools-version:5.9
import PackageDescription

// Covers the placement arithmetic and its tests only — not the app.
//
// The pet itself is built by ../build.sh with swiftc, as one module against the
// Rust core's C header. Bringing the whole surface under SwiftPM would mean
// splitting a library target out of main.swift and re-expressing the FFI bridge,
// which buys nothing: the code worth testing is the geometry, and it depends on
// neither AppKit windows nor the core.
//
// macOS 14 because swift-testing's framework is built for it. The app itself
// still targets macOS 13 — nothing here is compiled into it.
let package = Package(
    name: "PetGeometry",
    platforms: [.macOS(.v14)],
    targets: [
        .target(name: "PetGeometry"),
        .testTarget(name: "PetGeometryTests", dependencies: ["PetGeometry"]),
    ]
)
