use std::fmt;
use std::process::Command;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    Sink,
    Source,
}

impl Target {
    pub fn object_name(self) -> &'static str {
        match self {
            Self::Sink => "@DEFAULT_AUDIO_SINK@",
            Self::Source => "@DEFAULT_AUDIO_SOURCE@",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Sink => "Output",
            Self::Source => "Input",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Sink => "󰕾",
            Self::Source => "󰍬",
        }
    }

    fn default_device_args(self) -> [&'static str; 1] {
        match self {
            Self::Sink => ["get-default-sink"],
            Self::Source => ["get-default-source"],
        }
    }

    fn list_device_args(self) -> [&'static str; 2] {
        match self {
            Self::Sink => ["list", "sinks"],
            Self::Source => ["list", "sources"],
        }
    }

    fn status_icon(self, muted: bool) -> &'static str {
        match (self, muted) {
            (Self::Sink, true) => "󰖁",
            (Self::Sink, false) => "󰕾",
            (Self::Source, true) => "󰍭",
            (Self::Source, false) => "󰍬",
        }
    }

    fn mute_command(self) -> [&'static str; 3] {
        ["set-mute", self.object_name(), "toggle"]
    }
}

#[derive(Clone, Debug)]
pub struct AudioInfo {
    pub volume_percent: u32,
    pub muted: bool,
}

impl AudioInfo {
    pub fn status_label(&self) -> &'static str {
        if self.muted { "Muted" } else { "Live" }
    }

    pub fn button_icon(&self, target: Target) -> &'static str {
        target.status_icon(self.muted)
    }
}

#[derive(Clone, Debug)]
pub struct DeviceEntry {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct AudioSnapshot {
    pub sink: AudioResult<AudioInfo>,
    pub source: AudioResult<AudioInfo>,
}

#[derive(Clone, Debug)]
pub struct DeviceState {
    pub current_id: String,
    pub entries: Vec<DeviceEntry>,
}

#[derive(Clone, Debug)]
pub struct DeviceSnapshot {
    pub sink: AudioResult<DeviceState>,
    pub source: AudioResult<DeviceState>,
}

#[derive(Clone, Debug)]
pub struct AudioError {
    action: String,
    details: String,
}

impl AudioError {
    fn new(action: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            details: details.into(),
        }
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.action, self.details)
    }
}

type AudioResult<T> = Result<T, AudioError>;

pub fn load_audio_snapshot() -> AudioSnapshot {
    AudioSnapshot {
        sink: read_audio_info(Target::Sink),
        source: read_audio_info(Target::Source),
    }
}

pub fn load_device_snapshot() -> DeviceSnapshot {
    DeviceSnapshot {
        sink: read_device_state(Target::Sink),
        source: read_device_state(Target::Source),
    }
}

pub fn read_audio_info(target: Target) -> AudioResult<AudioInfo> {
    let raw = run_capture("wpctl", &["get-volume", target.object_name()])?;
    let volume_percent = parse_volume_percent(&raw).ok_or_else(|| {
        AudioError::new(
            format!("parse {} volume", target.title().to_lowercase()),
            format!("unexpected output: {}", raw.trim()),
        )
    })?;
    let muted = raw.contains("[MUTED]");

    Ok(AudioInfo {
        volume_percent,
        muted,
    })
}

pub fn set_default_device(target: Target, device_id: &str) -> AudioResult<()> {
    match target {
        Target::Sink => run_status("pactl", &["set-default-sink", device_id]),
        Target::Source => run_status("pactl", &["set-default-source", device_id]),
    }
}

pub fn set_volume(target: Target, value: f64) -> AudioResult<()> {
    let percent = value.clamp(0.0, 150.0);
    let volume = format!("{:.2}", percent / 100.0);
    run_status(
        "wpctl",
        &["set-volume", "-l", "1.5", target.object_name(), &volume],
    )
}

pub fn toggle_mute(target: Target) -> AudioResult<()> {
    run_status("wpctl", &target.mute_command())
}

fn read_device_state(target: Target) -> AudioResult<DeviceState> {
    Ok(DeviceState {
        current_id: get_default_device_id(target)?,
        entries: list_devices(target)?,
    })
}

fn get_default_device_id(target: Target) -> AudioResult<String> {
    let device_id = run_capture("pactl", &target.default_device_args())?;
    let device_id = device_id.trim();
    if device_id.is_empty() {
        return Err(AudioError::new(
            format!("read default {} device", target.title().to_lowercase()),
            "pactl returned an empty device id",
        ));
    }
    Ok(device_id.to_string())
}

