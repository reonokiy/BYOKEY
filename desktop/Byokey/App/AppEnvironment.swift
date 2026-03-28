import Foundation

enum AppEnvironment {
    static let bundleIdentifier = Bundle.main.bundleIdentifier ?? "io.byokey.desktop"
    static let isDev: Bool = bundleIdentifier.hasSuffix(".dev")
    static let defaultPort: Int = isDev ? 8019 : 8018
    static var baseURL: URL { URL(string: "http://127.0.0.1:\(defaultPort)")! }
}
