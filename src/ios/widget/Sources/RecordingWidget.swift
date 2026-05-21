import ActivityKit
import SwiftUI
import WidgetKit

let flowOrange = Color(red: 232/255, green: 93/255, blue: 10/255)

struct RecIndicator: View {
    var size: CGFloat
    var body: some View {
        if #available(iOS 17.0, *) {
            Image(systemName: "waveform")
                .font(.system(size: size, weight: .medium))
                .foregroundStyle(flowOrange)
                .symbolEffect(
                    .variableColor.iterative.reversing,
                    options: .repeating.speed(0.5)
                )
        } else {
            Image(systemName: "waveform")
                .font(.system(size: size, weight: .medium))
                .foregroundStyle(flowOrange)
        }
    }
}

struct RecordingLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: RecordingAttributes.self) { context in
            HStack(spacing: 10) {
                RecIndicator(size: 14)
                if context.state.isPaused {
                    Text("En pause")
                        .font(.subheadline.weight(.medium))
                        .foregroundStyle(.secondary)
                } else {
                    Text(context.state.startedAt, style: .timer)
                        .font(.subheadline.monospacedDigit().weight(.medium))
                        .contentTransition(.numericText())
                }
                Spacer()
                Text("FlowFlow")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
        } dynamicIsland: { context in
            DynamicIsland {
                DynamicIslandExpandedRegion(.leading) {
                    HStack(spacing: 6) {
                        RecIndicator(size: 16)
                        Text("REC")
                            .font(.caption.weight(.bold))
                            .foregroundStyle(flowOrange)
                    }
                }
                DynamicIslandExpandedRegion(.trailing) {
                    if context.state.isPaused {
                        Text("En pause")
                            .font(.headline.weight(.medium))
                            .foregroundStyle(.secondary)
                    } else {
                        Text(context.state.startedAt, style: .timer)
                            .font(.headline.monospacedDigit())
                            .frame(maxWidth: 90, alignment: .trailing)
                            .contentTransition(.numericText())
                    }
                }
                DynamicIslandExpandedRegion(.center) {
                    Text("FlowFlow")
                        .font(.caption2.weight(.medium))
                        .foregroundStyle(.secondary)
                }
            } compactLeading: {
                HStack(spacing: 5) {
                    RecIndicator(size: 10)
                    Text(context.state.startedAt, style: .timer)
                        .font(.caption2.monospacedDigit())
                        .frame(maxWidth: 48, alignment: .leading)
                        .contentTransition(.numericText())
                }
            } compactTrailing: {
                EmptyView()
            } minimal: {
                RecIndicator(size: 10)
            }
        }
    }
}

@main
struct RecordingWidgetBundle: WidgetBundle {
    var body: some Widget {
        RecordingLiveActivity()
    }
}
