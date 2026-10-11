import ActivityKit
import Foundation

// Copy of the app's type: ActivityKit matches the two by name and fields.
struct HermesTaskAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable {
        var startedAt: Double
        var result: String?
    }

    var jobId: String
    var name: String
}
