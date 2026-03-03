use crate::audio::Target;
use crate::style;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, ComboBoxText, Label, Orientation,
    Revealer, RevealerTransitionType, Scale, gdk, glib,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::Cell;
use std::process::Command;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;

const LAYER_SHELL_NAMESPACE: &str = "io.github.rikunamiki.audio-popup";

#[derive(Clone)]
pub struct SectionWidgets {
    pub target: Target,
    pub card: GtkBox,
    pub badge: Label,
    pub device_select: ComboBoxText,
    pub level_label: Label,
    pub level_scale: Scale,
    pub mute_button: Button,
    pub suppress_events: Rc<AtomicBool>,
    pub suppress_device_events: Rc<AtomicBool>,
    pub pending_volume_apply: Rc<Cell<u64>>,
    pub adjusting_volume: Rc<AtomicBool>,
}

#[derive(Clone)]
pub struct AudioWidgets {
    pub sink: SectionWidgets,
    pub source: SectionWidgets,
    pub refreshing_info: Rc<AtomicBool>,
    pub refreshing_devices: Rc<AtomicBool>,
}

pub struct PopupView {
    pub revealer: Revealer,
    pub window: ApplicationWindow,
    pub widgets: Rc<AudioWidgets>,
}

pub fn build(app: &Application) -> PopupView {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Audio Popup")
        .default_width(440)
        .default_height(330)
        .decorated(false)
        .resizable(false)
        .build();

    window.init_layer_shell();
    window.set_namespace(LAYER_SHELL_NAMESPACE);
    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Right, true);
    window.set_margin(Edge::Top, 0);
    window.set_margin(Edge::Right, 12);
    window.set_keyboard_mode(KeyboardMode::OnDemand);

    style::install_css();

    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .css_classes(["audio-popup"])
        .build();

    let title_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .valign(Align::Center)
        .build();

    let title = Label::builder()
        .label("Sound Control")
        .halign(Align::Start)
        .valign(Align::Center)
        .hexpand(true)
        .css_classes(["popup-title"])
        .build();

    title_row.append(&title);
    root.append(&title_row);

    let widgets = Rc::new(AudioWidgets {
        sink: build_section(Target::Sink),
        source: build_section(Target::Source),
        refreshing_info: Rc::new(AtomicBool::new(false)),
        refreshing_devices: Rc::new(AtomicBool::new(false)),
    });

    root.append(&widgets.sink.card);
    root.append(&widgets.source.card);
    root.append(&build_footer());

    let revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideDown)
        .transition_duration(180)
        .reveal_child(false)
        .build();
    revealer.set_child(Some(&root));

    window.set_child(Some(&revealer));
    attach_escape_to_close(&window);
    attach_focus_loss_to_close(&window, widgets.clone());

    PopupView {
        revealer,
        window,
        widgets,
    }
}

fn build_section(target: Target) -> SectionWidgets {
    let card = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .css_classes(["audio-card"])
        .build();

    let header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();

    let title = Label::builder()
        .label(format!("{}  {}", target.icon(), target.title()))
        .halign(Align::Start)
        .hexpand(true)
        .css_classes(["section-title"])
        .build();

    let badge = Label::builder()
        .label("Loading")
        .css_classes(["status-badge", "status-loading"])
        .build();

    header.append(&title);
    header.append(&badge);
    card.append(&header);

    let device_select = ComboBoxText::builder()
        .hexpand(true)
        .css_classes(["device-select"])
        .build();
    device_select.set_sensitive(false);
    card.append(&device_select);

    let level_label = Label::builder()
        .label("0%")
        .css_classes(["level-label"])
        .build();

    let level_scale = Scale::with_range(Orientation::Horizontal, 0.0, 150.0, 1.0);
    level_scale.set_hexpand(true);
    level_scale.set_draw_value(false);
    level_scale.add_css_class("level-scale");
    level_scale.set_sensitive(false);

    let mute_button = Button::builder()
        .label(target.icon())
        .css_classes(["icon-button"])
        .build();
    mute_button.set_sensitive(false);

    let level_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .valign(Align::Center)
        .build();
    level_row.append(&level_scale);
    level_row.append(&level_label);
    level_row.append(&mute_button);
    card.append(&level_row);

    SectionWidgets {
        target,
        card,
        badge,
        device_select,
        level_label,
        level_scale,
        mute_button,
        suppress_events: Rc::new(AtomicBool::new(false)),
        suppress_device_events: Rc::new(AtomicBool::new(false)),
        pending_volume_apply: Rc::new(Cell::new(0)),
        adjusting_volume: Rc::new(AtomicBool::new(false)),
    }
}

fn build_footer() -> GtkBox {
    let footer = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(0)
        .halign(Align::End)
        .margin_top(4)
        .build();

    let pavu_button = Button::builder()
        .label("Open pavucontrol")
        .css_classes(["popup-button", "primary"])
        .build();
    pavu_button.connect_clicked(|_| {
        if let Err(err) = Command::new("pavucontrol").spawn() {
            eprintln!("audio-popup: failed to launch pavucontrol: {err}");
        }
    });

    footer.append(&pavu_button);
    footer
}

fn attach_escape_to_close(window: &ApplicationWindow) {
    let controller = gtk4::EventControllerKey::new();
    let window_clone = window.clone();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            window_clone.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(controller);
}

fn attach_focus_loss_to_close(window: &ApplicationWindow, widgets: Rc<AudioWidgets>) {
    let window_clone = window.clone();
    window.connect_is_active_notify(move |window| {
        if !window.is_active() {
            let window_clone = window_clone.clone();
            let widgets = widgets.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(40), move || {
                let popup_open = widgets.sink.device_select.is_popup_shown()
                    || widgets.source.device_select.is_popup_shown();
                if !window_clone.is_active() && !popup_open {
                    window_clone.close();
                }
            });
        }
    });
}
