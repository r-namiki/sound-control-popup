use gtk4::gdk::Display;
use gtk4::{self as gtk, CssProvider};

pub fn install_css() {
    let provider = CssProvider::new();
    provider.load_from_data(
        r#"
        window {
            background: rgba(30, 30, 40, 0.96);
            border-color: rgba(255, 255, 255, 0.06);
            border-style: solid;
            border-width: 0 1px 1px 1px;
            border-radius: 0 0 20px 20px;
            color: #ffffff;
            font-family: "CommitMono Nerd Font", "Moralerspace Neon", "Noto Sans JP", sans-serif;
        }

        .audio-popup {
            min-width: 400px;
        }

        .popup-title {
            font-size: 21px;
            font-weight: 700;
            color: #ffffff;
        }

        .audio-card {
            padding: 14px;
            background: rgba(255, 255, 255, 0.035);
            border: 1px solid rgba(255, 255, 255, 0.03);
            border-radius: 16px;
        }

        .section-title {
            font-size: 15px;
            font-weight: 700;
            color: #ffffff;
        }

        .device-select {
            min-height: 38px;
            padding: 0 10px;
            border-radius: 12px;
            background: rgba(255, 255, 255, 0.045);
            color: #ffffff;
            border: 1px solid rgba(255, 255, 255, 0.045);
        }

        .device-select,
        .device-select * {
            color: #ffffff;
        }

        .device-select button {
            border: none;
            background: transparent;
            box-shadow: none;
        }

        .device-select.single-option,
        .device-select.single-option:disabled {
            opacity: 1;
        }

        .device-select popover {
            background: transparent;
            border-radius: 16px;
        }

        .device-select popover arrow,
        .device-select popover contents,
        .device-select popover box,
        .device-select popover viewport,
        .device-select popover scrolledwindow,
        .device-select popover listview,
        .device-select popover row,
        .device-select popover label,
        .device-select popover listitem,
        .device-select popover modelbutton,
        .device-select popover checkbutton,
        .device-select popover check {
            background: rgba(30, 30, 40, 0.98);
            color: #ffffff;
            border-radius: 14px;
        }

        .device-select popover row:hover,
        .device-select popover row:selected,
        .device-select popover listitem:selected,
        .device-select popover modelbutton:hover,
        .device-select popover modelbutton:checked,
        .device-select popover checkbutton:checked,
        .device-select popover check:checked {
            background: rgba(95, 126, 168, 0.22);
            color: #ffffff;
        }

        .device-select popover modelbutton {
            border-radius: 10px;
        }

        .device-select popover contents {
            padding: 4px;
            border: 1px solid rgba(255, 255, 255, 0.06);
            box-shadow: 0 14px 32px rgba(0, 0, 0, 0.24);
            border-radius: 16px;
        }

        .device-select popover scrolledwindow,
        .device-select popover viewport,
        .device-select popover listview {
            min-height: 0;
            padding: 0;
        }

        .device-select popover row,
        .device-select popover listitem,
        .device-select popover modelbutton,
        .device-select popover checkbutton {
            min-height: 0;
            padding-top: 2px;
            padding-bottom: 2px;
        }

        .level-label {
            color: #ffffff;
            font-size: 14px;
            font-weight: 700;
            min-width: 44px;
        }

        .level-scale trough {
            min-height: 10px;
            border-radius: 999px;
            background: rgba(255, 255, 255, 0.08);
        }

        .level-scale highlight {
            border-radius: 999px;
            background: #5f7ea8;
        }

        .level-scale slider {
            min-width: 18px;
            min-height: 18px;
            border-radius: 999px;
            background: #f5f7ff;
        }

        .status-badge {
            padding: 4px 10px;
            border-radius: 999px;
            font-size: 12px;
            font-weight: 700;
        }

        .status-live {
            background: rgba(111, 154, 148, 0.16);
            color: #9dc1bc;
        }

        .status-muted {
            background: rgba(176, 102, 102, 0.14);
            color: #d4a1a1;
        }

        .status-loading {
            background: rgba(95, 126, 168, 0.16);
            color: #a6bad4;
        }

        .status-unavailable {
            background: rgba(186, 148, 90, 0.18);
            color: #f0ca90;
        }

        .popup-button {
            min-height: 38px;
            padding: 0 12px;
            border-radius: 12px;
            background: rgba(255, 255, 255, 0.05);
            color: #ffffff;
            border: 1px solid rgba(255, 255, 255, 0.045);
        }

        .popup-button:hover {
            background: rgba(255, 255, 255, 0.09);
        }

        .popup-button.primary {
            background: #5f7ea8;
        }

        .popup-button.primary:hover {
            background: #6f90bc;
        }

        .popup-button.ghost {
            background: transparent;
        }

        .icon-button {
            min-width: 38px;
            min-height: 38px;
            padding: 0;
            border-radius: 12px;
            background: rgba(255, 255, 255, 0.05);
            color: #ffffff;
            border: 1px solid rgba(255, 255, 255, 0.045);
            font-size: 17px;
        }

        .icon-button:hover {
            background: rgba(255, 255, 255, 0.09);
        }

        "#,
    );

    if let Some(display) = Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
