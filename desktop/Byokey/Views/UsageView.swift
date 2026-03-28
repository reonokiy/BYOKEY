import Charts
import SwiftUI

struct UsageView: View {
    @Environment(ProcessManager.self) private var pm
    @Environment(DataService.self) private var dataService
    @State private var history: UsageHistoryResponse?
    @State private var selectedRange: TimeRange = .day
    @State private var isLoading = false

    private var snapshot: UsageSnapshot? { dataService.usage }

    var body: some View {
        Group {
            if pm.isReachable {
                Form {
                    if let snapshot {
                        summarySection(snapshot)
                        tokenBreakdownChart(snapshot)
                        modelsSection(snapshot)
                    }

                    chartSection
                }
                .formStyle(.grouped)
            } else if pm.isRunning {
                ContentUnavailableView {
                    ProgressView().controlSize(.large)
                } description: {
                    Text("Waiting for server…")
                }
            } else {
                ContentUnavailableView(
                    "Server Not Running",
                    systemImage: "chart.bar",
                    description: Text("Enable the proxy server to view usage.")
                )
            }
        }
        .navigationTitle("Usage")
        .task { await loadHistory() }
        .onChange(of: selectedRange) {
            Task { await loadHistory() }
        }
    }

    // MARK: - Summary

    private func summarySection(_ s: UsageSnapshot) -> some View {
        Section("Summary") {
            LabeledContent("Total Requests") {
                Text("\(s.total_requests)")
                    .monospacedDigit()
            }
            LabeledContent("Success Rate") {
                Text(successRate(s))
                    .monospacedDigit()
                    .foregroundStyle(s.failure_requests == 0 ? .green : .orange)
            }
            LabeledContent("Input Tokens") {
                Text(formatTokens(s.input_tokens))
                    .monospacedDigit()
            }
            LabeledContent("Output Tokens") {
                Text(formatTokens(s.output_tokens))
                    .monospacedDigit()
            }
            LabeledContent("Total Tokens") {
                Text(formatTokens(s.input_tokens + s.output_tokens))
                    .monospacedDigit()
                    .fontWeight(.semibold)
            }
        }
    }

    // MARK: - Token Breakdown Chart

    private func tokenBreakdownChart(_ s: UsageSnapshot) -> some View {
        Section("Token Distribution by Model") {
            if s.models.isEmpty {
                Text("No data")
                    .foregroundStyle(.secondary)
            } else {
                let sorted = s.models
                    .map { TokenSlice(model: $0.key, input: $0.value.input_tokens, output: $0.value.output_tokens) }
                    .sorted { ($0.input + $0.output) > ($1.input + $1.output) }

                let top = Array(sorted.prefix(6))

                Chart(top, id: \.model) { slice in
                    BarMark(
                        x: .value("Tokens", slice.input),
                        y: .value("Model", slice.model)
                    )
                    .foregroundStyle(by: .value("Type", "Input"))

                    BarMark(
                        x: .value("Tokens", slice.output),
                        y: .value("Model", slice.model)
                    )
                    .foregroundStyle(by: .value("Type", "Output"))
                }
                .chartForegroundStyleScale([
                    "Input": .indigo,
                    "Output": .cyan,
                ])
                .chartLegend(position: .bottom, alignment: .leading)
                .frame(height: CGFloat(max(top.count, 1)) * 32 + 20)
            }
        }
    }

    // MARK: - Per-Model Table

