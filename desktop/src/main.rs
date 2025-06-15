// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, error::Error, ops::Deref};

use deconz::Light;
use slint::{ModelRc, SharedString};

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

    let model = lights.iter().map(|light|{
        light.name.clone().into()
    }).collect::<Vec<slint::SharedString>>();

    let model = ModelRc::new(slint::VecModel::from(model));

    let mut selected_light_id: Option<u32> = None; 

    ui.set_lights_model(model);

    ui.on_light_clicked({
        let ui_handle = ui.as_weak();
        move |name| {

            let light = lights.iter().find(|l| l.name == *name);

            selected_light_id = light.and_then(|l| Some(l.id));

            println!("Lamp clicked: {}", name);
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
            ui.set_showLabel(ui.get_counter() % 2 == 0);

            ui.set_selected_light_name(SharedString::from(name));
        }
    });

    // Somehow need to react to change in on_state
    
    ui.run()?;

    Ok(())
}
