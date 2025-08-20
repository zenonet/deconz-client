// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    error::Error,
    fs::{create_dir, File},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use deconz::{DeconzClient, DemoLightClient, Light, LightClient, LightState};
use gtk::{
    self as gtk, Button, ColorDialog, ColorDialogButton, Label, ListBox, Orientation,
    ScrolledWindow, prelude::*,
};
use gtk::{ApplicationWindow, Scale, gdk::RGBA, prelude::BoxExt};
use gtk::{Entry, glib};
use palette::{FromColor, Hsv, IntoColor, RgbHue, Srgb, rgb::Rgb};
use serde::{Deserialize, Serialize};

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
            client: DemoLightClient::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Config {
    url: String,
    username: String,
}

fn config_file_path() -> PathBuf {
    glib::user_config_dir()
        .join("deconz-client")
        .join("config.json")
}

fn store_credentials(url: String, username: String) {
    let config = Config { url, username };

    // lets just ignore the result to allow for it failing because the directory already exists
    _ = create_dir(config_file_path().parent().unwrap());
    let file = File::create(config_file_path()).unwrap();
    serde_json::to_writer_pretty(file, &config).unwrap();
}

fn load_credentials() -> Option<Config> {
    File::open(config_file_path()).ok().and_then(|file| serde_json::from_reader::<_, Config>(file).ok())
}

struct MainWindow {
    window: ApplicationWindow,
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
impl MainWindow {
    fn new(application: &gtk::Application) -> Self {
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

        window.present();
        let ui = Self {
            window,
            list_box,
            toggle_button,
            light_name_label,
            light_status_label,
            toggle_button_text,
            controller_layout,
            color_control: col,
            search_bar,
            brightness_slider,
        };

        ui
    }
    fn add_app_logic<C: LightClient + 'static>(self, model: ViewModel<C>) {
        println!("Attaching app logic...");
        let ui = Arc::new(self);
        let model = Arc::new(model);