    private func modelsSection(_ s: UsageSnapshot) -> some View {
        Section("By Model") {
            if s.models.isEmpty {
                Text("No model usage recorded")
                    .foregroundStyle(.secondary)
            } else {
                ForEach(
                    s.models.sorted(by: { $0.value.requests > $1.value.requests }),
                    id: \.key
                ) { model, stats in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text(model)
                                .fontWeight(.medium)
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Spacer()
                            Text("\(stats.requests) req")
                                .foregroundStyle(.secondary)
                                .monospacedDigit()
                        }
                        HStack(spacing: 12) {
                            Label(formatTokens(stats.input_tokens), systemImage: "arrow.up")
                            Label(formatTokens(stats.output_tokens), systemImage: "arrow.down")
                            if stats.failure > 0 {
                                Label("\(stats.failure) failed", systemImage: "exclamationmark.triangle")
                                    .foregroundStyle(.red)
                            }
                        }
                        .font(.caption)
                        .foregroundStyle(.secondary)

                        // Token proportion bar
                        let total = stats.input_tokens + stats.output_tokens
                        if total > 0 {
                            GeometryReader { geo in
                                HStack(spacing: 0) {
                                    Rectangle()
                                        .fill(.indigo.gradient)
                                        .frame(width: geo.size.width * CGFloat(stats.input_tokens) / CGFloat(total))
                                    Rectangle()
                                        .fill(.cyan.gradient)
                                }
                            }
                            .frame(height: 4)
                            .clipShape(RoundedRectangle(cornerRadius: 2))
                        }
                    }
                    .padding(.vertical, 2)
                }
            }
        }
    }

    // MARK: - History Chart

    private var chartSection: some View {
        Section {
            Picker("Period", selection: $selectedRange) {
                ForEach(TimeRange.allCases) { range in
                    Text(range.label).tag(range)
                }
            }
            .pickerStyle(.segmented)
            .padding(.bottom, 4)

            if let history, !history.buckets.isEmpty {
                Chart {
                    ForEach(aggregatedBuckets(history), id: \.period_start) { bucket in
                        BarMark(
                            x: .value("Time", bucketDate(bucket.period_start)),
                            y: .value("Requests", bucket.request_count)
                        )
                        .foregroundStyle(.blue.gradient)
                    }
                }
                .chartXAxis {
                    AxisMarks(values: .automatic(desiredCount: 6)) { _ in
                        AxisGridLine()
                        AxisValueLabel(format: selectedRange.dateFormat)
                    }
                }
                .frame(height: 160)
            } else if isLoading {
                HStack {
                    Spacer()
                    ProgressView().controlSize(.small)
                    Spacer()
                }
                .frame(height: 160)
            } else {
                Text("No history data available")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .frame(height: 80)
            }
        } header: {
            Text("Request History")
        }
    }

    // MARK: - Data

    private func loadHistory() async {
        guard pm.isReachable else {
            history = nil
            return
        }
        isLoading = true
        defer { isLoading = false }
        let now = Int64(Date().timeIntervalSince1970)
        let from = now - selectedRange.seconds
        history = try? await APIClient.usageHistory(from: from, to: now)
    }

    // MARK: - Helpers

    private func successRate(_ s: UsageSnapshot) -> String {
        guard s.total_requests > 0 else { return "–" }
        let rate = Double(s.success_requests) / Double(s.total_requests) * 100
        return String(format: "%.1f%%", rate)
    }

    private func aggregatedBuckets(_ h: UsageHistoryResponse) -> [AggregateBucket] {
        Dictionary(grouping: h.buckets, by: \.period_start)
            .map { key, buckets in
                AggregateBucket(
                    period_start: key,
                    request_count: buckets.reduce(0) { $0 + $1.request_count },
                    input_tokens: buckets.reduce(0) { $0 + $1.input_tokens },
                    output_tokens: buckets.reduce(0) { $0 + $1.output_tokens }
                )
            }
            .sorted(by: { $0.period_start < $1.period_start })
    }

    private func bucketDate(_ ts: Int64) -> Date {
        Date(timeIntervalSince1970: TimeInterval(ts))
    }
}

// MARK: - Supporting Types

private struct TokenSlice {
    let model: String
    let input: UInt64
    let output: UInt64
}

private struct AggregateBucket {
    let period_start: Int64
    let request_count: UInt64
    let input_tokens: UInt64
    let output_tokens: UInt64
}

enum TimeRange: String, CaseIterable, Identifiable {
    case day = "24h"
    case week = "7d"
    case month = "30d"

    var id: Self { self }

    var label: String { rawValue }

    var seconds: Int64 {
        switch self {
        case .day: 86400
        case .week: 604_800
        case .month: 2_592_000
        }
    }

    var dateFormat: Date.FormatStyle {
        switch self {
        case .day: .dateTime.hour()
        case .week: .dateTime.weekday(.abbreviated)
        case .month: .dateTime.month(.abbreviated).day()
        }
    }
}

#Preview {
    UsageView()
        .environment(ProcessManager())
        .environment(DataService())
}
