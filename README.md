# Excel Export Consumer (Rust)

## Introduction

`excel-export-consumer_Rust` is a service that consumes messages from Kafka, exports data to Excel files, stores the files, updates status in the database, and sends notifications via HTTP API. This service supports logging, Prometheus metrics, flexible configuration via `.env`, and is easy to extend.

## Architecture Overview

- **Kafka Consumer**: Listens for Excel export requests from a Kafka topic.
- **Database (PostgreSQL)**: Stores export status information.
- **File Exporter**: Exports data to Excel files and saves them to a configured directory.
- **Notifier**: Sends notifications (success/failure) to an external API (such as ASP.NET or other services).
- **Prometheus Metrics**: Exposes metrics for Prometheus scraping.
- **Logging**: Writes detailed logs to daily log files.

## Processing Flow

1. Receive messages from Kafka.
2. Fetch data from the database (if needed).
3. Export Excel file and save to the configured directory.
4. Update status in the database.
5. Send notification via HTTP API.
6. Write logs and expose metrics.

## Configuration

Create a `.env` file in the project root with the following sample content:

```env
KAFKA_BROKERS=localhost:9092
KAFKA_TOPIC=excel_export_requests
DATABASE_URL=postgres://user:password@localhost:5432/your_database
NOTIFICATION_SERVICE_URL=http://localhost:5000/api/notifications
EXCEL_EXPORT_PATH=/app/exports
METRICS_LISTEN_ADDRESS=0.0.0.0:9000
```

**Notes:**
- `KAFKA_BROKERS`: Kafka cluster address.
- `KAFKA_TOPIC`: Topic name for Excel export requests.
- `DATABASE_URL`: PostgreSQL connection string.
- `NOTIFICATION_SERVICE_URL`: API for receiving export status notifications.
- `EXCEL_EXPORT_PATH`: Directory to store exported Excel files.
- `METRICS_LISTEN_ADDRESS`: Address to expose Prometheus metrics.

## How to Run

### 1. Install Rust and Cargo

- [Rust Install Guide](https://www.rust-lang.org/tools/install)

### 2. Install dependencies

```bash
cargo build
```

### 3. Set up supporting services

- Ensure Kafka, PostgreSQL, and Notification API are running and match the configuration in `.env`.
- Create the export directory (if not exists) and grant write permissions.

### 4. Run the service

```bash
cargo run --release
```

### 5. Check logs and metrics

- Logs: `logs/consumer.log` directory (daily logs).
- Metrics: Visit `http://<host>:9000/metrics` for Prometheus metrics.

## Docker

You can build and run the project with Docker. Example:

```bash
docker build -t excel-export-consumer .
docker run --env-file .env -v /path/to/exports:/app/exports excel-export-consumer
```

## Project Structure

```
src/
  config.rs           // Configuration management
  kafka_consumer.rs   // Kafka message listening and processing
  models.rs           // Data models
  services/
    db_store.rs       // Database interaction
    export_service.rs // Excel export logic
    file_exporter.rs  // Excel file storage
    notifier.rs       // HTTP notification sender
main.rs               // Application entry point
```

## Contribution

- Fork, create a new branch, and submit a pull request.
- Please write clear logs, comments, and follow Rust best practices.

## License

MIT License.
