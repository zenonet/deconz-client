// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    error::Error,
    sync::{Arc, Mutex},
};

use deconz::{DeconzClient, DemoLightClient, Light, LightClient, LightState};
use gtk::{gdk::RGBA, prelude::BoxExt, Scale};
use gtk::{
    self as gtk, Button, ColorDialog, ColorDialogButton, Label, ListBox, Orientation,
    ScrolledWindow, prelude::*,
};
use gtk::{Entry, glib};
use palette::{rgb::Rgb, FromColor, Hsv, IntoColor, RgbHue, Srgb};

struct ViewModel<C>
where
    C: LightClient,
{
    state: Mutex<State>,
    client: C,
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

impl Default for State {
    fn default() -> Self {
        State {
            lights: vec![],
            selected_index: usize::MAX,
            selected_light_state: None,
        }
    }
}

impl ViewModel<DeconzClient> {
    fn init() -> Self {
        let url = env::var("DECONZ_URL").expect("Missing DECONZ_URL in env vars");
        let token = env::var("DECONZ_TOKEN").expect("Missing DECONZ_TOKEN in env vars");
        ViewModel {
            state: Mutex::new(State::default()),
            client: DeconzClient::login_with_token(url, token)
                .expect("Failed to connect to deconz server"),
        }
    }
}

impl ViewModel<DemoLightClient> {
    fn init() -> Self {
        ViewModel {
            state: Mutex::new(State::default()),
            client: DemoLightClient {},
        }
    }
}

struct Ui {
    list_box: ListBox,
    toggle_button: Button,
    light_name_label: Label,
    light_status_label: Label,
    toggle_button_text: Label,
    controller_layout: gtk::Box,
    color_control: ColorDialogButton,
    search_bar: Entry,
    brightness_slider: Scale,
}

fn build_ui(application: &gtk::Application) -> Ui {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title(Some("Deconz Control"));
    window.set_default_size(500, 700);

    let list_box = gtk::ListBox::new();

    let scrolled_window = ScrolledWindow::builder().child(&list_box).build();
    scrolled_window.set_vexpand(true);

    let search_bar = Entry::builder()
        .hexpand(true)
        .placeholder_text("Search for lamps...")
        .build();

    let selection_layout = gtk::Box::new(Orientation::Vertical, 0);
    selection_layout.append(&search_bar);

    selection_layout.append(&scrolled_window);

    let controller_layout = gtk::Box::new(Orientation::Vertical, 10);
    controller_layout.set_margin_start(20);
    controller_layout.set_margin_end(20);
    controller_layout.set_margin_top(20);
    controller_layout.set_visible(false);

    let light_name_label = Label::new(Some("No lamp selected"));

    let light_status_label = Label::new(None);

    controller_layout.append(&light_name_label);
    controller_layout.append(&light_status_label);
    let toggle_button_text = Label::new(Some("Toggle lamp"));
    let toggle_button = Button::builder()
        .child(&toggle_button_text)
        .tooltip_text("Toggles the on/off state of the lamp")
        .build();
    controller_layout.append(&toggle_button);

    let dialog = ColorDialog::builder().with_alpha(false).build();
    let col = ColorDialogButton::builder().dialog(&dialog).build();

    controller_layout.append(&col);




    let brightness_slider = Scale::with_range(Orientation::Horizontal, 0.0, 255.0, 1.0);

    controller_layout.append(&brightness_slider);

    let layout = gtk::Box::new(Orientation::Horizontal, 0);
    layout.set_homogeneous(true);

    layout.append(&selection_layout);
    layout.append(&controller_layout);

    window.set_child(Some(&layout));

    let ui = Ui {
        list_box,
        toggle_button,
        light_name_label,
        light_status_label,
        toggle_button_text,
        controller_layout,
        color_control: col,
        search_bar,
        brightness_slider
    };

     window.present();

    ui
}

fn add_app_logic<C: LightClient + 'static>(ui: Ui, model: ViewModel<C>) {
    println!("Attaching app logic...");
    let ui = Arc::new(ui);
    let model = Arc::new(model);

    fn fetch_light_state<C: LightClient + 'static>(model: Arc<ViewModel<C>>, ui: Arc<Ui>) {
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
                ui.light_status_label.set_text(if light_state.reachable {
                    ""
                } else {
                    "Not reachable"
                });
                ui.toggle_button_text.set_text(if light_state.on {
                    "Turn off"
                } else {
                    "Turn on"
                });

                let hsv = Hsv::new(
                    RgbHue::from_degrees(light_state.hue.unwrap_or_default() as f32 / u16::MAX as f32 * 360.0),
                    light_state.sat.unwrap_or_default() as f32 / 255.0,
                    light_state.bri.unwrap_or(255) as f32 / 255.0
                );
                
                let rgb: Srgb = hsv.into_color();
                ui.color_control.set_rgba(&RGBA::new(rgb.red, rgb.green, rgb.blue, 1.0));
            }
        });
    }

    let update_light_list = {
        let ui = ui.clone();
        let model = model.clone();
        move ||{
            let mut state = model.state.lock().unwrap();
            
            // remember the last selected light
            let selected_light_id = state.lights.get(state.selected_index).and_then(|l| Some(l.id));

            while let Some(child) = ui.list_box.first_child(){
                ui.list_box.remove(&child);
            }
            
            let mut selected_light_index = usize::MAX;

            let lights = &state.lights;
            let search_query = ui.search_bar.text().to_lowercase();
            for (i, light) in lights.iter().enumerate() {
                if light.name.to_lowercase().matches(&*search_query).next().is_none() {
                    continue;
                }

                let label = Label::new(Some(&light.name));
                //let row = ListBoxRow::builder().child(&label).build();
                ui.list_box.append(&label);

                if selected_light_id.is_some_and(|id| light.id == id){
                    selected_light_index = i;
                }
            }

            // Reselect the light from before
            state.selected_index = selected_light_index;
            // TODO: set the selected row in the ui element
        }
    };
    let update_light_list = Arc::new(update_light_list);

    let fetch_light_list = {
        let update_light_list = update_light_list.clone();
        move |model: Arc<ViewModel<C>>| {
            glib::spawn_future_local(async move {
                {
                    let light_list = model.client.get_light_list().await.unwrap();

                    let mut state = model.state.lock().unwrap();
                    state.lights = light_list;
                }
                update_light_list();
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

    {
        let model = model.clone();
        ui.color_control.connect_rgba_notify(move |but| {
            let col = but.rgba();

            let rgb = Rgb::new(col.red(), col.green(), col.blue());
            let hsv: Hsv = Hsv::from_color(rgb);
            let h = (hsv.hue.into_positive_degrees()) / 360.0 * u16::MAX as f32;

            let model = model.clone();
            glib::spawn_future_local(async move {
                // Convert color from rgb to hsb

                let state = model.state.lock().unwrap(); // TODO: fix this lock staying while the request is done. fr this time
                let light = state.selected_light().unwrap(); // todo fix unwrap
                model
                    .client
                    .set_light_color(light,
                        Some(h as u16),
                        Some((hsv.value * 255.0) as u8),
                        Some((hsv.saturation * 255.0) as u8)
                    )
                    .await
                    .unwrap();
            });
        });
    }

    {
        let model = model.clone();
        ui.brightness_slider.connect_value_changed(move |s|{
            let val = s.value() as u8;

            let model = model.clone();
            glib::spawn_future_local(async move {
                let state = model.state.lock().unwrap();
                let light = state.selected_light().unwrap();

                model
                .client
                .set_light_color(light, None, Some(val), None)
                .await
                .unwrap();

            });
        });
    }    
    {
        let update_light_list = update_light_list.clone();
        ui.search_bar.connect_changed(move |_| {
            update_light_list();
        });
    }
    println!("UI logic attached");
    fetch_light_list(model);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let application = gtk::Application::builder()
        .application_id("de.zenonet.deconz")
        .build();

    fn init(app: &gtk::Application) {

        let ui = build_ui(&app);

        let model = ViewModel::<DeconzClient>::init();

        add_app_logic(ui, model);
    }

    application.connect_activate(init);
    application.run();

    Ok(())
}
