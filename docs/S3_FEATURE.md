# S3 Upload Feature

This document describes the S3 upload feature added to Proton Beam.

## Overview

Proton Beam now supports direct upload to AWS S3 buckets after conversion and indexing. This is particularly useful for cloud-based processing workflows and AWS deployments.

## Building with S3 Support

The S3 feature is optional and must be enabled at build time:

```bash
cargo build --release --features s3 -p proton-beam-cli
```

## Usage

### Convert and Upload

```bash
proton-beam convert events.jsonl \
  --output-dir ./pb_data \
  --s3-output s3://my-bucket/path/to/output/
```

This will:
1. Convert events to protobuf
2. Save them locally in `./pb_data`
3. Upload all `.pb.gz` files to S3
4. Upload the log file to S3

### Index and Upload

```bash
proton-beam index rebuild ./pb_data \
  --s3-output s3://my-bucket/path/to/output/
```

This will:
1. Rebuild the index from protobuf files
2. Upload all `.pb.gz` files to S3
3. Upload the `index.db` to S3
4. Upload the log file to S3

## S3 URI Format

S3 URIs follow the standard format:

```
s3://bucket-name/prefix/path/
```

Examples:
- `s3://my-bucket/` - Root of bucket
- `s3://my-bucket/events/` - With prefix
- `s3://my-bucket/nostr/2025/01/` - Nested path

## IAM Permissions Required

The AWS credentials (from environment, IAM role, or config file) need these permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:PutObjectAcl"
      ],
      "Resource": "arn:aws:s3:::your-bucket/*"
    }
  ]
}
```

## AWS Credentials

Proton Beam uses the AWS SDK, which automatically looks for credentials in this order:

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. AWS credentials file (`~/.aws/credentials`)
3. IAM role (when running on EC2)
4. ECS container credentials

### Setting Credentials

**Option 1: Environment Variables**
```bash
export AWS_ACCESS_KEY_ID="your-key-id"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
export AWS_REGION="us-east-1"
```

**Option 2: AWS CLI Configuration**
```bash
aws configure
```

**Option 3: IAM Role (Recommended for EC2)**
- Attach an IAM role to your EC2 instance with S3 permissions
- No credentials needed in code or environment

## What Gets Uploaded

The upload includes:

1. **Protobuf Files** (`.pb.gz`)
   - Date-organized event files
   - Example: `2025_09_27.pb.gz`

2. **Index Database** (`index.db`)
   - SQLite database with event metadata
   - Enables fast queries

3. **Log File** (`proton-beam.log`)
   - Processing logs and errors
   - Useful for debugging

**Not uploaded:**
- Temporary files (`.tmp`)
- Input files
- Other files in the output directory

## Implementation Details

### Module: `src/s3.rs`

The S3 functionality is implemented in a separate module:

```rust
use proton_beam::s3::{S3Uploader, parse_s3_uri};

// Parse S3 URI
let (bucket, prefix) = parse_s3_uri("s3://my-bucket/path/")?;

// Create uploader
let uploader = S3Uploader::new(bucket, prefix).await?;

// Upload all files
uploader.upload_all(&output_dir).await?;
```

### Feature Flag

S3 support is behind a feature flag to keep dependencies optional:

```toml
[features]
s3 = ["aws-config", "aws-sdk-s3"]
```

When the feature is disabled:
- Binary is smaller
- No AWS dependencies
- S3 flags show a warning message

## Error Handling

Common errors and solutions:

**Access Denied**
```
Error: Failed to upload to s3://bucket/file
```
→ Check IAM permissions and bucket name

**Invalid URI**
```
Error: S3 URI must start with s3://
```
→ Use correct format: `s3://bucket/path`

**Network Error**
```
Error: Failed to connect to S3
```
→ Check internet connectivity and AWS region

## Performance

Upload performance depends on:
- **File Size**: Larger files take longer
- **Network Speed**: AWS region proximity matters
- **Compression**: Higher compression = smaller uploads
- **Parallelism**: Uploads happen sequentially per file

Typical upload speeds:
- 10MB/s - 50MB/s for small files
- 50MB/s - 200MB/s for large files
- Depends on instance type and region

## Examples

### Basic Usage
```bash
proton-beam convert events.jsonl --s3-output s3://my-bucket/
```

### With Custom Output Directory
```bash
proton-beam convert events.jsonl \
  --output-dir /mnt/nvme/pb_data \
  --s3-output s3://my-bucket/processed/
```

### Disable Validation for Speed
```bash
proton-beam convert events.jsonl \
  --validate-signatures false \
  --parallel 32 \
  --s3-output s3://my-bucket/output/
```

### AWS Deployment (Automatic)
```bash
export INPUT_URL="https://example.com/events.jsonl"
export S3_OUTPUT="s3://my-bucket/output/"
./scripts/aws-deploy.sh
```

## Testing

To test S3 upload without AWS:

1. **Use LocalStack** (S3 mock)
   ```bash
   docker run -p 4566:4566 localstack/localstack
   export AWS_ENDPOINT_URL=http://localhost:4566
   ```

2. **Use MinIO** (S3-compatible)
   ```bash
   docker run -p 9000:9000 minio/minio server /data
   ```

3. **Dry Run** (not implemented yet)
   Would be nice to have `--s3-dry-run` flag

## Future Enhancements

Potential improvements:

- [ ] Parallel uploads for faster performance
- [ ] Resume interrupted uploads
- [ ] S3 download support (read from S3)
- [ ] Multipart upload for large files (>5GB)
- [ ] Progress bar for uploads
- [ ] Support for other cloud providers (GCS, Azure Blob)
- [ ] Encryption at rest (SSE-S3, SSE-KMS)
- [ ] Custom storage class selection

## See Also

- [AWS Deployment Guide](AWS_DEPLOYMENT.md)
- [CLI README](../examples/CLI_README.md)
- [Architecture](ARCHITECTURE.md)

