use anyhow::{Context, Result};
use std::path::Path;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::ProductData;

/// Trait định nghĩa giao diện cho việc tạo và lưu file Excel.
#[async_trait::async_trait]
pub trait FileExporter: Send + Sync + 'static {
    async fn export_to_excel(
        &self,
        request_id: Uuid,
        data: Vec<ProductData>,
        export_path: &str,
    ) -> Result<String>; // Trả về đường dẫn đầy đủ của file đã tạo
}

/// Implementation cụ thể để tạo và lưu file Excel cục bộ.
pub struct LocalFileExporter;

#[async_trait::async_trait]
impl FileExporter for LocalFileExporter {
    #[instrument(skip(self, data, export_path), fields(request_id = %request_id))]
    async fn export_to_excel(
        &self,
        request_id: Uuid,
        data: Vec<ProductData>,
        export_path: &str,
    ) -> Result<String> {
        let filename = format!("{}.xlsx", request_id);
        let full_path = format!("{}/{}", export_path, filename);

        tokio::fs::create_dir_all(export_path)
            .await
            .context("Failed to create export directory")?;

        #[cfg(feature = "xlsxwriter")]
        {
            use xlsxwriter::Workbook;
            info!("Creating Excel file at: {}", full_path);
            let workbook = Workbook::new(&full_path)?;
            let mut sheet = workbook.add_worksheet(None)?;

            // Write header
            sheet.write_string(0, 0, "Product ID", None)?;
            sheet.write_string(0, 1, "Name", None)?;
            sheet.write_string(0, 2, "Category", None)?;
            sheet.write_string(0, 3, "Price", None)?;
            sheet.write_string(0, 4, "Stock Quantity", None)?;
            sheet.write_string(0, 5, "Created At", None)?;

            // Write data
            for (i, row) in data.iter().enumerate() {
                let row_num = (i + 1) as u32;
                sheet.write_number(row_num, 0, row.product_id as f64, None)?;
                sheet.write_string(row_num, 1, &row.name, None)?;
                sheet.write_string(row_num, 2, &row.category, None)?;
                sheet.write_number(row_num, 3, row.price, None)?;
                sheet.write_number(row_num, 4, row.stock_quantity as f64, None)?;
                sheet.write_string(row_num, 5, &row.created_at.to_string(), None)?;
            }

            workbook.close().context("Failed to close Excel workbook")?;
            info!("✅ Excel file successfully created at: {}", full_path);
        }
        #[cfg(not(feature = "xlsxwriter"))]
        {
            warn!("`xlsxwriter` feature not enabled. Using placeholder file creation. For full functionality, enable it in Cargo.toml.");
            tokio::fs::write(&full_path, format!("Placeholder Excel content for request {}.\n", request_id))
                .await
                .context("Failed to write placeholder Excel file")?;
            for row in data {
                tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&full_path)
                    .await?
                    .write_all(format!("{:?}\n", row).as_bytes())
                    .await?;
            }
            info!("✅ Placeholder file created at: {}", full_path);
        }

        Ok(full_path)
    }
}