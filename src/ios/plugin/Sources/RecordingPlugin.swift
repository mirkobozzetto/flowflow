import ActivityKit
import Foundation

private var currentActivity: Activity<RecordingAttributes>?

@_cdecl("flowflow_start_live_activity")
public func startLiveActivity(_ startedAtUnix: Int64, _ isPaused: Bool) {
    Task { @MainActor in
        let attrs = RecordingAttributes()
        let state = RecordingAttributes.ContentState(
            startedAt: Date(timeIntervalSince1970: Double(startedAtUnix)),
            isPaused: isPaused
        )
        let content = ActivityContent(state: state, staleDate: nil)
        do {
            currentActivity = try Activity.request(
                attributes: attrs,
                content: content,
                pushType: nil
            )
        } catch {
            print("[live-activity] start failed: \(error)")
        }
    }
}

@_cdecl("flowflow_update_live_activity")
public func updateLiveActivity(_ startedAtUnix: Int64, _ isPaused: Bool) {
    Task { @MainActor in
        guard let activity = currentActivity else { return }
        let state = RecordingAttributes.ContentState(
            startedAt: Date(timeIntervalSince1970: Double(startedAtUnix)),
            isPaused: isPaused
        )
        let content = ActivityContent(state: state, staleDate: nil)
        await activity.update(content)
    }
}

@_cdecl("flowflow_end_live_activity")
public func endLiveActivity() {
    Task { @MainActor in
        guard let activity = currentActivity else { return }
        await activity.end(nil, dismissalPolicy: .immediate)
        currentActivity = nil
    }
}