        fn fetch_light_state<C: LightClient + 'static>(
            model: Arc<ViewModel<C>>,
            ui: Arc<MainWindow>,
        ) {
            glib::spawn_future_local(async move {
                let mut state = model.state.lock().unwrap();
                if let Some(light) = state.selected_light() {
                    let light_state = model
                        .client
                        .get_light_state(light)
                        .await
                        .expect(&format!("Failed to load state of light {}", light.name));
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
                        RgbHue::from_degrees(
                            light_state.hue.unwrap_or_default() as f32 / u16::MAX as f32 * 360.0,
                        ),
                        light_state.sat.unwrap_or_default() as f32 / 255.0,
                        light_state.bri.unwrap_or(255) as f32 / 255.0,
                    );

                    let rgb: Srgb = hsv.into_color();
                    ui.color_control
                        .set_rgba(&RGBA::new(rgb.red, rgb.green, rgb.blue, 1.0));

                    ui.brightness_slider.set_value(hsv.value as f64 * 255.0);
                }
            });
        }

        let update_light_list = {
            let ui = ui.clone();
            let model = model.clone();
            move || {
                let mut state = model.state.lock().unwrap();

                // remember the last selected light
                let selected_light_id = state
                    .lights
                    .get(state.selected_index)
                    .and_then(|l| Some(l.id));

                while let Some(child) = ui.list_box.first_child() {
                    ui.list_box.remove(&child);
                }

                let mut selected_light_index = usize::MAX;

                let lights = &state.lights;
                let search_query = ui.search_bar.text().to_lowercase();
                for (i, light) in lights.iter().enumerate() {
                    if light
                        .name
                        .to_lowercase()
                        .matches(&*search_query)
                        .next()
                        .is_none()
                    {
                        continue;
                    }

                    let label = Label::new(Some(&light.name));
                    //let row = ListBoxRow::builder().child(&label).build();
                    ui.list_box.append(&label);

                    if selected_light_id.is_some_and(|id| light.id == id) {
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
            let ui2 = ui.clone();
            ui.color_control.connect_rgba_notify(move |but| {
                let col = but.rgba();

                let rgb = Rgb::new(col.red(), col.green(), col.blue());
                let hsv: Hsv = Hsv::from_color(rgb);
                let h = (hsv.hue.into_positive_degrees()) / 360.0 * u16::MAX as f32;

                dbg!(hsv.value);
                ui2.brightness_slider.set_value(hsv.value as f64 * 255.0);

                let model = model.clone();
                glib::spawn_future_local(async move {
                    // Convert color from rgb to hsb

                    let state = model.state.lock().unwrap(); // TODO: fix this lock staying while the request is done. fr this time
                    let light = state.selected_light().unwrap(); // todo fix unwrap
                    model
                        .client
                        .set_light_color(
                            light,
                            Some(h as u16),
                            Some((hsv.value * 255.0) as u8),
                            Some((hsv.saturation * 255.0) as u8),
                        )
                        .await
                        .unwrap();
                });
            });
        }

        {
            let model = model.clone();
            let ui2 = ui.clone();
            ui.brightness_slider.connect_value_changed(move |s| {
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
}

struct SetupWindow {
    window: ApplicationWindow,
    ip_field: Entry,
    link_button: Button,
    error_msg: Label,
    demo_button: Button,
    on_login_completed: Box<dyn Fn(&SetupWindow, String, String)>,
    on_user_requested_demo: Box<dyn Fn(&SetupWindow)>
}

impl SetupWindow {
    fn new(
        app: &gtk::Application,
        on_login_completed: Box<dyn Fn(&SetupWindow, String, String)>,
        on_user_requested_demo: Box<dyn Fn(&SetupWindow)>,
    ) -> Self {
        let window = gtk::ApplicationWindow::new(app);

        window.set_title(Some("Setup"));
        window.set_default_size(500, 700);

        let layout = gtk::Box::new(Orientation::Vertical, 10);

        let label = Label::builder()
            .label("Please log in to your deconz server")
            .build();

        layout.append(&label);

        let ip_field = Entry::builder()
            .placeholder_text("Deconz Server address")
            .build();
        layout.append(&ip_field);

        let label = Label::builder()
            .label("please click the link button on your deconz server, then click the button here")
            .build();

        layout.append(&label);

        let label = Label::builder().label("The \"username\" which is used to authenticate users is saved in clear text").build();
        layout.append(&label);

        let link_button = Button::builder().label("Login").build();
        layout.append(&link_button);

        let error_msg = Label::builder().label("").build();
        layout.append(&error_msg);

        let demo_button = Button::builder().label("Start Demo").build();
        layout.append(&demo_button);

        window.set_child(Some(&layout));
        window.present();

        let w = Self {
            window,
            ip_field,
            link_button,
            on_login_completed,
            on_user_requested_demo,
            demo_button,
            error_msg
        };

        w
    }

    fn add_logic(self) {
        let s = Arc::new(self);
        let s_c = s.clone();
        s.clone().link_button.connect_clicked(move |_| {
            let s = &s_c;

            s.error_msg.set_text("");

            let s = s.clone();
            glib::spawn_future_local(async move {
                let ip = String::from(s.ip_field.text());
                
                let ip = if ip.contains("://"){
                    ip
                }else{
                    format!("http://{}", ip)
                };

                let client = DeconzClient::login_with_link_button(&ip).await; // TODO: Error handling

                match client {
                    Ok(client) => {
                        (&s.on_login_completed)(&*s, ip, client.username);
                    }
                    Err(e) => {
                        let msg = match &e{
                             deconz::Error::HttpError(e) => 
                            if let Some(status) = e.status(){
                                if status.as_u16() == 403{
                                    format!("Error: Authorization button was not pressed")
                                }else{
                                    format!("Error: {}", status.to_string())
                                }
                            }else{
                                e.to_string()
                            }
                            deconz::Error::ResponseParseError(e) => format!("Error: {}", e),
                            deconz::Error::IdParseError(e) => format!("Error: {}", e.to_string())
                        };
                        s.error_msg.set_text(&msg);
                        println!("{:#?}", e);
                    }
                }
            });
        });

        s.clone().demo_button.connect_clicked(move |_|{
            (&s.on_user_requested_demo)(&*s);
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let application = gtk::Application::builder()
        .application_id("de.zenonet.deconz")
        .build();

    fn main_window(app: &gtk::Application) {
        let ui = MainWindow::new(&app);

        let model = ViewModel::<DeconzClient>::init();
        ui.add_app_logic(model);
    }

    fn demo_window(app: &gtk::Application) {
        let ui = MainWindow::new(&app);

        let model = ViewModel::<DemoLightClient>::init();
        ui.add_app_logic(model);
    }   

    fn init(app: &gtk::Application) {
        // Load credentials here
        if let Some(config) = load_credentials() {
            unsafe {
                env::set_var("DECONZ_URL", config.url);
                env::set_var("DECONZ_TOKEN", config.username);
            };
            main_window(app);
        } else {
            // If no credentials are found
            let app_for_later = app.clone(); // this is reference counted (i think)
            let app_for_later_again = app.clone();
            let setup_window = SetupWindow::new(
                &app,
                Box::new(move |window, ip, token| {
                    println!("Got login data!");
                    unsafe {
                        env::set_var("DECONZ_URL", &ip);
                        env::set_var("DECONZ_TOKEN", &token);
                    };
                    store_credentials(ip, token);
                    window.window.close(); // This probably leaks the SetupWindow object but whatever
                    main_window(&app_for_later);
                }),
                Box::new(move |window|{
                    println!("Starting demo!");
                    window.window.close();
                    demo_window(&app_for_later_again);
                })
            );
            setup_window.add_logic();
        }
    }

    application.connect_activate(init);
    application.run();

    Ok(())
}
