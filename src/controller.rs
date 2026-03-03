use crate::audio::{
    AudioError, AudioInfo, AudioSnapshot, DeviceSnapshot, DeviceState, Target, load_audio_snapshot,
    load_device_snapshot, set_default_device, set_volume, toggle_mute,
};
use crate::ui::{AudioWidgets, PopupView, SectionWidgets};
use gtk4::glib;
use gtk4::prelude::*;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::Duration;

pub fn initialize(view: &PopupView) {
    wire_section_actions(&view.widgets, Target::Sink);
    wire_section_actions(&view.widgets, Target::Source);
    attach_auto_refresh(view.widgets.clone());
    refresh_devices_async(view.widgets.clone());
    refresh_ui_async(view.widgets.clone());
}

fn wire_section_actions(widgets: &Rc<AudioWidgets>, target: Target) {
    let section = match target {
        Target::Sink => widgets.sink.clone(),
        Target::Source => widgets.source.clone(),
    };

    connect_volume_slider(&section, widgets, target);
    connect_mute_button(&section, widgets, target);
    connect_device_select(&section, widgets, target);
}

fn connect_volume_slider(section: &SectionWidgets, widgets: &Rc<AudioWidgets>, target: Target) {
    let suppress_events = section.suppress_events.clone();
    let level_label = section.level_label.clone();
    let pending_volume_apply = section.pending_volume_apply.clone();
    let adjusting_volume = section.adjusting_volume.clone();
    let widgets = widgets.clone();

    section.level_scale.connect_value_changed(move |scale| {
        if suppress_events.load(Ordering::Relaxed) {
            return;
        }

        let value = scale.value();
        level_label.set_label(&format!("{}%", value.round() as u32));
        adjusting_volume.store(true, Ordering::Relaxed);

        let next = pending_volume_apply.get() + 1;
        pending_volume_apply.set(next);
        let pending_volume_apply = pending_volume_apply.clone();
        let adjusting_volume = adjusting_volume.clone();
        let widgets = widgets.clone();

        glib::timeout_add_local(Duration::from_millis(120), move || {
            if pending_volume_apply.get() != next {
                return glib::ControlFlow::Break;
            }

            run_command_async(
                widgets.clone(),
                false,
                Some(adjusting_volume.clone()),
                move || set_volume(target, value),
            );
            glib::ControlFlow::Break
        });
    });
}

fn connect_mute_button(section: &SectionWidgets, widgets: &Rc<AudioWidgets>, target: Target) {
    let widgets = widgets.clone();
    section.mute_button.connect_clicked(move |_| {
        run_command_async(widgets.clone(), false, None, move || toggle_mute(target));
    });
}

fn connect_device_select(section: &SectionWidgets, widgets: &Rc<AudioWidgets>, target: Target) {
    let widgets = widgets.clone();
    let suppress_device_events = section.suppress_device_events.clone();

    section.device_select.connect_changed(move |combo| {
        if suppress_device_events.load(Ordering::Relaxed) {
            return;
        }

        if let Some(device_id) = combo.active_id() {
            let device_id = device_id.to_string();
            run_command_async(widgets.clone(), true, None, move || {
                set_default_device(target, &device_id)
            });
        }
    });
}

fn attach_auto_refresh(widgets: Rc<AudioWidgets>) {
    glib::timeout_add_seconds_local(1, move || {
        refresh_ui_async(widgets.clone());
        glib::ControlFlow::Continue
    });
}

fn run_command_async<F>(
    widgets: Rc<AudioWidgets>,
    refresh_devices: bool,
    adjusting_volume: Option<Rc<AtomicBool>>,
    task: F,
) where
    F: FnOnce() -> Result<(), AudioError> + Send + 'static,
{
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(task());
    });

    glib::timeout_add_local(Duration::from_millis(10), move || {
        match receiver.try_recv() {
            Ok(result) => {
                if let Some(flag) = &adjusting_volume {
                    flag.store(false, Ordering::Relaxed);
                }
                if let Err(err) = result {
                    eprintln!("sound-control-popup: {err}");
                }
                if refresh_devices {
                    refresh_devices_async(widgets.clone());
                }
                refresh_ui_async(widgets.clone());
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                if let Some(flag) = &adjusting_volume {
                    flag.store(false, Ordering::Relaxed);
                }
                refresh_ui_async(widgets.clone());
                glib::ControlFlow::Break
            }
        }
    });
}

pub fn refresh_ui_async(widgets: Rc<AudioWidgets>) {
    if widgets.refreshing_info.swap(true, Ordering::Relaxed) {
        return;
    }

    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(load_audio_snapshot());
    });

    glib::timeout_add_local(Duration::from_millis(10), move || {
        match receiver.try_recv() {
            Ok(snapshot) => {
                apply_audio_snapshot(&widgets, snapshot);
                widgets.refreshing_info.store(false, Ordering::Relaxed);
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                widgets.refreshing_info.store(false, Ordering::Relaxed);
                glib::ControlFlow::Break
            }
        }
    });
}

