use crate::error::{DoctorError, DoctorResult};
use crate::evidence::{Evidence, EvidenceType, Reliability};
use std::time::Duration;

/// Collect runtime evidence from Spring Boot Actuator endpoints.
pub async fn collect(base_url: &str) -> DoctorResult<Vec<Evidence>> {
    let mut evidence = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| DoctorError::NetworkError { url: base_url.to_string(), source: e })?;

    let endpoints = ["health", "env", "beans", "conditions", "configprops"];

    for endpoint in &endpoints {
        let url = format!("{base_url}/actuator/{endpoint}");
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                evidence.push(Evidence::new(
                    EvidenceType::Runtime,
                    url,
                    format!("Actuator /{endpoint} endpoint accessible"),
                    Reliability::Confirmed,
                ));
            }
            Ok(resp) => {
                evidence.push(Evidence::new(
                    EvidenceType::Runtime,
                    url,
                    format!("Actuator /{endpoint} returned HTTP {}", resp.status()),
                    Reliability::Unverified,
                ));
            }
            Err(_) => {
                evidence.push(Evidence::new(
                    EvidenceType::Runtime,
                    url,
                    format!("Actuator /{endpoint} unreachable"),
                    Reliability::Unverified,
                ));
            }
        }
    }

    Ok(evidence)
}
