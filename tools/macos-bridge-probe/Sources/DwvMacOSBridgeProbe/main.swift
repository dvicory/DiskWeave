import Darwin
import Foundation

func parseArguments(_ values: ArraySlice<String>) throws -> ProbeArguments {
    var root: URL?
    var size: UInt64 = 16 * 1024 * 1024
    var keep = false
    var index = values.startIndex

    while index < values.endIndex {
        switch values[index] {
        case "--root":
            index = values.index(after: index)
            guard index < values.endIndex else { throw ProbeError.invalidArgument("--root requires a path") }
            root = URL(fileURLWithPath: values[index], isDirectory: true)
        case "--size":
            index = values.index(after: index)
            guard index < values.endIndex, let parsed = UInt64(values[index]), parsed > 0 else {
                throw ProbeError.invalidArgument("--size requires a positive byte count")
            }
            size = parsed
        case "--keep":
            keep = true
        case "--help", "-h":
            print("usage: dwv-macos-bridge-probe [--root PATH] [--size BYTES] [--keep]")
            exit(0)
        default:
            throw ProbeError.invalidArgument(values[index])
        }
        index = values.index(after: index)
    }

    return ProbeArguments(root: root, logicalSize: size, keepFixture: keep)
}

do {
    let arguments = try parseArguments(CommandLine.arguments.dropFirst())
    let evidence = try ProbeRunner().run(arguments: arguments)
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    let data = try encoder.encode(evidence)
    print(String(decoding: data, as: UTF8.self))
} catch {
    fputs("dwv-macos-bridge-probe: \(error)\n", stderr)
    exit(1)
}
