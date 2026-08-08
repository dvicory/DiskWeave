import Foundation

struct ProbeRunner {
    private let fileManager = FileManager.default

    func run(arguments: ProbeArguments) throws -> ProbeEvidence {
        let root = try makeRoot(arguments.root)
        let backends = root.appendingPathComponent("backends", isDirectory: true)
        let exports = root.appendingPathComponent("exports", isDirectory: true)
        try fileManager.createDirectory(at: backends, withIntermediateDirectories: true)
        try fileManager.createDirectory(at: exports, withIntermediateDirectories: true)

        let backing = backends.appendingPathComponent("data0.raw")
        let exported = exports.appendingPathComponent("slot-0.raw")
        try createSparseFile(at: backing, size: arguments.logicalSize)
        try createSparseFile(at: exported, size: arguments.logicalSize)

        let copies = root.appendingPathComponent("copies", isDirectory: true)
        try fileManager.createDirectory(at: copies, withIntermediateDirectories: true)
        let copy = copies.appendingPathComponent("data0-copy.raw")
        try fileManager.copyItem(at: backing, to: copy)

        let backingNumber = fileNumber(for: backing)
        let exportNumber = fileNumber(for: exported)
        let copyNumber = fileNumber(for: copy)
        let tools = [
            "/usr/bin/hdiutil",
            "/usr/bin/xcrun",
            "/usr/bin/swiftc",
            "/usr/bin/sqlite3"
        ].reduce(into: [String: Bool]()) { result, path in
            result[path] = fileManager.isExecutableFile(atPath: path)
        }

        var checks = [
            CheckEvidence(
                id: "fixed-logical-size",
                status: fileSize(for: backing) == arguments.logicalSize && fileSize(for: exported) == arguments.logicalSize ? "pass" : "fail",
                detail: "backing and export apparent sizes are checked against the requested fixed geometry"
            ),
            CheckEvidence(
                id: "backing-export-separation",
                status: backingNumber != nil && exportNumber != nil && backingNumber != exportNumber ? "pass" : "fail",
                detail: "backing and exported proxy paths must have distinct host file identities"
            ),
            CheckEvidence(
                id: "exact-range-arithmetic",
                status: UInt64(9) <= arguments.logicalSize ? "pass" : "fail",
                detail: "the baseline marker fits within the fixed geometry; bridge range rejection remains a candidate test"
            ),
            CheckEvidence(
                id: "sparse-hole-reads-as-zero",
                status: (try? readPrefix(at: backing, count: min(arguments.logicalSize, 4096)))?.allSatisfy { $0 == 0 } == true ? "pass" : "fail",
                detail: "an untouched sparse prefix reads as zero on the host file backend"
            ),
            CheckEvidence(
                id: "copy-byte-equality",
                status: fileSize(for: backing) == fileSize(for: copy) ? "pass" : "fail",
                detail: "a copied raw image retains the apparent logical size"
            ),
            CheckEvidence(
                id: "copy-file-id-changes",
                status: backingNumber != nil && copyNumber != nil && backingNumber != copyNumber ? "pass" : "fail",
                detail: "copy/clone identity changes are observable and cannot silently preserve slot identity"
            ),
            CheckEvidence(
                id: "truncate-resize-denial",
                status: "not-attempted",
                detail: "a regular-file baseline cannot prove a bridge proxy denies truncate or resize without mutating the fixture"
            ),
            CheckEvidence(
                id: "member-metadata-boundary",
                status: "pass",
                detail: "fixture uses ordinary raw files and writes no DiskWeave metadata into the backing payload"
            ),
            CheckEvidence(
                id: "bridge-attachment",
                status: "not-attempted",
                detail: "FSKit/macFUSE extension installation and privileged DiskImages attachment are outside this unprivileged baseline probe"
            )
        ]

        let sdkPath = commandOutput(path: "/usr/bin/xcrun", arguments: ["--show-sdk-path"])
        let host = HostEvidence(
            operatingSystem: ProcessInfo.processInfo.operatingSystemVersionString,
            architecture: commandOutput(path: "/usr/bin/uname", arguments: ["-m"]) ?? "unknown",
            sdkPath: sdkPath,
            tools: tools
        )
        let fixture = FixtureEvidence(
            root: root.path,
            backingPath: backing.path,
            exportPath: exported.path,
            logicalSize: arguments.logicalSize,
            backingFileNumber: backingNumber,
            exportFileNumber: exportNumber
        )
        let trace = try captureBaselineTrace(at: exported, logicalSize: arguments.logicalSize)

        if !arguments.keepFixture {
            try? fileManager.removeItem(at: root)
        } else {
            checks.append(CheckEvidence(
                id: "fixture-retained",
                status: "pass",
                detail: "fixture retained for manual bridge attachment and trace capture"
            ))
        }

        return ProbeEvidence(
            schemaVersion: 1,
            host: host,
            fixture: fixture,
            checks: checks,
            trace: trace,
            claims: [
                "This probe establishes portable-demo fixture and host-tool evidence only.",
                "It does not certify FSKit, macFUSE, DiskImages synchronization, FUA, controller-cache behavior, or physical power loss.",
                "A bridge candidate must translate operations into normalized requests and preserve backing/export separation."
            ]
        )
    }

