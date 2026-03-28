import ServiceManagement
import SwiftUI

struct SettingsView: View {
    @Environment(ProcessManager.self) private var pm
    @State private var config = ConfigManager()
    @State private var launchAtLogin = SMAppService.mainApp.status == .enabled

    var body: some View {
        Form {
            Section("General") {
                Toggle("Launch at Login", isOn: $launchAtLogin)
                    .onChange(of: launchAtLogin) { _, newValue in
                        do {
                            if newValue {
                                try SMAppService.mainApp.register()
                            } else {
                                SMAppService.mainApp.unregister { _ in }
                            }
                        } catch {
                            launchAtLogin = SMAppService.mainApp.status == .enabled
                        }
                    }
            }

            Section("Server") {
                TextField("Port", value: $config.port, format: .number.grouping(.never))
                    .monospacedDigit()
                TextField("Host", text: $config.host)
                    .monospacedDigit()
            }

            Section("Network") {
                TextField("Proxy URL", text: $config.proxyUrl, prompt: Text("socks5://host:port"))
            }

            Section("Streaming") {
                TextField(
                    "SSE Keepalive (seconds)",
                    value: $config.keepaliveSeconds,
                    format: .number.grouping(.never)
                )
                .monospacedDigit()

                TextField(
                    "Bootstrap Retries",
                    value: $config.bootstrapRetries,
                    format: .number.grouping(.never)
                )
                .monospacedDigit()
            }

            Section("Logging") {
                Picker("Level", selection: $config.logLevel) {
                    Text("Error").tag("error")
                    Text("Warn").tag("warn")
                    Text("Info").tag("info")
                    Text("Debug").tag("debug")
                    Text("Trace").tag("trace")
                }
            }

            Section {
                LabeledContent("Path") {
                    Text(config.configURL.path)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }

                HStack {
                    Button("Reveal in Finder") {
                        config.revealInFinder()
                    }
                    Button("Open in Editor") {
                        config.openInEditor()
                    }
                }
            } header: {
                Text("Config File")
            } footer: {
                Text("Changes are saved automatically. Restart the server to apply.")
            }

            Section {
                Button("Restart Server") {
                    pm.restart(port: config.port)
                }
                .disabled(!pm.isRunning)
            }
        }
        .formStyle(.grouped)
        .navigationTitle("Settings")
        .onAppear { config.load() }
    }
}

#Preview {
    SettingsView()
        .environment(ProcessManager())
}
