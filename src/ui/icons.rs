use dioxus::prelude::*;

#[component]
pub fn IconNewNote(#[props(default = 28)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "none",
            path {
                d: "M117.333 224H74.667C62.885 224 53.333 214.449 53.333 202.667V192V53.333C53.333 41.551 62.885 32 74.667 32H138.667M202.667 96L138.667 32M202.667 96H160C148.218 96 138.667 86.449 138.667 74.667V32M202.667 96V117.333M85.333 181.333H117.333M85.333 138.667H138.667M85.333 96H106.667",
                stroke: "currentColor", stroke_width: "14", stroke_linecap: "round", stroke_linejoin: "round",
            }
            path {
                d: "M181.333 213.333V160M208 186.667H154.667",
                stroke: "#007aff", stroke_width: "14", stroke_linecap: "round", stroke_linejoin: "round",
            }
        }
    }
}

#[component]
pub fn IconMic(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none",
            stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
            path { d: "M12 3C10 3 10 5 10 5L10 11C10 11 10 13 12 13C14 13 14 11 14 11L14 5C14 5 14 3 12 3ZM5 10C5 10 5 17 12 17C19 17 19 10 19 10M12 17L12 21M9 21L15 21" }
        }
    }
}

#[component]
pub fn IconStop(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M200,40H56A16,16,0,0,0,40,56V200a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V56A16,16,0,0,0,200,40Zm0,160H56V56H200V200Z" }
        }
    }
}

#[component]
pub fn IconTrash(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M216,48H176V40a24,24,0,0,0-24-24H104A24,24,0,0,0,80,40v8H40a8,8,0,0,0,0,16h8V208a16,16,0,0,0,16,16H192a16,16,0,0,0,16-16V64h8a8,8,0,0,0,0-16ZM96,40a8,8,0,0,1,8-8h48a8,8,0,0,1,8,8v8H96Zm96,168H64V64H192ZM112,104v64a8,8,0,0,1-16,0V104a8,8,0,0,1,16,0Zm48,0v64a8,8,0,0,1-16,0V104a8,8,0,0,1,16,0Z" }
        }
    }
}

#[component]
pub fn IconPencil(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M227.31,73.37,182.63,28.68a16,16,0,0,0-22.63,0L36.69,152A15.86,15.86,0,0,0,32,163.31V208a16,16,0,0,0,16,16H92.69A15.86,15.86,0,0,0,104,219.31L227.31,96a16,16,0,0,0,0-22.63ZM92.69,208H48V163.31l88-88L180.69,120ZM192,108.68,147.31,64l24-24L216,84.68Z" }
        }
    }
}

#[component]
pub fn IconFolderPlus(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M216,72H131.31L104,44.69A15.86,15.86,0,0,0,92.69,40H40A16,16,0,0,0,24,56V200.62A15.4,15.4,0,0,0,39.38,216H216.89A15.13,15.13,0,0,0,232,200.89V88A16,16,0,0,0,216,72ZM92.69,56l16,16H40V56ZM216,200H40V88H216Zm-88-88a8,8,0,0,1,8,8v16h16a8,8,0,0,1,0,16H136v16a8,8,0,0,1-16,0V152H104a8,8,0,0,1,0-16h16V120A8,8,0,0,1,128,112Z" }
        }
    }
}

#[component]
pub fn IconFolder(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M216,72H130.67L102.93,51.2a16.12,16.12,0,0,0-9.6-3.2H40A16,16,0,0,0,24,64V200a16,16,0,0,0,16,16H216.89A15.13,15.13,0,0,0,232,200.89V88A16,16,0,0,0,216,72Zm0,128H40V64H93.33L123.2,86.4A8,8,0,0,0,128,88h88Z" }
        }
    }
}

#[component]
pub fn IconDotsThree(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M140,128a12,12,0,1,1-12-12A12,12,0,0,1,140,128Zm56-12a12,12,0,1,0,12,12A12,12,0,0,0,196,116ZM60,116a12,12,0,1,0,12,12A12,12,0,0,0,60,116Z" }
        }
    }
}

#[component]
pub fn IconPlus(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M224,128a8,8,0,0,1-8,8H136v80a8,8,0,0,1-16,0V136H40a8,8,0,0,1,0-16h80V40a8,8,0,0,1,16,0v80h80A8,8,0,0,1,224,128Z" }
        }
    }
}

#[component]
pub fn IconArrowLeft(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M224,128a8,8,0,0,1-8,8H59.31l58.35,58.34a8,8,0,0,1-11.32,11.32l-72-72a8,8,0,0,1,0-11.32l72-72a8,8,0,0,1,11.32,11.32L59.31,120H216A8,8,0,0,1,224,128Z" }
        }
    }
}

#[component]
pub fn IconList(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M224,128a8,8,0,0,1-8,8H40a8,8,0,0,1,0-16H216A8,8,0,0,1,224,128ZM40,72H216a8,8,0,0,0,0-16H40a8,8,0,0,0,0,16ZM216,184H40a8,8,0,0,0,0,16H216a8,8,0,0,0,0-16Z" }
        }
    }
}

#[component]
pub fn IconFloppyDisk(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M219.31,72,184,36.69A15.86,15.86,0,0,0,172.69,32H48A16,16,0,0,0,32,48V208a16,16,0,0,0,16,16H208a16,16,0,0,0,16-16V83.31A15.86,15.86,0,0,0,219.31,72ZM168,208H88V152h80Zm40,0H184V152a16,16,0,0,0-16-16H88a16,16,0,0,0-16,16v56H48V48H172.69L208,83.31ZM160,72a8,8,0,0,1-8,8H96a8,8,0,0,1,0-16h56A8,8,0,0,1,160,72Z" }
        }
    }
}

#[component]
pub fn IconCheck(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M229.66,77.66l-128,128a8,8,0,0,1-11.32,0l-56-56a8,8,0,0,1,11.32-11.32L96,188.69,218.34,66.34a8,8,0,0,1,11.32,11.32Z" }
        }
    }
}

#[component]
pub fn IconX(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M205.66,194.34a8,8,0,0,1-11.32,11.32L128,139.31,61.66,205.66a8,8,0,0,1-11.32-11.32L116.69,128,50.34,61.66A8,8,0,0,1,61.66,50.34L128,116.69l66.34-66.35a8,8,0,0,1,11.32,11.32L139.31,128Z" }
        }
    }
}
