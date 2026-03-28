import SwiftUI
import OpenAPIURLSession

struct GeneralView: View {
    @Environment(ProcessManager.self) private var pm
    @State private var providers: [Components.Schemas.ProviderStatus] = []
    @State private var pollTask: Task<Void, Never>?

    var body: some View {
        Form {
            Section("Proxy Server") {
                LabeledContent("Status") {
                    HStack(spacing: 6) {
                        if pm.isReachable {
                            Circle().fill(.green).frame(width: 8, height: 8)
                            Text("Running")
                        } else if pm.isRunning {
                            ProgressView()
                                .controlSize(.small)
                            Text("Starting…")
                                .foregroundStyle(.secondary)
                        } else {
                            Circle().fill(.red).frame(width: 8, height: 8)
                            Text("Stopped")
                                .foregroundStyle(.secondary)
                        }
                    }
                }

                Toggle("Enabled", isOn: Binding(
                    get: { pm.isRunning },
                    set: { newValue in
                        if newValue {
                            pm.start()
                        } else {
                            pm.stop()
                        }
                    }
                ))

                if let error = pm.errorMessage {
                    Label(error, systemImage: "exclamationmark.triangle.fill")
                        .foregroundStyle(.red)
                        .font(.caption)
                }
            }

            if pm.isReachable {
                Section("Providers") {
                    if providers.isEmpty {
                        Text("No providers configured")
                            .foregroundStyle(.secondary)
                    } else {
                        ForEach(providers, id: \.id) { provider in
                            ProviderRow(provider: provider)
                        }
                    }
                }
            }

            LogSection()
        }
        .formStyle(.grouped)
        .navigationTitle("General")
        .onAppear { startPolling() }
        .onDisappear { pollTask?.cancel() }
    }

    private func startPolling() {
        pollTask?.cancel()
        pollTask = Task {
            let client = Client(
                serverURL: AppEnvironment.baseURL,
                transport: URLSessionTransport()
            )
            while !Task.isCancelled {
                if pm.isReachable {
                    do {
                        let response = try await client.status_handler()
                        let status = try response.ok.body.json
                        providers = status.providers
                    } catch {
                        providers = []
                    }
                } else {
                    providers = []
                }
                try? await Task.sleep(for: .seconds(3))
            }
        }
    }
}

private struct ProviderRow: View {
    let provider: Components.Schemas.ProviderStatus

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(provider.display_name)
                Text("\(provider.models_count) models")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }

            Spacer()

            HStack(spacing: 6) {
                Text(authLabel)
                    .font(.caption)
                    .foregroundStyle(authColor)
                Circle()
                    .fill(authColor)
                    .frame(width: 8, height: 8)
            }
        }
        .opacity(provider.enabled ? 1 : 0.5)
    }

    private var authColor: Color {
        switch provider.auth_status {
        case .valid: .green
        case .expired: .orange
        case .not_configured: .gray
        }
    }

    private var authLabel: String {
        switch provider.auth_status {
        case .valid: "Active"
        case .expired: "Expired"
        case .not_configured: "Not Configured"
        }
    }
}

/// Inline log section that reads from ProcessManager.logs.
private struct LogSection: View {
    @Environment(ProcessManager.self) private var pm

    var body: some View {
        Section("Log") {
            VStack(spacing: 0) {
                logContent
                    .frame(height: 48)

                Divider()

                HStack(spacing: 12) {
                    Text("\(pm.logs.count) lines")
                        .foregroundStyle(.tertiary)
                        .monospacedDigit()
                    Spacer()
                    Button("Clear", systemImage: "trash") {
                        pm.clearLogs()
                    }
                    .buttonStyle(.borderless)
                    .labelStyle(.iconOnly)
                }
                .font(.caption2)
                .padding(.top, 4)
            }
        }
    }

    private var logContent: some View {
        ScrollViewReader { proxy in
            ScrollView(.vertical, showsIndicators: false) {
                if pm.logs.isEmpty {
                    Text("Waiting for log entries…")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(.tertiary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                } else {
                    VStack(alignment: .leading, spacing: 0) {
                        ForEach(Array(pm.logs.enumerated()), id: \.offset) { index, line in
                            Text(line)
                                .font(.system(size: 11, design: .monospaced))
                                .lineLimit(1)
                                .truncationMode(.tail)
                                .textSelection(.enabled)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .id(index)
                        }
                    }
                }
            }
            .onChange(of: pm.logs.count) {
                proxy.scrollTo(pm.logs.count - 1, anchor: .bottom)
            }
        }
    }
}

#Preview {
    GeneralView()
        .environment(ProcessManager())
}
