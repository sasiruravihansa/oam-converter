# OAM to IaC Converter

This project is a web service that converts Open Application Model (OAM) specifications into Infrastructure as Code (IaC). It takes an OAM file URL and generates IaC for a specified cloud provider and tool.

## Features

- Converts OAM YAML files to IaC.
- Supports multiple cloud providers (GCP, AWS, Azure) and IaC tools (Terraform, OpenTofu, Pulumi, gcloud).
- Generates a zip archive of the IaC files.
- Uploads the generated archive to a configured cloud storage bucket.
- Logs the entire process and includes the log file in the generated archive.
- Provides a JSON API for easy integration.

## Configuration

The application is configured via environment variables. You can place these in a `.env` file in the root of the project.

| Variable | Description |
| --- | --- |
| `DATABASE_URL` | The connection string for the PostgreSQL database. |
| `STORAGE_PROVIDER` | The cloud storage provider to use. Supported values: `gcs`, `s3`, `azure`. Defaults to `gcs`. |
| `GCS_BUCKET` | The name of the Google Cloud Storage bucket to upload the generated files to. |
| `GOOGLE_APPLICATION_CREDENTIALS` | The path to the Google Cloud service account JSON file. |

## Running the Application

1.  **Install Dependencies:**
    ```bash
    cargo build
    ```

2.  **Set up the Database:**
    Make sure you have a PostgreSQL database running and have created the `oam_requests` table:
    ```sql
    CREATE TABLE oam_requests (
        id SERIAL PRIMARY KEY,
        external_id TEXT NOT NULL,
        storage_path TEXT NOT NULL,
        code INTEGER NOT NULL,
        response TEXT NOT NULL,
        created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
    );
    ```

3.  **Run the Application:**
    ```bash
    cargo run
    ```

The server will start on `http://127.0.0.1:8080`.

## API Usage

To trigger a conversion, send a POST request to the `/generate` endpoint:

```bash
curl -X POST http://127.0.0.1:8080/generate \
-H "Content-Type: application/json" \
-d 
    "{
    "oam_url": "https://raw.githubusercontent.com/kubevela/samples/refs/heads/master/01.Helloworld/app.yaml",
    "external_id": "req-123",
    "provider": "gcp",
    "tool": "gcloud"
}"
```

### Response

The service will respond with a JSON object containing a success message and the generated `deploy.sh` script (if applicable):

```json
{
    "message": "Successfully generated and uploaded IaC to gs://your-bucket/req-123/timestamp.zip",
    "deploy_script": "#!/bin/bash\n..."
}
```
