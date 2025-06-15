// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, env, error::Error, ops::Deref, rc::Rc, sync::OnceLock};

use deconz::Light;
use gtk::{gio, glib, prelude::*};
use gtk4::{self as gtk, builders::ScrolledWindowBuilder, gio::ListStore, glib::GStr, Label, ScrolledWindow};
use tokio::runtime::Runtime;

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title(Some("Deconz Control"));
    window.set_default_size(350, 600);

    /* lights.iter().for_each(|app_info| {
        model.append(&GStr::from(app_info.name));
    }); */

    // the bind stage is used for "binding" the data to the created widgets on the
    // "setup" stage

    let list_box = gtk::ListBox::new();

    let l = Label::new(Some("Nee"));
    let l2 = Label::new(Some("Ndocj"));

    list_box.append(&l);
    list_box.append(&l2);

    let scrolled_window = ScrolledWindow::builder()
        .child(&list_box)
        .build();
    
    window.set_child(Some(&scrolled_window));
    window.present();

    glib::spawn_future_local(async move {
        let url = env::var("DECONZ_URL").expect("Missing DECONZ_URL in env vars");
        let token = env::var("DECONZ_TOKEN").expect("Missing DECONZ_TOKEN in env vars");
        let client = deconz::DeconzClient::login_with_token(url, token).unwrap();

        let lights = client.get_light_list().await.unwrap();

        for light in lights {
            let label = Label::new(Some(&light.name));
            list_box.append(&label);
        }
    });
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //let client = deconz::DeconzClient::login_with_link_button("http://192.168.1.239/").await.unwrap();
    //let client = deconz::DeconzClient::login_with_token(url, token).unwrap();

    //let lights = client.get_light_list().await.unwrap();
    //let lights = RefCell::from(lights);

    //println!("{:#?}", lights);

    let application = gtk::Application::builder()
        .application_id("de.zenonet.deconz")
        .build();
    application.connect_activate(build_ui);
    application.run();

    Ok(())
}
