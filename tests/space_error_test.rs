// Every SpaceError variant owns its own i18n key: a wildcard arm added later
// would fold two variants onto one key and fail this pairwise check.

use flowflow::application::space::{error_key, SpaceError};

#[test]
fn every_variant_has_a_distinct_error_key() {
    let all = [
        SpaceError::NoBackend,
        SpaceError::Offline,
        SpaceError::Refused,
        SpaceError::Gone,
        SpaceError::ReadOnly,
        SpaceError::Limit("members".into()),
        SpaceError::Other("boom".into()),
    ];
    let keys: Vec<&str> = all.iter().map(error_key).collect();
    for (i, a) in keys.iter().enumerate() {
        assert!(a.starts_with("space-error-"), "{a}");
        for (j, b) in keys.iter().enumerate() {
            assert!(i == j || a != b, "{a} shared by two variants");
        }
    }
}

#[test]
fn error_keys_exist_in_both_locales() {
    let fr = include_str!("../src/application/i18n/locales/fr.ftl");
    let en = include_str!("../src/application/i18n/locales/en.ftl");
    for k in [
        SpaceError::NoBackend,
        SpaceError::Offline,
        SpaceError::Refused,
        SpaceError::Gone,
        SpaceError::ReadOnly,
        SpaceError::Limit(String::new()),
        SpaceError::Other(String::new()),
    ]
    .iter()
    .map(error_key)
    {
        assert!(fr.contains(&format!("{k} = ")), "fr missing {k}");
        assert!(en.contains(&format!("{k} = ")), "en missing {k}");
    }
}
