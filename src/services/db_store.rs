use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{Pool, Postgres, Transaction};
use tracing::{info, instrument, warn};
use uuid::Uuid;

use crate::models::{ExportRequest, ExportStatus, ProductData, ReportParams};

/// Trait định nghĩa giao diện cho việc tương tác với database để lưu trữ/truy vấn ExportRequests.
#[async_trait::async_trait]
pub trait DbStore: Send + Sync + 'static {
    async fn fetch_and_update_request_status(
        &self,
        request_id: Uuid,
        new_status: ExportStatus,
    ) -> Result<ExportRequest>;

    async fn update_request_status(
        &self,
        request_id: Uuid,
        new_status: ExportStatus,
        file_path: Option<String>,
        error_message: Option<String>,
    ) -> Result<()>;

    async fn update_notification_sent_status(
        &self,
        request_id: Uuid,
        sent: bool,
    ) -> Result<()>;

    async fn query_product_data(
        &self,
        params: &ReportParams,
    ) -> Result<Vec<ProductData>>;
}

/// Implementation cụ thể cho PostgreSQL.
pub struct PostgresDbStore {
    pool: Pool<Postgres>,
}

impl PostgresDbStore {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl DbStore for PostgresDbStore {
    #[instrument(skip(self), fields(request_id = %request_id))]
    async fn fetch_and_update_request_status(
        &self,
        request_id: Uuid,
        new_status: ExportStatus,
    ) -> Result<ExportRequest> {
        let mut tx = self.pool.begin().await.context("Failed to begin database transaction")?;
        info!("Starting transaction to fetch and update status to '{}'.", new_status.as_str());

        let request = sqlx::query_as!(
            ExportRequest,
            r#"
            SELECT
                id, user_id, request_payload, requested_at, status, file_path, completed_at, error_message, notification_sent
            FROM ExportRequests
            WHERE id = $1
            FOR UPDATE
            "#,
            request_id
        )
        .fetch_optional(&mut *tx)
        .await
        .context("Failed to fetch export request from DB")?
        .context("Export request not found in DB")?;

        if request.status == ExportStatus::Completed.as_str() || request.status == ExportStatus::Failed.as_str() {
            warn!(
                "Request {} already in final state: {}. Rolling back transaction and skipping processing.",
                request_id, request.status
            );
            tx.rollback().await?;
            return Err(anyhow::anyhow!("Request already processed and in final state"));
        }

        sqlx::query!(
            "UPDATE ExportRequests SET status = $1 WHERE id = $2",
            new_status.as_str(),
            request_id
        )
        .execute(&mut *tx)
        .await
        .context("Failed to update request status in DB")?;

        tx.commit().await.context("Failed to commit database transaction")?;
        info!("Successfully fetched and updated status to '{}' for request {}. Transaction committed.", new_status.as_str(), request_id);
        Ok(request)
    }

    #[instrument(skip(self))]
    async fn update_request_status(
        &self,
        request_id: Uuid,
        new_status: ExportStatus,
        file_path: Option<String>,
        error_message: Option<String>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.context("Failed to begin transaction for status update")?;
        info!("Updating final status to '{}' for request {}.", new_status.as_str(), request_id);

        sqlx::query!(
            r#"
            UPDATE ExportRequests
            SET
                status = $1,
                file_path = $2,
                completed_at = $3,
                error_message = $4
            WHERE id = $5
            "#,
            new_status.as_str(),
            file_path,
            Some(Utc::now()),
            error_message,
            request_id
        )
        .execute(&mut *tx)
        .await
        .context("Failed to update export request final status in DB")?;

        tx.commit().await.context("Failed to commit final status update transaction")?;
        info!("Final status updated successfully to '{}' for request {}.", new_status.as_str(), request_id);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn update_notification_sent_status(
        &self,
        request_id: Uuid,
        sent: bool,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE ExportRequests SET notification_sent = $1 WHERE id = $2",
            sent,
            request_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to update notification_sent status")?;
        info!("Notification sent status updated to {} for request {}.", sent, request_id);
        Ok(())
    }

    #[instrument(skip(self, params))]
    async fn query_product_data(
        &self,
        params: &ReportParams,
    ) -> Result<Vec<ProductData>> {
        info!("Querying product data with parameters: {:?}", params);
        let raw_data = sqlx::query_as!(
            ProductData,
            r#"
            SELECT
                product_id,
                name,
                category,
                price,
                stock_quantity,
                created_at
            FROM products
            WHERE created_at BETWEEN $1 AND $2
            AND ($3 IS NULL OR category = $3)
            "#,
            params.start_date.and_time(NaiveTime::MIN),
            params.end_date.and_time(NaiveTime::MAX),
            params.product_category,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to query product data from database")?;

        info!("Fetched {} records for export.", raw_data.len());
        Ok(raw_data)
    }
}