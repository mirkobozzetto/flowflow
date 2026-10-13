//! Pieces shared by the AI screens: brand marks, the provider card and its
//! status lines.
use crate::application::i18n::t;
use crate::infrastructure::Provider;
use dioxus::prelude::*;

// Simple Icons `claude`, #D97757.
const CLAUDE_PATH: &str = "m4.7144 15.9555 4.7174-2.6471.079-.2307-.079-.1275h-.2307l-.7893-.0486-2.6956-.0729-2.3375-.0971-2.2646-.1214-.5707-.1215-.5343-.7042.0546-.3522.4797-.3218.686.0608 1.5179.1032 2.2767.1578 1.6514.0972 2.4468.255h.3886l.0546-.1579-.1336-.0971-.1032-.0972L6.973 9.8356l-2.55-1.6879-1.3356-.9714-.7225-.4918-.3643-.4614-.1578-1.0078.6557-.7225.8803.0607.2246.0607.8925.686 1.9064 1.4754 2.4893 1.8336.3643.3035.1457-.1032.0182-.0728-.164-.2733-1.3539-2.4467-1.445-2.4893-.6435-1.032-.17-.6194c-.0607-.255-.1032-.4674-.1032-.7285L6.287.1335 6.6997 0l.9957.1336.419.3642.6192 1.4147 1.0018 2.2282 1.5543 3.0296.4553.8985.2429.8318.091.255h.1579v-.1457l.1275-1.706.2368-2.0947.2307-2.6957.0789-.7589.3764-.9107.7468-.4918.5828.2793.4797.686-.0668.4433-.2853 1.8517-.5586 2.9021-.3643 1.9429h.2125l.2429-.2429.9835-1.3053 1.6514-2.0643.7286-.8196.85-.9046.5464-.4311h1.0321l.759 1.1293-.34 1.1657-1.0625 1.3478-.8804 1.1414-1.2628 1.7-.7893 1.36.0729.1093.1882-.0183 2.8535-.607 1.5421-.2794 1.8396-.3157.8318.3886.091.3946-.3278.8075-1.967.4857-2.3072.4614-3.4364.8136-.0425.0304.0486.0607 1.5482.1457.6618.0364h1.621l3.0175.2247.7892.522.4736.6376-.079.4857-1.2142.6193-1.6393-.3886-3.825-.9107-1.3113-.3279h-.1822v.1093l1.0929 1.0686 2.0035 1.8092 2.5075 2.3314.1275.5768-.3218.4554-.34-.0486-2.2039-1.6575-.85-.7468-1.9246-1.621h-.1275v.17l.4432.6496 2.3436 3.5214.1214 1.0807-.17.3521-.6071.2125-.6679-.1214-1.3721-1.9246L14.38 17.959l-1.1414-1.9428-.1397.079-.674 7.2552-.3156.3703-.7286.2793-.6071-.4614-.3218-.7468.3218-1.4753.3886-1.9246.3157-1.53.2853-1.9004.17-.6314-.0121-.0425-.1397.0182-1.4328 1.9672-2.1796 2.9446-1.7243 1.8456-.4128.164-.7164-.3704.0667-.6618.4008-.5889 2.386-3.0357 1.4389-1.882.929-1.0868-.0062-.1579h-.0546l-6.3385 4.1164-1.1293.1457-.4857-.4554.0608-.7467.2307-.2429 1.9064-1.3114Z";
const OPENAI_PATH: &str = "M249.176 323.434V298.276C249.176 296.158 249.971 294.569 251.825 293.509L302.406 264.381C309.29 260.409 317.5 258.555 325.973 258.555C357.75 258.555 377.877 283.185 377.877 309.399C377.877 311.253 377.877 313.371 377.611 315.49L325.178 284.771C322.001 282.919 318.822 282.919 315.645 284.771L249.176 323.434ZM367.283 421.415V361.301C367.283 357.592 365.694 354.945 362.516 353.092L296.048 314.43L317.763 301.982C319.617 300.925 321.206 300.925 323.058 301.982L373.639 331.112C388.205 339.586 398.003 357.592 398.003 375.069C398.003 395.195 386.087 413.733 367.283 421.412V421.415ZM233.553 368.452L211.838 355.742C209.986 354.684 209.19 353.095 209.19 350.975V292.718C209.19 264.383 230.905 242.932 260.301 242.932C271.423 242.932 281.748 246.641 290.49 253.26L238.321 283.449C235.146 285.303 233.555 287.951 233.555 291.659V368.455L233.553 368.452ZM280.292 395.462L249.176 377.985V340.913L280.292 323.436L311.407 340.913V377.985L280.292 395.462ZM300.286 475.968C289.163 475.968 278.837 472.259 270.097 465.64L322.264 435.449C325.441 433.597 327.03 430.949 327.03 427.239V350.445L349.011 363.155C350.865 364.213 351.66 365.802 351.66 367.922V426.179C351.66 454.514 329.679 475.965 300.286 475.965V475.968ZM237.525 416.915L186.944 387.785C172.378 379.31 162.582 361.305 162.582 343.827C162.582 323.436 174.763 305.164 193.563 297.485V357.861C193.563 361.571 195.154 364.217 198.33 366.071L264.535 404.467L242.82 416.915C240.967 417.972 239.377 417.972 237.525 416.915ZM234.614 460.343C204.689 460.343 182.71 437.833 182.71 410.028C182.71 407.91 182.976 405.792 183.238 403.672L235.405 433.863C238.582 435.715 241.763 435.715 244.938 433.863L311.407 395.466V420.622C311.407 422.742 310.612 424.331 308.758 425.389L258.179 454.519C251.293 458.491 243.083 460.343 234.611 460.343H234.614ZM300.286 491.854C332.329 491.854 359.073 469.082 365.167 438.892C394.825 431.211 413.892 403.406 413.892 375.073C413.892 356.535 405.948 338.529 391.648 325.552C392.972 319.991 393.766 314.43 393.766 308.87C393.766 271.003 363.048 242.666 327.562 242.666C320.413 242.666 313.528 243.723 306.644 246.109C294.725 234.457 278.307 227.042 260.301 227.042C228.258 227.042 201.513 249.815 195.42 280.004C165.761 287.685 146.694 315.49 146.694 343.824C146.694 362.362 154.638 380.368 168.938 393.344C167.613 398.906 166.819 404.467 166.819 410.027C166.819 447.894 197.538 476.231 233.024 476.231C240.172 476.231 247.058 475.173 253.943 472.788C265.859 484.441 282.278 491.854 300.286 491.854Z";
// Exa's docs mark, #0143D9 (assets/exa-mark.svg is the white variant).
const EXA_PATH: &str = "M106.417 9.30741C106.417 9.57392 106.324 9.83227 106.155 10.0388L61.5395 64.5L106.155 118.96C106.324 119.166 106.417 119.425 106.417 119.691V127.843C106.417 128.482 105.897 129 105.255 129H1.16175C0.520132 129 0 128.482 0 127.843V1.15695C0 0.517982 0.520131 0 1.16174 0H105.255C105.897 0 106.417 0.517984 106.417 1.15695V9.30741ZM19.0364 116.382H87.6921L53.363 74.4795L19.0364 116.382ZM12.6699 104.192L40.1426 70.6585H12.6699V104.192ZM12.6699 58.0409H39.8964L12.6699 24.8064V58.0409ZM53.363 54.5193L87.6921 12.6176H19.0364L53.363 54.5193Z";