pub fn refresh_devices_async(widgets: Rc<AudioWidgets>) {
    if widgets.refreshing_devices.swap(true, Ordering::Relaxed) {
        return;
    }

    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(load_device_snapshot());
    });

    glib::timeout_add_local(Duration::from_millis(10), move || {
        match receiver.try_recv() {
            Ok(snapshot) => {
                apply_device_snapshot(&widgets, snapshot);
                widgets.refreshing_devices.store(false, Ordering::Relaxed);
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                widgets.refreshing_devices.store(false, Ordering::Relaxed);
                glib::ControlFlow::Break
            }
        }
    });
}

fn apply_audio_snapshot(widgets: &AudioWidgets, snapshot: AudioSnapshot) {
    apply_audio_result(&widgets.sink, snapshot.sink);
    apply_audio_result(&widgets.source, snapshot.source);
}

fn apply_device_snapshot(widgets: &AudioWidgets, snapshot: DeviceSnapshot) {
    apply_device_result(&widgets.sink, snapshot.sink);
    apply_device_result(&widgets.source, snapshot.source);
}

fn apply_audio_result(section: &SectionWidgets, result: Result<AudioInfo, AudioError>) {
    match result {
        Ok(info) => update_audio_section(section, info),
        Err(err) => show_audio_unavailable(section, &err),
    }
}

fn apply_device_result(section: &SectionWidgets, result: Result<DeviceState, AudioError>) {
    match result {
        Ok(state) => update_device_select(section, &state),
        Err(err) => show_device_unavailable(section, &err),
    }
}

fn update_audio_section(section: &SectionWidgets, info: AudioInfo) {
    if !section.adjusting_volume.load(Ordering::Relaxed) {
        section
            .level_label
            .set_label(&format!("{}%", info.volume_percent));
        section.suppress_events.store(true, Ordering::Relaxed);
        section
            .level_scale
            .set_value(info.volume_percent.min(150) as f64);
        section.suppress_events.store(false, Ordering::Relaxed);
    }

    section.level_scale.set_sensitive(true);
    section.level_scale.set_tooltip_text(None);
    section.mute_button.set_sensitive(true);
    section.mute_button.set_tooltip_text(None);
    section
        .mute_button
        .set_label(info.button_icon(section.target));

    reset_status_badge(section);

    if info.muted {
        section.badge.set_label(info.status_label());
        section.badge.add_css_class("status-muted");
    } else {
        section.badge.set_label(info.status_label());
        section.badge.add_css_class("status-live");
    }
}

fn show_audio_unavailable(section: &SectionWidgets, err: &AudioError) {
    section.level_label.set_label("--");
    section.level_scale.set_sensitive(false);
    section.level_scale.set_tooltip_text(Some(&err.to_string()));
    section.mute_button.set_sensitive(false);
    section.mute_button.set_label(section.target.icon());
    section.mute_button.set_tooltip_text(Some(&err.to_string()));
    reset_status_badge(section);
    section.badge.set_label("Unavailable");
    section.badge.add_css_class("status-unavailable");
    section.badge.set_tooltip_text(Some(&err.to_string()));
}

fn update_device_select(section: &SectionWidgets, state: &DeviceState) {
    section
        .suppress_device_events
        .store(true, Ordering::Relaxed);
    section.device_select.remove_all();
    for entry in &state.entries {
        section.device_select.append(Some(&entry.id), &entry.label);
    }
    let _ = section.device_select.set_active_id(Some(&state.current_id));
    let has_multiple_choices = state.entries.len() > 1;
    section.device_select.set_sensitive(has_multiple_choices);
    section.device_select.set_tooltip_text(None);
    if has_multiple_choices {
        section.device_select.remove_css_class("single-option");
    } else {
        section.device_select.add_css_class("single-option");
    }
    section
        .suppress_device_events
        .store(false, Ordering::Relaxed);
}

fn show_device_unavailable(section: &SectionWidgets, err: &AudioError) {
    section
        .suppress_device_events
        .store(true, Ordering::Relaxed);
    section.device_select.remove_all();
    section.device_select.append_text("Unavailable");
    section.device_select.set_active(Some(0));
    section.device_select.set_sensitive(false);
    section.device_select.add_css_class("single-option");
    section
        .device_select
        .set_tooltip_text(Some(&err.to_string()));
    section
        .suppress_device_events
        .store(false, Ordering::Relaxed);
}

fn reset_status_badge(section: &SectionWidgets) {
    section.badge.remove_css_class("status-live");
    section.badge.remove_css_class("status-muted");
    section.badge.remove_css_class("status-loading");
    section.badge.remove_css_class("status-unavailable");
    section.badge.set_tooltip_text(None);
}
