# Deployment Guide for 1.2TB Dataset

Quick reference for deploying Proton Beam to process your 1.2TB Nostr dataset on AWS.

## Configuration

### Instance Specs
- **Instance Type**: c6i.32xlarge
- **vCPUs**: 128
- **RAM**: 256GB
- **EBS Volume**: 5TB (gp3)
- **Expected Time**: ~10 minutes
- **Estimated Cost**:
  - On-Demand: ~$1.10
  - Spot: ~$0.33 (70% savings!)

## Quick Deploy

### Option 1: CloudFormation (Recommended)

```bash
cd /Users/jeff/Code/proton-beam

# Set your configuration
export INPUT_URL="https://dev.primal.net/_share/nostr-events-2025-09-27.jsonl.zst"
export S3_OUTPUT_BUCKET="your-output-bucket"
export S3_OUTPUT_PREFIX="processed-output/"
export KEY_NAME="your-ec2-keypair"

# Optional: Use Spot instance to save 70%
export USE_SPOT_INSTANCE="true"

# Optional: Disable validation for faster processing (~5 min instead of 10)
export VALIDATE_SIGNATURES="false"
export VALIDATE_EVENT_IDS="false"

# Deploy!
./scripts/deploy-cloudformation.sh
```

The defaults are now set for your use case:
- ✅ c6i.32xlarge (128 cores)
- ✅ 5TB EBS volume
- ✅ Auto-shutdown when done
- ✅ All output uploaded to S3

### Option 2: Manual Deployment

```bash
# SSH into your EC2 instance
ssh -i your-key.pem ubuntu@<instance-ip>

# Set environment
export INPUT_URL="https://dev.primal.net/_share/nostr-events-2025-09-27.jsonl.zst"
export S3_OUTPUT="s3://your-output-bucket/processed/"
export PARALLEL_THREADS="128"
export OUTPUT_DIR="/data/pb_data"
export SHUTDOWN_WHEN_DONE="true"

# Optional: Speed optimizations
export VALIDATE_SIGNATURES="false"
export VALIDATE_EVENT_IDS="false"
export COMPRESSION_LEVEL="3"

# Download and run
wget https://raw.githubusercontent.com/parres-hq/proton-beam/master/scripts/aws-deploy.sh
chmod +x aws-deploy.sh
./aws-deploy.sh
```

## File Format

The input file is **Zstandard compressed** (`.jsonl.zst`). The deployment script will:
1. Download the compressed file (~400GB)
2. Decompress to JSONL (~1.2TB)
3. Process the JSONL file
4. Upload compressed protobuf to S3

**Total disk needed**: ~1.6TB during processing (compressed input + decompressed + output)

## Performance Expectations

Based on your local test (14 cores → 80 minutes):

| Configuration | Time | Cost (On-Demand) | Cost (Spot) |
|---------------|------|------------------|-------------|
| Full validation | ~10 min | $1.10 | $0.33 |
| No validation | ~5 min | $0.55 | $0.17 |

**Throughput**: ~2-2.5 million events/second on 128 cores

## Monitoring Progress

### SSH into Instance
```bash
# Get IP from CloudFormation outputs
ssh -i your-key.pem ubuntu@<instance-ip>
```

### Watch Logs
```bash
# Deployment log
tail -f /var/log/proton-beam-deployment.log

# Conversion log
tail -f /data/pb_data/proton-beam.log

# Watch progress
watch -n 1 'grep "Valid:" /data/pb_data/proton-beam.log | tail -1'
```

### Check System Resources
```bash
# CPU usage (should be near 100% across all cores)
htop

# Disk usage
df -h

# I/O stats
iostat -x 1

# Memory
free -h
```

## After Completion

### Verify Upload
```bash
# List files in S3
aws s3 ls s3://your-output-bucket/processed-output/

# Check sizes
aws s3 ls --recursive --human-readable s3://your-output-bucket/processed-output/
```

Expected output:
```
2025-09-27 12:34:56   45.2 GiB  2025_09_27.pb.gz
2025-09-28 12:35:10   48.1 GiB  2025_09_28.pb.gz
...
2025-09-30 12:36:45   42.8 GiB  2025_09_30.pb.gz
2025-09-30 12:37:00  125.5 MiB  index.db
2025-09-30 12:37:01    2.1 MiB  proton-beam.log
```

