use anyhow::{Context, Result};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::models::ExportNotification;

/// Trait định nghĩa giao diện cho việc gửi thông báo.
#[async_trait::async_trait]
pub trait Notifier: Send + Sync + 'static {
    async fn send_notification(
        &self,
        request_id: Uuid,
        status: &str,
        file_url: Option<String>,
        error_message: Option<String>,
    ) -> Result<()>;
}

/// Implementation cụ thể để gửi thông báo qua HTTP POST.
pub struct HttpNotifier {
    notification_service_url: String,
    client: reqwest::Client,
}

impl HttpNotifier {
    pub fn new(notification_service_url: String) -> Self {
        Self {
            notification_service_url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl Notifier for HttpNotifier {
    #[instrument(skip(self), fields(request_id = %request_id))]
    async fn send_notification(
        &self,
        request_id: Uuid,
        status: &str,
        file_url: Option<String>,
        error_message: Option<String>,
    ) -> Result<()> {
        let notification = ExportNotification {
            request_id,
            status: status.to_string(),
            file_url,
            error_message,
        };

        info!(
            "Attempting to send notification to {} for request {} with status '{}'. Payload: {:?}",
            self.notification_service_url, request_id, status, notification
        );

        let response = self.client
            .post(&self.notification_service_url)
            .json(&notification)
            .send()
            .await
            .context("Failed to send notification HTTP request")?;

        if response.status().is_success() {
            info!(
                "✅ Successfully sent notification for request {}. HTTP Status: {}",
                request_id,
                response.status()
            );
            Ok(())
        } else {
            let status_code = response.status();
            let response_text = response.text().await.unwrap_or_default();
            error!(
                "Failed to send notification. HTTP Status: {}, Response Body: {}",
                status_code, response_text
            );
            Err(anyhow::anyhow!(
                "Notification service responded with error status {}: {}",
                status_code,
                response_text
            ))
        }
    }
}