fn list_devices(target: Target) -> AudioResult<Vec<DeviceEntry>> {
    let raw = run_capture("pactl", &target.list_device_args())?;
    Ok(parse_device_entries(&raw, target))
}

fn parse_device_entries(raw: &str, target: Target) -> Vec<DeviceEntry> {
    let header = match target {
        Target::Sink => "Sink #",
        Target::Source => "Source #",
    };

    let mut entries = Vec::new();
    let mut current_id = String::new();
    let mut current_label = String::new();

    let push_current = |entries: &mut Vec<DeviceEntry>, id: &mut String, label: &mut String| {
        if id.is_empty() {
            return;
        }
        if matches!(target, Target::Source) && id.ends_with(".monitor") {
            id.clear();
            label.clear();
            return;
        }

        let label_value = if label.is_empty() {
            id.clone()
        } else {
            label.clone()
        };

        entries.push(DeviceEntry {
            id: id.clone(),
            label: label_value,
        });
        id.clear();
        label.clear();
    };

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(header) {
            push_current(&mut entries, &mut current_id, &mut current_label);
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("Name: ") {
            current_id = value.trim().to_string();
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("Description: ") {
            current_label = value.trim().to_string();
        }
    }

    push_current(&mut entries, &mut current_id, &mut current_label);
    entries
}

fn parse_volume_percent(raw: &str) -> Option<u32> {
    let token = raw
        .split_whitespace()
        .find(|part| part.chars().any(|ch| ch.is_ascii_digit()))?;
    let value = token.parse::<f32>().ok()?;
    Some((value * 100.0).round() as u32)
}

fn run_status(cmd: &str, args: &[&str]) -> AudioResult<()> {
    let output = run_output(cmd, args)?;
    ensure_success(cmd, args, output)?;
    Ok(())
}

fn run_capture(cmd: &str, args: &[&str]) -> AudioResult<String> {
    let output = run_output(cmd, args)?;
    let output = ensure_success(cmd, args, output)?;
    String::from_utf8(output.stdout).map_err(|err| {
        AudioError::new(
            format!("decode stdout from {}", render_command(cmd, args)),
            err.to_string(),
        )
    })
}

fn run_output(cmd: &str, args: &[&str]) -> AudioResult<std::process::Output> {
    Command::new(cmd).args(args).output().map_err(|err| {
        AudioError::new(
            format!("spawn {}", render_command(cmd, args)),
            err.to_string(),
        )
    })
}

fn ensure_success(
    cmd: &str,
    args: &[&str],
    output: std::process::Output,
) -> AudioResult<std::process::Output> {
    if !output.status.success() {
        let status = output.status.code().map_or_else(
            || "terminated by signal".to_string(),
            |code| format!("exit status {code}"),
        );
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let details = if stderr.is_empty() {
            status
        } else {
            format!("{status}: {stderr}")
        };
        return Err(AudioError::new(render_command(cmd, args), details));
    }
    Ok(output)
}

fn render_command(cmd: &str, args: &[&str]) -> String {
    if args.is_empty() {
        cmd.to_string()
    } else {
        format!("{cmd} {}", args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::{Target, parse_device_entries, parse_volume_percent};

    #[test]
    fn parses_wpctl_volume_output() {
        assert_eq!(parse_volume_percent("Volume: 0.37 [MUTED]\n"), Some(37));
    }

    #[test]
    fn device_entries_fall_back_to_name_when_description_is_missing() {
        let raw = "\
Sink #52
\tName: alsa_output.usb-DAC-00.analog-stereo
";

        let entries = parse_device_entries(raw, Target::Sink);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "alsa_output.usb-DAC-00.analog-stereo");
        assert_eq!(entries[0].label, "alsa_output.usb-DAC-00.analog-stereo");
    }

    #[test]
    fn source_entries_skip_monitor_devices() {
        let raw = "\
Source #31
\tName: alsa_output.pci-0000_00_1f.3.analog-stereo.monitor
\tDescription: Monitor of Built-in Audio Analog Stereo
Source #32
\tName: alsa_input.usb-Logitech_USB_Microphone-00.mono-fallback
\tDescription: Logitech USB Microphone Mono
";

        let entries = parse_device_entries(raw, Target::Source);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].id,
            "alsa_input.usb-Logitech_USB_Microphone-00.mono-fallback"
        );
        assert_eq!(entries[0].label, "Logitech USB Microphone Mono");
    }
}
