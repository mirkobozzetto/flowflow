import ActivityKit
import SwiftUI
import WidgetKit

private struct HermesMark: View {
    var size: CGFloat
    var done: Bool
    var body: some View {
        Image(systemName: done ? "checkmark.circle.fill" : "sparkles")
            .font(.system(size: size, weight: .medium))
            .foregroundStyle(flowOrange)
    }
}

private extension ActivityViewContext<HermesTaskAttributes> {
    var started: Date { Date(timeIntervalSince1970: state.startedAt) }
    // A tap opens the task, as the result's notification does.
    var taskLink: URL? {
        let name = attributes.name.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? ""
        return URL(string: "flowflow://hermes/jobs?name=\(name)")
    }
    var running: Bool { state.result == nil && !isStale }
    // The server ends a task without a result after 30 minutes, with an
    // empty one; a stale activity whose end never came reads the same.
    var outcome: String {
        guard let result = state.result, !result.isEmpty else { return "Pas de résultat" }
        return result
    }
}

private struct Status: View {
    let context: ActivityViewContext<HermesTaskAttributes>
    var body: some View {
        if context.running {
            HStack(spacing: 4) {
                Text("En cours")
                Text(context.started, style: .timer)
                    .monospacedDigit()
            }
        } else {
            Text(context.outcome)
                .lineLimit(2)
        }
    }
}

struct HermesTaskLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: HermesTaskAttributes.self) { context in
            HStack(alignment: .top, spacing: 12) {
                HermesMark(size: 18, done: !context.running)
                VStack(alignment: .leading, spacing: 2) {
                    Text(context.attributes.name)
                        .font(.subheadline.weight(.semibold))
                        .lineLimit(1)
                    Status(context: context)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Text("Hermes")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
            .widgetURL(context.taskLink)
        } dynamicIsland: { context in
            DynamicIsland {
                DynamicIslandExpandedRegion(.leading) {
                    HermesMark(size: 18, done: !context.running)
                }
                DynamicIslandExpandedRegion(.center) {
                    Text(context.attributes.name)
                        .font(.subheadline.weight(.semibold))
                        .lineLimit(1)
                }
                DynamicIslandExpandedRegion(.bottom) {
                    Status(context: context)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            } compactLeading: {
                HermesMark(size: 12, done: !context.running)
            } compactTrailing: {
                if context.running {
                    Text(context.started, style: .timer)
                        .font(.caption2.monospacedDigit())
                        .frame(maxWidth: 44, alignment: .trailing)
                } else {
                    EmptyView()
                }
            } minimal: {
                HermesMark(size: 12, done: !context.running)
            }
            .widgetURL(context.taskLink)
        }
    }
}
