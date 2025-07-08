use anyhow::{Context, Result};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, instrument, Span, warn};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::services::export_service::ExportService;
use crate::services::db_store::DbStore;
use crate::services::file_exporter::FileExporter;
use crate::services::notifier::Notifier;

pub async fn run_kafka_consumer<D, F, N>(
    config: Arc<AppConfig>,
    export_service: Arc<ExportService<D, F, N>>,
) -> Result<()>
where
    D: DbStore,
    F: FileExporter,
    N: Notifier,
{
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "excel_export_group")
        .set("bootstrap.servers", &config.kafka_brokers)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create()
        .context("Failed to create Kafka consumer")?;

    consumer
        .subscribe(&[&config.kafka_topic])
        .context(format!(
            "Failed to subscribe to Kafka topic: {}",
            config.kafka_topic
        ))?;

    info!("Subscribed to Kafka topic: `{}`. Listening for messages...", config.kafka_topic);

    loop {
        match consumer.recv().await {
            Ok(message) => {
                let payload = match message.payload_view::<str>() {
                    Some(Ok(s)) => s,
                    _ => {
                        warn!(
                            "Received empty or non-string payload, skipping. Message offset: {}",
                            message.offset()
                        );
                        continue;
                    }
                };

                let request_id_str = payload;
                let request_id = match Uuid::parse_str(request_id_str) {
                    Ok(id) => id,
                    Err(e) => {
                        error!(
                            "Failed to parse UUID from Kafka message '{}' on topic {} partition {} offset {}: {:?}",
                            request_id_str,
                            message.topic(),
                            message.partition(),
                            message.offset(),
                            e
                        );
                        continue;
                    }
                };

                let export_service_clone = Arc::clone(&export_service);
                let consumer_clone = consumer.clone();
                let owned_message = message.detach();
                let span_clone = Span::current(); // Capture the current span context

                tokio::spawn(async move {
                    if let Err(e) = export_service_clone
                        .process_export_request(request_id, span_clone)
                        .await
                    {
                        error!("❌ Error processing export request {}: {:?}", request_id, e);
                    }

                    // Quan trọng: Commit offset Kafka sau khi xử lý hoàn tất (thành công hoặc thất bại)
                    if let Err(e) = consumer_clone
                        .commit_message(&owned_message, rdkafka::consumer::CommitMode::Async)
                        .await
                    {
                        error!("Failed to commit Kafka message offset for request {}: {:?}", request_id, e);
                    } else {
                        info!("🔗 Committed Kafka message for request {}.", request_id);
                    }
                });
            }
            Err(e) => {
                error!("⚡ Kafka error: {:?}. Attempting to reconnect in 5 seconds...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}