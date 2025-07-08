# Excel Export Consumer (Rust)

## Giới thiệu

`excel-export-consumer_Rust` là một service tiêu thụ message từ Kafka, thực hiện xuất dữ liệu ra file Excel, lưu trữ file, cập nhật trạng thái vào database và gửi thông báo qua HTTP API. Service này hỗ trợ logging, metrics Prometheus, cấu hình linh hoạt qua file `.env` và dễ dàng mở rộng.

## Kiến trúc tổng quan

- **Kafka Consumer**: Lắng nghe các yêu cầu xuất Excel từ topic Kafka.
- **Database (PostgreSQL)**: Lưu trữ thông tin trạng thái xuất file.
- **File Exporter**: Xuất dữ liệu ra file Excel và lưu vào thư mục cấu hình.
- **Notifier**: Gửi thông báo (thành công/thất bại) tới API ngoài (ASP.NET hoặc service khác).
- **Prometheus Metrics**: Expose metrics cho Prometheus scrape.
- **Logging**: Ghi log chi tiết vào file theo ngày.

## Sơ đồ luồng xử lý

1. Nhận message từ Kafka.
2. Lấy dữ liệu từ database (nếu cần).
3. Xuất file Excel và lưu vào thư mục cấu hình.
4. Cập nhật trạng thái vào database.
5. Gửi thông báo qua HTTP API.
6. Ghi log và expose metrics.

## Cấu hình

Tạo file `.env` ở thư mục gốc với nội dung mẫu:

```env
KAFKA_BROKERS=localhost:9092
KAFKA_TOPIC=excel_export_requests
DATABASE_URL=postgres://user:password@localhost:5432/your_database
NOTIFICATION_SERVICE_URL=http://localhost:5000/api/notifications
EXCEL_EXPORT_PATH=/app/exports
METRICS_LISTEN_ADDRESS=0.0.0.0:9000
```

**Chú thích:**
- `KAFKA_BROKERS`: Địa chỉ Kafka cluster.
- `KAFKA_TOPIC`: Tên topic nhận yêu cầu xuất Excel.
- `DATABASE_URL`: Chuỗi kết nối PostgreSQL.
- `NOTIFICATION_SERVICE_URL`: API nhận thông báo trạng thái.
- `EXCEL_EXPORT_PATH`: Thư mục lưu file Excel.
- `METRICS_LISTEN_ADDRESS`: Địa chỉ expose metrics cho Prometheus.

## Hướng dẫn chạy dự án

### 1. Cài đặt Rust và Cargo

- [Rust Install Guide](https://www.rust-lang.org/tools/install)

### 2. Cài đặt các dependency

```bash
cargo build
```

### 3. Thiết lập các service phụ trợ

- Đảm bảo Kafka, PostgreSQL, Notification API đã chạy và đúng cấu hình trong `.env`.
- Tạo thư mục lưu file Excel (nếu chưa có) và cấp quyền ghi.

### 4. Chạy service

```bash
cargo run --release
```

### 5. Kiểm tra log và metrics

- Log: Thư mục `logs/consumer.log` (log theo ngày).
- Metrics: Truy cập `http://<host>:9000/metrics` để xem Prometheus metrics.

## Docker

Dự án có thể build và chạy bằng Docker. Ví dụ:

```bash
docker build -t excel-export-consumer .
docker run --env-file .env -v /path/to/exports:/app/exports excel-export-consumer
```

## Cấu trúc thư mục

```
src/
  config.rs           // Đọc và quản lý cấu hình
  kafka_consumer.rs   // Lắng nghe và xử lý message Kafka
  models.rs           // Định nghĩa model dữ liệu
  services/
    db_store.rs       // Tương tác database
    export_service.rs // Logic xuất file Excel
    file_exporter.rs  // Lưu file Excel
    notifier.rs       // Gửi thông báo HTTP
main.rs               // Điểm khởi động ứng dụng
```

## Đóng góp

- Fork, tạo branch mới, gửi pull request.
- Vui lòng viết log, comment rõ ràng và tuân thủ chuẩn Rust.

## License

MIT License.
