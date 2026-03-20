pub(crate) mod betterstack;
pub(crate) mod fallback;
pub(crate) mod google;
pub(crate) mod instatus;
pub(crate) mod onlineornot;
pub(crate) mod status_io;
pub(crate) mod statuspage;

// ---------------------------------------------------------------------------
// Normalize component status strings from various platforms
// ---------------------------------------------------------------------------

pub(crate) fn normalize_component_status(raw: &str) -> String {
    match raw {
        // Instatus (UPPERCASECONCATENATED)
        "OPERATIONAL" => "operational".to_string(),
        "DEGRADEDPERFORMANCE" => "degraded_performance".to_string(),
        "UNDERMAINTENANCE" => "under_maintenance".to_string(),
        "MAJOROUTAGE" => "major_outage".to_string(),
        "PARTIALOUTAGE" => "partial_outage".to_string(),
        // Better Stack / OnlineOrNot
        "degraded" => "degraded_performance".to_string(),
        "downtime" | "outage" => "major_outage".to_string(),
        // Already normalized or unknown — lowercase passthrough
        other => other.to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_component_status_maps_all_platforms() {
        // Better Stack
        assert_eq!(
            normalize_component_status("degraded"),
            "degraded_performance"
        );
        assert_eq!(normalize_component_status("downtime"), "major_outage");
        // OnlineOrNot
        assert_eq!(normalize_component_status("outage"), "major_outage");
        // Instatus
        assert_eq!(normalize_component_status("OPERATIONAL"), "operational");
        assert_eq!(
            normalize_component_status("DEGRADEDPERFORMANCE"),
            "degraded_performance"
        );
        assert_eq!(
            normalize_component_status("UNDERMAINTENANCE"),
            "under_maintenance"
        );
        assert_eq!(normalize_component_status("MAJOROUTAGE"), "major_outage");
        assert_eq!(
            normalize_component_status("PARTIALOUTAGE"),
            "partial_outage"
        );
        // Already normalized
        assert_eq!(normalize_component_status("operational"), "operational");
        assert_eq!(normalize_component_status("major_outage"), "major_outage");
    }
}
