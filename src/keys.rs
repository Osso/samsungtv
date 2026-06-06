/// Normalize user input into a KEY_* code: uppercase, prefix if missing.
pub fn normalize(input: &str) -> String {
    let upper = input.to_uppercase();
    if upper.starts_with("KEY_") {
        upper
    } else {
        format!("KEY_{upper}")
    }
}

/// Curated common key codes (the TV accepts many more; any KEY_* works).
pub const COMMON_KEYS: &[(&str, &str)] = &[
    ("KEY_POWER", "power toggle"),
    ("KEY_VOLUP", "volume up"),
    ("KEY_VOLDOWN", "volume down"),
    ("KEY_MUTE", "mute toggle"),
    ("KEY_HOME", "smart hub home"),
    ("KEY_MENU", "menu"),
    ("KEY_RETURN", "back"),
    ("KEY_ENTER", "select"),
    ("KEY_UP", "navigate up"),
    ("KEY_DOWN", "navigate down"),
    ("KEY_LEFT", "navigate left"),
    ("KEY_RIGHT", "navigate right"),
    ("KEY_CHUP", "channel up"),
    ("KEY_CHDOWN", "channel down"),
    ("KEY_SOURCE", "input source"),
    ("KEY_HDMI", "HDMI input"),
    ("KEY_0", "digit 0"),
    ("KEY_1", "digit 1"),
    ("KEY_2", "digit 2"),
    ("KEY_3", "digit 3"),
    ("KEY_4", "digit 4"),
    ("KEY_5", "digit 5"),
    ("KEY_6", "digit 6"),
    ("KEY_7", "digit 7"),
    ("KEY_8", "digit 8"),
    ("KEY_9", "digit 9"),
    ("KEY_PLAY", "play"),
    ("KEY_PAUSE", "pause"),
    ("KEY_STOP", "stop"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_prefix_and_uppercases() {
        assert_eq!(normalize("volup"), "KEY_VOLUP");
    }

    #[test]
    fn keeps_existing_prefix() {
        assert_eq!(normalize("KEY_POWER"), "KEY_POWER");
    }

    #[test]
    fn uppercases_prefixed_input() {
        assert_eq!(normalize("key_mute"), "KEY_MUTE");
    }

    #[test]
    fn digits_get_prefix() {
        assert_eq!(normalize("5"), "KEY_5");
    }
}
