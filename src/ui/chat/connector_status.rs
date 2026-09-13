use crate::infrastructure::backend::Connector;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConnectionStatus {
    Loading,
    Disconnected,
    Connected,
    Unavailable,
}

impl ConnectionStatus {
    pub(super) fn from_query<E>(
        query: Option<&Result<Vec<Connector>, E>>,
    ) -> Self {
        match query {
            None => Self::Loading,
            Some(Err(_)) => Self::Unavailable,
            Some(Ok(connectors)) => {
                if connectors
                    .iter()
                    .any(|c| c.provider == "google" && c.connected)
                {
                    Self::Connected
                } else {
                    Self::Disconnected
                }
            }
        }
    }

    pub(super) fn label_key(self) -> &'static str {
        match self {
            Self::Loading => "chat-tools-connection-loading",
            Self::Disconnected => "connections-not-connected",
            Self::Connected => "chat-tools-google-connected",
            Self::Unavailable => "chat-tools-connection-unavailable",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connector(provider: &str, connected: bool) -> Connector {
        Connector {
            provider: provider.into(),
            name: provider.into(),
            connected,
            scopes: String::new(),
        }
    }

    #[test]
    fn pending_and_failed_checks_never_claim_a_connection() {
        assert_eq!(
            ConnectionStatus::from_query::<()>(None),
            ConnectionStatus::Loading
        );
        assert_eq!(
            ConnectionStatus::from_query(Some(&Err::<Vec<Connector>, _>(()))),
            ConnectionStatus::Unavailable
        );
    }

    #[test]
    fn absent_disconnected_or_other_provider_is_not_google_connected() {
        for connectors in [
            vec![],
            vec![connector("google", false)],
            vec![connector("github", true)],
        ] {
            assert_eq!(
                ConnectionStatus::from_query(Some(&Ok::<_, ()>(connectors))),
                ConnectionStatus::Disconnected
            );
        }
    }

    #[test]
    fn only_confirmed_google_connection_is_connected() {
        assert_eq!(
            ConnectionStatus::from_query(Some(&Ok::<_, ()>(vec![
                connector("github", true),
                connector("google", true)
            ]))),
            ConnectionStatus::Connected
        );
    }

    #[test]
    fn all_status_labels_exist_in_both_languages() {
        for status in [
            ConnectionStatus::Loading,
            ConnectionStatus::Disconnected,
            ConnectionStatus::Connected,
            ConnectionStatus::Unavailable,
        ] {
            for lang in ["en", "fr"] {
                let key = status.label_key();
                assert_ne!(crate::application::i18n::t(lang, key), key);
            }
        }
    }
}
