//! What a question to Hermes carries besides its text. The Hermes API reads
//! text and images only: a file or a note travels as text in a tagged block,
//! a skill as an instruction to load it. The same tags let a conversation read
//! back show the question and the attachment names, never their body.

use base64::Engine;

const OPEN: &str = "<attachment kind=\"";

#[derive(Clone, Debug, PartialEq)]
pub enum Attachment {
    /// A photo Hermes looks at, already reduced to a JPEG.
    Photo {
        name: String,
        jpeg: Vec<u8>,
    },
    /// A file's text, extracted on the device.
    File {
        name: String,
        text: String,
    },
    Note {
        title: String,
        text: String,
    },
    /// A skill installed on Hermes, for it to load.
    Skill {
        name: String,
    },
}

impl Attachment {
    pub fn label(&self) -> &str {
        match self {
            Attachment::Photo { name, .. }
            | Attachment::File { name, .. }
            | Attachment::Skill { name } => name,
            Attachment::Note { title, .. } => title,
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// The message text Hermes reads, and the photos as data URLs.
pub fn compose(
    question: &str,
    attachments: &[Attachment],
) -> (String, Vec<String>) {
    let mut text = question.trim().to_string();
    let mut images = Vec::new();
    for a in attachments {
        let (kind, body) = match a {
            Attachment::Photo { jpeg, .. } => {
                images.push(format!(
                    "data:image/jpeg;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(jpeg)
                ));
                ("photo", None)
            }
            Attachment::File { text, .. } => ("file", Some(text.trim().to_string())),
            Attachment::Note { text, .. } => ("note", Some(text.trim().to_string())),
            // The API server runs no slash command: the skill is named for
            // Hermes to load it itself.
            Attachment::Skill { name } => (
                "skill",
                Some(format!(
                    "Load the skill \"{name}\" with skill_view and follow it for this request."
                )),
            ),
        };
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        let name = escape(a.label());
        match body {
            Some(body) => text.push_str(&format!(
                "{OPEN}{kind}\" name=\"{name}\">\n{body}\n</attachment>"
            )),
            None => text.push_str(&format!("{OPEN}{kind}\" name=\"{name}\"/>")),
        }
    }
    (text, images)
}

/// A sent message read back.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    pub question: String,
    /// Photos, files and notes, by name.
    pub attachments: Vec<String>,
    /// Skills called on purpose.
    pub skills: Vec<String>,
}

pub fn split(message: &str) -> Asked {
    let start = if message.starts_with(OPEN) {
        Some(0)
    } else {
        message.find(&format!("\n\n{OPEN}"))
    };
    let Some(start) = start else {
        return Asked {
            question: message.to_string(),
            ..Asked::default()
        };
    };
    let mut asked = Asked {
        question: message[..start].trim().to_string(),
        ..Asked::default()
    };
    for rest in message[start..]
        .lines()
        .filter_map(|l| l.strip_prefix(OPEN))
    {
        let Some((kind, rest)) = rest.split_once('"') else {
            continue;
        };
        let Some(name) = rest
            .split_once("name=\"")
            .and_then(|(_, n)| n.split_once('"'))
            .map(|(n, _)| unescape(n))
        else {
            continue;
        };
        if kind == "skill" {
            asked.skills.push(name);
        } else {
            asked.attachments.push(name);
        }
    }
    asked
}

/// A conversation's title: the question, else what was attached.
pub fn title(
    question: &str,
    attachments: &[Attachment],
    chars: usize,
) -> String {
    let source = match question.trim() {
        "" => attachments
            .iter()
            .map(Attachment::label)
            .collect::<Vec<_>>()
            .join(", "),
        q => q.to_string(),
    };
    source.chars().take(chars).collect()
}