#[derive(Clone, Copy, PartialEq)]
pub enum Brand {
    Claude,
    OpenAi,
    Exa,
}

impl Brand {
    pub fn of(source: Provider) -> Self {
        match source {
            Provider::OpenAi | Provider::ChatGpt => Brand::OpenAi,
            _ => Brand::Claude,
        }
    }
}

#[component]
pub fn BrandMark(brand: Brand, size: u32) -> Element {
    let (view_box, fill, path) = match brand {
        Brand::Claude => ("0 0 24 24", "#D97757", CLAUDE_PATH),
        Brand::OpenAi => ("146 227 268 266", "#1c1917", OPENAI_PATH),
        Brand::Exa => ("-11.3 0 129 129", "#0143D9", EXA_PATH),
    };
    rsx! {
        svg {
            width: "{size}",
            height: "{size}",
            view_box,
            fill,
            path { fill_rule: "evenodd", d: path }
        }
    }
}

/// The mark on its white tile: large in cards, small in rows.
#[component]
pub fn BrandTile(brand: Brand, small: bool) -> Element {
    let (class, size) = if small {
        ("grid h-8 w-8 shrink-0 place-items-center rounded-[10px] border border-stone-200 bg-white", 18)
    } else {
        ("grid h-11 w-11 shrink-0 place-items-center rounded-xl border border-stone-200 bg-white", 24)
    };
    rsx! {
        div { class, BrandMark { brand, size } }
    }
}

