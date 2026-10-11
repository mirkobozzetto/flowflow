import ActivityKit
import Foundation

// The server starts and ends these by push; the name and every field must
// match its payload and the widget's copy of this type.
struct HermesTaskAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable {
        // Unix seconds: ActivityKit decodes a pushed Date as seconds since 2001.
        var startedAt: Double
        // None while the task runs; empty when it ended without a result.
        var result: String?
    }

    var jobId: String
    var name: String
}

public typealias HermesStartToken = @convention(c) (UnsafePointer<CChar>) -> Void
public typealias HermesActivityToken = @convention(c) (UnsafePointer<CChar>, UnsafePointer<CChar>) -> Void

private func hex(_ data: Data) -> String {
    data.map { String(format: "%02x", $0) }.joined()
}

private func observeToken(_ activity: Activity<HermesTaskAttributes>, _ onToken: HermesActivityToken) {
    Task {
        for await token in activity.pushTokenUpdates {
            activity.attributes.jobId.withCString { job in
                hex(token).withCString { onToken(job, $0) }
            }
        }
    }
}

/// Hands over the push-to-start token (iOS 17.2 and later), then the update
/// token of every Hermes activity: those already running, which a push may
/// have started before this launch, and each new one.
@_cdecl("flowflow_hermes_activities_observe")
public func hermesActivitiesObserve(_ onStart: HermesStartToken, _ onToken: HermesActivityToken) {
    if #available(iOS 17.2, *) {
        Task {
            for await token in Activity<HermesTaskAttributes>.pushToStartTokenUpdates {
                hex(token).withCString { onStart($0) }
            }
        }
    }
    for activity in Activity<HermesTaskAttributes>.activities {
        observeToken(activity, onToken)
    }
    Task {
        for await activity in Activity<HermesTaskAttributes>.activityUpdates {
            observeToken(activity, onToken)
        }
    }
}
