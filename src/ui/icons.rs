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
                stroke: "#E86A10", stroke_width: "14", stroke_linecap: "round", stroke_linejoin: "round",
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
pub fn IconChatAi(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "none",
            path {
                d: "M232,124A100.11,100.11,0,0,1,132,224H48a16,16,0,0,1-16-16V124a100,100,0,0,1,200,0ZM216,124a84,84,0,0,0-168,0v84h84A84.09,84.09,0,0,0,216,124Z",
                fill: "currentColor",
            }
            path {
                d: "M172,112a8,8,0,0,1-8,8H96a8,8,0,0,1,0-16h68A8,8,0,0,1,172,112ZM164,136H96a8,8,0,0,0,0,16h68a8,8,0,0,0,0-16Z",
                fill: "#E86A10",
            }
        }
    }
}

#[component]
pub fn IconHeadCircuit(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M190.37,170.62A86.27,86.27,0,0,0,222,102c-1-44.68-36.76-81.51-81.34-83.86A86,86,0,0,0,50,102.51l-22.69,43.6c-.07.13-.13.26-.19.4a14,14,0,0,0,6.61,18l.18.09l24.08,11V208a14,14,0,0,0,14,14h48a6,6,0,0,0,0-12H72a2,2,0,0,1-2-2v-36.19a6,6,0,0,0-3.5-5.46L39,153.78a2,2,0,0,1-.93-2.4l23.21-44.61A6,6,0,0,0,62,104a74.05,74.05,0,0,1,60-72.68v19.52a22,22,0,1,0,12,0V30.05c2-.05,4-.05,6,.06A74.29,74.29,0,0,1,206.63,82H184a6,6,0,0,0-4.61,2.16l-26.45,31.74a22.06,22.06,0,1,0,9.21,7.69L186.81,94h22.5a72,72,0,0,1,.67,8.26,74.24,74.24,0,0,1-29.58,60.94,6,6,0,0,0-2.35,5.54l8,64A6,6,0,0,0,192,238a6,6,0,0,0,.75-.05,6,6,0,0,0,5.21-6.7ZM138,72a10,10,0,1,1-10-10,10,10,0,0,1,10,10Zm6,74a10,10,0,1,1,10-10,10,10,0,0,1-10,10Z" }
        }
    }
}

#[component]
pub fn IconPaperPlaneRight(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M231.87,114l-168-95.89A16,16,0,0,0,40.92,37.34L71.55,128,40.92,218.67A16,16,0,0,0,56,240a16.15,16.15,0,0,0,7.93-2.1l167.92-96.05a16,16,0,0,0,.05-27.89ZM56,224a.56.56,0,0,0,0-.12L85.74,136H144a8,8,0,0,0,0-16H85.74L56.06,32.16A.46.46,0,0,0,56,32l168,95.83Z" }
        }
    }
}

#[component]
pub fn IconNotePencil(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M229.66,58.34l-32-32a8,8,0,0,0-11.32,0l-96,96A8,8,0,0,0,88,128v32a8,8,0,0,0,8,8h32a8,8,0,0,0,5.66-2.34l96-96A8,8,0,0,0,229.66,58.34ZM124.69,152H104V131.31l64-64L188.69,88ZM200,76.69,179.31,56,192,43.31,212.69,64ZM224,128v80a16,16,0,0,1-16,16H48a16,16,0,0,1-16-16V48A16,16,0,0,1,48,32h80a8,8,0,0,1,0,16H48V208H208V128a8,8,0,0,1,16,0Z" }
        }
    }
}

#[component]
pub fn IconChats(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M232,96a16,16,0,0,0-16-16H184V48a16,16,0,0,0-16-16H40A16,16,0,0,0,24,48V176a8,8,0,0,0,13,6.22L72,154V176a16,16,0,0,0,16,16h93.59L219,222.22A8,8,0,0,0,232,216Zm-16,0V196L202.41,176H88V80H216Z" }
        }
    }
}

#[component]
pub fn IconNotebook(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M208,32H48A16,16,0,0,0,32,48V208a16,16,0,0,0,16,16H208a16,16,0,0,0,16-16V48A16,16,0,0,0,208,32ZM48,48H72V208H48ZM208,208H88V48H208Z" }
        }
    }
}

