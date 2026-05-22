use std::collections::HashMap;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use replay_core::config::HotkeyConfig;
use tracing::info;

use crate::control_api::ControlApi;

pub fn spawn_hotkey_handler(
    api: ControlApi,
    hotkeys: HotkeyConfig,
    runtime: tokio::runtime::Handle,
) {
    std::thread::spawn(move || {
        if let Err(e) = run_hotkeys(api, hotkeys, runtime) {
            tracing::warn!(error = %e, "Hotkey handler failed");
        }
    });
}

fn run_hotkeys(
    api: ControlApi,
    hotkeys: HotkeyConfig,
    rt: tokio::runtime::Handle,
) -> anyhow::Result<()> {
    let manager = GlobalHotKeyManager::new()?;

    let mut id_to_action: HashMap<u32, HotkeyAction> = HashMap::new();

    let bindings = [
        (hotkeys.mark.as_str(), HotkeyAction::Mark),
        (hotkeys.replay.as_str(), HotkeyAction::Replay),
        (hotkeys.replay_last.as_str(), HotkeyAction::ReplayLast),
        (hotkeys.return_live.as_str(), HotkeyAction::ReturnLive),
        (hotkeys.clear_mark.as_str(), HotkeyAction::ClearMark),
    ];

    for (key_spec, action) in bindings {
        if key_spec.is_empty() {
            continue;
        }
        if let Some(hotkey) = parse_hotkey_spec(key_spec) {
            manager.register(hotkey)?;
            id_to_action.insert(hotkey.id(), action);
            info!(action = ?action, key = %key_spec, "Registered hotkey");
        } else {
            tracing::warn!(key = %key_spec, "Unrecognized hotkey — not registered");
        }
    }

    if id_to_action.is_empty() {
        tracing::warn!("No hotkeys registered");
    }

    let event_rx = GlobalHotKeyEvent::receiver();
    loop {
        if let Ok(event) = event_rx.recv() {
            if event.state != HotKeyState::Pressed {
                continue;
            }
            let Some(action) = id_to_action.get(&event.id).copied() else {
                continue;
            };
            let a = api.clone();
            rt.spawn(async move {
                match action {
                    HotkeyAction::Mark => {
                        let _ = a.mark().await;
                    }
                    HotkeyAction::Replay => {
                        let _ = a.replay().await;
                    }
                    HotkeyAction::ReplayLast => {
                        let _ = a.replay_last(0).await;
                    }
                    HotkeyAction::ReturnLive => {
                        let _ = a.return_live().await;
                    }
                    HotkeyAction::ClearMark => {
                        let _ = a.clear_mark().await;
                    }
                }
            });
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum HotkeyAction {
    Mark,
    Replay,
    ReplayLast,
    ReturnLive,
    ClearMark,
}

/// Parse config strings like `M`, `Space`, `F5`, `Ctrl+R`, `Cmd+Shift+M`.
fn parse_hotkey_spec(spec: &str) -> Option<HotKey> {
    let parts: Vec<&str> = spec.split('+').map(str::trim).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut mods = Modifiers::empty();
    let mut key_token: Option<&str> = None;
    for part in &parts {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" | "option" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "super" | "cmd" | "meta" | "command" => mods |= Modifiers::SUPER,
            _ => key_token = Some(part),
        }
    }
    let key = key_token.or_else(|| parts.last().copied())?;
    let code = key_name_to_code(key)?;
    Some(if mods.is_empty() {
        HotKey::new(None, code)
    } else {
        HotKey::new(Some(mods), code)
    })
}

fn key_name_to_code(name: &str) -> Option<Code> {
    let upper = name.to_uppercase();
    match upper.as_str() {
        "M" => Some(Code::KeyM),
        "R" => Some(Code::KeyR),
        "L" => Some(Code::KeyL),
        "C" => Some(Code::KeyC),
        "SPACE" => Some(Code::Space),
        "ESCAPE" | "ESC" => Some(Code::Escape),
        "ENTER" | "RETURN" => Some(Code::Enter),
        "TAB" => Some(Code::Tab),
        "BACKSPACE" => Some(Code::Backspace),
        "DELETE" | "DEL" => Some(Code::Delete),
        "UP" => Some(Code::ArrowUp),
        "DOWN" => Some(Code::ArrowDown),
        "LEFT" => Some(Code::ArrowLeft),
        "RIGHT" => Some(Code::ArrowRight),
        "F1" => Some(Code::F1),
        "F2" => Some(Code::F2),
        "F3" => Some(Code::F3),
        "F4" => Some(Code::F4),
        "F5" => Some(Code::F5),
        "F6" => Some(Code::F6),
        "F7" => Some(Code::F7),
        "F8" => Some(Code::F8),
        "F9" => Some(Code::F9),
        "F10" => Some(Code::F10),
        "F11" => Some(Code::F11),
        "F12" => Some(Code::F12),
        s if s.len() == 1 => letter_or_digit_code(s.chars().next()?),
        _ => None,
    }
}

fn letter_or_digit_code(c: char) -> Option<Code> {
    match c {
        'A' => Some(Code::KeyA),
        'B' => Some(Code::KeyB),
        'D' => Some(Code::KeyD),
        'E' => Some(Code::KeyE),
        'F' => Some(Code::KeyF),
        'G' => Some(Code::KeyG),
        'H' => Some(Code::KeyH),
        'I' => Some(Code::KeyI),
        'J' => Some(Code::KeyJ),
        'K' => Some(Code::KeyK),
        'N' => Some(Code::KeyN),
        'O' => Some(Code::KeyO),
        'P' => Some(Code::KeyP),
        'Q' => Some(Code::KeyQ),
        'S' => Some(Code::KeyS),
        'T' => Some(Code::KeyT),
        'U' => Some(Code::KeyU),
        'V' => Some(Code::KeyV),
        'W' => Some(Code::KeyW),
        'X' => Some(Code::KeyX),
        'Y' => Some(Code::KeyY),
        'Z' => Some(Code::KeyZ),
        '0' => Some(Code::Digit0),
        '1' => Some(Code::Digit1),
        '2' => Some(Code::Digit2),
        '3' => Some(Code::Digit3),
        '4' => Some(Code::Digit4),
        '5' => Some(Code::Digit5),
        '6' => Some(Code::Digit6),
        '7' => Some(Code::Digit7),
        '8' => Some(Code::Digit8),
        '9' => Some(Code::Digit9),
        _ => None,
    }
}
