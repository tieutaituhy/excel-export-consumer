use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExportRequest {
    pub id: Uuid,
    pub user_id: i64,
    pub request_payload: serde_json::Value, // JSONB
    pub requested_at: DateTime<Utc>,
    pub status: String,
    pub file_path: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub notification_sent: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportParams {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub product_category: Option<String>,
    // Thêm các trường khác tùy theo yêu cầu của bạn
}

#[derive(Debug, Serialize)]
pub struct ExportNotification {
    pub request_id: Uuid,
    pub status: String,
    pub file_url: Option<String>, // URL công khai của file Excel
    pub error_message: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct ProductData {
    pub product_id: i64,
    pub name: String,
    pub category: String,
    pub price: f64,
    pub stock_quantity: i32,
    pub created_at: DateTime<Utc>,
}

pub enum ExportStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl ExportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportStatus::Pending => "PENDING",
            ExportStatus::Processing => "PROCESSING",
            ExportStatus::Completed => "COMPLETED",
            ExportStatus::Failed => "FAILED",
        }
    }
}