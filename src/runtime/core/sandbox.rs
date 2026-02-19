use serde::{Deserialize, Serialize};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SandboxFlags: u32 {
        const NONE = 0;
        /// allow-scripts: Enables JavaScript execution
        const ALLOW_SCRIPTS = 1 << 0;
        /// allow-same-origin: Allows the content to be treated as being from its real origin
        const ALLOW_SAME_ORIGIN = 1 << 1;
        /// allow-forms: Allows form submission
        const ALLOW_FORMS = 1 << 2;
        /// allow-popups: Allows window.open and similar
        const ALLOW_POPUPS = 1 << 3;
        /// allow-top-navigation: Allows the subframe to navigate the top-level browsing context
        const ALLOW_TOP_NAVIGATION = 1 << 4;
        /// allow-modals: Allows window.alert, etc.
        const ALLOW_MODALS = 1 << 5;
        /// allow-downloads: Allows triggering downloads
        const ALLOW_DOWNLOADS = 1 << 6;
    }
}

impl Default for SandboxFlags {
    fn default() -> Self {
        // If the sandbox attribute is present but empty, all flags are disabled (strict sandbox)
        // If the attribute is MISSING, all flags are effectively enabled (normalized to a specific bypass state or just not applied)
        Self::NONE
    }
}

impl SandboxFlags {
    pub fn from_attr(attr: &str) -> Self {
        let mut flags = Self::NONE;
        for part in attr.split_whitespace() {
            match part {
                "allow-scripts" => flags.insert(Self::ALLOW_SCRIPTS),
                "allow-same-origin" => flags.insert(Self::ALLOW_SAME_ORIGIN),
                "allow-forms" => flags.insert(Self::ALLOW_FORMS),
                "allow-popups" => flags.insert(Self::ALLOW_POPUPS),
                "allow-top-navigation" => flags.insert(Self::ALLOW_TOP_NAVIGATION),
                "allow-modals" => flags.insert(Self::ALLOW_MODALS),
                "allow-downloads" => flags.insert(Self::ALLOW_DOWNLOADS),
                _ => {} // Ignore unknown flags
            }
        }
        flags
    }
}