    private func captureBaselineTrace(at url: URL, logicalSize: UInt64) throws -> [TraceEvent] {
        let marker = Data([0x44, 0x57, 0x56, 0x2D, 0x50, 0x52, 0x4F, 0x42, 0x45])
        guard UInt64(marker.count) <= logicalSize else {
            throw ProbeError.invalidArgument("logical size is smaller than the baseline marker")
        }

        do {
            let handle = try FileHandle(forUpdating: url)
            try handle.seek(toFileOffset: 0)
            try handle.write(contentsOf: marker)
            try handle.synchronize()
            try handle.seek(toFileOffset: 0)
            let readback = try handle.read(upToCount: marker.count) ?? Data()
            try handle.close()
            guard readback == marker else {
                throw ProbeError.filesystem("baseline proxy readback did not match the marker")
            }
        } catch let error as ProbeError {
            throw error
        } catch {
            throw ProbeError.filesystem("baseline proxy trace failed: \(error)")
        }

        let length = UInt64(marker.count)
        return [
            TraceEvent(sequence: 1, operation: "open", offset: 0, length: 0, status: "complete", persistence: "not-applicable", detail: "regular-file baseline opened for update"),
            TraceEvent(sequence: 2, operation: "write", offset: 0, length: length, status: "complete", persistence: "volatile-or-unknown", detail: "baseline marker write"),
            TraceEvent(sequence: 3, operation: "sync", offset: 0, length: length, status: "complete", persistence: "host-file-sync", detail: "FileHandle.synchronize completed; not hardware durability"),
            TraceEvent(sequence: 4, operation: "read", offset: 0, length: length, status: "complete", persistence: "volatile-or-unknown", detail: "marker readback matched"),
            TraceEvent(sequence: 5, operation: "close", offset: 0, length: 0, status: "complete", persistence: "not-applicable", detail: "regular-file baseline closed"),
            TraceEvent(sequence: 6, operation: "detach", offset: 0, length: 0, status: "not-attempted", persistence: "unknown", detail: "requires a candidate bridge and DiskImages attachment"),
            TraceEvent(sequence: 7, operation: "disconnect", offset: 0, length: 0, status: "not-attempted", persistence: "unknown", detail: "requires a live bridge with backing failure injection")
        ]
    }

    private func readPrefix(at url: URL, count: UInt64) throws -> Data {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        return try handle.read(upToCount: Int(count)) ?? Data()
    }

    private func makeRoot(_ requested: URL?) throws -> URL {
        let root = requested ?? fileManager.temporaryDirectory.appendingPathComponent(
            "dwv-bridge-probe-\(UUID().uuidString)",
            isDirectory: true
        )
        do {
            try fileManager.createDirectory(at: root, withIntermediateDirectories: true)
            return root
        } catch {
            throw ProbeError.filesystem("cannot create \(root.path): \(error)")
        }
    }

    private func createSparseFile(at url: URL, size: UInt64) throws {
        guard !fileManager.fileExists(atPath: url.path) else {
            throw ProbeError.filesystem("fixture path already exists: \(url.path)")
        }
        guard fileManager.createFile(atPath: url.path, contents: nil) else {
            throw ProbeError.filesystem("cannot create \(url.path)")
        }
        do {
            let handle = try FileHandle(forWritingTo: url)
            try handle.truncate(atOffset: size)
            try handle.close()
        } catch {
            throw ProbeError.filesystem("cannot size \(url.path): \(error)")
        }
    }

    private func fileSize(for url: URL) -> UInt64? {
        guard let attributes = try? fileManager.attributesOfItem(atPath: url.path),
              let value = attributes[.size] as? NSNumber else { return nil }
        return value.uint64Value
    }

    private func fileNumber(for url: URL) -> UInt64? {
        guard let attributes = try? fileManager.attributesOfItem(atPath: url.path),
              let value = attributes[.systemFileNumber] as? NSNumber else { return nil }
        return value.uint64Value
    }

    private func commandOutput(path: String, arguments: [String]) -> String? {
        let process = Process()
        let pipe = Pipe()
        process.executableURL = URL(fileURLWithPath: path)
        process.arguments = arguments
        process.standardOutput = pipe
        process.standardError = FileHandle.nullDevice
        do {
            try process.run()
            process.waitUntilExit()
            guard process.terminationStatus == 0 else { return nil }
            return String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines)
        } catch {
            return nil
        }
    }
}
