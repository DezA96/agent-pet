// swift-tools-version:5.9
import PackageDescription

// Covers the pure parts of the surface and their tests — not the app itself.
//
// The pet itself is built by ../build.sh with swiftc, as one module against the
// Rust core's C header. Bringing the whole surface under SwiftPM would mean
// splitting a library target out of main.swift and re-expressing the FFI bridge,
// which buys nothing: the code worth testing is the geometry and the state
// priority, and neither depends on AppKit windows or on the core.
//
// macOS 14 because swift-testing's framework is built for it. The app itself
// still targets macOS 13 — nothing here is compiled into it.
let package = Package(
    name: "PetGeometry",
    platforms: [.macOS(.v14)],
    targets: [
        .target(name: "PetGeometry"),
        .testTarget(name: "PetGeometryTests", dependencies: ["PetGeometry"]),
        .target(name: "PetState"),
        .testTarget(name: "PetStateTests", dependencies: ["PetState"]),
    ]
)
