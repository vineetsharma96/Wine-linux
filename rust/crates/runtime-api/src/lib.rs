pub mod error;
pub mod models;
pub mod traits;

pub use error::RuntimeError;
pub use models::*;
pub use traits::*;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_properties() {
        let aarch64 = Architecture::Aarch64;
        assert_eq!(aarch64.as_str(), "aarch64");
    }

    #[test]
    fn test_health_report_serialization() {
        let report = HealthReport {
            status: HealthLevel::Healthy,
            subsystems: vec![SubsystemHealth {
                name: "Memory".to_string(),
                level: HealthLevel::Healthy,
                message: "OK".to_string(),
                latency_ms: 1,
            }],
            timestamp_epoch_ms: 123456789,
        };
        let json = serde_json::to_string(&report).expect("Serialize health report");
        let decoded: HealthReport = serde_json::from_str(&json).expect("Deserialize health report");
        assert_eq!(decoded.status, HealthLevel::Healthy);
        assert_eq!(decoded.subsystems.len(), 1);
    }
}
