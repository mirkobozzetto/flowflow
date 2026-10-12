import ObjectiveC
import UIKit
import UserNotifications

// A Hermes run followed by iOS itself while the app sleeps: a background
// download of the run's event stream, which Hermes closes when the run ends.
// iOS keeps it open out of process and wakes the app at the end, which then
// says so with a notification.

private let identifier = (Bundle.main.bundleIdentifier ?? "flowflow") + ".hermes-runs"
private var finishEvents: (() -> Void)?

private final class Watcher: NSObject, URLSessionDownloadDelegate {
    // The file is gone once this returns: read it here.
    func urlSession(
        _ session: URLSession,
        downloadTask: URLSessionDownloadTask,
        didFinishDownloadingTo location: URL
    ) {
        let stream = (try? String(contentsOf: location, encoding: .utf8)) ?? ""
        let status = (downloadTask.response as? HTTPURLResponse)?.statusCode ?? 0
        let label = downloadTask.taskDescription ?? ""
        NSLog("[hermes-watch] run stream closed: HTTP %d, %d bytes", status, stream.utf8.count)
        guard status == 200 else { return }
        DispatchQueue.main.async { announce(stream, label) }
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        if let error {
            NSLog("[hermes-watch] run stream lost: %@", error.localizedDescription)
        }
    }

    func urlSessionDidFinishEvents(forBackgroundURLSession session: URLSession) {
        DispatchQueue.main.async {
            finishEvents?()
            finishEvents = nil
        }
    }
}

private let watcher = Watcher()

// One session per process: iOS hands the runs still followed back to the
// session made with the same identifier at the next launch.
private let session: URLSession = {
    let config = URLSessionConfiguration.background(withIdentifier: identifier)
    config.sessionSendsLaunchEvents = true
    config.isDiscretionary = false
    // Hermes writes a keepalive every 10 s; a minute of silence is a drop.
    config.timeoutIntervalForRequest = 60
    return URLSession(configuration: config, delegate: watcher, delegateQueue: nil)
}()

private struct Label: Decodable {
    let session: String
    let title: String
    let failed: String
}

// The run's last word: its answer, or why it failed. A run the person
// stopped needs no alert.
private func announce(_ stream: String, _ raw: String) {
    guard UIApplication.shared.applicationState != .active else {
        NSLog("[hermes-watch] app open, no alert")
        return
    }
    guard let label = try? JSONDecoder().decode(Label.self, from: Data(raw.utf8)) else { return }
    var event = "", output = ""
    for line in stream.split(separator: "\n") where line.hasPrefix("data:") {
        guard let json = try? JSONSerialization.jsonObject(with: Data(line.dropFirst(5).utf8)) as? [String: Any],
              let name = json["event"] as? String, name.hasPrefix("run.") else { continue }
        event = name
        output = json["output"] as? String ?? ""
    }
    NSLog("[hermes-watch] run ended with %@", event)
    let body: String
    switch event {
    case "run.completed": body = output.replacingOccurrences(of: "\n", with: " ")
    case "run.failed", "run.interrupted": body = label.failed
    default: return
    }
    let content = UNMutableNotificationContent()
    content.title = "Hermes"
    content.subtitle = label.title
    content.body = body
    content.sound = .default
    content.threadIdentifier = "hermes"
    content.userInfo = ["hermes_session": label.session]
    let request = UNNotificationRequest(identifier: label.session, content: content, trigger: nil)
    UNUserNotificationCenter.current().add(request) { error in
        NSLog("[hermes-watch] alert %@", error.map { $0.localizedDescription } ?? "shown")
    }
}

// tao's `AppDelegate` lacks the background session callback; it exists only
// once launching has finished, before iOS delivers the session's events.
@_cdecl("flowflow_hermes_watch_boot")
public func hermesWatchBoot() {
    NotificationCenter.default.addObserver(
        forName: UIApplication.didFinishLaunchingNotification, object: nil, queue: .main
    ) { _ in
        guard let cls = NSClassFromString("AppDelegate") else { return }
        let handle: @convention(block) (AnyObject, UIApplication, NSString, @escaping () -> Void) -> Void = { _, _, id, done in
            NSLog("[hermes-watch] woken for %@", id)
            finishEvents = done
            _ = session
        }
        class_addMethod(
            cls,
            #selector(UIApplicationDelegate.application(_:handleEventsForBackgroundURLSession:completionHandler:)),
            imp_implementationWithBlock(handle),
            "v@:@@@?"
        )
        _ = session
    }
}

/// Follows one run until Hermes closes its stream. `label` is JSON:
/// {"session", "title", "failed"}.
@_cdecl("flowflow_hermes_watch_run")
public func hermesWatchRun(_ url: UnsafePointer<CChar>, _ key: UnsafePointer<CChar>, _ label: UnsafePointer<CChar>) {
    guard let target = URL(string: String(cString: url)) else { return }
    var request = URLRequest(url: target)
    request.setValue("Bearer \(String(cString: key))", forHTTPHeaderField: "Authorization")
    request.setValue("text/event-stream", forHTTPHeaderField: "Accept")
    let task = session.downloadTask(with: request)
    task.taskDescription = String(cString: label)
    task.resume()
    NSLog("[hermes-watch] following %@", target.path)
}
