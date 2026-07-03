import AppIntents
import SwiftUI
import WidgetKit

// One-gesture capture entry points. Both funnel into flowflow://record,
// handled by the Rust app (deeplink watcher starts a new voice note).

@available(iOS 18.0, *)
struct StartRecordingIntent: AppIntent {
    static var title: LocalizedStringResource = "Dicter une note"
    static var description = IntentDescription(
        "Ouvre FlowFlow et démarre l'enregistrement d'une note vocale."
    )
    static var openAppWhenRun: Bool = true

    // OpenURLIntent rejects custom schemes (universal links only), so the
    // deep link goes through EnvironmentValues().openURL instead.
    @MainActor
    func perform() async throws -> some IntentResult {
        EnvironmentValues().openURL(URL(string: "flowflow://record")!)
        return .result()
    }
}

// Control Center button; assignable to the Action Button and the Lock
// Screen bottom controls (iOS 18 lets the user place any control there).
@available(iOS 18.0, *)
struct RecordControl: ControlWidget {
    var body: some ControlWidgetConfiguration {
        StaticControlConfiguration(
            kind: "com.mirkobozzetto.flowflow.record"
        ) {
            ControlWidgetButton(action: StartRecordingIntent()) {
                Label("Dicter une note", systemImage: "mic.fill")
            }
        }
        .displayName("FlowFlow")
        .description("Enregistrer une note vocale")
    }
}

// Lock Screen circular widget (iOS 16+): tap opens the app recording.
struct LockRecordEntry: TimelineEntry {
    let date: Date
}

struct LockRecordProvider: TimelineProvider {
    func placeholder(in context: Context) -> LockRecordEntry {
        LockRecordEntry(date: Date())
    }
    func getSnapshot(
        in context: Context,
        completion: @escaping (LockRecordEntry) -> Void
    ) {
        completion(LockRecordEntry(date: Date()))
    }
    func getTimeline(
        in context: Context,
        completion: @escaping (Timeline<LockRecordEntry>) -> Void
    ) {
        completion(Timeline(entries: [LockRecordEntry(date: Date())], policy: .never))
    }
}

struct LockRecordView: View {
    var body: some View {
        if #available(iOS 17.0, *) {
            Image(systemName: "mic.fill")
                .font(.system(size: 22, weight: .medium))
                .widgetURL(URL(string: "flowflow://record"))
                .containerBackground(.clear, for: .widget)
        } else {
            Image(systemName: "mic.fill")
                .font(.system(size: 22, weight: .medium))
                .widgetURL(URL(string: "flowflow://record"))
        }
    }
}

struct LockRecordWidget: Widget {
    var body: some WidgetConfiguration {
        StaticConfiguration(
            kind: "com.mirkobozzetto.flowflow.lockrecord",
            provider: LockRecordProvider()
        ) { _ in
            LockRecordView()
        }
        .configurationDisplayName("Dicter une note")
        .description("Ouvre FlowFlow en enregistrement.")
        .supportedFamilies([.accessoryCircular])
    }
}
