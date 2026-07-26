use super::client::SonioxClient;
use super::models;
use super::whisper::WhisperLocal;
use crate::infrastructure::persistence::settings_repo::{
    STT_PROVIDER_KEY, WHISPER_MODEL_KEY,
};
use crate::infrastructure::persistence::Database;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SttProvider {
    #[default]
    Soniox,
    WhisperLocal,
}

impl SttProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            SttProvider::Soniox => "soniox",
            SttProvider::WhisperLocal => "whisper_local",
        }
    }
}

impl std::fmt::Display for SttProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SttProvider {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "soniox" => Ok(SttProvider::Soniox),
            "whisper_local" | "whisper-local" | "whisper" => {
                Ok(SttProvider::WhisperLocal)
            }
            _ => Err(()),
        }
    }
}

pub enum TranscriptionClient {
    Soniox(SonioxClient),
    Whisper(WhisperLocal),
}

impl TranscriptionClient {
    pub fn provider_from_db(db: &Database) -> SttProvider {
        db.get_setting(STT_PROVIDER_KEY)
            .and_then(|v| SttProvider::from_str(&v).ok())
            .unwrap_or_default()
    }

    pub fn from_db(db: &Database) -> Result<Self, String> {
        match Self::provider_from_db(db) {
            SttProvider::Soniox => Ok(Self::Soniox(SonioxClient::from_db(db)?)),
            SttProvider::WhisperLocal => {
                Ok(Self::Whisper(Self::whisper_from_db(db)?))
            }
        }
    }

    pub fn whisper_from_db(db: &Database) -> Result<WhisperLocal, String> {
        let lang = crate::application::i18n::ui_lang(db);
        if db.get_setting("ai_consent") != Some("true".to_string()) {
            return Err(crate::application::i18n::t(&lang, "error-ai-consent"));
        }
        let model_id = db
            .get_setting(WHISPER_MODEL_KEY)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                crate::application::i18n::t(&lang, "stt-error-no-model")
            })?;
        let dir = models::models_dir();
        let path = models::model_path(&dir, &model_id).ok_or_else(|| {
            crate::application::i18n::t_args(
                &lang,
                "stt-error-model-unknown",
                &[("id", &model_id)],
            )
        })?;
        if !path.is_file() {
            return Err(crate::application::i18n::t_args(
                &lang,
                "stt-error-model-missing",
                &[("id", &model_id)],
            ));
        }
        Ok(WhisperLocal::new(path).with_dictionary(
            crate::application::transcription_dictionary::load(db),
        ))
    }

    pub fn provider(&self) -> SttProvider {
        match self {
            Self::Soniox(_) => SttProvider::Soniox,
            Self::Whisper(_) => SttProvider::WhisperLocal,
        }
    }

    pub async fn transcribe(
        &self,
        path: &Path,
        language: Option<&str>,
    ) -> Result<crate::domain::Transcript, String> {
        match self {
            Self::Soniox(c) => c.transcribe(path, language).await,
            Self::Whisper(w) => w.transcribe(path, language).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_parses_known_values_and_defaults_to_soniox() {
        assert_eq!(
            SttProvider::from_str("soniox").unwrap(),
            SttProvider::Soniox
        );
        assert_eq!(
            SttProvider::from_str("whisper_local").unwrap(),
            SttProvider::WhisperLocal
        );
        assert_eq!(
            SttProvider::from_str(" Whisper_Local ").unwrap(),
            SttProvider::WhisperLocal
        );
        assert!(SttProvider::from_str("deepgram").is_err());
        assert_eq!(SttProvider::default(), SttProvider::Soniox);
    }

    #[test]
    fn provider_roundtrips_as_str() {
        for p in [SttProvider::Soniox, SttProvider::WhisperLocal] {
            assert_eq!(SttProvider::from_str(p.as_str()).unwrap(), p);
        }
    }
}
