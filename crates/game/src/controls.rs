//! Rebindable controls. Bindings are saved as plain strings ("KeyA",
//! "Mouse:Left") so old saves, and actions that get removed later, never
//! break loading: unknown entries are skipped and missing ones get defaults.

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    MoveLeft,
    MoveRight,
    Jump,
    Down,
    Dash,
    Dig,
    UseItem,
    NextItem,
    PrevItem,
    ToggleDigMode,
    LightOrb,
    ToggleStaffLight,
    Interact,
    Pause,
}

impl Action {
    pub const ALL: [Action; 14] = [
        Action::MoveLeft,
        Action::MoveRight,
        Action::Jump,
        Action::Down,
        Action::Dash,
        Action::Dig,
        Action::UseItem,
        Action::NextItem,
        Action::PrevItem,
        Action::ToggleDigMode,
        Action::LightOrb,
        Action::ToggleStaffLight,
        Action::Interact,
        Action::Pause,
    ];

    /// Stable name used in the save file.
    pub fn id(self) -> &'static str {
        match self {
            Action::MoveLeft => "move_left",
            Action::MoveRight => "move_right",
            Action::Jump => "jump",
            Action::Down => "down",
            Action::Dash => "dash",
            Action::Dig => "dig",
            Action::UseItem => "use_item",
            Action::NextItem => "next_item",
            Action::PrevItem => "prev_item",
            Action::ToggleDigMode => "toggle_dig_mode",
            Action::LightOrb => "light_orb",
            Action::ToggleStaffLight => "toggle_staff_light",
            Action::Interact => "interact",
            Action::Pause => "pause",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::MoveLeft => "Move left",
            Action::MoveRight => "Move right",
            Action::Jump => "Jump / climb",
            Action::Down => "Down / dig down",
            Action::Dash => "Dash",
            Action::Dig => "Dig spell",
            Action::UseItem => "Cast selected spell",
            Action::NextItem => "Next item",
            Action::PrevItem => "Previous item",
            Action::ToggleDigMode => "Toggle smart dig",
            Action::LightOrb => "Cast light orb",
            Action::ToggleStaffLight => "Dim / light staff",
            Action::Interact => "Interact",
            Action::Pause => "Pause menu",
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Action::MoveLeft | Action::MoveRight | Action::Jump | Action::Down | Action::Dash => "Movement",
            Action::Dig
            | Action::UseItem
            | Action::NextItem
            | Action::PrevItem
            | Action::ToggleDigMode
            | Action::LightOrb
            | Action::ToggleStaffLight => "Magic & digging",
            Action::Interact | Action::Pause => "Interface",
        }
    }

    fn from_id(id: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|a| a.id() == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
}

/// Keys that can be bound (and therefore saved and parsed by name).
const KEYS: &[KeyCode] = {
    use KeyCode::*;
    &[
        KeyA,
        KeyB,
        KeyC,
        KeyD,
        KeyE,
        KeyF,
        KeyG,
        KeyH,
        KeyI,
        KeyJ,
        KeyK,
        KeyL,
        KeyM,
        KeyN,
        KeyO,
        KeyP,
        KeyQ,
        KeyR,
        KeyS,
        KeyT,
        KeyU,
        KeyV,
        KeyW,
        KeyX,
        KeyY,
        KeyZ,
        Digit0,
        Digit1,
        Digit2,
        Digit3,
        Digit4,
        Digit5,
        Digit6,
        Digit7,
        Digit8,
        Digit9,
        F1,
        F2,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        F10,
        F11,
        F12,
        ArrowUp,
        ArrowDown,
        ArrowLeft,
        ArrowRight,
        Space,
        Tab,
        Enter,
        Escape,
        Backspace,
        ShiftLeft,
        ShiftRight,
        ControlLeft,
        ControlRight,
        AltLeft,
        AltRight,
        CapsLock,
        Backquote,
        Minus,
        Equal,
        BracketLeft,
        BracketRight,
        Backslash,
        Semicolon,
        Quote,
        Comma,
        Period,
        Slash,
        Insert,
        Delete,
        Home,
        End,
        PageUp,
        PageDown,
        Numpad0,
        Numpad1,
        Numpad2,
        Numpad3,
        Numpad4,
        Numpad5,
        Numpad6,
        Numpad7,
        Numpad8,
        Numpad9,
    ]
};

const MOUSE: &[(MouseButton, &str)] = &[
    (MouseButton::Left, "Left"),
    (MouseButton::Right, "Right"),
    (MouseButton::Middle, "Middle"),
    (MouseButton::Back, "Back"),
    (MouseButton::Forward, "Forward"),
];

impl Binding {
    pub fn is_bindable_key(key: KeyCode) -> bool {
        KEYS.contains(&key)
    }

    fn to_id(self) -> String {
        match self {
            Binding::Key(k) => format!("{k:?}"),
            Binding::Mouse(b) => {
                let name = MOUSE.iter().find(|(m, _)| *m == b).map_or("Left", |(_, n)| n);
                format!("Mouse:{name}")
            }
        }
    }

