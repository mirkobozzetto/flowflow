import Intents
import UserNotifications

// A Hermes task result reaches the phone as a plain alert titled with the
// task's name. It leaves here as a message from Hermes, the way Messages and
// Telegram show a person: his face and name, the task as subtitle.
final class NotificationService: UNNotificationServiceExtension {
    override func didReceive(
        _ request: UNNotificationRequest,
        withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void
    ) {
        guard let content = request.content.mutableCopy() as? UNMutableNotificationContent else {
            return contentHandler(request.content)
        }
        let task = content.title
        content.subtitle = task
        // The app opens this task when the alert is tapped.
        if !task.isEmpty {
            content.userInfo["job"] = task
        }
        let intent = fromHermes(content.body)
        let interaction = INInteraction(intent: intent, response: nil)
        interaction.direction = .incoming
        interaction.donate { error in
            if let error {
                NSLog("[hermes-notification] donation failed: \(error.localizedDescription)")
            }
            do {
                contentHandler(try content.updating(from: intent))
                NSLog("[hermes-notification] shown as a message from Hermes")
            } catch {
                NSLog("[hermes-notification] left as a plain alert: \(error.localizedDescription)")
                contentHandler(content)
            }
        }
    }
}

private func fromHermes(_ text: String) -> INSendMessageIntent {
    let face = Bundle.main.url(forResource: "hermes-agent", withExtension: "png")
        .flatMap { try? Data(contentsOf: $0) }
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
    intent.setImage(face, forParameterNamed: \.sender)
    return intent
}
