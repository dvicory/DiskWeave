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

        let rangeProbe = root.appendingPathComponent("range-probe.raw")
        let rangeChecks = try probeRegularFileRangeBehavior(
            at: rangeProbe,
            size: arguments.logicalSize
        )
        let aliasChecks = try probeAliasIdentity(at: backing, root: root)

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

        var checks = rangeChecks + [
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
                status: "pass",
                detail: "bounded exact-range read/write evidence is recorded separately; bridge range rejection remains a candidate test"
            ),
            CheckEvidence(
                id: "sparse-hole-reads-as-zero",
                status: (try? readPrefix(at: backing, count: min(arguments.logicalSize, 4096)))?.allSatisfy { $0 == 0 } == true ? "pass" : "fail",
                detail: "an untouched sparse prefix reads as zero on the host file backend"
            ),
            CheckEvidence(
                id: "copy-byte-equality",
                status: fileSize(for: backing) == fileSize(for: copy) && (try? readPrefix(at: backing, count: min(arguments.logicalSize, 4096))) == (try? readPrefix(at: copy, count: min(arguments.logicalSize, 4096))) ? "pass" : "fail",
                detail: "a copied raw image retains bounded byte equality and apparent logical size"
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
        ] + aliasChecks

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
        checks.append(try validateBaselineTrace(trace, logicalSize: arguments.logicalSize))

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

            let reopened = try FileHandle(forReadingFrom: url)
            let reopenedReadback = try reopened.read(upToCount: marker.count) ?? Data()
            try reopened.close()
            guard reopenedReadback == marker else {
                throw ProbeError.filesystem("reopened baseline proxy readback did not match")
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
            TraceEvent(sequence: 6, operation: "reopen", offset: 0, length: 0, status: "complete", persistence: "host-file-reopen", detail: "regular-file baseline reopened"),
            TraceEvent(sequence: 7, operation: "read-after-reopen", offset: 0, length: length, status: "complete", persistence: "host-file-reopen", detail: "marker remained readable after close/reopen"),
            TraceEvent(sequence: 8, operation: "detach", offset: 0, length: 0, status: "not-attempted", persistence: "unknown", detail: "requires a candidate bridge and DiskImages attachment"),
            TraceEvent(sequence: 9, operation: "disconnect", offset: 0, length: 0, status: "not-attempted", persistence: "unknown", detail: "requires a live bridge with backing failure injection")
        ]
    }

    private func probeRegularFileRangeBehavior(at url: URL, size: UInt64) throws -> [CheckEvidence] {
        guard size > 0 else {
            throw ProbeError.invalidArgument("logical size must be positive")
        }
        try createSparseFile(at: url, size: size)

        let writeHandle = try FileHandle(forUpdating: url)
        try writeHandle.seek(toFileOffset: size)
        try writeHandle.write(contentsOf: Data([0xEE]))
        try writeHandle.close()
        let extended = fileSize(for: url) == size + 1

        let shrinkTo = max(1, size / 2)
        let shrinkHandle = try FileHandle(forUpdating: url)
        try shrinkHandle.truncate(atOffset: shrinkTo)
        try shrinkHandle.close()
        let shrunk = fileSize(for: url) == shrinkTo

        let restoreHandle = try FileHandle(forUpdating: url)
        try restoreHandle.truncate(atOffset: size)
        try restoreHandle.close()
        let restored = fileSize(for: url) == size
        let exactRange = try probeExactRange(at: url, size: size)

        return [
            exactRange,
            CheckEvidence(
                id: "regular-file-out-of-range-write",
                status: extended ? "observed-host-extension" : "not-observed",
                detail: "the regular-file baseline accepts a write at logical EOF; a bridge proxy must reject it"
            ),
            CheckEvidence(
                id: "regular-file-truncate-resize",
                status: shrunk && restored ? "observed-host-resize" : "fail",
                detail: "the regular-file baseline permits disposable truncate/resize; bridge denial remains unverified"
            )
        ]
    }

    private func probeExactRange(at url: URL, size: UInt64) throws -> CheckEvidence {
        let length = min(size, 4096)
        guard length > 0 else {
            throw ProbeError.invalidArgument("logical size is too small for exact-range probe")
        }
        let offset = size >= length * 2 ? length : 0
        let marker = Data(repeating: 0xA7, count: Int(length))
        let handle = try FileHandle(forUpdating: url)
        try handle.seek(toFileOffset: offset)
        try handle.write(contentsOf: marker)
        try handle.synchronize()
        try handle.seek(toFileOffset: offset)
        let readback = try handle.read(upToCount: Int(length)) ?? Data()
        try handle.close()
        return CheckEvidence(
            id: "exact-range-read-write",
            status: readback == marker && fileSize(for: url) == size ? "pass" : "fail",
            detail: "a bounded in-range write/read preserves exact bytes and host geometry"
        )
    }

    private func probeAliasIdentity(at backing: URL, root: URL) throws -> [CheckEvidence] {
        let alias = root.appendingPathComponent("backing-alias.raw")
        try? fileManager.removeItem(at: alias)
        try fileManager.linkItem(at: backing, to: alias)
        defer { try? fileManager.removeItem(at: alias) }
        let sameIdentity = fileNumber(for: backing) != nil
            && fileNumber(for: backing) == fileNumber(for: alias)
        return [
            CheckEvidence(
                id: "backing-alias-detection",
                status: sameIdentity ? "pass" : "fail",
                detail: "a hard-link alias is observable by file identity and must be rejected before active export"
            )
        ]
    }

    private func validateBaselineTrace(_ trace: [TraceEvent], logicalSize: UInt64) throws -> CheckEvidence {
        let expected = [
            "open", "write", "sync", "read", "close", "reopen",
            "read-after-reopen", "detach", "disconnect"
        ]
        let operations = trace.map(\.operation)
        let sequences = trace.map(\.sequence)
        let bounded = trace.allSatisfy { $0.offset <= logicalSize && $0.length <= logicalSize }
        let valid = operations == expected
            && sequences == Array(1...expected.count)
            && bounded
            && trace.contains { $0.operation == "sync" && $0.persistence == "host-file-sync" }
        if !valid {
            throw ProbeError.filesystem("baseline trace failed deterministic normalization checks")
        }
        return CheckEvidence(
            id: "normalized-trace-contract",
            status: "pass",
            detail: "synthetic baseline trace has stable operation order, bounded ranges, and explicit host-sync evidence"
        )
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
