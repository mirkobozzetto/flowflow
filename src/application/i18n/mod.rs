use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use std::sync::OnceLock;
use unic_langid::LanguageIdentifier;

const EN_FTL: &str = include_str!("locales/en.ftl");
const FR_FTL: &str = include_str!("locales/fr.ftl");

struct Bundles {
    en: FluentBundle<FluentResource>,
    fr: FluentBundle<FluentResource>,
}

static BUNDLES: OnceLock<Bundles> = OnceLock::new();

fn build_bundle(lang: &str, ftl: &str) -> FluentBundle<FluentResource> {
    let langid: LanguageIdentifier =
        lang.parse().unwrap_or_else(|_| "en".parse().unwrap());
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    bundle.set_use_isolating(false);
    if let Ok(res) = FluentResource::try_new(ftl.to_string()) {
        let _ = bundle.add_resource(res);
    }
    bundle
}

fn bundles() -> &'static Bundles {
    BUNDLES.get_or_init(|| Bundles {
        en: build_bundle("en", EN_FTL),
        fr: build_bundle("fr", FR_FTL),
    })
}

fn lookup(
    bundle: &FluentBundle<FluentResource>,
    key: &str,
    args: Option<&FluentArgs>,
) -> Option<String> {
    let msg = bundle.get_message(key)?;
    let pattern = msg.value()?;
    let mut errors = vec![];
    let out = bundle.format_pattern(pattern, args, &mut errors);
    Some(out.into_owned())
}

pub fn month_abbr(lang: &str, month: u32) -> &'static str {
    const FR: [&str; 13] = [
        "", "jan.", "fév.", "mars", "avr.", "mai", "juin", "juil.", "août",
        "sept.", "oct.", "nov.", "déc.",
    ];
    const EN: [&str; 13] = [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep",
        "Oct", "Nov", "Dec",
    ];
    let table = if lang == "fr" { &FR } else { &EN };
    table.get(month as usize).copied().unwrap_or("")
}

pub fn weekday_name(lang: &str, wd: chrono::Weekday) -> &'static str {
    const FR: [&str; 7] = [
        "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
    ];
    const EN: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    let table = if lang == "fr" { &FR } else { &EN };
    table[wd.num_days_from_monday() as usize]
}

/// How the date reads back to someone who typed "demain". Within the week the day is named
/// ("Demain", "Jeudi"); past that, the calendar date. Someone who said "tomorrow" and reads
/// "19 août" has to do the conversion themselves just to check the agent understood.
pub fn reminder_due_label(
    lang: &str,
    r: &crate::domain::NoteReminder,
) -> String {
    use chrono::{Datelike, Local, NaiveDate};
    let (m, d) = match (r.due_month, r.due_day) {
        (Some(m), Some(d)) => (m, d),
        _ => return String::new(),
    };
    let day_part = r
        .due_year
        .and_then(|y| NaiveDate::from_ymd_opt(y, m as u32, d as u32))
        .and_then(|date| {
            let today = Local::now().date_naive();
            match (date - today).num_days() {
                0 => Some(t(lang, "reminder-today")),
                1 => Some(t(lang, "reminder-tomorrow")),
                2..=6 => {
                    let name = weekday_name(lang, date.weekday());
                    let mut c = name.chars();
                    Some(match c.next() {
                        Some(f) => {
                            f.to_uppercase().collect::<String>() + c.as_str()
                        }
                        None => name.to_string(),
                    })
                }
                _ => None,
            }
        })
        .unwrap_or_else(|| format!("{d} {}", month_abbr(lang, m as u32)));

    if r.is_all_day {
        day_part
    } else {
        let h = r.due_hour.unwrap_or(9);
        let min = r.due_minute.unwrap_or(0);
        format!("{day_part} {h:02}:{min:02}")
    }
}

pub fn ui_lang(db: &crate::infrastructure::persistence::Database) -> String {
    db.get_setting(
        crate::infrastructure::persistence::settings_repo::LANGUAGE_KEY,
    )
    .filter(|v| !v.is_empty())
    .unwrap_or_else(crate::infrastructure::platform::detect_system_language)
}

pub fn t(lang: &str, key: &str) -> String {
    t_args(lang, key, &[])
}

pub fn t_args(lang: &str, key: &str, args: &[(&str, &str)]) -> String {
    let b = bundles();
    let primary = match lang {
        "fr" => &b.fr,
        _ => &b.en,
    };
    let mut fluent_args = FluentArgs::new();
    for (k, v) in args {
        fluent_args.set(k.to_string(), FluentValue::from(v.to_string()));
    }
    let args_opt = if args.is_empty() {
        None
    } else {
        Some(&fluent_args)
    };
    if let Some(v) = lookup(primary, key, args_opt) {
        return v;
    }
    if !std::ptr::eq(primary, &b.en) {
        if let Some(v) = lookup(&b.en, key, args_opt) {
            return v;
        }
    }
    key.to_string()
}
