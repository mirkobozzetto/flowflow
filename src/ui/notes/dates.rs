use crate::application::i18n::{month_abbr, t, t_args};
use chrono::{Datelike, NaiveDateTime, Utc};

pub fn format_relative_date(iso: &str, lang: &str) -> String {
    let parsed = NaiveDateTime::parse_from_str(
        &iso.replace('T', " ").replace('Z', ""),
        "%Y-%m-%d %H:%M:%S%.f",
    );
    let dt = match parsed {
        Ok(d) => d,
        Err(_) => return iso.to_string(),
    };
    let now = Utc::now().naive_utc();
    let diff = now.signed_duration_since(dt);
    let secs = diff.num_seconds();

    if secs < 60 {
        return t(lang, "date-just-now");
    }
    if secs < 3600 {
        let mins = (secs / 60).to_string();
        return t_args(lang, "date-mins-ago", &[("mins", &mins)]);
    }
    if secs < 86400 {
        let hours = (secs / 3600).to_string();
        return t_args(lang, "date-hours-ago", &[("hours", &hours)]);
    }

    let today = now.date();
    let note_date = dt.date();
    if today.pred_opt() == Some(note_date) {
        let time = if lang == "fr" {
            dt.format("%H:%M").to_string()
        } else {
            dt.format("%-I:%M %p").to_string()
        };
        return t_args(lang, "date-yesterday", &[("time", &time)]);
    }

    let m = month_abbr(lang, note_date.month());
    let d = note_date.day();

    if note_date.year() == today.year() {
        format!("{d} {m}")
    } else {
        format!("{d} {m} {}", note_date.year())
    }
}

pub fn format_absolute_short(iso: &str, lang: &str) -> String {
    let parsed = NaiveDateTime::parse_from_str(
        &iso.replace('T', " ").replace('Z', ""),
        "%Y-%m-%d %H:%M:%S%.f",
    );
    let dt = match parsed {
        Ok(d) => d,
        Err(_) => return iso.to_string(),
    };
    let d = dt.date();
    let now = Utc::now().naive_utc().date();
    let m = month_abbr(lang, d.month());
    if d.year() == now.year() {
        let time = if lang == "fr" {
            dt.format("%H:%M").to_string()
        } else {
            dt.format("%-I:%M %p").to_string()
        };
        format!("{} {}, {}", d.day(), m, time)
    } else {
        format!("{} {} {}", d.day(), m, d.year())
    }
}

pub fn feed_group_label(iso: &str, lang: &str) -> String {
    let Ok(dt) = NaiveDateTime::parse_from_str(
        &iso.replace('T', " ").replace('Z', ""),
        "%Y-%m-%d %H:%M:%S%.f",
    ) else {
        return iso.get(..10).unwrap_or(iso).to_string();
    };
    let date = dt.date();
    let today = Utc::now().naive_utc().date();
    match today.signed_duration_since(date).num_days() {
        0 => t(lang, "reminder-today"),
        1 => t(lang, "note-group-yesterday"),
        2..=6 => t(lang, "note-group-this-week"),
        _ if date.year() == today.year() => month_abbr(lang, date.month()).to_string(),
        _ => format!("{} {}", month_abbr(lang, date.month()), date.year()),
    }
}
