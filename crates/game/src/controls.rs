//! Rebindable controls. Bindings are saved as plain strings ("KeyA",
//! "Mouse:Left") so old settings, and actions that get removed later, never
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
    Primary,
    Fire,
    Interact,
    Ability,
    Multitool,
    DropItem,
    NextItem,
    PrevItem,
    Hotbar1,
    Hotbar2,
    Hotbar3,
    Hotbar4,
    Hotbar5,
    Hotbar6,
    Hotbar7,
    Hotbar8,
    Hotbar9,
    Hotbar10,
    Inventory,
    Map,
    Codex,
    QuickStack,
    WaterOverlay,
    PowerOverlay,
    Pause,
}

impl Action {
    pub const ALL: [Action; 30] = [
        Action::MoveLeft,
        Action::MoveRight,
        Action::Jump,
        Action::Down,
        Action::Dash,
        Action::Primary,
        Action::Fire,
        Action::Interact,
        Action::Ability,
        Action::Multitool,
        Action::DropItem,
        Action::NextItem,
        Action::PrevItem,
        Action::Hotbar1,
        Action::Hotbar2,
        Action::Hotbar3,
        Action::Hotbar4,
        Action::Hotbar5,
        Action::Hotbar6,
        Action::Hotbar7,
        Action::Hotbar8,
        Action::Hotbar9,
        Action::Hotbar10,
        Action::Inventory,
        Action::Map,
        Action::Codex,
        Action::QuickStack,
        Action::WaterOverlay,
        Action::PowerOverlay,
        Action::Pause,
    ];

