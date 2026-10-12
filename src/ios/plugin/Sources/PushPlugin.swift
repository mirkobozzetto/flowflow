import ObjectiveC
import UIKit
import UserNotifications

public typealias PushReply = @convention(c) (Bool, UnsafePointer<CChar>) -> Void
public typealias PushTap = @convention(c) (UnsafePointer<CChar>) -> Void

private var pendingReply: PushReply?
private var onTap: PushTap?
private var delegateReady = false

private func answer(_ ok: Bool, _ text: String) {
    guard let reply = pendingReply else { return }
    pendingReply = nil
    text.withCString { reply(ok, $0) }
}

// tao declares the app delegate class `AppDelegate` without the remote
// notification callbacks: add them at runtime, the approach of
// tauri-plugin-mobile-push, which also runs on tao.
private func addDelegateCallbacks() {
    guard !delegateReady, let cls = NSClassFromString("AppDelegate") else { return }
    delegateReady = true
    let registered: @convention(block) (AnyObject, UIApplication, Data) -> Void = { _, _, token in
        answer(true, token.map { String(format: "%02x", $0) }.joined())
    }
    let failed: @convention(block) (AnyObject, UIApplication, NSError) -> Void = { _, _, error in
        answer(false, error.localizedDescription)
    }
    class_addMethod(
        cls,
        #selector(UIApplicationDelegate.application(_:didRegisterForRemoteNotificationsWithDeviceToken:)),
        imp_implementationWithBlock(registered),
        "v@:@@"
    )
    class_addMethod(
        cls,
        #selector(UIApplicationDelegate.application(_:didFailToRegisterForRemoteNotificationsWithError:)),
        imp_implementationWithBlock(failed),
        "v@:@@"
    )
}

// iOS drops a notification that lands while the app is open unless the app
// asks to show it; a Hermes result is worth seeing either way.
private final class Presenter: NSObject, UNUserNotificationCenterDelegate {
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .list, .sound])
    }

    // The notification service extension names the task in `job`; without
    // it the tap opens the tasks list.
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        var link = "flowflow://hermes/jobs"
        if let job = response.notification.request.content.userInfo["job"] as? String,
           let name = job.addingPercentEncoding(withAllowedCharacters: .alphanumerics) {
            link += "?name=\(name)"
        }
        link.withCString { onTap?($0) }
        completionHandler()
    }
}

// The center keeps its delegate weakly.
private let presenter = Presenter()

/// Replies once: the hex token, or `denied` when the person refuses.
@_cdecl("flowflow_push_register")
public func pushRegister(_ reply: PushReply) {
    DispatchQueue.main.async {
        pendingReply = reply
        addDelegateCallbacks()
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) { granted, _ in
            DispatchQueue.main.async {
                if granted {
                    UIApplication.shared.registerForRemoteNotifications()
                } else {
                    answer(false, "denied")
                }
            }
        }
    }
}

/// Called at launch: iOS hands the tap that opened the app only to a delegate
/// set before launching ends.
@_cdecl("flowflow_push_observe_taps")
public func pushObserveTaps(_ tap: PushTap) {
    onTap = tap
    UNUserNotificationCenter.current().delegate = presenter
}