    fn from_id(s: &str) -> Option<Binding> {
        if let Some(name) = s.strip_prefix("Mouse:") {
            return MOUSE
                .iter()
                .find(|(_, n)| *n == name)
                .map(|(m, _)| Binding::Mouse(*m));
        }
        KEYS.iter()
            .find(|k| format!("{k:?}") == s)
            .map(|&k| Binding::Key(k))
    }

    /// Short name for the UI.
    pub fn label(self) -> String {
        match self {
            Binding::Mouse(MouseButton::Left) => "Left mouse".into(),
            Binding::Mouse(MouseButton::Right) => "Right mouse".into(),
            Binding::Mouse(MouseButton::Middle) => "Middle mouse".into(),
            Binding::Mouse(b) => format!("Mouse {b:?}"),
            Binding::Key(k) => {
                let raw = format!("{k:?}");
                let pretty = match k {
                    KeyCode::ArrowUp => "↑",
                    KeyCode::ArrowDown => "↓",
                    KeyCode::ArrowLeft => "←",
                    KeyCode::ArrowRight => "→",
                    KeyCode::ShiftLeft => "Left Shift",
                    KeyCode::ShiftRight => "Right Shift",
                    KeyCode::ControlLeft => "Left Ctrl",
                    KeyCode::ControlRight => "Right Ctrl",
                    KeyCode::AltLeft => "Left Alt",
                    KeyCode::AltRight => "Right Alt",
                    KeyCode::Escape => "Esc",
                    KeyCode::Backquote => "`",
                    _ => "",
                };
                if !pretty.is_empty() {
                    pretty.to_string()
                } else if let Some(c) = raw.strip_prefix("Key") {
                    c.to_string()
                } else if let Some(d) = raw.strip_prefix("Digit") {
                    d.to_string()
                } else {
                    raw
                }
            }
        }
    }

    pub fn pressed(self, keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
        match self {
            Binding::Key(k) => keys.pressed(k),
            Binding::Mouse(b) => mouse.pressed(b),
        }
    }

    pub fn just_pressed(self, keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
        match self {
            Binding::Key(k) => keys.just_pressed(k),
            Binding::Mouse(b) => mouse.just_pressed(b),
        }
    }
}

/// Bindings slots per action (primary and alternate).
pub const SLOTS: usize = 2;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    from = "BTreeMap<String, Vec<String>>",
    into = "BTreeMap<String, Vec<String>>"
)]
pub struct Bindings {
    map: BTreeMap<Action, [Option<Binding>; SLOTS]>,
}

impl Default for Bindings {
    fn default() -> Self {
        use Binding::{Key, Mouse};
        use KeyCode::*;
        let d = |a: Option<Binding>, b: Option<Binding>| [a, b];
        let map = [
            (Action::MoveLeft, d(Some(Key(KeyA)), Some(Key(ArrowLeft)))),
            (Action::MoveRight, d(Some(Key(KeyD)), Some(Key(ArrowRight)))),
            (Action::Jump, d(Some(Key(KeyW)), Some(Key(Space)))),
            (Action::Down, d(Some(Key(KeyS)), Some(Key(ArrowDown)))),
            (Action::Dash, d(Some(Key(ShiftLeft)), Some(Key(ShiftRight)))),
            (Action::Dig, d(Some(Mouse(MouseButton::Left)), None)),
            (Action::UseItem, d(Some(Mouse(MouseButton::Right)), None)),
            (Action::NextItem, d(Some(Key(KeyE)), None)),
            (Action::PrevItem, d(Some(Key(KeyQ)), None)),
            (Action::ToggleDigMode, d(Some(Key(Tab)), None)),
            (Action::LightOrb, d(Some(Key(KeyT)), None)),
            (Action::ToggleStaffLight, d(Some(Key(KeyL)), None)),
            (Action::Interact, d(Some(Key(KeyF)), None)),
            (Action::Pause, d(Some(Key(Escape)), None)),
        ]
        .into_iter()
        .collect();
        Bindings { map }
    }
}

impl From<BTreeMap<String, Vec<String>>> for Bindings {
    fn from(raw: BTreeMap<String, Vec<String>>) -> Self {
        let mut b = Bindings::default();
        for (id, list) in raw {
            let Some(action) = Action::from_id(&id) else {
                continue;
            };
            let mut slots = [None; SLOTS];
            for (slot, s) in slots.iter_mut().zip(list.iter()) {
                *slot = Binding::from_id(s);
            }
            b.map.insert(action, slots);
        }
        b
    }
}

impl From<Bindings> for BTreeMap<String, Vec<String>> {
    fn from(b: Bindings) -> Self {
        b.map
            .into_iter()
            .map(|(a, slots)| {
                (
                    a.id().to_string(),
                    slots.iter().flatten().map(|s| s.to_id()).collect(),
                )
            })
            .collect()
    }
}

impl Bindings {
    pub fn slots(&self, action: Action) -> [Option<Binding>; SLOTS] {
        self.map.get(&action).copied().unwrap_or([None; SLOTS])
    }

