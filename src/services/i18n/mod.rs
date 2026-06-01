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

pub fn reminder_due_label(
    lang: &str,
    r: &crate::models::NoteReminder,
) -> String {
    let (m, d) = match (r.due_month, r.due_day) {
        (Some(m), Some(d)) => (m, d),
        _ => return String::new(),
    };
    let mon = month_abbr(lang, m as u32);
    if r.is_all_day {
        format!("{d} {mon}")
    } else {
        let h = r.due_hour.unwrap_or(9);
        let min = r.due_minute.unwrap_or(0);
        format!("{d} {mon}, {h:02}:{min:02}")
    }
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
