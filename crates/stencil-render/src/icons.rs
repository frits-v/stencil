//! The icons of section 8.2, embedded byte for byte, one module per pack. They are never
//! parsed, rewritten or minified; `<image>` scales them.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use stencil_model::IconName;

mod aws;
mod azure;
mod gcp;
mod oss;

pub fn icon_svg_bytes(icon: IconName) -> &'static [u8] {
    match icon {
        IconName::Aws(icon) => aws::svg_bytes(icon),
        IconName::Azure(icon) => azure::svg_bytes(icon),
        IconName::Gcp(icon) => gcp::svg_bytes(icon),
        IconName::Oss(icon) => oss::svg_bytes(icon),
    }
}

/// `data:image/svg+xml;base64,` followed by the base64 of the icon file's exact bytes.
pub fn icon_data_uri(icon: IconName) -> String {
    let mut uri = String::from("data:image/svg+xml;base64,");
    STANDARD.encode_string(icon_svg_bytes(icon), &mut uri);
    uri
}
