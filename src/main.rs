#![allow(unused)]
#![allow(dead_code)]
mod dex;
mod pp;
mod save_data;

use std::{format, sync::LazyLock};

use dioxus::{logger::tracing, prelude::*};
use pp::{PiiSex, SDPiiPersonalData};
use save_data::{decrypt_savedata, extract_piibox, SAVEDATA_SIZE};
use web_sys::{
    js_sys::{self, Reflect},
    wasm_bindgen::JsValue,
};

// ew
pub static LOCALE: LazyLock<String> = LazyLock::new(|| {
    let f = js_sys::Intl::DateTimeFormat::new(
        &JsValue::undefined().into(),
        &JsValue::undefined().into(),
    )
    .resolved_options();
    Reflect::get(&f, &"locale".into())
        .unwrap()
        .as_string()
        .unwrap()
});

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut pii_box_signal: Signal<Vec<SDPiiPersonalData>> = use_signal(Vec::new);
    let mut onfile =
        async move |evt: Event<FormData>, mut pii_box_signal: Signal<Vec<SDPiiPersonalData>>| {
            let Some(file_engine) = evt.files() else {
                return;
            };
            let files = file_engine.files();
            let file_name = files.first().unwrap().clone();
            let mut save_file = file_engine.read_file(&file_name).await.unwrap();

            assert_eq!(save_file.len(), SAVEDATA_SIZE);
            decrypt_savedata(&mut save_file);
            let pii_box = extract_piibox(&save_file);
            *pii_box_signal.write() = std::mem::take(&mut pii_box.into_vec());
        };

    rsx! {
        document::Stylesheet { href: asset!("/assets/tailwind.css") }
        div {
            class: "flex gap-4 w-full h-full",
            if pii_box_signal.is_empty() {
                div {
                    class: "w-full h-full flex flex-col gap-4 items-center justify-center",
                    h1 {
                        class: "text-8xl",
                        "PiiPii"
                    }
                    h2 {
                        class: "italic text-4xl",
                        "A WIP save editor for Pokémon Rumble"
                    }
                    label {
                        class: "flex flex-col items-center justify-center p-4 bg-stone-800 hover:bg-stone-700 rounded-lg",
                        "Select your savedata.bin file.",
                        input {
                            r#type: "file",
                            accept: ".bin",
                            onchange: move |e| onfile(e, pii_box_signal)
                        }
                    }
                }
            } else {
                div {
                    class: "flex gap-4 flex-wrap",
                    // class: "w-fit flex flex-col gap-4",
                    for pii in pii_box_signal.iter() {
                        PiiListItem { pii: pii.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn PiiListItem(pii: SDPiiPersonalData) -> Element {
    let sex_symbol = match pii.sex() {
        Ok(PiiSex::Male) => "♂",
        Ok(PiiSex::Female) => "♀",
        _ => "",
    };
    let trait_prefix = pii.trait_().map_or("", |t| t.name);
    let pii_name = format!("{} {}{}", trait_prefix, pii.name(), sex_symbol);
    let name_color = if pii.is_shiny() {
        "text-blue-300"
    } else if pii.trait_().is_some() {
        "text-purple-300"
    } else {
        ""
    };
    rsx! {
        div {
            // class: "flex flex-col p-4 bg-stone-800 hover:bg-stone-700 border-4 border-transparent active:border-white items-center gap-8 rounded-4xl",
            class: "flex flex-col p-4 bg-stone-800 border-4 border-transparent items-center gap-8 rounded-4xl",
            div {
                class: "flex flex-col items-center",
                img { src: pii.sprite_src(), alt: pii.name(), class: "w-[128]" }
                p { class: "-mt-5 z-10 bg-emerald-600 rounded-md p-1.5", "Lvl. {pii.level}"}
            }
            div {
                class: "flex flex-col",
                p { class: "text-2xl {name_color}", "{pii_name}" }
                {(1..=2).map( |move_no| {
                    if let Some(move_) = pii.move_name(move_no) {
                        rsx!(
                            p { class: "italic", "Knows {move_}" }
                        )
                    } else {
                        rsx!()
                    }
                })}
                if pii.trainer_id == 1 {
                    p { class: "italic", "From afar" }
                }
                p { {pii.unix_time()} }
            }
        }
    }
}
