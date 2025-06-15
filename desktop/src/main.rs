// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    error::Error,
    sync::{Arc, Mutex, OnceLock},
};

use deconz::{Light, LightState};
use gtk::{gio, glib, prelude::*};
use gtk4::{
    self as gtk, Button, Label, Orientation, ScrolledWindow,
};
use tokio::runtime::Runtime;

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title(Some("Deconz Control"));
    window.set_default_size(500, 700);

    let url = env::var("DECONZ_URL").expect("Missing DECONZ_URL in env vars");
    let token = env::var("DECONZ_TOKEN").expect("Missing DECONZ_TOKEN in env vars");
    let client = Arc::new(deconz::DeconzClient::login_with_token(url, token).unwrap());

    let mut lights: Arc<Mutex<Vec<Light>>> = Arc::new(Mutex::new(vec![]));

    let mut selected_light: Arc<Mutex<Option<Light>>> = Arc::new(Mutex::new(None));
    let mut selected_light_state: Arc<Mutex<Option<LightState>>> = Arc::new(Mutex::new(None));

    let list_box = gtk::ListBox::new();

    let scrolled_window = ScrolledWindow::builder().child(&list_box).build();

    let controller = gtk::Box::new(Orientation::Vertical, 10);
    let light_name_label = Label::new(Some("No lamp selected"));

    controller.append(&light_name_label);
    let l = Label::new(Some("Toggle lamp"));
    let toggle_button = Button::builder()
        .child(&l)
        .tooltip_text("Toggles the on/off state of the lamp")
        .build();
    controller.append(&toggle_button);

    let layout = gtk::Box::new(gtk4::Orientation::Horizontal, 0);
    layout.set_homogeneous(true);

    layout.append(&scrolled_window);
    layout.append(&controller);

    window.set_child(Some(&layout));

    let a_lights = lights.clone();
    let a_selected_light= selected_light.clone();
    let a_client = client.clone();
    let a_light_state = selected_light_state.clone();
    list_box.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            let label = row.child().unwrap().downcast::<Label>().unwrap();
            println!("Row {} was selected", label.text());
            light_name_label.set_text(&label.text());
            
            // Find the selected light:
            let lights = a_lights.lock().unwrap();
            let light = lights.iter().find(|l| &l.name == &label.text());
            
            let Some(light) = light else { return };
            *a_selected_light.lock().unwrap() = Some(light.clone());


            // Load current light state
            let b_client = a_client.clone();
            let b_selected_light = a_selected_light.clone();
            let b_light_state = a_light_state.clone();
            glib::spawn_future_local(async move{
                if let Some(light) = &*b_selected_light.lock().unwrap(){
                    let state = b_client.get_light_state(light).await.expect(&format!("Failed to load state of light {}", light.name));
                    println!("Got state for {}:\n{:#?}", light.name, state);
                    *b_light_state.lock().unwrap() = Some(state);
                }
            });
        }
    });
    
    let a_selected_light = selected_light.clone();
    let a_client = client.clone();
    let a_light_state = selected_light_state.clone();
    toggle_button.connect_clicked(move |_| {

        let Some(state) = &*a_light_state.lock().unwrap() else {return};
        let new_on_state = !state.on;

        let b_selected_light = a_selected_light.clone();
        let b_client = a_client.clone();
        glib::spawn_future_local(async move {
            b_client.set_on_state(&b_selected_light.lock().unwrap().clone().unwrap(), new_on_state).await.unwrap();
        });
    });

    window.present();

    let a_lights = lights.clone();
    let client = client.clone();
    glib::spawn_future_local(async move {
        let mut lights = a_lights.lock().unwrap();
        *lights.as_mut() = client.get_light_list().await.unwrap();

        for light in lights.iter() {
            let label = Label::new(Some(&light.name));
            // let row = ListBoxRow::builder().child(&label).build();
            list_box.append(&label);
        }
    });
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let application = gtk::Application::builder()
        .application_id("de.zenonet.deconz")
        .build();
    application.connect_activate(build_ui);
    application.run();

    Ok(())
}
