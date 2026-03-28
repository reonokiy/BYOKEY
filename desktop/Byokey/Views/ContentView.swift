import SwiftUI

enum SidebarItem: String, CaseIterable, Identifiable {
    case general = "General"
    case accounts = "Accounts"
    case settings = "Settings"

    var id: Self { self }

    var icon: String {
        switch self {
        case .general: "house"
        case .accounts: "person.2"
        case .settings: "gearshape"
        }
    }
}

struct ContentView: View {
    @Environment(ProcessManager.self) private var pm
    @State private var selection: SidebarItem? = .general

    var body: some View {
        @Bindable var pm = pm

        NavigationSplitView {
            List(SidebarItem.allCases, selection: $selection) { item in
                Label(item.rawValue, systemImage: item.icon)
            }
            .navigationSplitViewColumnWidth(min: 160, ideal: 180, max: 220)
            .listStyle(.sidebar)
        } detail: {
            switch selection {
            case .general:  GeneralView()
            case .accounts: AccountsView()
            case .settings: SettingsView()
            case nil:       Text("Select a page")
            }
        }
        .frame(minWidth: 480, minHeight: 320)
        .alert("Server Error", isPresented: $pm.showError) {
            Button("Reload") { pm.restart() }
            Button("OK", role: .cancel) {}
        } message: {
            Text(pm.errorMessage ?? "Unknown error")
        }
    }
}

#Preview {
    ContentView()
        .environment(ProcessManager())
}
