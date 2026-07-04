import AppIntents
import SwiftUI
import WidgetKit

// One-gesture capture entry points. The lock-screen widget travels through
// flowflow://record (widgetURL supports custom schemes); the control cannot:
// iOS 26 ERRORS on openAppWhenRun in an extension and refuses custom-scheme
// openURL from there. OpenIntent lets the SYSTEM launch the app, and the
// "start recording" order rides the app group instead of a URL.

@available(iOS 18.0, *)
struct StartRecordingIntent: OpenIntent {
    static var title: LocalizedStringResource = "Dicter une note"
    static var description = IntentDescription(
        "Ouvre FlowFlow et démarre l'enregistrement d'une note vocale."
    )

    @Parameter(title: "Cible")
    var target: RecordTarget

    @MainActor
    func perform() async throws -> some IntentResult {
        UserDefaults(suiteName: "group.com.mirkobozzetto.flowflow")?.set(
            Date().timeIntervalSince1970,
            forKey: "pending_record"
        )
        return .result()
    }
}

// Exactly ONE case: a second one triggers Siri disambiguation and the
// intent silently never runs.
@available(iOS 18.0, *)
enum RecordTarget: String, AppEnum {
    case record

    static var typeDisplayRepresentation =
        TypeDisplayRepresentation(name: "FlowFlow")
    static var caseDisplayRepresentations:
        [RecordTarget: DisplayRepresentation] = [
            .record: DisplayRepresentation(title: "Dicter une note")
        ]
}

// Control Center button; assignable to the Action Button and the Lock
// Screen bottom controls (iOS 18 lets the user place any control there).
@available(iOS 18.0, *)
struct RecordControl: ControlWidget {
    var body: some ControlWidgetConfiguration {
        // Kind bumped once: chronod pins a placed control to its kind and
        // kept serving the old broken intent binding after reinstalls.
        StaticControlConfiguration(
            kind: "com.mirkobozzetto.flowflow.record2"
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
