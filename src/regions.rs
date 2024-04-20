use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RegionInfo {
    pub code: &'static str,
    pub region: &'static str,
    pub is_legacy: bool,
}

// Provides a mapping of legacy linode region names to IATA suffixed
// naming as used by Akamai regions. Useful for consistency but not
// part of the official API.
lazy_static! {
    pub static ref REGIONS: HashMap<&'static str, RegionInfo> = {
        let mut m = HashMap::new();
        m.insert(
            "eu-west",
            RegionInfo {
                code: "uk-lhr",
                is_legacy: true,
                region: "eu-west",
            },
        );
        m.insert(
            "se-sto",
            RegionInfo {
                code: "se-sto",
                is_legacy: false,
                region: "se-sto",
            },
        );
        m.insert(
            "us-iad",
            RegionInfo {
                code: "us-iad",
                is_legacy: false,
                region: "us-iad",
            },
        );
        m.insert(
            "us-lax",
            RegionInfo {
                code: "us-lax",
                is_legacy: false,
                region: "us-lax",
            },
        );
        m.insert(
            "us-ord",
            RegionInfo {
                code: "us-ord",
                is_legacy: false,
                region: "us-ord",
            },
        );
        m.insert(
            "us-mia",
            RegionInfo {
                code: "us-mia",
                is_legacy: false,
                region: "us-mia",
            },
        );
        m.insert(
            "us-sea",
            RegionInfo {
                code: "us-sea",
                is_legacy: false,
                region: "us-sea",
            },
        );
        m.insert(
            "us-southeast",
            RegionInfo {
                code: "us-atl",
                is_legacy: true,
                region: "us-southeast",
            },
        );
        m.insert(
            "us-central",
            RegionInfo {
                code: "us-dfw",
                is_legacy: true,
                region: "us-central",
            },
        );
        m.insert(
            "us-east",
            RegionInfo {
                code: "us-ewr",
                is_legacy: true,
                region: "us-east",
            },
        );
        m.insert(
            "ca-central",
            RegionInfo {
                code: "ca-yyz",
                is_legacy: true,
                region: "ca-central",
            },
        );
        m.insert(
            "br-gru",
            RegionInfo {
                code: "br-gru",
                is_legacy: false,
                region: "br-gru",
            },
        );
        m.insert(
            "jp-osa",
            RegionInfo {
                code: "jp-osa",
                is_legacy: false,
                region: "jp-osa",
            },
        );
        m.insert(
            "fr-par",
            RegionInfo {
                code: "fr-par",
                is_legacy: false,
                region: "fr-par",
            },
        );
        m.insert(
            "it-mil",
            RegionInfo {
                code: "it-mil",
                is_legacy: false,
                region: "it-mil",
            },
        );
        m.insert(
            "ap-southeast",
            RegionInfo {
                code: "au-syd",
                is_legacy: true,
                region: "ap-southeast",
            },
        );
        m
    };
}
