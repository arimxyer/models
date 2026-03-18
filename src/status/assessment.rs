use std::collections::BTreeSet;

use chrono::{Duration, Utc};

use super::types::{
    ActiveIncident, AffectedSurface, ProviderAssessment, ProviderHealth, ProviderStatus,
    StatusConfidence, StatusContradiction, StatusCoverage, StatusDetailAvailability,
    StatusFreshness, StatusProvenance,
};

impl ProviderStatus {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn user_visible_affected_items(&self) -> Vec<String> {
        let assessment = self.assessment();
        if !assessment.affected_surfaces.is_empty() {
            let mut items: Vec<String> = assessment
                .affected_surfaces
                .iter()
                .map(|surface| surface.label().to_string())
                .collect();
            if items.len() > 1 {
                items.retain(|item| item != AffectedSurface::Unknown.label());
            }
            return items;
        }

        self.active_incidents()
            .into_iter()
            .flat_map(|incident| incident.affected_components.iter().cloned())
            .chain(
                self.scheduled_maintenances
                    .iter()
                    .flat_map(|maint| maint.affected_components.iter().cloned()),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn user_visible_caveat(&self) -> Option<&'static str> {
        let assessment = self.assessment();
        if self.provenance == StatusProvenance::Unavailable {
            Some("Status unavailable")
        } else if self.provenance == StatusProvenance::Fallback {
            Some("Limited detail available")
        } else if self.has_partial_data() {
            Some("Some status details failed to load")
        } else if !self.component_detail_available() || !self.incident_detail_available() {
            Some("Limited detail available")
        } else if assessment
            .warnings
            .iter()
            .any(|warning| warning.contains("stale") || warning.contains("reliable freshness"))
        {
            Some("Verify details on the official status page")
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn assessment(&self) -> ProviderAssessment {
        let coverage = self.coverage();
        let freshness = self.freshness();
        let active_incidents = self.active_incidents();
        let contradictions = self.contradictions(coverage, freshness, active_incidents.len());
        let confidence = self.confidence(coverage, freshness, &contradictions);
        let affected_surfaces = self.affected_surfaces();
        let mut reconciliation_notes = Vec::new();
        if self.component_detail_available() {
            reconciliation_notes.push(format!(
                "{} component signal(s) normalized into the app health model.",
                self.components.len()
            ));
        }
        if self.incident_detail_available() {
            reconciliation_notes.push(format!(
                "{} active incident(s) contribute to the current assessment.",
                active_incidents.len()
            ));
        }
        if self.provenance == StatusProvenance::Fallback {
            reconciliation_notes.push(
                "Assessment uses fallback aggregator evidence because official data was unavailable."
                    .to_string(),
            );
        }
        if self.provenance == StatusProvenance::Unavailable {
            reconciliation_notes.push(
                "Assessment is constrained by missing machine-readable status data.".to_string(),
            );
        }
        for note in [
            self.components_state.note.as_deref(),
            self.incidents_state.note.as_deref(),
            self.scheduled_maintenances_state.note.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            reconciliation_notes.push(note.to_string());
        }

        let mut warnings = Vec::new();
        if self.provenance == StatusProvenance::Fallback {
            warnings.push(
                "Fallback data is lower-trust than an official machine-readable feed.".to_string(),
            );
        }
        if matches!(freshness, StatusFreshness::Stale | StatusFreshness::Unknown) {
            warnings.push(match freshness {
                StatusFreshness::Stale => {
                    "Status data appears stale; verify on the provider status page.".to_string()
                }
                StatusFreshness::Unknown => {
                    "Status source did not provide a reliable freshness timestamp.".to_string()
                }
                _ => unreachable!(),
            });
        }
        if coverage == StatusCoverage::None {
            warnings
                .push("No machine-readable coverage is available for this provider.".to_string());
        } else if coverage == StatusCoverage::SummaryOnly {
            warnings.push(
                "Coverage is summary-only; incidents/components may be missing from this view."
                    .to_string(),
            );
        }
        for (label, state) in [
            ("Component details", &self.components_state),
            ("Incident details", &self.incidents_state),
            ("Maintenance details", &self.scheduled_maintenances_state),
        ] {
            if state.availability == StatusDetailAvailability::FetchFailed {
                warnings.push(
                    state
                        .error
                        .clone()
                        .unwrap_or_else(|| format!("{label} failed to load.")),
                );
            }
        }
        warnings.extend(contradictions.iter().map(|c| c.detail.clone()));
        if let Some(error) = self.error_summary() {
            warnings.push(format!("Fetch error: {error}"));
        }

        let assessment_summary = self.assessment_summary(confidence, coverage, freshness);
        let evidence_summary = self.evidence_summary(coverage, active_incidents.len());

        ProviderAssessment {
            overall_health: self.health,
            confidence,
            coverage,
            freshness,
            active_incident_count: active_incidents.len(),
            affected_surfaces,
            assessment_summary,
            evidence_summary,
            reconciliation_notes,
            warnings,
            contradictions,
        }
    }

    fn coverage(&self) -> StatusCoverage {
        match self.provenance {
            StatusProvenance::Unavailable => StatusCoverage::None,
            StatusProvenance::Fallback => StatusCoverage::SummaryOnly,
            StatusProvenance::Official => {
                let has_incidents = self.incident_detail_available();
                let has_components = self.component_detail_available();
                match (has_incidents, has_components) {
                    (true, true) => StatusCoverage::Full,
                    (true, false) => StatusCoverage::IncidentOnly,
                    (false, true) => StatusCoverage::ComponentOnly,
                    (false, false) => {
                        if self.provider_summary.is_some() {
                            StatusCoverage::SummaryOnly
                        } else {
                            StatusCoverage::None
                        }
                    }
                }
            }
        }
    }

    fn freshness(&self) -> StatusFreshness {
        let Some(last_checked) = self.source_updated_at.as_deref() else {
            return StatusFreshness::Unknown;
        };
        let Some(parsed) = crate::formatting::parse_date(last_checked) else {
            return StatusFreshness::Unknown;
        };
        let age = Utc::now().signed_duration_since(parsed);
        if age <= Duration::hours(6) {
            StatusFreshness::Fresh
        } else if age <= Duration::hours(24) {
            StatusFreshness::Aging
        } else {
            StatusFreshness::Stale
        }
    }

    fn confidence(
        &self,
        coverage: StatusCoverage,
        freshness: StatusFreshness,
        contradictions: &[StatusContradiction],
    ) -> StatusConfidence {
        let mut confidence = match self.provenance {
            StatusProvenance::Official => match coverage {
                StatusCoverage::Full => StatusConfidence::High,
                StatusCoverage::IncidentOnly
                | StatusCoverage::ComponentOnly
                | StatusCoverage::SummaryOnly => StatusConfidence::Medium,
                StatusCoverage::None => StatusConfidence::None,
            },
            StatusProvenance::Fallback => StatusConfidence::Low,
            StatusProvenance::Unavailable => StatusConfidence::None,
        };

        if matches!(freshness, StatusFreshness::Stale | StatusFreshness::Unknown) {
            confidence = downgrade_confidence(confidence);
        }
        if self.has_detail_fetch_failures() {
            confidence = downgrade_confidence(confidence);
        }
        if !contradictions.is_empty() {
            confidence = downgrade_confidence(confidence);
        }
        confidence
    }

    fn affected_surfaces(&self) -> Vec<AffectedSurface> {
        let mut surfaces = BTreeSet::new();
        for text in self
            .components
            .iter()
            .map(|component| component.name.as_str())
            .chain(self.incidents.iter().map(|incident| incident.name.as_str()))
            .chain(
                self.incidents
                    .iter()
                    .flat_map(|incident| incident.affected_components.iter().map(String::as_str)),
            )
            .chain(
                self.scheduled_maintenances
                    .iter()
                    .flat_map(|maint| maint.affected_components.iter().map(String::as_str)),
            )
        {
            surfaces.insert(normalize_surface(text));
        }
        if surfaces.is_empty() {
            surfaces.insert(AffectedSurface::Unknown);
        }
        surfaces.into_iter().collect()
    }

    fn contradictions(
        &self,
        coverage: StatusCoverage,
        freshness: StatusFreshness,
        active_incident_count: usize,
    ) -> Vec<StatusContradiction> {
        let mut contradictions = Vec::new();
        let summary = self
            .provider_summary
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();
        let summary_claims_operational = summary.contains("operational")
            || summary.contains("all systems operational")
            || summary.contains("fully operational");
        let degraded_components = if self.component_detail_available() {
            self.components
                .iter()
                .filter(|component| {
                    matches!(
                        component_health(&component.status),
                        ProviderHealth::Degraded | ProviderHealth::Outage
                    )
                })
                .count()
        } else {
            0
        };

        if summary_claims_operational && active_incident_count > 0 {
            contradictions.push(StatusContradiction {
                summary: "Operational summary with active incident".to_string(),
                detail: format!(
                    "Source summary looks operational, but {} active incident(s) remain unresolved.",
                    active_incident_count
                ),
            });
        }
        if summary_claims_operational && degraded_components > 0 {
            contradictions.push(StatusContradiction {
                summary: "Operational summary with degraded components".to_string(),
                detail: format!(
                    "Source summary looks operational, but {} component(s) are degraded or in outage.",
                    degraded_components
                ),
            });
        }
        if self.provenance == StatusProvenance::Fallback
            && self.health == ProviderHealth::Operational
            && coverage == StatusCoverage::SummaryOnly
            && matches!(freshness, StatusFreshness::Stale | StatusFreshness::Unknown)
        {
            contradictions.push(StatusContradiction {
                summary: "Fallback operational snapshot is low-trust".to_string(),
                detail: "Fallback data reports operational status with limited or stale coverage."
                    .to_string(),
            });
        }
        contradictions
    }

    fn assessment_summary(
        &self,
        confidence: StatusConfidence,
        coverage: StatusCoverage,
        freshness: StatusFreshness,
    ) -> String {
        let source = self.source_label.as_deref().unwrap_or("No source");
        let summary = self
            .provider_summary
            .as_deref()
            .or(self.status_note.as_deref())
            .unwrap_or("No provider summary was supplied.");
        format!(
            "{} is {} based on {} evidence ({}, {}, {}). {}",
            self.display_name,
            self.health.label().to_lowercase(),
            source,
            confidence.label().to_lowercase(),
            coverage.label().to_lowercase(),
            freshness.label().to_lowercase(),
            summary
        )
    }

    fn evidence_summary(&self, coverage: StatusCoverage, active_incident_count: usize) -> String {
        let component_count = self.components.len();
        let maintenance_count = self.scheduled_maintenances.len();
        match self.provenance {
            StatusProvenance::Official => format!(
                "Official {} feed with {} coverage: {} active incident(s), {} component signal(s), {} scheduled maintenance item(s).",
                self.source_method.map(|method| method.label()).unwrap_or("status"),
                coverage.label().to_lowercase(),
                active_incident_count,
                if self.component_detail_available() {
                    component_count
                } else {
                    0
                },
                if self.maintenance_detail_available() {
                    maintenance_count
                } else {
                    0
                }
            ),
            StatusProvenance::Fallback => format!(
                "Fallback {} snapshot with {} coverage. Raw incidents/components are unavailable in this adapter.",
                self.source_method.map(|method| method.label()).unwrap_or("status"),
                coverage.label().to_lowercase()
            ),
            StatusProvenance::Unavailable => {
                "No usable provider evidence could be loaded for this provider.".to_string()
            }
        }
    }
}

fn downgrade_confidence(confidence: StatusConfidence) -> StatusConfidence {
    match confidence {
        StatusConfidence::High => StatusConfidence::Medium,
        StatusConfidence::Medium => StatusConfidence::Low,
        StatusConfidence::Low | StatusConfidence::None => confidence,
    }
}

fn component_health(status: &str) -> ProviderHealth {
    let normalized = status.trim().to_lowercase();
    if normalized.contains("major_outage") || normalized.contains("outage") {
        ProviderHealth::Outage
    } else if normalized.contains("degraded") || normalized.contains("partial") {
        ProviderHealth::Degraded
    } else if normalized.contains("maint") {
        ProviderHealth::Maintenance
    } else {
        ProviderHealth::Operational
    }
}

fn normalize_surface(text: &str) -> AffectedSurface {
    let normalized = text.trim().to_lowercase();
    if normalized.is_empty() {
        return AffectedSurface::Unknown;
    }
    if normalized.contains("auth") || normalized.contains("login") || normalized.contains("oauth") {
        AffectedSurface::Auth
    } else if normalized.contains("chat") || normalized.contains("assistant") {
        AffectedSurface::Chat
    } else if normalized.contains("upload")
        || normalized.contains("file")
        || normalized.contains("storage")
    {
        AffectedSurface::UploadsFiles
    } else if normalized.contains("model")
        || normalized.contains("inference")
        || normalized.contains("completion")
        || normalized.contains("embedding")
        || normalized.contains("fine-tun")
    {
        AffectedSurface::ModelsInference
    } else if normalized.contains("console")
        || normalized.contains("dashboard")
        || normalized.contains("studio")
        || normalized.contains("portal")
    {
        AffectedSurface::Console
    } else if normalized.contains("api") || normalized.contains("gateway") {
        AffectedSurface::Api
    } else {
        AffectedSurface::Unknown
    }
}

impl ActiveIncident {
    pub fn is_active(&self) -> bool {
        let normalized = self.status.to_lowercase();
        !normalized.contains("resolved")
            && !normalized.contains("postmortem")
            && !normalized.contains("completed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{
        status_seed_for_provider, ComponentStatus, StatusDetailSource, StatusDetailState,
        StatusLoadState, StatusSourceMethod, StatusSupportTier,
    };

    fn sample_status() -> ProviderStatus {
        ProviderStatus {
            slug: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            source_slug: "openai".to_string(),
            support_tier: StatusSupportTier::Required,
            health: ProviderHealth::Operational,
            provenance: StatusProvenance::Official,
            load_state: StatusLoadState::Loaded,
            source_label: Some("OpenAI Status".to_string()),
            source_method: Some(StatusSourceMethod::StatuspageV2),
            official_url: Some("https://status.openai.com".to_string()),
            fallback_url: None,
            source_updated_at: Some(Utc::now().to_rfc3339()),
            provider_summary: Some("All Systems Operational".to_string()),
            status_note: None,
            components: Vec::new(),
            components_state: StatusDetailState {
                availability: StatusDetailAvailability::NoneReported,
                source: StatusDetailSource::Inline,
                note: None,
                error: None,
            },
            incidents: Vec::new(),
            incidents_state: StatusDetailState {
                availability: StatusDetailAvailability::NoneReported,
                source: StatusDetailSource::Inline,
                note: None,
                error: None,
            },
            scheduled_maintenances: Vec::new(),
            scheduled_maintenances_state: StatusDetailState {
                availability: StatusDetailAvailability::NoneReported,
                source: StatusDetailSource::Inline,
                note: None,
                error: None,
            },
            official_error: None,
            fallback_error: None,
        }
    }

    #[test]
    fn assessment_surfaces_operational_contradictions() {
        let mut status = sample_status();
        status.components.push(ComponentStatus {
            name: "API".to_string(),
            status: "major_outage".to_string(),
            group_name: None,
        });
        status.incidents.push(ActiveIncident {
            name: "API elevated errors".to_string(),
            status: "investigating".to_string(),
            impact: "minor".to_string(),
            shortlink: None,
            created_at: None,
            updated_at: None,
            latest_update: None,
            affected_components: vec!["API".to_string()],
        });

        let assessment = status.assessment();
        assert_eq!(assessment.coverage, StatusCoverage::Full);
        assert_eq!(assessment.confidence, StatusConfidence::Medium);
        assert_eq!(assessment.active_incident_count, 1);
        assert!(assessment
            .contradictions
            .iter()
            .any(|entry| { entry.summary == "Operational summary with active incident" }));
        assert!(assessment.affected_surfaces.contains(&AffectedSurface::Api));
    }

    #[test]
    fn fallback_stale_summary_only_is_low_trust() {
        let mut status = sample_status();
        status.provenance = StatusProvenance::Fallback;
        status.source_method = Some(StatusSourceMethod::ApiStatusCheck);
        status.fallback_url = Some("https://apistatuscheck.com/api/openai".to_string());
        status.source_updated_at = Some((Utc::now() - Duration::hours(36)).to_rfc3339());
        status.components_state = StatusDetailState {
            availability: StatusDetailAvailability::Unsupported,
            source: StatusDetailSource::SummaryOnly,
            note: Some("Service details are unavailable from the fallback adapter.".to_string()),
            error: None,
        };
        status.incidents_state = StatusDetailState {
            availability: StatusDetailAvailability::Unsupported,
            source: StatusDetailSource::SummaryOnly,
            note: Some("Incident details are unavailable from the fallback adapter.".to_string()),
            error: None,
        };
        status.scheduled_maintenances_state = StatusDetailState {
            availability: StatusDetailAvailability::Unsupported,
            source: StatusDetailSource::SummaryOnly,
            note: Some(
                "Maintenance details are unavailable from the fallback adapter.".to_string(),
            ),
            error: None,
        };

        let assessment = status.assessment();
        assert_eq!(assessment.coverage, StatusCoverage::SummaryOnly);
        assert_eq!(assessment.freshness, StatusFreshness::Stale);
        assert_eq!(assessment.confidence, StatusConfidence::Low);
        assert!(assessment
            .contradictions
            .iter()
            .any(|entry| { entry.summary == "Fallback operational snapshot is low-trust" }));
    }

    #[test]
    fn user_visible_caveat_prefers_simple_messages() {
        let mut fallback = sample_status();
        fallback.provenance = StatusProvenance::Fallback;
        assert_eq!(
            fallback.user_visible_caveat(),
            Some("Limited detail available")
        );

        let unavailable =
            ProviderStatus::placeholder(&status_seed_for_provider("some-unknown-provider"));
        assert_eq!(
            unavailable.user_visible_caveat(),
            Some("Status unavailable")
        );

        let mut stale = sample_status();
        stale.source_updated_at = Some((Utc::now() - Duration::hours(30)).to_rfc3339());
        assert_eq!(
            stale.user_visible_caveat(),
            Some("Verify details on the official status page")
        );
    }

    #[test]
    fn user_visible_affected_items_prefers_surface_labels() {
        let mut status = sample_status();
        status.incidents.push(ActiveIncident {
            name: "API elevated errors".to_string(),
            status: "investigating".to_string(),
            impact: "minor".to_string(),
            shortlink: None,
            created_at: None,
            updated_at: None,
            latest_update: None,
            affected_components: vec!["API".to_string(), "Auth".to_string()],
        });

        assert_eq!(
            status.user_visible_affected_items(),
            vec!["API".to_string(), "Auth".to_string()]
        );
    }

    #[test]
    fn user_visible_affected_items_drops_unknown_when_known_surfaces_exist() {
        let mut status = sample_status();
        status.components.push(ComponentStatus {
            name: "Claude".to_string(),
            status: "operational".to_string(),
            group_name: None,
        });
        status.components.push(ComponentStatus {
            name: "API".to_string(),
            status: "operational".to_string(),
            group_name: None,
        });

        assert_eq!(
            status.user_visible_affected_items(),
            vec!["API".to_string()]
        );
    }

    #[test]
    fn unavailable_status_reports_missing_coverage() {
        let status =
            ProviderStatus::placeholder(&status_seed_for_provider("some-unknown-provider"));
        let assessment = status.assessment();
        assert_eq!(assessment.coverage, StatusCoverage::None);
        assert_eq!(assessment.confidence, StatusConfidence::None);
        assert!(assessment
            .warnings
            .iter()
            .any(|warning| { warning.contains("No machine-readable coverage") }));
    }
}
