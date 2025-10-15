# Testing AWS Integration

This guide explains how to test the AWS S3 integration and deployment scripts.

## Prerequisites

- AWS account with appropriate permissions
- AWS CLI configured (`aws configure`)
- Rust toolchain installed
- EC2 key pair created

## Local Testing

### 1. Test S3 Module Compilation

```bash
cd /Users/jeff/Code/proton-beam

# Check it compiles
cargo check --features s3 -p proton-beam-cli

# Run tests
cargo test --features s3 -p proton-beam-cli
```

### 2. Test S3 Upload (Local)

Create a test bucket:
```bash
aws s3 mb s3://proton-beam-test-$(whoami)
```

Build and test:
```bash
# Build with S3 support
cargo build --release --features s3 -p proton-beam-cli

# Create test data
echo '{"id":"0000000000000000000000000000000000000000000000000000000000000000","pubkey":"0000000000000000000000000000000000000000000000000000000000000000","created_at":1695686400,"kind":1,"tags":[],"content":"test","sig":"0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"}' > test.jsonl

# Convert and upload
./target/release/proton-beam convert test.jsonl \
  --output-dir /tmp/pb_test \
  --s3-output s3://proton-beam-test-$(whoami)/test1/

# Verify upload
aws s3 ls s3://proton-beam-test-$(whoami)/test1/
```

Clean up:
```bash
rm test.jsonl
rm -rf /tmp/pb_test
aws s3 rb s3://proton-beam-test-$(whoami) --force
```

## Testing Deployment Scripts

### 1. Test Deployment Script (Dry Run)

```bash
cd scripts

# Set minimal test environment
export INPUT_URL="https://raw.githubusercontent.com/nostr-protocol/nostr/master/README.md"
export S3_OUTPUT="s3://proton-beam-test-$(whoami)/"
export SHUTDOWN_WHEN_DONE="false"
export OUTPUT_DIR="/tmp/pb_test"

# Check script syntax
bash -n aws-deploy.sh

# Run with small dataset
./aws-deploy.sh
```

### 2. Test CloudFormation Template (Validation)

```bash
cd scripts

# Validate template syntax
aws cloudformation validate-template \
  --template-body file://aws-cloudformation.yaml

# Check for errors
echo $?  # Should be 0
```

### 3. Test CloudFormation Deployment (Small Instance)

```bash
# Create a minimal test stack
aws cloudformation create-stack \
  --stack-name proton-beam-test \
  --template-body file://scripts/aws-cloudformation.yaml \
  --capabilities CAPABILITY_IAM \
  --parameters \
    ParameterKey=InputURL,ParameterValue="https://example.com/small.jsonl" \
    ParameterKey=S3OutputBucket,ParameterValue="proton-beam-test" \
    ParameterKey=InstanceType,ParameterValue="t3.medium" \
    ParameterKey=VolumeSize,ParameterValue="50" \
    ParameterKey=KeyName,ParameterValue="my-key" \
    ParameterKey=VpcId,ParameterValue="vpc-xxxxx" \
    ParameterKey=SubnetId,ParameterValue="subnet-xxxxx" \
    ParameterKey=ShutdownWhenDone,ParameterValue="true"

# Wait for completion
aws cloudformation wait stack-create-complete \
  --stack-name proton-beam-test

# Get outputs
aws cloudformation describe-stacks \
  --stack-name proton-beam-test \
  --query 'Stacks[0].Outputs'

# Clean up
aws cloudformation delete-stack --stack-name proton-beam-test
```

## Integration Testing

### Test Plan

1. **Small Dataset (Quick Test)**
   - Events: ~1,000
   - Instance: t3.medium
   - Duration: ~5 minutes
   - Cost: ~$0.02

2. **Medium Dataset (Standard Test)**
   - Events: ~100,000
   - Instance: c6i.2xlarge
   - Duration: ~15 minutes
   - Cost: ~$0.50

3. **Large Dataset (Performance Test)**
   - Events: ~10,000,000
   - Instance: c6i.8xlarge
   - Duration: ~30 minutes
   - Cost: ~$1.50

