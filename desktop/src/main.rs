// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, env, error::Error, ops::Deref, rc::Rc, sync::OnceLock};

use deconz::Light;
use gtk::{gio, glib, prelude::*};
use gtk4::{
    self as gtk, Button, Label, ListBoxRow, Orientation, ScrolledWindow, Stack,
    builders::ScrolledWindowBuilder, gio::ListStore, glib::GStr,
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

    /* lights.iter().for_each(|app_info| {
        model.append(&GStr::from(app_info.name));
    }); */

    // the bind stage is used for "binding" the data to the created widgets on the
    // "setup" stage

    let list_box = gtk::ListBox::new();


    let scrolled_window = ScrolledWindow::builder().child(&list_box).build();

    let controller = gtk::Box::new(Orientation::Vertical, 10);
    let light_name_label = Label::new(Some("No lamp selected"));

    controller.append(&light_name_label);
    let l = Label::new(Some("Toggle lamp"));
    let but = Button::builder()
        .child(&l)
        .tooltip_text("Toggles the on/off state of the lamp")
        .build();
    controller.append(&but);

    let layout = gtk::Box::new(gtk4::Orientation::Horizontal, 0);
    layout.set_homogeneous(true);

    layout.append(&scrolled_window);
    layout.append(&controller);

    window.set_child(Some(&layout));

    list_box.connect_row_selected(move |list_box, row| {
        if let Some(row) = row {
            let label = row.child().unwrap().downcast::<Label>().unwrap();
            println!("Row {} was selected", label.text());
            light_name_label.set_text(&label.text());
        }
    });

    window.present();

    glib::spawn_future_local(async move {
        let url = env::var("DECONZ_URL").expect("Missing DECONZ_URL in env vars");
        let token = env::var("DECONZ_TOKEN").expect("Missing DECONZ_TOKEN in env vars");
        let client = deconz::DeconzClient::login_with_token(url, token).unwrap();

        let lights = client.get_light_list().await.unwrap();

        for light in lights {
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
