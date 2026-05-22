// FlowFlow provisioning renewal stub.
//
// This app is never installed, never launched, never useful at runtime.
// Its only purpose: give xcodebuild a valid Swift target so it triggers
// Apple Developer Portal signing, which regenerates the expired
// provisioning profile for `com.mirkobozzetto.flowflow` (and the widget).
//
// Workflow: scripts/renew-profiles.sh copies this template to /tmp,
// substitutes __BUNDLE_ID__ and __TEAM_ID__ in project.pbxproj, runs
// `xcodebuild build -allowProvisioningUpdates`, then trashes /tmp.
// The renewed .mobileprovision lands in
// ~/Library/Developer/Xcode/UserData/Provisioning Profiles/ and is
// picked up by `dx build` on the next `make all`.
//
// Free Personal Team profiles expire every 7 days. `make all` checks
// expiration first and runs `make renew` automatically when < 24h left.

import SwiftUI

@main
struct RenewApp: App {
    var body: some Scene {
        WindowGroup {
            Text("FlowFlow provisioning renewal stub - never launched")
        }
    }
}
