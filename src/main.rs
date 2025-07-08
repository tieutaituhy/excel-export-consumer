mod config;
mod models;
mod services;
mod kafka_consumer;

use anyhow::{Context, Result};
use metrics_exporter_prometheus::PrometheusBuilder;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber::{self, fmt::format::FmtSpan, EnvFilter};
use tracing_appender::rolling::{Rotation, daily};

use crate::config::AppConfig;
use crate::services::db_store::PostgresDbStore;
use crate::services::file_exporter::LocalFileExporter;
use crate::services::notifier::HttpNotifier;
use crate::services::export_service::ExportService;

#[tokio::main]
async fn main() -> Result<()> {
    // --- Cấu hình logging với `tracing` và ghi vào file ---
    let log_dir = "logs"; // Thư mục để lưu file log
    let file_appender = tracing_appender::rolling::daily(log_dir, "consumer.log"); // Ghi log hàng ngày vào file consumer.log
    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::FULL)
        .with_writer(non_blocking_appender)
        .init();

    info!("🚀 Starting Excel Export Consumer...");

    let config = AppConfig::load().context("Failed to load application configuration")?;

    // --- Khởi tạo Prometheus Exporter cho Metrics ---
    info!("📊 Metrics will be exposed on: {}", config.metrics_listen_address);
    PrometheusBuilder::new()
        .listen_address(config.metrics_listen_address)
        .install()
        .context("Failed to install Prometheus metrics exporter")?;
    // --------------------------------------------------

    // Kết nối database
    let pool = PgPool::connect(&config.db_url)
        .await
        .context("Failed to connect to database")?;
    info!("Database connection established. 🎉");

    // Khởi tạo các service implementation
    let db_store = Arc::new(PostgresDbStore::new(pool));
    let file_exporter = Arc::new(LocalFileExporter);
    let notifier = Arc::new(HttpNotifier::new(config.notification_service_url.clone()));

    // Khởi tạo ExportService với các dependency đã được inject
    let export_service = Arc::new(ExportService::new(
        db_store,
        file_exporter,
        notifier,
        config.excel_export_path.clone(),
        // Giả định notification_service_url cũng là base URL cho file downloads
        config.notification_service_url.clone()
    ));

    // Chạy Kafka consumer (bây giờ nó chỉ tập trung vào việc nhận message và ủy quyền xử lý)
    if let Err(e) = kafka_consumer::run_kafka_consumer(Arc::new(config), export_service).await {
        error!("Fatal error in Kafka consumer: {:?}", e);
        return Err(e);
    }

    Ok(())
}