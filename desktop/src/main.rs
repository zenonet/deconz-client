// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    error::Error,
    sync::{Arc, Mutex, OnceLock},
};

use deconz::{DeconzClient, Light, LightState};
use gtk::{gio, glib, prelude::*};
use gtk4::{self as gtk, Button, Label, ListBox, Orientation, ScrolledWindow};
use tokio::runtime::Runtime;

struct ViewModel {
    state: Mutex<State>,
    client: DeconzClient,
}

struct State {
    lights: Vec<Light>,
    selected_index: usize,
    selected_light_state: Option<LightState>,
}

impl State {
    fn selected_light<'a>(&'a self) -> Option<&'a Light> {
        let i = self.selected_index;
        self.lights.get(i)
    }
}
impl ViewModel {
    fn init() -> Self {
        let url = env::var("DECONZ_URL").expect("Missing DECONZ_URL in env vars");
        let token = env::var("DECONZ_TOKEN").expect("Missing DECONZ_TOKEN in env vars");
        ViewModel {
            state: Mutex::new(State {
                lights: vec![],
                selected_index: usize::MAX,
                selected_light_state: None,
            }),
            client: DeconzClient::login_with_token(url, token)
                .expect("Failed to connect to deconz server"),
        }
    }
}

struct Ui {
    list_box: ListBox,
    toggle_button: Button,
    light_name_label: Label,
    toggle_button_text: Label,
    controller_layout: gtk::Box,
}

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

fn build_ui(application: &gtk::Application) -> Ui {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title(Some("Deconz Control"));
    window.set_default_size(500, 700);

    let list_box = gtk::ListBox::new();

    let scrolled_window = ScrolledWindow::builder().child(&list_box).build();

    let controller_layout = gtk::Box::new(Orientation::Vertical, 10);
    controller_layout.set_visible(false);
    let light_name_label = Label::new(Some("No lamp selected"));

    controller_layout.append(&light_name_label);
    let toggle_button_text = Label::new(Some("Toggle lamp"));
    let toggle_button = Button::builder()
        .child(&toggle_button_text)
        .tooltip_text("Toggles the on/off state of the lamp")
        .build();
    controller_layout.append(&toggle_button);

    let layout = gtk::Box::new(gtk4::Orientation::Horizontal, 0);
    layout.set_homogeneous(true);

    layout.append(&scrolled_window);
    layout.append(&controller_layout);

    window.set_child(Some(&layout));

    let ui = Ui {
        list_box,
        toggle_button,
        light_name_label,
        toggle_button_text,
        controller_layout,
    };

    window.present();

    ui
}

fn add_app_logic(ui: Ui) {
    let model = Arc::new(ViewModel::init());
    let ui = Arc::new(ui);

    fn fetch_light_state(model: Arc<ViewModel>, ui: Arc<Ui>){
        glib::spawn_future_local(async move {
            let mut state = model.state.lock().unwrap();
            if let Some(light) = state.selected_light() {
                let light_state = model
                    .client
                    .get_light_state(light)
                    .await
                    .expect(&format!("Failed to load state of light {}", light.name));
                println!("Got state for {}:\n{:#?}", light.name, light_state);
                state.selected_light_state = Some(light_state);
                ui.controller_layout.set_visible(true);
                ui.toggle_button_text.set_text(if light_state.on {"Turn off"} else {"Turn on"});
            }
        });
    }

    let fetch_light_list = {
        let ui = ui.clone();
        move |model: Arc<ViewModel>| {
            glib::spawn_future_local(async move {
                let light_list = model.client.get_light_list().await.unwrap();

                for light in light_list.iter() {
                    let label = Label::new(Some(&light.name));
                    // let row = ListBoxRow::builder().child(&label).build();
                    ui.list_box.append(&label);
                }
                let mut state = model.state.lock().unwrap();
                state.lights = light_list;
            });
        }
    };

    {
        let model = model.clone();
        let a_ui = ui.clone();
        ui.list_box.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let label = row.child().unwrap().downcast::<Label>().unwrap();
                println!("Row {} was selected", label.text());
                a_ui.light_name_label.set_text(&label.text());

                let mut state = model.state.lock().unwrap();

                // Find the selected light:
                let light_index = state.lights.iter().position(|l| &l.name == &label.text());

                let Some(light) = light_index else { return };
                state.selected_index = light;

                // Load current light state
                fetch_light_state(model.clone(), a_ui.clone());
            }
        });
    }

    {
        let model = model.clone();
        let a_ui = ui.clone();
        ui.toggle_button.connect_clicked(move |_| {
            let state = &model.state.lock().unwrap();

            let Some(light_state) = state.selected_light_state else {
                return;
            };
            let new_on_state = !light_state.on;

            {
                let model = model.clone();
                let ui = a_ui.clone();
                glib::spawn_future_local(async move {
                    model
                        .client
                        .set_on_state(
                            // TODO: Only give the light id so that the mutex is not locked while the request is sent
                            &*model.state.lock().unwrap().selected_light().unwrap(),
                            new_on_state,
                        )
                        .await
                        .unwrap();

                    // Update light state
                    fetch_light_state(model, ui.clone());
                });
            }
        });
    }

    fetch_light_list(model);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let application = gtk::Application::builder()
        .application_id("de.zenonet.deconz")
        .build();

    fn init(app: &gtk::Application) {
        let ui = build_ui(&app);
        add_app_logic(ui);
    }

    application.connect_activate(init);
    application.run();

    Ok(())
}
