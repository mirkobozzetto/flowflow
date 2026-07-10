import UIKit
import UniformTypeIdentifiers

// Headless share sheet: grab whatever the host app shares, drop it into the
// app-group inbox, close immediately. ALL note logic stays in Rust - the app
// drains the inbox on foreground (application/share_inbox.rs).
class ShareViewController: UIViewController {
    private let appGroup = "group.com.mirkobozzetto.flowflow"

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .clear
        Task {
            await ingest()
            extensionContext?.completeRequest(returningItems: nil)
        }
    }

    private func inboxDir() -> URL? {
        guard
            let base = FileManager.default.containerURL(
                forSecurityApplicationGroupIdentifier: appGroup)
        else { return nil }
        let dir = base.appendingPathComponent(
            "shared-inbox", isDirectory: true)
        try? FileManager.default.createDirectory(
            at: dir, withIntermediateDirectories: true)
        return dir
    }

    private func writeEntry(_ entry: [String: String], to dir: URL) {
        guard let data = try? JSONSerialization.data(withJSONObject: entry)
        else { return }
        let name = UUID().uuidString + ".json"
        try? data.write(
            to: dir.appendingPathComponent(name), options: .atomic)
    }

    private func ingest() async {
        guard let dir = inboxDir(),
            let items = extensionContext?.inputItems as? [NSExtensionItem]
        else { return }
        for item in items {
            for provider in item.attachments ?? [] {
                await ingestOne(provider, into: dir)
            }
        }
    }

    private func ingestOne(
        _ provider: NSItemProvider, into dir: URL
    ) async {
        // Order matters: a shared document is BOTH a file URL and a URL;
        // check the file shape first so it rides the attachment pipeline.
        if provider.hasItemConformingToTypeIdentifier(
            UTType.fileURL.identifier)
        {
            guard
                let raw = try? await provider.loadItem(
                    forTypeIdentifier: UTType.fileURL.identifier),
                let url = raw as? URL
            else { return }
            let dest = dir.appendingPathComponent(
                UUID().uuidString + "-" + url.lastPathComponent)
            let scoped = url.startAccessingSecurityScopedResource()
            defer {
                if scoped { url.stopAccessingSecurityScopedResource() }
            }
            if (try? FileManager.default.copyItem(at: url, to: dest)) != nil {
                writeEntry(
                    [
                        "kind": "file",
                        "file": dest.lastPathComponent,
                        "name": url.lastPathComponent,
                    ], to: dir)
            }
        } else if provider.hasItemConformingToTypeIdentifier(
            UTType.url.identifier)
        {
            guard
                let raw = try? await provider.loadItem(
                    forTypeIdentifier: UTType.url.identifier),
                let url = raw as? URL
            else { return }
            writeEntry(["kind": "url", "url": url.absoluteString], to: dir)
        } else if provider.hasItemConformingToTypeIdentifier(
            UTType.plainText.identifier)
        {
            guard
                let raw = try? await provider.loadItem(
                    forTypeIdentifier: UTType.plainText.identifier)
            else { return }
            let text =
                (raw as? String) ?? (raw as? NSString).map(String.init) ?? ""
            if !text.isEmpty {
                writeEntry(["kind": "text", "text": text], to: dir)
            }
        }
    }
}
