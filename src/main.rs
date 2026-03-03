mod audio;
mod controller;
mod style;
mod ui;

use std::cell::Cell;
use std::rc::Rc;

use gtk4::Application;
use gtk4::glib;
use gtk4::prelude::{ApplicationExt, ApplicationExtManual, GtkWindowExt, WidgetExt};

const APP_ID: &str = "io.github.rikunamiki.sound_control_popup";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        let view = ui::build(app);
        controller::initialize(&view);
        view.window.set_opacity(0.18);
        view.window.present();

        let revealer = view.revealer.clone();
        glib::idle_add_local_once(move || {
            revealer.set_reveal_child(true);
        });

        let window = view.window.clone();
        let step = Rc::new(Cell::new(0u8));
        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let next = step.get() + 1;
            step.set(next);

            let progress = f64::from(next) / 10.0;
            let opacity = 0.18 + (0.82 * progress.min(1.0));
            window.set_opacity(opacity);

            if next >= 10 {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    });

    app.run()
}