### Test Procedure

```bash
# 1. Create test data (or use existing)
# For testing, use a small subset of real data

# 2. Upload test data to S3
aws s3 cp test-data.jsonl s3://my-test-bucket/input/

# 3. Deploy using CloudFormation
./scripts/deploy-cloudformation.sh

# 4. Monitor progress
# SSH into instance (see CloudFormation outputs)
ssh -i my-key.pem ubuntu@<instance-ip>

# Tail logs
tail -f /var/log/proton-beam-deployment.log

# 5. Verify results
# Wait for completion
aws s3 ls s3://my-test-bucket/output/

# Download and verify
aws s3 cp s3://my-test-bucket/output/ ./verify/ --recursive

# Check files exist
ls -lh verify/*.pb.gz
ls -lh verify/index.db
ls -lh verify/proton-beam.log

# 6. Clean up
aws cloudformation delete-stack --stack-name <stack-name>
aws s3 rm s3://my-test-bucket/output/ --recursive
```

## Unit Tests

### S3 Module Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_s3_uri() {
        let (bucket, prefix) = parse_s3_uri("s3://my-bucket/path/").unwrap();
        assert_eq!(bucket, "my-bucket");
        assert_eq!(prefix, "path/");
    }

    #[tokio::test]
    #[ignore] // Requires AWS credentials
    async fn test_s3_upload() {
        // Create uploader
        let uploader = S3Uploader::new(
            "test-bucket".to_string(),
            "test/".to_string()
        ).await.unwrap();

        // Create test file
        std::fs::write("/tmp/test.txt", "test").unwrap();

        // Upload
        uploader.upload_file(
            Path::new("/tmp/test.txt"),
            "test.txt"
        ).await.unwrap();

        // Clean up
        std::fs::remove_file("/tmp/test.txt").unwrap();
    }
}
```

Run tests:
```bash
# Run non-ignored tests
cargo test --features s3 -p proton-beam-cli

# Run all tests including ignored (requires AWS)
cargo test --features s3 -p proton-beam-cli -- --ignored
```

## Error Testing

### 1. Test Invalid S3 URI

```bash
./target/release/proton-beam convert test.jsonl \
  --s3-output "invalid-uri"

# Expected: Error message about invalid URI format
```

### 2. Test Missing Permissions

```bash
# Temporarily remove S3 permissions from IAM role
./target/release/proton-beam convert test.jsonl \
  --s3-output s3://no-access-bucket/

# Expected: Access denied error
```

### 3. Test Missing Bucket

```bash
./target/release/proton-beam convert test.jsonl \
  --s3-output s3://nonexistent-bucket-12345/

# Expected: Bucket not found error
```

### 4. Test Network Issues

```bash
# Simulate network issues with iptables or firewall
sudo iptables -A OUTPUT -p tcp --dport 443 -j DROP

./target/release/proton-beam convert test.jsonl \
  --s3-output s3://my-bucket/

# Expected: Network error or timeout

# Restore network
sudo iptables -D OUTPUT -p tcp --dport 443 -j DROP
```

## Performance Testing

### Benchmark S3 Upload Speed

```bash
# Create large test file
dd if=/dev/urandom of=/tmp/large-test.pb.gz bs=1M count=100

# Time upload
time ./target/release/proton-beam convert test.jsonl \
  --output-dir /tmp \
  --s3-output s3://my-bucket/perf-test/

# Clean up
rm /tmp/large-test.pb.gz
aws s3 rm s3://my-bucket/perf-test/ --recursive
```

### Stress Test

```bash
# Process large dataset with monitoring
export INPUT_URL="s3://large-dataset/events.jsonl"
export S3_OUTPUT="s3://output-bucket/stress-test/"
export PARALLEL_THREADS="64"

# Monitor resources
watch -n 1 'free -h && df -h && uptime'

