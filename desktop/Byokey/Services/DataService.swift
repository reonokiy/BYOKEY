import Foundation
import OpenAPIURLSession

@Observable
final class DataService {
    // MARK: - Shared State

    private(set) var providers: [Components.Schemas.ProviderStatus] = []
    private(set) var providerAccounts: [Components.Schemas.ProviderAccounts] = []
    private(set) var usage: UsageSnapshot?
    private(set) var history: UsageHistoryResponse?
    private(set) var rateLimits: RateLimitsResponse?
    private(set) var models: [ModelEntry] = []
    private(set) var isLoading = false

    var isServerReachable = false {
        didSet {
            if isServerReachable, !oldValue {
                startPolling()
            } else if !isServerReachable {
                stopPolling()
                clearAll()
            }
        }
    }

    private var pollTask: Task<Void, Never>?

    private var client: Client {
        Client(
            serverURL: AppEnvironment.shared.baseURL,
            transport: URLSessionTransport()
        )
    }

    // MARK: - Polling

    func startPolling() {
        pollTask?.cancel()
        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self else { return }
                await self.fetchAll()
                try? await Task.sleep(for: .seconds(3))
            }
        }
    }

    func stopPolling() {
        pollTask?.cancel()
        pollTask = nil
    }

    func reload() async {
        await fetchAll()
    }

    func reloadAccounts() async {
        do {
            let response = try await client.accounts_handler()
            providerAccounts = try response.ok.body.json.providers
        } catch {
            // keep existing data on error
        }
    }

    // MARK: - Mutations

    func activateAccount(provider: String, accountId: String) async throws {
        _ = try await client.activate_account_handler(
            path: .init(provider: provider, account_id: accountId)
        )
        await reloadAccounts()
    }

    func removeAccount(provider: String, accountId: String) async throws {
        _ = try await client.remove_account_handler(
            path: .init(provider: provider, account_id: accountId)
        )
        await reloadAccounts()
    }

    // MARK: - Private

    private func fetchAll() async {
        isLoading = true
        defer { isLoading = false }

        do {
            let resp = try await client.status_handler()
            providers = try resp.ok.body.json.providers
        } catch {
            providers = []
        }

        do {
            let resp = try await client.accounts_handler()
            providerAccounts = try resp.ok.body.json.providers
        } catch {
            // keep existing
        }

        do {
            let resp = try await client.usage_handler()
            usage = try resp.ok.body.json
        } catch {
            usage = nil
        }

        do {
            let resp = try await client.ratelimits_handler()
            rateLimits = try resp.ok.body.json
        } catch {
            rateLimits = nil
        }

        do {
            let resp = try await client.list_models()
            models = try resp.ok.body.json.data
        } catch {
            models = []
        }

        let now = Int64(Date().timeIntervalSince1970)
        do {
            let resp = try await client.usage_history_handler(
                .init(path: .init(from: now - 86400, to: now, model: ""))
            )
            history = try resp.ok.body.json
        } catch {
            history = nil
        }
    }

    private func clearAll() {
        providers = []
        providerAccounts = []
        usage = nil
        history = nil
        rateLimits = nil
        models = []
    }
}
