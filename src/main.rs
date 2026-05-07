use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div {
            style: "display: flex; justify-content: center; align-items: center; height: 100vh; font-family: -apple-system, BlinkMacSystemFont, sans-serif;",
            h1 { style: "font-size: 2rem;", "FlowFlow" }
        }
    }
}
