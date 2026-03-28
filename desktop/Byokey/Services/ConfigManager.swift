import AppKit
import Foundation

/// Reads and writes `~/.config/byokey/settings.json`.
///
/// Uses a typed `Codable` struct for known fields. Unknown keys are preserved
/// across load/save cycles via a raw overlay so hand-edited config is never destroyed.
@Observable
final class ConfigManager {
    // MARK: - Server

    var port: Int = AppEnvironment.defaultPort { didSet { scheduleSave() } }
    var host: String = "127.0.0.1" { didSet { scheduleSave() } }

    // MARK: - Network

    var proxyUrl: String = "" { didSet { scheduleSave() } }

    // MARK: - Logging

    var logLevel: String = "info" { didSet { scheduleSave() } }

    // MARK: - Streaming

    var keepaliveSeconds: Int = 15 { didSet { scheduleSave() } }
    var bootstrapRetries: Int = 1 { didSet { scheduleSave() } }

    // MARK: - State

    private(set) var configFileExists = false
    private(set) var needsRestart = false
    private var rawOverlay: [String: Any] = [:]
    private var isLoading = false
    private var saveTask: Task<Void, Never>?

    var configURL: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".config/byokey/settings.json")
    }

    // MARK: - Codable Schema

    private struct ConfigFile: Codable {
        var port: Int?
        var host: String?
        var proxy_url: String?
        var log: LogConfig?
        var streaming: StreamingConfig?

        struct LogConfig: Codable {
            var level: String?
        }

        struct StreamingConfig: Codable {
            var keepalive_seconds: Int?
            var bootstrap_retries: Int?
        }
    }

    // MARK: - Load

    func load() {
        isLoading = true
        defer { isLoading = false }

        let url = configURL
        configFileExists = FileManager.default.fileExists(atPath: url.path)
        guard configFileExists, let data = try? Data(contentsOf: url) else { return }

        // Preserve raw overlay for unknown keys
        rawOverlay = (try? JSONSerialization.jsonObject(with: data) as? [String: Any]) ?? [:]

        // Decode typed fields
        guard let config = try? JSONDecoder().decode(ConfigFile.self, from: data) else { return }

        port = config.port ?? AppEnvironment.defaultPort
        host = config.host ?? "127.0.0.1"
        proxyUrl = config.proxy_url ?? ""
        logLevel = config.log?.level ?? "info"
        keepaliveSeconds = config.streaming?.keepalive_seconds ?? 15
        bootstrapRetries = config.streaming?.bootstrap_retries ?? 1
    }

    // MARK: - Save

    private func scheduleSave() {
        guard !isLoading else { return }
        needsRestart = true
        saveTask?.cancel()
        saveTask = Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(500))
            guard !Task.isCancelled else { return }
            self.save()
        }
    }

    func save() {
        // Build typed config
        var config = ConfigFile()
        config.port = port
        config.host = host
        config.proxy_url = proxyUrl.isEmpty ? nil : proxyUrl
        config.log = .init(level: logLevel)
        config.streaming = .init(
            keepalive_seconds: keepaliveSeconds,
            bootstrap_retries: bootstrapRetries
        )

        // Encode typed → merge onto raw overlay (preserving unknown keys)
        if let typedData = try? JSONEncoder().encode(config),
           let typedDict = try? JSONSerialization.jsonObject(with: typedData) as? [String: Any]
        {
            for (key, value) in typedDict {
                rawOverlay[key] = value
            }
            // Remove proxy_url key entirely if empty
            if proxyUrl.isEmpty {
                rawOverlay.removeValue(forKey: "proxy_url")
            }
        }

        let dir = configURL.deletingLastPathComponent()
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)

        guard let data = try? JSONSerialization.data(
            withJSONObject: rawOverlay,
            options: [.prettyPrinted, .sortedKeys]
        ) else { return }
        try? data.write(to: configURL, options: .atomic)
        configFileExists = true
    }

    func revealInFinder() {
        NSWorkspace.shared.selectFile(configURL.path, inFileViewerRootedAtPath: "")
    }

    func openInEditor() {
        NSWorkspace.shared.open(configURL)
    }

    func clearRestartFlag() {
        needsRestart = false
    }
}
