use anyhow::{Context, Result};
use std::env;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub kafka_brokers: String,
    pub kafka_topic: String,
    pub db_url: String,
    pub notification_service_url: String,
    pub excel_export_path: String,
    pub metrics_listen_address: SocketAddr,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        dotenv::dotenv().ok(); // Load .env file
        Ok(AppConfig {
            kafka_brokers: env::var("KAFKA_BROKERS")
                .context("KAFKA_BROKERS must be set in .env")?,
            kafka_topic: env::var("KAFKA_TOPIC")
                .context("KAFKA_TOPIC must be set in .env")?,
            db_url: env::var("DATABASE_URL")
                .context("DATABASE_URL must be set in .env")?,
            notification_service_url: env::var("NOTIFICATION_SERVICE_URL")
                .context("NOTIFICATION_SERVICE_URL must be set in .env")?,
            excel_export_path: env::var("EXCEL_EXPORT_PATH")
                .context("EXCEL_EXPORT_PATH must be set in .env")?,
            metrics_listen_address: env::var("METRICS_LISTEN_ADDRESS")
                .context("METRICS_LISTEN_ADDRESS must be set in .env")?
                .parse()
                .context("METRICS_LISTEN_ADDRESS is not a valid socket address")?,
        })
    }
}