Total output: ~400-500GB (from 1.2TB JSON, ~60-70% compression)

### Download Results (Optional)
```bash
# Download everything
aws s3 sync s3://your-output-bucket/processed-output/ ./local-backup/

# Or just the index
aws s3 cp s3://your-output-bucket/processed-output/index.db ./
```

## Troubleshooting

### Out of Disk Space
```bash
# Check usage
df -h

# Should show ~5TB available initially
# If running low, the 5TB should be plenty
```

### Slow Performance
```bash
# Check CPU usage
top

# Should see ~12,800% CPU (all 128 cores at 100%)
# If not, check parallel threads:
ps aux | grep proton-beam
```

### S3 Upload Fails
```bash
# Check IAM role
aws sts get-caller-identity

# Test S3 write
echo "test" > /tmp/test.txt
aws s3 cp /tmp/test.txt s3://your-output-bucket/test.txt
```

## Cost Breakdown

### With Spot Instance (Recommended)
```
Compute: 0.17 hrs × $1.92/hr = $0.33
Storage: 5TB × 0.17 hrs × $0.007/hr/GB = $0.06
Data Transfer: Free (within region)
S3 Storage: ~450GB × $0.023/GB/mo = $10.35/month
S3 PUTs: ~100 requests × $0.005/1000 = $0.0005

Total Job Cost: $0.39
Monthly S3 Storage: $10.35
```

### Without Spot (On-Demand)
```
Compute: 0.17 hrs × $6.40/hr = $1.09
Storage: 5TB × 0.17 hrs × $0.007/hr/GB = $0.06

Total Job Cost: $1.15
```

**Recommendation**: Always use Spot for ~70% savings unless you need guaranteed availability.

## Optimization Tips

### Maximum Speed
```bash
export VALIDATE_SIGNATURES="false"
export VALIDATE_EVENT_IDS="false"
export COMPRESSION_LEVEL="1"
export PARALLEL_THREADS="128"
```
**Result**: ~5 minutes, ~$0.17 (Spot)

### Quality & Validation
```bash
export VALIDATE_SIGNATURES="true"
export VALIDATE_EVENT_IDS="true"
export COMPRESSION_LEVEL="6"
export PARALLEL_THREADS="128"
```
**Result**: ~10 minutes, ~$0.33 (Spot)

### Best Compression
```bash
export VALIDATE_SIGNATURES="true"
export VALIDATE_EVENT_IDS="true"
export COMPRESSION_LEVEL="9"
export PARALLEL_THREADS="128"
```
**Result**: ~12 minutes, ~$0.39 (Spot), smaller output files

## Next Steps

After successful processing:

1. **Verify Data Quality**
   ```bash
   # Check event count
   proton-beam inspect s3://your-output-bucket/processed-output/ --count
   ```

2. **Build Queries** (if you need the index locally)
   ```bash
   aws s3 cp s3://your-output-bucket/processed-output/index.db ./
   proton-beam query ./index.db "SELECT COUNT(*) FROM events"
   ```

3. **Archive or Delete Source**
   ```bash
   # Archive source (cheaper storage class)
   aws s3 cp s3://your-bucket/your-1.2TB-file.jsonl \
     s3://your-archive-bucket/ \
     --storage-class GLACIER_IR

   # Delete source if no longer needed
   aws s3 rm s3://your-bucket/your-1.2TB-file.jsonl
   ```

4. **Set S3 Lifecycle Policies**
   - Transition to Glacier after 30 days
   - Delete after 1 year (if applicable)

## Summary

For your 1.2TB dataset:
- ✅ Instance: c6i.32xlarge (128 cores, 256GB RAM)
- ✅ Storage: 5TB EBS (plenty of headroom)
- ✅ Time: ~10 minutes
- ✅ Cost: ~$0.33 (Spot) or ~$1.10 (On-Demand)
- ✅ Output: ~400-500GB compressed protobuf + index

Ready to deploy? Run `./scripts/deploy-cloudformation.sh`! 🚀