    /// Stable name used in the settings file.
    pub fn id(self) -> &'static str {
        match self {
            Action::MoveLeft => "move_left",
            Action::MoveRight => "move_right",
            Action::Jump => "jump",
            Action::Down => "down",
            Action::Dash => "dash",
            Action::Primary => "primary",
            Action::Fire => "fire",
            Action::Interact => "interact",
            Action::Ability => "ability",
            Action::Multitool => "multitool",
            Action::DropItem => "drop_item",
            Action::NextItem => "next_item",
            Action::PrevItem => "prev_item",
            Action::Hotbar1 => "hotbar_1",
            Action::Hotbar2 => "hotbar_2",
            Action::Hotbar3 => "hotbar_3",
            Action::Hotbar4 => "hotbar_4",
            Action::Hotbar5 => "hotbar_5",
            Action::Hotbar6 => "hotbar_6",
            Action::Hotbar7 => "hotbar_7",
            Action::Hotbar8 => "hotbar_8",
            Action::Hotbar9 => "hotbar_9",
            Action::Hotbar10 => "hotbar_10",
            Action::Inventory => "inventory",
            Action::Map => "map",
            Action::Codex => "codex",
            Action::QuickStack => "quick_stack",
            Action::WaterOverlay => "water_overlay",
            Action::PowerOverlay => "power_overlay",
            Action::Pause => "pause",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::MoveLeft => "Move left",
            Action::MoveRight => "Move right",
            Action::Jump => "Jump / swim up / fly",
            Action::Down => "Down / swim down",
            Action::Dash => "Dash (with Dash Pistons)",
            Action::Primary => "Dig / use the selected item",
            Action::Fire => "Fire the laser",
            Action::Interact => "Interact",
            Action::Ability => "Mod ability",
            Action::Multitool => "Put the item away (multitool)",
            Action::DropItem => "Drop the selected item",
            Action::NextItem => "Next hotbar slot",
            Action::PrevItem => "Previous hotbar slot",
            Action::Hotbar1 => "Hotbar slot 1",
            Action::Hotbar2 => "Hotbar slot 2",
            Action::Hotbar3 => "Hotbar slot 3",
            Action::Hotbar4 => "Hotbar slot 4",
            Action::Hotbar5 => "Hotbar slot 5",
            Action::Hotbar6 => "Hotbar slot 6",
            Action::Hotbar7 => "Hotbar slot 7",
            Action::Hotbar8 => "Hotbar slot 8",
            Action::Hotbar9 => "Hotbar slot 9",
            Action::Hotbar10 => "Hotbar slot 10",
            Action::Inventory => "Inventory",
            Action::Map => "Map",
            Action::Codex => "Codex",
            Action::QuickStack => "Quick stack to nearby chests",
            Action::WaterOverlay => "Water overlay",
            Action::PowerOverlay => "Power overlay",
            Action::Pause => "Pause menu",
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Action::MoveLeft => "Movement",
            Action::MoveRight => "Movement",
            Action::Jump => "Movement",
            Action::Down => "Movement",
            Action::Dash => "Movement",
            Action::Primary => "Tools",
            Action::Fire => "Tools",
            Action::Interact => "Tools",
            Action::Ability => "Tools",
            Action::Multitool => "Tools",
            Action::DropItem => "Tools",
            Action::NextItem => "Hotbar",
            Action::PrevItem => "Hotbar",
            Action::Hotbar1 => "Hotbar",
            Action::Hotbar2 => "Hotbar",
            Action::Hotbar3 => "Hotbar",
            Action::Hotbar4 => "Hotbar",
            Action::Hotbar5 => "Hotbar",
            Action::Hotbar6 => "Hotbar",
            Action::Hotbar7 => "Hotbar",
            Action::Hotbar8 => "Hotbar",
            Action::Hotbar9 => "Hotbar",
            Action::Hotbar10 => "Hotbar",
            Action::Inventory => "Interface",
            Action::Map => "Interface",
            Action::Codex => "Interface",
            Action::QuickStack => "Interface",
            Action::WaterOverlay => "Interface",
            Action::PowerOverlay => "Interface",
            Action::Pause => "Interface",
        }
    }

    /// The hotbar slot this action selects, if it is one of the ten.
    pub fn hotbar_slot(self) -> Option<u8> {
        let first = Action::Hotbar1 as u8;
        let i = self as u8;
        (first..first + 10).contains(&i).then(|| i - first)
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
            (Action::Primary, d(Some(Mouse(MouseButton::Left)), None)),
            (Action::Fire, d(Some(Mouse(MouseButton::Right)), None)),
            (Action::Interact, d(Some(Key(KeyE)), None)),
            (Action::Ability, d(Some(Key(KeyR)), None)),
            (Action::Multitool, d(Some(Key(KeyX)), Some(Key(Backquote)))),
            (Action::DropItem, d(Some(Key(KeyG)), None)),
            (Action::NextItem, d(Some(Key(BracketRight)), None)),
            (Action::PrevItem, d(Some(Key(BracketLeft)), None)),
            (Action::Hotbar1, d(Some(Key(Digit1)), None)),
            (Action::Hotbar2, d(Some(Key(Digit2)), None)),
            (Action::Hotbar3, d(Some(Key(Digit3)), None)),
            (Action::Hotbar4, d(Some(Key(Digit4)), None)),
            (Action::Hotbar5, d(Some(Key(Digit5)), None)),
            (Action::Hotbar6, d(Some(Key(Digit6)), None)),
            (Action::Hotbar7, d(Some(Key(Digit7)), None)),
            (Action::Hotbar8, d(Some(Key(Digit8)), None)),
            (Action::Hotbar9, d(Some(Key(Digit9)), None)),
            (Action::Hotbar10, d(Some(Key(Digit0)), None)),
            (Action::Inventory, d(Some(Key(Tab)), Some(Key(KeyI)))),
            (Action::Map, d(Some(Key(KeyM)), None)),
            (Action::Codex, d(Some(Key(KeyC)), None)),
            (Action::QuickStack, d(Some(Key(KeyQ)), None)),
            (Action::WaterOverlay, d(Some(Key(KeyV)), None)),
            (Action::PowerOverlay, d(Some(Key(KeyB)), None)),
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
    mut config: ResMut<crate::settings::Config>,
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
    match config.bindings.find(binding, target) {
        Some(other) => rebind.conflict = Some((binding, target, other)),
        None => {
            config.bindings.set(target.0, target.1, Some(binding));
            config.store();
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
        b.set(Action::Primary, 1, Some(Binding::Mouse(MouseButton::Middle)));
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
        let f = Binding::Key(KeyCode::KeyE);
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
