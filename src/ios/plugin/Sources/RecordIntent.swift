import AppIntents
import Foundation

// MUST be byte-identical to the widget's copy (RecordControl.swift), same
// type names: iOS requires the intent COMPILED into both the app binary and
// the appex, or the Control Center tap is silently dropped. Same duplication
// precedent as RecordingAttributes.swift.

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

// Linker anchor: nothing in the Rust host references this object file, so
// without a pulled symbol the static-lib linker dead-strips the intent type
// and the Control Center tap finds no runtime type to perform.
@_cdecl("flowflow_register_record_intent")
public func registerRecordIntent() {}
