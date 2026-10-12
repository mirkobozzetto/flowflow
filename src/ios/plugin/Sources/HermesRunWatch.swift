import Intents
import ObjectiveC
import UIKit
import UserNotifications

// A Hermes run followed by iOS itself while the app sleeps: a background
// download of the run's event stream, which Hermes closes when the run ends.
// iOS keeps it open out of process and wakes the app at the end, which then
// says so with a notification.

private let identifier = (Bundle.main.bundleIdentifier ?? "flowflow") + ".hermes-runs"
private var finishEvents: (() -> Void)?

/// The Hermes conversation on screen: its own alert is not shown there.
var hermesViewingSession = ""

private let stamp: DateFormatter = {
    let f = DateFormatter()
    f.dateFormat = "HH:mm:ss.SSS"
    f.timeZone = TimeZone(identifier: "UTC")
    return f
}()

// Also into the app's log file (device_log): the console drops off once
// the phone locks, which is when this runs.
private func log(_ text: String) {
    NSLog("[hermes-watch] %@", text)
    let file = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("flowflow.log")
    let line = Data("\(stamp.string(from: Date())) [hermes-watch] \(text)\n".utf8)
    if let handle = try? FileHandle(forWritingTo: file) {
        handle.seekToEndOfFile()
        handle.write(line)
        try? handle.close()
    } else {
        try? line.write(to: file)
    }
}

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
        log("run stream closed: HTTP \(status), \(stream.utf8.count) bytes")
        guard status == 200 else { return }
        DispatchQueue.main.async { announce(stream, label) }
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        if let error {
            log("run stream lost: \(error.localizedDescription)")
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
// stopped needs no alert; one whose conversation is on screen shows none
// (PushPlugin's presenter).
private func announce(_ stream: String, _ raw: String) {
    guard let label = try? JSONDecoder().decode(Label.self, from: Data(raw.utf8)) else { return }
    var event = "", output = ""
    for line in stream.split(separator: "\n") where line.hasPrefix("data:") {
        guard let json = try? JSONSerialization.jsonObject(with: Data(line.dropFirst(5).utf8)) as? [String: Any],
              let name = json["event"] as? String, name.hasPrefix("run.") else { continue }
        event = name
        output = json["output"] as? String ?? ""
    }
    log("run ended with \(event)")
    let body: String
    switch event {
    case "run.completed": body = plain(output)
    case "run.failed", "run.interrupted": body = label.failed
    default: return
    }
    let content = UNMutableNotificationContent()
    content.title = label.title
    content.subtitle = label.title
    content.body = body
    content.sound = .default
    content.userInfo = ["hermes_session": label.session]
    // Shown as a message from Hermes, like his task results
    // (NotificationService.swift): his face and name, the conversation below.
    let intent = fromHermes(body)
    let interaction = INInteraction(intent: intent, response: nil)
    interaction.direction = .incoming
    interaction.donate { _ in
        let shown = (try? content.updating(from: intent)) ?? content
        let request = UNNotificationRequest(identifier: label.session, content: shown, trigger: nil)
        UNUserNotificationCenter.current().add(request) { error in
            log("alert \(error.map { $0.localizedDescription } ?? "posted")")
        }
    }
}

// One line, without the Markdown marks the chat renders.
private func plain(_ text: String) -> String {
    ["**", "__", "`", "#"].reduce(text) { $0.replacingOccurrences(of: $1, with: "") }
        .split(whereSeparator: \.isNewline)
        .map { $0.trimmingCharacters(in: .whitespaces) }
        .filter { !$0.isEmpty }
        .joined(separator: " ")
}

private func fromHermes(_ text: String) -> INSendMessageIntent {
    let face = FileManager.default
        .containerURL(forSecurityApplicationGroupIdentifier: "group.com.mirkobozzetto.flowflow")
        .flatMap { try? Data(contentsOf: $0.appendingPathComponent("hermes-face.png")) }
        .map { INImage(imageData: $0) }
    let hermes = INPerson(
        personHandle: INPersonHandle(value: "hermes", type: .unknown),
        nameComponents: nil,
        displayName: "Hermes",
        image: face,
        contactIdentifier: nil,
        customIdentifier: "hermes"
    )
    let intent = INSendMessageIntent(
        recipients: nil,
        outgoingMessageType: .outgoingMessageText,
        content: text,
        speakableGroupName: nil,
        conversationIdentifier: "hermes",
        serviceName: nil,
        sender: hermes,
        attachments: nil
    )
    intent.setImage(face, forParameterNamed: \INSendMessageIntent.sender)
    return intent
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
            log("woken for \(id)")
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

/// The Hermes conversation now on screen; empty when none is.
@_cdecl("flowflow_hermes_watch_viewing")
public func hermesWatchViewing(_ session: UnsafePointer<CChar>) {
    let id = String(cString: session)
    DispatchQueue.main.async { hermesViewingSession = id }
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
    log("following \(target.path)")
}
