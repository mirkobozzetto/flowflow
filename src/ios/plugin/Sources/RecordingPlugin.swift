import ActivityKit
import Foundation

private var currentActivity: Activity<RecordingAttributes>?

// If the process dies mid-recording nothing ever calls end(): the stale date
// caps how long the zombie indicator can survive on the lock screen.
private let staleAfterSeconds: TimeInterval = 2 * 60 * 60

@_cdecl("flowflow_start_live_activity")
public func startLiveActivity(_ startedAtUnix: Int64, _ isPaused: Bool) {
    Task { @MainActor in
        // A fresh start means anything already registered is an orphan.
        for stale in Activity<RecordingAttributes>.activities
        where stale.id != currentActivity?.id {
            await stale.end(nil, dismissalPolicy: .immediate)
        }
        let attrs = RecordingAttributes()
        let state = RecordingAttributes.ContentState(
            startedAt: Date(timeIntervalSince1970: Double(startedAtUnix)),
            isPaused: isPaused
        )
        let content = ActivityContent(
            state: state,
            staleDate: Date().addingTimeInterval(staleAfterSeconds)
        )
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
        let content = ActivityContent(
            state: state,
            staleDate: Date().addingTimeInterval(staleAfterSeconds)
        )
        await activity.update(content)
    }
}

// App launch sweep: a killed process leaves its activity alive for hours
// (iOS keeps un-ended activities up to 8-12h). Kill every activity that is
// not the one this process owns.
@_cdecl("flowflow_cleanup_live_activities")
public func cleanupLiveActivities() {
    Task { @MainActor in
        for activity in Activity<RecordingAttributes>.activities
        where activity.id != currentActivity?.id {
            await activity.end(nil, dismissalPolicy: .immediate)
        }
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
