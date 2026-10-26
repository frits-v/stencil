// Integration-test crates are not cfg(test), so clippy's allow-*-in-tests settings do not
// reach their helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Icon packs (section 8.2): every bundled pack's directory, its provenance rows and its
//! enum agree, and an icon stem resolves only in its own pack.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use stencil_model::{AwsIcon, AzureIcon, GcpIcon, IconName, IconPack, IconStem, OssIcon};

fn pack_directory(pack: IconPack) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/icons")
        .join(pack.as_str())
}

/// The `.svg` file names in the pack's directory.
fn files_on_disk(pack: IconPack) -> BTreeSet<String> {
    fs::read_dir(pack_directory(pack))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".svg"))
        .collect()
}

/// The local file of every mapping row of the pack's PROVENANCE.md: a table row whose first
/// cell is a backticked `.svg` name. A file listed twice fails.
fn provenance_rows(pack: IconPack) -> BTreeSet<String> {
    let provenance = fs::read_to_string(pack_directory(pack).join("PROVENANCE.md")).unwrap();
    let mut rows = BTreeSet::new();
    for line in provenance.lines() {
        let Some(cell) = line
            .strip_prefix("| `")
            .and_then(|rest| rest.split_once("` |"))
            .map(|(cell, _)| cell)
        else {
            continue;
        };
        if cell.ends_with(".svg") {
            assert!(
                rows.insert(cell.to_string()),
                "{} lists {cell} twice",
                pack.as_str()
            );
        }
    }
    rows
}

#[test]
fn every_file_of_every_bundled_pack_has_one_provenance_row_and_every_row_a_file() {
    let mut examined = 0;
    for pack in IconPack::BUNDLED {
        let on_disk = files_on_disk(pack);
        let rows = provenance_rows(pack);
        assert!(!on_disk.is_empty(), "{} has no files", pack.as_str());
        assert_eq!(
            on_disk.difference(&rows).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "files of {} with no provenance row",
            pack.as_str()
        );
        assert_eq!(
            rows.difference(&on_disk).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "provenance rows of {} with no file",
            pack.as_str()
        );
        examined += on_disk.len();
    }
    println!(
        "examined {examined} files in {} packs",
        IconPack::BUNDLED.len()
    );
    assert!(examined > 0);
}

#[test]
fn every_bundled_pack_enum_names_exactly_the_files_in_its_directory() {
    let mut examined = 0;
    for pack in IconPack::BUNDLED {
        let icons = pack.icons();
        let names: BTreeSet<String> = icons
            .iter()
            .map(|icon| icon.file_name().to_string())
            .collect();
        assert_eq!(
            names.len(),
            icons.len(),
            "{} names a file twice",
            pack.as_str()
        );
        assert_eq!(names, files_on_disk(pack), "{}", pack.as_str());
        for icon in icons {
            assert_eq!(icon.pack(), pack);
            assert_eq!(format!("{}.svg", icon.as_str()), icon.file_name());
            let stem = IconStem::try_from(icon.as_str()).unwrap();
            assert_eq!(pack.resolve(&stem), Some(icon));
            examined += 1;
        }
    }
    println!("examined {examined} icons");
    assert!(examined > 0);
}

