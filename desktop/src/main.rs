// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, error::Error};

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    let url = env::var("DECONZ_URL").expect("Missing DECONZ_URL in env vars");
    let token = env::var("DECONZ_TOKEN").expect("Missing DECONZ_TOKEN in env vars");

    //let client = deconz::DeconzClient::login_with_link_button("http://192.168.1.239/").await.unwrap();
    let client = deconz::DeconzClient::login_with_token(url, token).unwrap();

    let lights = client.get_light_list().await.unwrap();

    println!("{:#?}", lights);

    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
            ui.set_showLabel(ui.get_counter() % 2 == 0)
        }
    });

    
    ui.run()?;

    Ok(())
}
