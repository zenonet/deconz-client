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
    client: DeconzClient
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
            state: Mutex::new(State{
                lights: vec![],
                selected_index: usize::MAX,
                selected_light_state: None,
            }),
            client: DeconzClient::login_with_token(url, token).expect("Failed to connect to deconz server")
        }
    }
}


struct Ui{
    list_box: ListBox
}

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title(Some("Deconz Control"));
    window.set_default_size(500, 700);

    let model = Arc::new(ViewModel::init());
    
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
    
    let ui = Arc::new(Ui{
        list_box,
    });

    fn fetch_light_state(model: Arc<ViewModel>){
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
            }
            println!("Completed light state init");
        });
    }
    
    
    let fetch_light_list = {
        let ui = ui.clone();
         move |model: Arc<ViewModel>|{
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
        ui.list_box.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let label = row.child().unwrap().downcast::<Label>().unwrap();
                println!("Row {} was selected", label.text());
                light_name_label.set_text(&label.text());
                
                let mut state = model.state.lock().unwrap();

                // Find the selected light:
                let light_index = state.lights.iter().position(|l| &l.name == &label.text());

                let Some(light) = light_index else { return };
                state.selected_index = light;

                // Load current light state
                fetch_light_state(model.clone());
            }
        });
    }

    {
        let model = model.clone();
        toggle_button.connect_clicked(move |_| {
            let state = &model.state.lock().unwrap();

            let Some(light_state) = state.selected_light_state else {
                return;
            };
            let new_on_state = !light_state.on;

            {
                let model = model.clone();
                glib::spawn_future_local(async move {
                    model.client
                        .set_on_state( // TODO: Only give the light id so that the mutex is not locked while the request is sent
                            &*model.state.lock().unwrap().selected_light().unwrap(),
                            new_on_state,
                        )
                        .await
                        .unwrap();

                    // Update light state
                    fetch_light_state(model);
                });
            }
        });
    }

    window.present();

    fetch_light_list(model);
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