#[test]
fn the_bundled_packs_are_in_alphabetical_order_and_none_is_not_one() {
    let names: Vec<&str> = IconPack::BUNDLED.iter().map(|pack| pack.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
    assert!(!IconPack::BUNDLED.contains(&IconPack::None));
    for pack in IconPack::BUNDLED.into_iter().chain([IconPack::None]) {
        assert_eq!(serde_json::to_value(pack).unwrap(), pack.as_str());
    }
}

#[test]
fn azure_icons_are_in_file_name_order() {
    assert_eq!(AzureIcon::ALL.len(), 41);
    assert_eq!(AzureIcon::ALL.first(), Some(&AzureIcon::Aks));
    assert_eq!(AzureIcon::EntraId.file_name(), "entra-id.svg");
    let names: Vec<&str> = AzureIcon::ALL.iter().map(|icon| icon.file_name()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
    assert_eq!(
        IconPack::Azure.icons(),
        AzureIcon::ALL.map(IconName::Azure).to_vec()
    );
}

#[test]
fn oss_icons_are_in_file_name_order() {
    assert_eq!(OssIcon::ALL.len(), 30);
    assert_eq!(OssIcon::ALL.first(), Some(&OssIcon::Airflow));
    assert_eq!(OssIcon::Opentelemetry.file_name(), "opentelemetry.svg");
    let names: Vec<&str> = OssIcon::ALL.iter().map(|icon| icon.file_name()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
    assert_eq!(
        IconPack::Oss.icons(),
        OssIcon::ALL.map(IconName::Oss).to_vec()
    );
}

/// The oss files come from many sources, so each row names its own: the cell after the
/// product is an https URL and the last cell an ISO date.
#[test]
fn every_oss_provenance_row_carries_a_source_url_and_a_fetch_date() {
    let provenance =
        fs::read_to_string(pack_directory(IconPack::Oss).join("PROVENANCE.md")).unwrap();
    let mut examined = 0;
    for line in provenance.lines().filter(|line| line.starts_with("| `")) {
        let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
        assert_eq!(cells.len(), 5, "{line}");
        assert!(cells[0].ends_with(".svg`"), "{line}");
        assert!(!cells[1].is_empty(), "{line} names no product");
        assert!(
            cells[2].starts_with("<https://") && cells[2].ends_with('>'),
            "{line} has no source URL"
        );
        assert!(!cells[3].is_empty(), "{line} names no source path");
        let date = cells[4];
        let digits: Vec<&str> = date.split('-').collect();
        assert!(
            date.len() == 10
                && digits.len() == 3
                && digits
                    .iter()
                    .all(|part| part.bytes().all(|byte| byte.is_ascii_digit())),
            "{line} has no fetch date"
        );
        examined += 1;
    }
    println!("examined {examined} provenance rows");
    assert_eq!(examined, OssIcon::ALL.len());
}

#[test]
fn gcp_icons_keep_their_order_and_file_names() {
    assert_eq!(GcpIcon::ALL.len(), 23);
    assert_eq!(GcpIcon::ALL.first(), Some(&GcpIcon::Agents));
    assert_eq!(GcpIcon::AiMl.file_name(), "ai-ml.svg");
    assert_eq!(
        IconPack::Gcp.icons(),
        GcpIcon::ALL.map(IconName::Gcp).to_vec()
    );
}

#[test]
fn aws_icons_keep_their_order_and_file_names() {
    assert_eq!(AwsIcon::ALL.len(), 46);
    assert_eq!(AwsIcon::ALL.first(), Some(&AwsIcon::Analytics));
    assert_eq!(AwsIcon::Route53.file_name(), "route-53.svg");
    assert_eq!(
        IconPack::Aws.icons(),
        AwsIcon::ALL.map(IconName::Aws).to_vec()
    );
}

#[test]
fn a_stem_in_two_packs_is_two_icons() {
    let compute = IconStem::try_from("compute").unwrap();
    let aws = IconPack::Aws.resolve(&compute).unwrap();
    let gcp = IconPack::Gcp.resolve(&compute).unwrap();
    assert_eq!(aws, IconName::Aws(AwsIcon::Compute));
    assert_eq!(gcp, IconName::Gcp(GcpIcon::Compute));
    assert_ne!(aws, gcp);
    let s3 = IconStem::try_from("s3").unwrap();
    assert_eq!(IconPack::Aws.resolve(&s3), Some(IconName::Aws(AwsIcon::S3)));
    assert_eq!(IconPack::Gcp.resolve(&s3), None);
}

#[test]
fn a_stem_resolves_only_in_a_pack_that_has_its_file() {
    let gke = IconStem::try_from("gke").unwrap();
    assert_eq!(
        IconPack::Gcp.resolve(&gke),
        Some(IconName::Gcp(GcpIcon::Gke))
    );
    assert_eq!(IconPack::None.resolve(&gke), None);
    assert_eq!(IconPack::Aws.resolve(&gke), None);
    let bigtable = IconStem::try_from("bigtable").unwrap();
    assert_eq!(IconPack::Gcp.resolve(&bigtable), None);
    assert_eq!(IconPack::Azure.resolve(&gke), None);
    let key_vault = IconStem::try_from("key-vault").unwrap();
    assert_eq!(
        IconPack::Azure.resolve(&key_vault),
        Some(IconName::Azure(AzureIcon::KeyVault))
    );
    assert_eq!(IconPack::Gcp.resolve(&key_vault), None);
    let postgresql = IconStem::try_from("postgresql").unwrap();
    assert_eq!(
        IconPack::Oss.resolve(&postgresql),
        Some(IconName::Oss(OssIcon::Postgresql))
    );
    assert_eq!(
        IconPack::Azure.resolve(&postgresql),
        Some(IconName::Azure(AzureIcon::Postgresql))
    );
    assert_eq!(IconPack::Oss.resolve(&key_vault), None);
}

#[test]
fn an_icon_stem_is_lowercase_runs_joined_by_single_hyphens() {
    for stem in ["gke", "ai-ml", "cloud-run-flat", "s3", &"a".repeat(64)] {
        let parsed: IconStem = serde_json::from_value(serde_json::json!(stem)).unwrap();
        assert_eq!(parsed.as_str(), stem);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), stem);
    }
    for stem in [
        "",
        "Gke",
        "cloud run",
        "-gke",
        "gke-",
        "cloud--run",
        "gke.svg",
        &"a".repeat(65),
    ] {
        assert!(
            serde_json::from_value::<IconStem>(serde_json::json!(stem)).is_err(),
            "{stem:?} parsed"
        );
    }
}
