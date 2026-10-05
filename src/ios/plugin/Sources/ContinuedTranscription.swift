import BackgroundTasks
import Foundation

// Local jobs run one after another, so one task covers them all.
private let taskIdentifier = "com.mirkobozzetto.flowflow.transcription"

private let lock = NSLock()
// From submit until Rust ends the task or iOS expires it.
private var submitted = false
private var running: AnyObject?
private var latest = (title: "", subtitle: "", done: Int64(0), total: Int64(0))

@available(iOS 26.0, *)
private func show(_ task: BGContinuedProcessingTask) {
    task.progress.totalUnitCount = max(latest.total, 1)
    task.progress.completedUnitCount = latest.done
    task.updateTitle(latest.title, subtitle: latest.subtitle)
}

/// `onRun(true)` once iOS runs the task, `onRun(false)` when it expires or the
/// user stops it from the Live Activity. Registering twice kills the app.
@_cdecl("flowflow_register_continued_transcription")
public func registerContinuedTranscription(_ onRun: @convention(c) (Bool) -> Void) {
    guard #available(iOS 26.0, *) else { return }
    print("[bg-transcription] background gpu supported: \(BGTaskScheduler.supportedResources.contains(.gpu))")
    BGTaskScheduler.shared.register(forTaskWithIdentifier: taskIdentifier, using: nil) { task in
        guard let task = task as? BGContinuedProcessingTask else {
            task.setTaskCompleted(success: false)
            return
        }
        lock.lock()
        guard submitted else {
            lock.unlock()
            task.setTaskCompleted(success: true)
            return
        }
        running = task
        show(task)
        lock.unlock()
        task.expirationHandler = {
            lock.lock()
            submitted = false
            running = nil
            lock.unlock()
            print("[bg-transcription] expired")
            onRun(false)
            task.setTaskCompleted(success: false)
        }
        print("[bg-transcription] running")
        onRun(true)
    }
}

/// Must come from a user action with the app in the foreground.
@_cdecl("flowflow_begin_continued_transcription")
public func beginContinuedTranscription(
    _ title: UnsafePointer<CChar>, _ subtitle: UnsafePointer<CChar>
) -> Bool {
    guard #available(iOS 26.0, *) else { return false }
    let title = String(cString: title)
    let subtitle = String(cString: subtitle)
    lock.lock()
    latest.title = title
    latest.subtitle = subtitle
    if submitted {
        if let task = running as? BGContinuedProcessingTask { show(task) }
        lock.unlock()
        return true
    }
    submitted = true
    lock.unlock()
    let request = BGContinuedProcessingTaskRequest(
        identifier: taskIdentifier, title: title, subtitle: subtitle
    )
    request.strategy = .fail
    do {
        try BGTaskScheduler.shared.submit(request)
        return true
    } catch {
        print("[bg-transcription] submit failed: \(error)")
        lock.lock()
        submitted = false
        lock.unlock()
        return false
    }
}

@_cdecl("flowflow_continued_transcription_progress")
public func continuedTranscriptionProgress(
    _ title: UnsafePointer<CChar>, _ subtitle: UnsafePointer<CChar>,
    _ done: Int64, _ total: Int64
) {
    guard #available(iOS 26.0, *) else { return }
    lock.lock()
    latest = (String(cString: title), String(cString: subtitle), done, total)
    if let task = running as? BGContinuedProcessingTask { show(task) }
    lock.unlock()
}

@_cdecl("flowflow_end_continued_transcription")
public func endContinuedTranscription(_ success: Bool) {
    guard #available(iOS 26.0, *) else { return }
    lock.lock()
    let task = running as? BGContinuedProcessingTask
    submitted = false
    running = nil
    lock.unlock()
    task?.setTaskCompleted(success: success)
}
