import SwiftUI

struct AmpView: View {
    @Environment(ProcessManager.self) private var pm
    @Environment(AppEnvironment.self) private var appEnv
    @State private var isInjecting = false
    @State private var isTogglingAds = false
    @State private var resultMessage: ResultMessage?
    @State private var injectionStatus: InjectionStatus = .unknown

    private var proxyURL: String {
        "\(appEnv.baseURL.absoluteString)/amp"
    }

    var body: some View {
        Form {
            Section {
                LabeledContent("Proxy URL") {
                    Text(proxyURL)
                        .font(.caption)
                        .fontDesign(.monospaced)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }

                injectionStatusRow

                Button {
                    Task { await inject() }
                } label: {
                    HStack(spacing: 6) {
                        if isInjecting {
                            ProgressView().controlSize(.small)
                        }
                        Text("Inject into Amp Settings")
                    }
                }
                .disabled(isInjecting)
            } header: {
                Text("Proxy Injection")
            } footer: {
                Text("Writes the BYOKEY proxy URL into ~/.config/amp/settings.json")
            }

            Section {
                HStack {
                    Button {
                        Task { await toggleAds(disable: true) }
                    } label: {
                        HStack(spacing: 6) {
                            if isTogglingAds {
                                ProgressView().controlSize(.small)
                            }
                            Text("Disable Ads")
                        }
                    }
                    .disabled(isTogglingAds)

                    Button {
                        Task { await toggleAds(disable: false) }
                    } label: {
                        Text("Restore Ads")
                    }
                    .disabled(isTogglingAds)
                }
            } header: {
                Text("Ads Control")
            } footer: {
                Text(
                    "Patches Amp CLI and editor extensions to hide ads. Restart Amp / reload editor window to apply."
                )
            }

            if let result = resultMessage {
                Section {
                    Label {
                        Text(result.text)
                            .font(.caption)
                            .textSelection(.enabled)
                    } icon: {
                        Image(
                            systemName: result.isError
                                ? "exclamationmark.triangle.fill" : "checkmark.circle.fill"
                        )
                        .foregroundStyle(result.isError ? .red : .green)
                    }
                }
            }
        }
        .formStyle(.grouped)
        .navigationTitle("Amp")
        .onAppear { checkInjectionStatus() }
    }

    // MARK: - Injection Status

    @ViewBuilder
    private var injectionStatusRow: some View {
        switch injectionStatus {
        case .unknown:
            EmptyView()
        case .injected:
            Label("Already injected", systemImage: "checkmark.circle.fill")
                .foregroundStyle(.green)
                .font(.caption)
        case .differentURL(let current):
            Label("Different URL configured: \(current)", systemImage: "exclamationmark.circle.fill")
                .foregroundStyle(.orange)
                .font(.caption)
        case .notConfigured:
            Label("Not configured", systemImage: "circle.dashed")
                .foregroundStyle(.secondary)
                .font(.caption)
        case .noFile:
            Label("Amp settings file not found", systemImage: "doc.badge.plus")
                .foregroundStyle(.secondary)
                .font(.caption)
        }
    }

    private func checkInjectionStatus() {
        let settingsURL = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".config/amp/settings.json")

        guard FileManager.default.fileExists(atPath: settingsURL.path),
              let data = try? Data(contentsOf: settingsURL),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            injectionStatus = .noFile
            return
        }

        guard let ampURL = json["amp.url"] as? String else {
            injectionStatus = .notConfigured
            return
        }

        if ampURL == proxyURL {
            injectionStatus = .injected
        } else {
            injectionStatus = .differentURL(ampURL)
        }
    }

    // MARK: - Actions

    private func inject() async {
        isInjecting = true
        defer { isInjecting = false }
        do {
            let output = try await CLIRunner.ampInject()
            resultMessage = .init(
                text: output.trimmingCharacters(in: .whitespacesAndNewlines), isError: false)
            checkInjectionStatus()
        } catch {
            resultMessage = .init(text: error.localizedDescription, isError: true)
        }
    }

    private func toggleAds(disable: Bool) async {
        isTogglingAds = true
        defer { isTogglingAds = false }
        do {
            let output =
                disable
                ? try await CLIRunner.ampAdsDisable()
                : try await CLIRunner.ampAdsEnable()
            resultMessage = .init(
                text: output.trimmingCharacters(in: .whitespacesAndNewlines), isError: false)
        } catch {
            resultMessage = .init(text: error.localizedDescription, isError: true)
        }
    }
}

// MARK: - Supporting Types

private enum InjectionStatus {
    case unknown
    case injected
    case differentURL(String)
    case notConfigured
    case noFile
}

private struct ResultMessage {
    let text: String
    let isError: Bool
}

#Preview {
    AmpView()
        .environment(AppEnvironment.shared)
        .environment(ProcessManager())
}