#[component]
pub fn ProviderCard(
    brand: Brand,
    title: String,
    status: Element,
    right: Element,
    #[props(default)] selected: bool,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: if selected {
                "rounded-xl border border-ios-orange-100 bg-warm-white p-4 shadow-card"
            } else {
                "rounded-xl border border-stone-200 bg-warm-white p-4"
            },
            div { class: "flex min-w-0 items-center gap-3",
                BrandTile { brand, small: false }
                div { class: "min-w-0 flex-1",
                    p { class: "truncate text-[15px] font-semibold text-stone-900", "{title}" }
                    {status}
                }
                {right}
            }
            {children}
        }
    }
}

#[component]
pub fn StatusConnected(text: String) -> Element {
    rsx! {
        span { class: "mt-1 flex items-center gap-2 text-xs font-semibold text-ios-orange-dark",
            span { class: "h-2 w-2 shrink-0 rounded-full bg-ios-orange ring-[3px] ring-ios-orange-50" }
            span { "{text}" }
        }
    }
}

#[component]
pub fn StatusMuted(text: String, #[props(default)] pending: bool) -> Element {
    rsx! {
        span {
            class: if pending {
                "mt-1 block text-xs font-medium text-ios-orange-dark"
            } else {
                "mt-1 block text-xs text-stone-500"
            },
            "{text}"
        }
    }
}

#[component]
pub fn KeyBadge(configured: bool) -> Element {
    let app: crate::ui::AppState = use_context();
    let lang = (app.current_lang)();
    rsx! {
        span {
            class: if configured {
                "shrink-0 rounded-full bg-ios-orange-50 px-2 py-1 text-[10px] font-bold text-ios-orange-dark"
            } else {
                "shrink-0 rounded-full bg-stone-100 px-2 py-1 text-[10px] font-bold text-stone-500"
            },
            if configured {
                {t(&lang, "settings-key-configured")}
            } else {
                {t(&lang, "settings-key-missing")}
            }
        }
    }
}

/// "Use for chat" once the source can answer; "Answers the chat" when it does.
#[component]
pub fn UseForChat(
    source: Provider,
    ready: bool,
    active: Signal<Provider>,
) -> Element {
    let db: Signal<
        std::sync::Arc<crate::infrastructure::persistence::Database>,
    > = use_context();
    let app: crate::ui::AppState = use_context();
    let lang = (app.current_lang)();
    if !ready {
        return rsx! {};
    }
    if active() == source {
        return rsx! {
            p { class: "mt-3 flex items-center gap-1.5 text-xs font-semibold text-ios-orange-dark",
                crate::ui::icons::IconCheck { size: 14 }
                {t(&lang, "settings-answers-chat")}
            }
        };
    }
    rsx! {
        button {
            class: "mt-3 text-xs font-semibold text-ios-orange-dark",
            onclick: move |_| {
                let _ = crate::application::chat_source::choose(&db(), source);
                active.set(source);
            },
            {t(&lang, "settings-use-for-chat")}
        }
    }
}
