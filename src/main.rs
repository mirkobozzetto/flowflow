const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html>
    <head>
        <title>FlowFlow</title>
        <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
        <!-- CUSTOM HEAD -->
    </head>
    <body>
        <div id="main"></div>
        <!-- MODULE LOADER -->
    </body>
</html>"#;

#[cfg(feature = "desktop")]
use dioxus::desktop::Config as DesktopConfig;
#[cfg(all(feature = "mobile", not(feature = "desktop")))]
use dioxus::mobile::Config as DesktopConfig;

#[cfg(feature = "desktop")]
use dioxus::desktop::{tao, WindowBuilder};
#[cfg(all(feature = "mobile", not(feature = "desktop")))]
use dioxus::mobile::{tao, WindowBuilder};

#[cfg(feature = "desktop")]
fn build_window() -> WindowBuilder {
    WindowBuilder::new()
        .with_title("FlowFlow")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 850.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(760.0, 520.0))
}

#[cfg(all(feature = "mobile", not(feature = "desktop")))]
fn build_window() -> WindowBuilder {
    WindowBuilder::new().with_title("FlowFlow")
}

fn main() {
    flowflow::application::backup::apply_pending_restore_or_abort();
    dotenvy::dotenv().ok();
    #[cfg(target_os = "ios")]
    flowflow::infrastructure::platform::ios::configure_audio_session();
    #[cfg(target_os = "ios")]
    flowflow::infrastructure::platform::ios::hide_keyboard_accessory();
    #[cfg(target_os = "ios")]
    flowflow::infrastructure::platform::ios::observe_launch_url();
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::prelude::client! {
            DesktopConfig::new()
                .with_window(build_window())
                .with_custom_index(INDEX_HTML.to_string())
                .with_custom_event_handler(|event, _target| {
                    if let tao::event::Event::Opened { urls } = event {
                        for url in urls {
                            let s = url.as_str();
                            if s.starts_with("flowflow://") {
                                flowflow::infrastructure::sync::deeplink::push(s.to_string());
                                break;
                            }
                        }
                    }
                })
        })
        .launch(flowflow::ui::App);
}