    pub fn pressed(
        &self,
        action: Action,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        self.slots(action)
            .iter()
            .flatten()
            .any(|b| b.pressed(keys, mouse))
    }

    pub fn just_pressed(
        &self,
        action: Action,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        self.slots(action)
            .iter()
            .flatten()
            .any(|b| b.just_pressed(keys, mouse))
    }

    /// Which action and slot (other than `except`) already uses `binding`.
    pub fn find(&self, binding: Binding, except: (Action, usize)) -> Option<(Action, usize)> {
        self.map.iter().find_map(|(&a, slots)| {
            slots
                .iter()
                .position(|s| *s == Some(binding))
                .filter(|&i| (a, i) != except)
                .map(|i| (a, i))
        })
    }

    pub fn set(&mut self, action: Action, slot: usize, binding: Option<Binding>) {
        self.map.entry(action).or_insert([None; SLOTS])[slot] = binding;
    }

    /// Binds `binding` to `target`, giving `target`'s old binding to whoever had it.
    pub fn swap(&mut self, target: (Action, usize), other: (Action, usize), binding: Binding) {
        let old = self.slots(target.0)[target.1];
        self.set(target.0, target.1, Some(binding));
        self.set(other.0, other.1, old);
    }

    /// The primary binding's label, for HUD hints.
    pub fn hint(&self, action: Action) -> String {
        self.slots(action)
            .iter()
            .flatten()
            .next()
            .map_or("unbound".into(), |b| b.label())
    }
}

/// An action and one of its binding slots.
pub type Slot = (Action, usize);
/// A key already used elsewhere: (binding, slot being set, slot that has it).
pub type Conflict = (Binding, Slot, Slot);

/// State of the Controls page while waiting for a key press.
#[derive(Resource, Default)]
pub struct Rebind {
    /// Waiting for input for this action and slot.
    pub waiting: Option<(Action, usize)>,
    /// A key that's already used elsewhere: (binding, target, other).
    pub conflict: Option<Conflict>,
    /// Frames to ignore input after starting (the click that opened it).
    arm: u8,
    /// Frames during which Esc shouldn't also toggle the pause menu.
    pub swallow_escape: u8,
}

impl Rebind {
    pub fn start(&mut self, action: Action, slot: usize) {
        self.waiting = Some((action, slot));
        self.conflict = None;
        self.arm = 2;
    }

    /// True while capturing or just after, so game input should ignore Esc.
    pub fn blocking(&self) -> bool {
        self.waiting.is_some() || self.swallow_escape > 0
    }
}

/// Captures the next key or mouse button for the Controls page.
pub fn capture_binding(
    mut rebind: ResMut<Rebind>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut save: ResMut<crate::run::save::SaveData>,
) {
    rebind.swallow_escape = rebind.swallow_escape.saturating_sub(1);
    let Some(target) = rebind.waiting else { return };
    if rebind.arm > 0 {
        rebind.arm -= 1;
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        rebind.waiting = None;
        rebind.swallow_escape = 2;
        return;
    }
    let pressed = keys
        .get_just_pressed()
        .copied()
        .find(|&k| Binding::is_bindable_key(k))
        .map(Binding::Key)
        .or_else(|| mouse.get_just_pressed().next().map(|&b| Binding::Mouse(b)));
    let Some(binding) = pressed else { return };
    rebind.waiting = None;
    match save.bindings.find(binding, target) {
        Some(other) => rebind.conflict = Some((binding, target, other)),
        None => {
            save.bindings.set(target.0, target.1, Some(binding));
            crate::run::save::store(&save);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_roundtrip_through_json() {
        let mut b = Bindings::default();
        b.set(Action::Dash, 0, Some(Binding::Key(KeyCode::KeyX)));
        b.set(Action::Dig, 1, Some(Binding::Mouse(MouseButton::Middle)));
        let json = serde_json::to_string(&b).unwrap();
        let back: Bindings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
    }

    #[test]
    fn unknown_and_missing_entries_fall_back_to_defaults() {
        let json = r#"{"dash":["KeyX"],"removed_action":["KeyZ"],"jump":["NotAKey","Space"]}"#;
        let b: Bindings = serde_json::from_str(json).unwrap();
        assert_eq!(b.slots(Action::Dash), [Some(Binding::Key(KeyCode::KeyX)), None]);
        assert_eq!(
            b.slots(Action::Jump)[0],
            None,
            "unparseable key is dropped, not fatal"
        );
        assert_eq!(
            b.slots(Action::Interact),
            Bindings::default().slots(Action::Interact)
        );
    }

    #[test]
    fn conflicts_are_found_and_swapped() {
        let mut b = Bindings::default();
        let f = Binding::Key(KeyCode::KeyF);
        let other = b.find(f, (Action::Dash, 0)).unwrap();
        assert_eq!(other, (Action::Interact, 0));
        b.swap((Action::Dash, 0), other, f);
        assert_eq!(b.slots(Action::Dash)[0], Some(f));
        assert_eq!(
            b.slots(Action::Interact)[0],
            Some(Binding::Key(KeyCode::ShiftLeft))
        );
    }
}
