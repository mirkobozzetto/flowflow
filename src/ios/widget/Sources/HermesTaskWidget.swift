import ActivityKit
import SwiftUI
import UIKit
import WidgetKit

// The app writes it into the shared container at launch; a symbol stands in
// until then.
private let hermesFace: UIImage? = FileManager.default
    .containerURL(forSecurityApplicationGroupIdentifier: "group.com.mirkobozzetto.flowflow")
    .flatMap { UIImage(contentsOfFile: $0.appendingPathComponent("hermes-face.png").path) }

private extension ActivityViewContext<HermesTaskAttributes> {
    var started: Date { Date(timeIntervalSince1970: state.startedAt) }
    var running: Bool { state.result == nil && !isStale }
    // The server ends a task without a result after 30 minutes, with an
    // empty one; a stale activity whose end never came reads the same.
    var outcome: String {
        guard let result = state.result, !result.isEmpty else { return "Pas de résultat" }
        return result
    }
    var took: String {
        let seconds = Int((state.endedAt ?? state.startedAt) - state.startedAt)
        return String(format: "%d:%02d", seconds / 60, seconds % 60)
    }
    // A tap opens the task, as the result's notification does.
    var taskLink: URL? {
        let name = attributes.name.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? ""
        return URL(string: "flowflow://hermes/jobs?name=\(name)")
    }
}

private struct Face: View {
    var size: CGFloat
    var body: some View {
        Group {
            if let face = hermesFace {
                Image(uiImage: face).resizable().scaledToFill()
            } else {
                Image(systemName: "sparkles")
                    .font(.system(size: size * 0.5, weight: .semibold))
                    .foregroundStyle(flowOrange)
                    .frame(width: size, height: size)
                    .background(Circle().fill(Color.white.opacity(0.12)))
            }
        }
        .frame(width: size, height: size)
        .clipShape(Circle())
    }
}

private struct Working: View {
    var body: some View {
        HStack(spacing: 3) {
            Text("Hermes travaille")
            if #available(iOS 17.0, *) {
                Image(systemName: "ellipsis")
                    .symbolEffect(.variableColor.iterative, options: .repeating)
            } else {
                Image(systemName: "ellipsis")
            }
        }
    }
}

private struct Clock: View {
    let context: ActivityViewContext<HermesTaskAttributes>
    var body: some View {
        Group {
            if context.running {
                Text(context.started, style: .timer)
            } else {
                HStack(spacing: 4) {
                    Image(systemName: "checkmark")
                    Text(context.took)
                }
            }
        }
        .monospacedDigit()
        .foregroundStyle(flowOrange)
    }
}

// Who works and on what while it runs; the start of the answer once done.
private struct Headline: View {
    let context: ActivityViewContext<HermesTaskAttributes>
    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            if context.running {
                Working().font(.subheadline.weight(.semibold))
                Text(context.attributes.name)
                    .font(.caption)
                    .foregroundStyle(.white.opacity(0.7))
                    .lineLimit(1)
            } else {
                Text(context.attributes.name)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(1)
                Text(context.outcome)
                    .font(.caption)
                    .foregroundStyle(.white.opacity(0.7))
                    .lineLimit(2)
            }
        }
        .foregroundStyle(.white)
    }
}

struct HermesTaskLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: HermesTaskAttributes.self) { context in
            HStack(alignment: .center, spacing: 12) {
                Face(size: 40)
                Headline(context: context)
                Spacer(minLength: 8)
                Clock(context: context).font(.subheadline.weight(.semibold))
            }
            .padding(.horizontal, 18)
            .padding(.vertical, 14)
            .activityBackgroundTint(.black.opacity(0.85))
            .activitySystemActionForegroundColor(.white)
            .widgetURL(context.taskLink)
        } dynamicIsland: { context in
            DynamicIsland {
                DynamicIslandExpandedRegion(.leading) {
                    Face(size: 44)
                        .padding(.leading, 4)
                }
                DynamicIslandExpandedRegion(.trailing) {
                    Clock(context: context)
                        .font(.headline)
                        .padding(.trailing, 4)
                }
                DynamicIslandExpandedRegion(.center) {
                    Headline(context: context)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            } compactLeading: {
                Face(size: 22)
            } compactTrailing: {
                // Who on the left, where it stands on the right.
                Group {
                    if context.running {
                        Text(context.started, style: .timer)
                            .monospacedDigit()
                            .frame(maxWidth: 44, alignment: .trailing)
                    } else {
                        Image(systemName: "checkmark")
                    }
                }
                .font(.caption.weight(.semibold))
                .foregroundStyle(flowOrange)
            } minimal: {
                Face(size: 22)
            }
            .widgetURL(context.taskLink)
            .keylineTint(flowOrange)
        }
    }
}
