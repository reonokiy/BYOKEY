/// Type aliases mapping old hand-written types to the generated OpenAPI types.
/// This lets Views continue using short names (e.g. `UsageSnapshot`)
/// while all data comes from the generated `Components.Schemas` namespace.

typealias UsageSnapshot = Components.Schemas.UsageSnapshot
typealias UsageHistoryResponse = Components.Schemas.UsageHistoryResponse
typealias UsageBucket = Components.Schemas.UsageBucket
typealias ModelStats = Components.Schemas.ModelStats
typealias ModelEntry = Components.Schemas.ModelEntry
typealias ModelsResponse = Components.Schemas.ModelsResponse

extension Components.Schemas.ModelEntry: Identifiable {}
typealias RateLimitsResponse = Components.Schemas.RateLimitsResponse
typealias ProviderRateLimits = Components.Schemas.ProviderRateLimits
typealias AccountRateLimit = Components.Schemas.AccountRateLimit
typealias RateLimitSnapshot = Components.Schemas.RateLimitSnapshot
