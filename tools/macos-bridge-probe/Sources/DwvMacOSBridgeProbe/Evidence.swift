import Foundation

struct ProbeArguments {
    let root: URL?
    let logicalSize: UInt64
    let keepFixture: Bool
}

struct ProbeEvidence: Codable {
    let schemaVersion: Int
    let host: HostEvidence
    let fixture: FixtureEvidence
    let checks: [CheckEvidence]
    let trace: [TraceEvent]
    let claims: [String]
}

struct HostEvidence: Codable {
    let operatingSystem: String
    let architecture: String
    let sdkPath: String?
    let tools: [String: Bool]
}

struct FixtureEvidence: Codable {
    let root: String
    let backingPath: String
    let exportPath: String
    let logicalSize: UInt64
    let backingFileNumber: UInt64?
    let exportFileNumber: UInt64?
}

struct CheckEvidence: Codable {
    let id: String
    let status: String
    let detail: String
}

struct TraceEvent: Codable {
    let sequence: Int
    let operation: String
    let offset: UInt64
    let length: UInt64
    let status: String
    let persistence: String
    let detail: String
}

enum ProbeError: Error, CustomStringConvertible {
    case invalidArgument(String)
    case filesystem(String)

    var description: String {
        switch self {
        case .invalidArgument(let value): return "invalid argument: \(value)"
        case .filesystem(let value): return "filesystem error: \(value)"
        }
    }
}
