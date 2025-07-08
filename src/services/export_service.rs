use anyhow::{Context, Result};
use chrono::{NaiveTime, Utc};
use metrics::{gauge, histogram, increment};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, instrument, Span};
use uuid::Uuid;

use crate::models::{ExportRequest, ExportStatus, ReportParams};
use crate::services::db_store::DbStore;
use crate::services::file_exporter::FileExporter;
use crate::services::notifier::Notifier;

/// ExportService đóng gói toàn bộ logic xử lý một yêu cầu xuất Excel.
/// Nó nhận các dependency của nó (DbStore, FileExporter, Notifier) thông qua trait objects.
pub struct ExportService<D, F, N>
where
    D: DbStore,
    F: FileExporter,
    N: Notifier,
{
    db_store: Arc<D>,
    file_exporter: Arc<F>,
    notifier: Arc<N>,
    excel_export_path: String,
    notification_service_base_url: String, // Base URL để xây dựng public file URL
}

impl<D, F, N> ExportService<D, F, N>
where
    D: DbStore,
    F: FileExporter,
    N: Notifier,
{
    pub fn new(
        db_store: Arc<D>,
        file_exporter: Arc<F>,
        notifier: Arc<N>,
        excel_export_path: String,
        notification_service_base_url: String,
    ) -> Self {
        Self {
            db_store,
            file_exporter,
            notifier,
            excel_export_path,
            notification_service_base_url,
        }
    }

    #[instrument(
        skip(self, current_span), // current_span không cần thiết để in ra log
        fields(
            request_id = %request_id,
            user_id = tracing::field::Empty // Sẽ điền sau
        )
    )]
    pub async fn process_export_request(
        &self,
        request_id: Uuid,
        current_span: Span, // Lấy span hiện tại để ghi thêm field
    ) -> Result<()> {
        let start_time = Instant::now(); // Bắt đầu đo tổng thời gian xử lý request
        gauge!("excel_export_requests_in_progress", 1.0, "request_id" => request_id.to_string()); // Tăng gauge

        let mut final_status = ExportStatus::Failed;
        let mut file_path: Option<String> = None;
        let mut error_message: Option<String> = None;

        // Use a dedicated block to capture processing results and ensure
        // cleanup (gauge decrement) and Kafka commit happen reliably.
        let processing_result = async {
            // 1. Fetch request and update status to PROCESSING
            let fetch_start_time = Instant::now();
            let export_request: ExportRequest = self.db_store
                .fetch_and_update_request_status(request_id, ExportStatus::Processing)
                .await
                .context("Failed to fetch or update request status to PROCESSING")?;
            histogram!("excel_export_db_fetch_duration_seconds", fetch_start_time.elapsed().as_secs_f64());
            
            // Record user_id on the current span
            current_span.record("user_id", export_request.user_id);
            info!("✅ Request fetched and status updated to PROCESSING for user_id: {}.", export_request.user_id);

            // 2. Parse RequestPayload and query data
            let parse_and_query_start_time = Instant::now();
            let params: ReportParams = serde_json::from_value(export_request.request_payload)
                .context("Failed to parse request_payload JSON")?;
            info!("🔍 Report parameters parsed: {:?}", params);

            let raw_data = self.db_store.query_product_data(&params).await
                .context("Failed to query product data")?;
            histogram!("excel_export_db_query_duration_seconds", parse_and_query_start_time.elapsed().as_secs_f64());

            // 3. Generate Excel file
            let excel_gen_start_time = Instant::now();
            let exported_file_path = self.file_exporter.export_to_excel(
                request_id,
                raw_data,
                &self.excel_export_path,
            ).await.context("Failed to export data to Excel")?;
            histogram!("excel_export_excel_generation_duration_seconds", excel_gen_start_time.elapsed().as_secs_f64());
            
            file_path = Some(exported_file_path);
            final_status = ExportStatus::Completed;
            Ok(())
        }
        .await; // End of processing_result block

        // 4. Update final status in DB and send notification
        let update_notify_start_time = Instant::now();
        match processing_result {
            Ok(_) => {
                info!("Export request {} completed successfully.", request_id);
                self.db_store.update_request_status(
                    request_id,
                    final_status,
                    file_path.clone(),
                    None,
                ).await?;
                increment!("excel_export_completed_total");
            }
            Err(e) => {
                error!("Export request {} failed: {:?}", request_id, e);
                error_message = Some(format!("Error: {:?}", e));
                self.db_store.update_request_status(
                    request_id,
                    final_status,
                    None,
                    error_message.clone(),
                ).await?;
                increment!("excel_export_failed_total");
            }
        }

        // Send notification
        let public_file_url = file_path.map(|p| {
            format!(
                "{}/exports/{}",
                self.notification_service_base_url,
                Path::new(&p).file_name().unwrap_or_default().to_str().unwrap_or_default()
            )
        });

        if let Err(e) = self.notifier.send_notification(
            request_id,
            final_status.as_str(),
            public_file_url,
            error_message,
        ).await {
            error!(
                "Failed to send notification for request {}: {:?}. Will mark as not sent.",
                request_id, e
            );
            // Mark as not sent in DB for potential retry
            self.db_store.update_notification_sent_status(request_id, false).await.ok();
            increment!("excel_export_notification_failed_total");
        } else {
            self.db_store.update_notification_sent_status(request_id, true).await?;
            increment!("excel_export_notification_sent_total");
        }
        histogram!("excel_export_update_notify_duration_seconds", update_notify_start_time.elapsed().as_secs_f64());

        // Final metrics and cleanup
        gauge!("excel_export_requests_in_progress", -1.0, "request_id" => request_id.to_string());
        histogram!("excel_export_total_processing_duration_seconds", start_time.elapsed().as_secs_f64());
        info!("🏁 Finished processing request {}. Total duration: {:.2}s", request_id, start_time.elapsed().as_secs_f64());

        Ok(())
    }
}