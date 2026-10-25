//! Icon packs (section 8.2). A page and a grammar name an icon by its file stem; the stem
//! means an icon only in the pack of the item's kind, so the same stem in two packs is two
//! icons and a page never names a pack.

use std::borrow::Cow;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod aws;
mod gcp;

pub use aws::AwsIcon;
pub use gcp::GcpIcon;

/// An icon file stem: lowercase ASCII letters and digits in runs joined by single `-`.
pub const ICON_STEM_PATTERN: &str = r"^[a-z0-9]+(-[a-z0-9]+)*$";
pub const ICON_STEM_BYTES_MAX: usize = 64;

/// The `icon` of an Item or of a products row: a file stem, resolved against the pack of
/// the kind it sits on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct IconStem(String);

impl IconStem {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IconStem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// True when `stem` matches ICON_STEM_PATTERN and has at most ICON_STEM_BYTES_MAX bytes.
pub fn is_icon_stem(stem: &str) -> bool {
    stem.len() <= ICON_STEM_BYTES_MAX
        && !stem.is_empty()
        && stem.split('-').all(|run| {
            !run.is_empty()
                && run
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

impl TryFrom<String> for IconStem {
    type Error = String;

    fn try_from(stem: String) -> Result<Self, Self::Error> {
        if is_icon_stem(&stem) {
            Ok(IconStem(stem))
        } else {
            Err(format!(
                "icon \"{stem}\" is not a file stem matching {ICON_STEM_PATTERN} of at most {ICON_STEM_BYTES_MAX} bytes"
            ))
        }
    }
}

impl TryFrom<&str> for IconStem {
    type Error = String;

    fn try_from(stem: &str) -> Result<Self, Self::Error> {
        IconStem::try_from(stem.to_string())
    }
}

impl JsonSchema for IconStem {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "IconStem".into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": ICON_STEM_PATTERN,
            "minLength": 1,
            "maxLength": ICON_STEM_BYTES_MAX,
        })
    }
}

/// The pack an item kind draws its icons from.
// The bundled packs in alphabetical order, then None; the per-pack match arms and the
// section 8.2 tables follow the same order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IconPack {
    Aws,
    Gcp,
    None,
}

impl IconPack {
    /// Every pack with files under `assets/icons/<pack>/`.
    pub const BUNDLED: [IconPack; 2] = [IconPack::Aws, IconPack::Gcp];

    /// The serialized name, which is also the pack's directory under `assets/icons/`.
    pub fn as_str(self) -> &'static str {
        match self {
            IconPack::Aws => "aws",
            IconPack::Gcp => "gcp",
            IconPack::None => "none",
        }
    }

    /// The pack's icons in its `ALL` order; empty for `None`.
    pub fn icons(self) -> Vec<IconName> {
        match self {
            IconPack::Aws => AwsIcon::ALL.into_iter().map(IconName::Aws).collect(),
            IconPack::Gcp => GcpIcon::ALL.into_iter().map(IconName::Gcp).collect(),
            IconPack::None => Vec::new(),
        }
    }

    /// The icon `stem` names in this pack, or None when the pack has no such file.
    pub fn resolve(self, stem: &IconStem) -> Option<IconName> {
        match self {
            IconPack::Aws => AwsIcon::from_stem(stem.as_str()).map(IconName::Aws),
            IconPack::Gcp => GcpIcon::from_stem(stem.as_str()).map(IconName::Gcp),
            IconPack::None => None,
        }
    }
}

/// One bundled icon file: a stem scoped by its pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconName {
    Aws(AwsIcon),
    Gcp(GcpIcon),
}

impl IconName {
    pub fn pack(self) -> IconPack {
        match self {
            IconName::Aws(_) => IconPack::Aws,
            IconName::Gcp(_) => IconPack::Gcp,
        }
    }

    /// The file stem, for example "cloud-run".
    pub fn as_str(self) -> &'static str {
        let file_name = self.file_name();
        file_name.strip_suffix(".svg").unwrap_or(file_name)
    }

    /// The icon's file name in `assets/icons/<pack>/`: the stem plus `.svg`.
    pub fn file_name(self) -> &'static str {
        match self {
            IconName::Aws(icon) => icon.file_name(),
            IconName::Gcp(icon) => icon.file_name(),
        }
    }
}