# Run conversion
./scripts/aws-deploy.sh
```

## Regression Testing

### Test Matrix

| Test | Dataset Size | Instance Type | Validation | S3 Upload | Expected Result |
|------|--------------|---------------|------------|-----------|----------------|
| 1 | 1K events | t3.small | On | Yes | Success |
| 2 | 100K events | c6i.2xlarge | On | Yes | Success |
| 3 | 1M events | c6i.4xlarge | Off | Yes | Success (faster) |
| 4 | 10M events | c6i.8xlarge | On | Yes | Success |
| 5 | 100M events | c6i.16xlarge | Off | Yes | Success |

### Automated Testing Script

```bash
#!/bin/bash
# regression-test.sh

TESTS=(
  "1000:t3.small:true:test1"
  "100000:c6i.2xlarge:true:test2"
  "1000000:c6i.4xlarge:false:test3"
)

for test in "${TESTS[@]}"; do
  IFS=':' read -r size instance validate name <<< "$test"

  echo "Running test: $name"

  export INPUT_URL="s3://test-data/${size}.jsonl"
  export S3_OUTPUT="s3://results/${name}/"
  export INSTANCE_TYPE="$instance"
  export VALIDATE_SIGNATURES="$validate"
  export SHUTDOWN_WHEN_DONE="true"

  ./scripts/deploy-cloudformation.sh

  # Wait and verify
  sleep 300  # 5 minutes
  aws s3 ls s3://results/${name}/

  echo "Test $name complete"
done
```

## Troubleshooting Tests

### Common Issues

**1. Compilation fails with S3 feature**
```bash
# Clean and rebuild
cargo clean
cargo build --release --features s3 -p proton-beam-cli
```

**2. AWS credentials not found**
```bash
# Check credentials
aws sts get-caller-identity

# Configure if needed
aws configure
```

**3. S3 upload timeout**
```bash
# Increase timeout (if implemented)
export AWS_TIMEOUT_SECONDS=300
```

**4. Instance fails to launch**
```bash
# Check CloudFormation events
aws cloudformation describe-stack-events \
  --stack-name proton-beam-test \
  --max-items 10
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Test AWS Integration

on:
  pull_request:
    paths:
      - 'proton-beam-cli/src/s3.rs'
      - 'scripts/aws-*.sh'
      - 'scripts/aws-cloudformation.yaml'

jobs:
  test-compilation:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Check S3 feature compiles
        run: cargo check --features s3 -p proton-beam-cli
      - name: Run tests
        run: cargo test --features s3 -p proton-beam-cli

  test-scripts:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Validate bash syntax
        run: |
          bash -n scripts/aws-deploy.sh
          bash -n scripts/aws-userdata.sh
      - name: Validate CloudFormation
        run: |
          pip install cfn-lint
          cfn-lint scripts/aws-cloudformation.yaml
```

## Checklist

Before deploying to production, verify:

- [ ] S3 feature compiles without warnings
- [ ] Unit tests pass
- [ ] Deployment script runs successfully
- [ ] CloudFormation template validates
- [ ] Small dataset test completes successfully
- [ ] S3 files uploaded correctly
- [ ] Index database created
- [ ] Logs written properly
- [ ] Auto-shutdown works (if enabled)
- [ ] IAM permissions are correct
- [ ] Cost estimates are reasonable
- [ ] Documentation is accurate

## Cost of Testing

Estimated costs for testing:

| Test Type | Instance | Duration | Cost |
|-----------|----------|----------|------|
| Unit tests | Local | - | Free |
| Small test | t3.medium | 5 min | $0.02 |
| Medium test | c6i.2xlarge | 15 min | $0.50 |
| Large test | c6i.8xlarge | 30 min | $1.50 |

Total: ~$2.00 for comprehensive testing

**Tip**: Use AWS Free Tier or educational credits for testing!

## Next Steps

After successful testing:

1. Tag release version
2. Update documentation
3. Create deployment guide for users
4. Set up monitoring for production
5. Configure alerts for failures
6. Document lessons learned

## Support

If tests fail:
- Check logs in `/var/log/proton-beam-deployment.log`
- Review CloudFormation events
- Verify IAM permissions
- Check S3 bucket policies
- Open an issue on GitHub

