import SwiftUI

@main
struct ByokeyApp: App {
    @State private var processManager = ProcessManager()

    var body: some Scene {
        MenuBarExtra("BYOKEY", systemImage: processManager.isReachable ? "server.rack" : "server.rack.slash") {
            ContentView()
                .environment(processManager)
                .onAppear {
                    processManager.start()
                }
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView()
                .environment(processManager)
        }
    }
}