#[component]
pub fn IconGear(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M128,80a48,48,0,1,0,48,48A48.05,48.05,0,0,0,128,80Zm0,80a32,32,0,1,1,32-32A32,32,0,0,1,128,160Zm109.94-52.79a8,8,0,0,0-3.89-5.4l-29.83-17-.12-33.62a8,8,0,0,0-2.83-6.08,111.91,111.91,0,0,0-36.72-20.67,8,8,0,0,0-6.46.59L128,42.89,97.88,25a8,8,0,0,0-6.47-.6A112.1,112.1,0,0,0,54.73,45.15a8,8,0,0,0-2.83,6.07l-.15,33.65-29.83,17a8,8,0,0,0-3.89,5.4,106.47,106.47,0,0,0,0,41.56,8,8,0,0,0,3.89,5.4l29.83,17,.12,33.62a8,8,0,0,0,2.83,6.08,111.91,111.91,0,0,0,36.72,20.67,8,8,0,0,0,6.46-.59L128,213.11,158.12,231a7.94,7.94,0,0,0,3.9,1,8.09,8.09,0,0,0,2.57-.42,112.1,112.1,0,0,0,36.68-20.73,8,8,0,0,0,2.83-6.07l.15-33.65,29.83-17a8,8,0,0,0,3.89-5.4A106.47,106.47,0,0,0,237.94,107.21Zm-15,34.91-28.57,16.25a8,8,0,0,0-3,3c-.58,1-1.19,2.06-1.81,3.06a7.94,7.94,0,0,0-1.22,4.21l-.15,32.25a95.89,95.89,0,0,1-25.37,14.3L134,199.13a8,8,0,0,0-3.91-1h-.19c-1.21,0-2.43,0-3.64,0a8.08,8.08,0,0,0-4.1,1l-28.84,16.1A96,96,0,0,1,67.88,201l.11-32.2a8,8,0,0,0-1.22-4.22c-.62-1-1.23-2-1.8-3.06a8.09,8.09,0,0,0-3-3.06L33.4,142.12a90.29,90.29,0,0,1,0-28.24L61.97,97.63a8,8,0,0,0,3-3c.58-1,1.19-2.06,1.81-3.06a7.94,7.94,0,0,0,1.22-4.21l.15-32.25a95.89,95.89,0,0,1,25.37-14.3L122,56.87a8,8,0,0,0,4.1,1c1.21,0,2.43,0,3.64,0a8.08,8.08,0,0,0,4.1-1l28.84-16.1A96,96,0,0,1,188.12,55l-.11,32.2a8,8,0,0,0,1.22,4.22c.62,1,1.23,2,1.8,3.06a8.09,8.09,0,0,0,3,3.06l28.57,16.25A90.29,90.29,0,0,1,222.94,142.12Z" }
        }
    }
}

#[component]
pub fn IconMagnifyingGlass(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M229.66,218.34l-50.07-50.06a88.11,88.11,0,1,0-11.31,11.31l50.06,50.07a8,8,0,0,0,11.32-11.32ZM40,112a72,72,0,1,1,72,72A72.08,72.08,0,0,1,40,112Z" }
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

#[component]
pub fn IconDotsThreeVertical(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M140,128a12,12,0,1,1-12-12A12,12,0,0,1,140,128ZM128,72a12,12,0,1,0-12-12A12,12,0,0,0,128,72Zm0,112a12,12,0,1,0,12,12A12,12,0,0,0,128,184Z" }
        }
    }
}

#[component]
pub fn IconFileArrowUp(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M213.66,82.34l-56-56A8,8,0,0,0,152,24H56A16,16,0,0,0,40,40V216a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V88A8,8,0,0,0,213.66,82.34ZM160,51.31,188.69,80H160ZM200,216H56V40h88V88a8,8,0,0,0,8,8h48V216Zm-42.34-77.66a8,8,0,0,1-11.32,11.32L136,139.31V184a8,8,0,0,1-16,0V139.31l-10.34,10.35a8,8,0,0,1-11.32-11.32l24-24a8,8,0,0,1,11.32,0Z" }
        }
    }
}

#[component]
pub fn IconPlay(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M240,128a112,112,0,1,1-112-112A112,112,0,0,1,240,128ZM120,86.63,162.69,128,120,169.37Zm0,105.74V63.63L162.69,128Z" }
        }
    }
}

#[component]
pub fn IconPause(#[props(default = 20)] size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}", height: "{size}", view_box: "0 0 256 256", fill: "currentColor",
            path { d: "M240,128a112,112,0,1,1-112-112A112,112,0,0,1,240,128ZM96,80v96a8,8,0,0,0,16,0V80a8,8,0,0,0-16,0Zm64,0v96a8,8,0,0,0,16,0V80a8,8,0,0,0-16,0Z" }
        }
    }
}
