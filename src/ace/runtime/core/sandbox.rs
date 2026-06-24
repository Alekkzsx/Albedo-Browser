#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SandboxFlags(pub u32);

impl Default for SandboxFlags {
    /// TODO: add docs
    fn default() -> Self {
        // If the sandbox attribute is present but empty, all flags are disabled (strict sandbox)
        // If the attribute is MISSING, all flags are effectively enabled (normalized to a specific bypass state or just not applied)
        Self::NONE
    }
}

impl SandboxFlags {
    pub const NONE: Self = Self(0);
    pub const ALLOW_SCRIPTS: Self = Self(1 << 0);
    pub const ALLOW_SAME_ORIGIN: Self = Self(1 << 1);
    pub const ALLOW_FORMS: Self = Self(1 << 2);
    pub const ALLOW_POPUPS: Self = Self(1 << 3);
    pub const ALLOW_TOP_NAVIGATION: Self = Self(1 << 4);
    pub const ALLOW_MODALS: Self = Self(1 << 5);
    pub const ALLOW_DOWNLOADS: Self = Self(1 << 6);

    /// TODO: add docs
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// TODO: add docs
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// TODO: add docs
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
