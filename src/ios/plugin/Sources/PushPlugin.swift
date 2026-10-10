import ObjectiveC
import UIKit
import UserNotifications

public typealias PushReply = @convention(c) (Bool, UnsafePointer<CChar>) -> Void

private var pendingReply: PushReply?
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
