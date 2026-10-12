// The web view's script behind the native glass buttons, as the app embeds it.
const SCRIPT: &str = include_str!("../src/ui/app/glass_burger.js");

// A view switched while the app was in the background (a tapped notification)
// was measured hidden; nothing else re-judges it when the page shows again.
#[test]
fn a_page_shown_again_re_judges_the_native_buttons() {
    let at = SCRIPT
        .find("\"visibilitychange\"")
        .expect("the script listens to visibilitychange");
    let listener = &SCRIPT[at..];
    let end = listener.find("});").expect("the listener closes");
    assert!(
        listener[..end].contains("schedule()"),
        "the listener schedules a new measure"
    );
}
