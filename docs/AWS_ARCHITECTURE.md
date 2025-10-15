# AWS Architecture

This document describes the AWS architecture for Proton Beam deployments.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                          AWS Cloud                               │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │                    VPC (Virtual Private Cloud)          │    │
│  │                                                         │    │
│  │  ┌──────────────────────────────────────────────┐     │    │
│  │  │            Public Subnet                      │     │    │
│  │  │                                               │     │    │
│  │  │  ┌─────────────────────────────────────┐    │     │    │
│  │  │  │    EC2 Instance (c6i.8xlarge)       │    │     │    │
│  │  │  │                                      │    │     │    │
│  │  │  │  ┌────────────────────────────┐    │    │     │    │
│  │  │  │  │   Proton Beam CLI          │    │    │     │    │
│  │  │  │  │   - Download from S3/HTTP  │    │    │     │    │
│  │  │  │  │   - Convert to protobuf    │    │    │     │    │
│  │  │  │  │   - Build index            │    │    │     │    │
│  │  │  │  │   - Upload to S3           │    │    │     │    │
│  │  │  │  └────────────────────────────┘    │    │     │    │
│  │  │  │                                      │    │     │    │
│  │  │  │  Attached IAM Role:                 │    │     │    │
│  │  │  │  - S3 Read/Write                    │    │     │    │
│  │  │  │  - CloudWatch Logs                  │    │     │    │
│  │  │  │                                      │    │     │    │
│  │  │  │  EBS Volume (gp3):                  │    │     │    │
│  │  │  │  - 500GB - 2TB                      │    │     │    │
│  │  │  │  - 3000+ IOPS                       │    │     │    │
│  │  │  └─────────────────────────────────────┘    │     │    │
│  │  │                                               │     │    │
│  │  └───────────────────────────────────────────────┘     │    │
│  │                                                         │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    S3 Buckets                            │   │
│  │                                                          │   │
│  │  ┌──────────────────┐      ┌──────────────────────┐    │   │
│  │  │  Input Bucket    │      │   Output Bucket       │    │   │
│  │  │  (Optional)      │      │   (Required)          │    │   │
│  │  │                  │      │                       │    │   │
│  │  │  - events.jsonl  │      │  - 2025_09_27.pb.gz  │    │   │
│  │  │  - data.jsonl    │      │  - 2025_09_28.pb.gz  │    │   │
│  │  │                  │      │  - index.db           │    │   │
│  │  │                  │      │  - proton-beam.log    │    │   │
│  │  └──────────────────┘      └──────────────────────┘    │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              CloudWatch (Optional)                       │   │
│  │  - Logs                                                  │   │
│  │  - Metrics                                               │   │
│  │  - Alarms                                                │   │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘

┌────────────────────┐
│   External Data    │
│   Sources          │
│   (Optional)       │
│                    │
│   - HTTP/HTTPS     │────► Downloaded by EC2
│   - Archive.org    │
│   - Other APIs     │
└────────────────────┘
```

## Component Details

### EC2 Instance

**Purpose**: Compute resource for processing

**Recommended Types**:
- c6i family (compute-optimized)
- c7i family (latest generation)
- 8xlarge or larger for production

**Configuration**:
- Ubuntu 22.04 LTS
- IAM role attached (no access keys needed)
- EBS gp3 volume for storage
- Public IP for internet access
- Security group for SSH (optional)

### EBS Volume

**Purpose**: Temporary storage during processing

**Configuration**:
- Type: gp3 (general purpose SSD)
- Size: 500GB - 2TB (depends on dataset)
- IOPS: 3000+ for optimal performance
- Throughput: 125+ MB/s

**Usage**:
- Input file download
- Temporary conversion files
- Output protobuf files
- Index database
- Can be deleted after upload to S3

### S3 Buckets

**Input Bucket** (Optional):
- Stores source JSONL files
- Read-only access needed
- Can be in different AWS account

**Output Bucket** (Required):
- Stores converted protobuf files
- Stores index database
- Stores log files
- Write access required
- Can enable versioning for safety

### IAM Role

**Purpose**: Grant EC2 instance access to AWS services

**Attached Policies**:
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["s3:GetObject"],
      "Resource": "arn:aws:s3:::input-bucket/*"
    },
    {
      "Effect": "Allow",
      "Action": ["s3:PutObject"],
      "Resource": "arn:aws:s3:::output-bucket/*"
    }
  ]
}
```

### Security Group

**Inbound Rules**:
- SSH (22) from trusted IPs (optional, for debugging)

**Outbound Rules**:
- All traffic allowed (for downloading and S3 upload)

## Data Flow

### 1. Initialization

```
User → CloudFormation → EC2 Launch
                      ↓
                User Data Script Executes
                      ↓
                Install Dependencies
                      ↓
                Build Proton Beam
```

### 2. Data Ingestion

```
HTTP/S3 Source → Download → EC2 Local Storage
                              (/tmp/input.jsonl)
```

### 3. Processing

```
Input JSONL
    ↓
Parse JSON (multi-threaded)
    ↓
Validate (optional)
    ↓
Convert to Protobuf
    ↓
Write to dated files
    ↓
/data/pb_data/2025_09_27.pb.gz
```

### 4. Indexing

```
Protobuf Files
    ↓
Read all events
    ↓
Extract metadata (id, pubkey, kind, timestamp)
    ↓
Insert into SQLite
    ↓
/data/pb_data/index.db
```

### 5. Upload to S3

```
Local Files → S3 Upload (async) → S3 Bucket
    ↓                                  ↓
*.pb.gz files              s3://bucket/prefix/*.pb.gz
index.db                   s3://bucket/prefix/index.db
proton-beam.log            s3://bucket/prefix/proton-beam.log
```

### 6. Cleanup & Shutdown

```
Delete Local Files
    ↓
Shutdown Instance (optional)
    ↓
Stop Billing
```

## Network Flow

```
                Internet
                    ↕
            Internet Gateway
                    ↕
               VPC Router
                    ↕
              Public Subnet
                    ↕
            EC2 Instance
                    ↕
        ┌───────────┴───────────┐
        ↓                       ↓
    S3 Endpoint             HTTP(S) Download
    (via AWS Network)       (via Internet)
        ↓                       ↓
    Output Bucket           Input Source
```

## Processing Workflow

### Sequential Mode (Single Thread)

```
Read Line 1 → Parse → Validate → Convert → Write
Read Line 2 → Parse → Validate → Convert → Write
Read Line 3 → Parse → Validate → Convert → Write
...
```

### Parallel Mode (Multi-Thread)

```
Thread 1: Lines 1-1000     → Process → Write temp file 1
Thread 2: Lines 1001-2000  → Process → Write temp file 2
Thread 3: Lines 2001-3000  → Process → Write temp file 3
...
Thread N: Lines N-end      → Process → Write temp file N
                                            ↓
                                    Merge temp files
                                            ↓
                                    Deduplicate
                                            ↓
                                    Final output files
```

## Scaling Strategies

### Vertical Scaling (Single Instance)

```
Small Dataset  → c6i.2xlarge  (8 vCPUs)
Medium Dataset → c6i.4xlarge  (16 vCPUs)
Large Dataset  → c6i.8xlarge  (32 vCPUs)
Huge Dataset   → c6i.16xlarge (64 vCPUs)
```

### Horizontal Scaling (Multiple Instances)

```
Dataset 1 → Instance 1 → S3://bucket/dataset1/
Dataset 2 → Instance 2 → S3://bucket/dataset2/
Dataset 3 → Instance 3 → S3://bucket/dataset3/
Dataset 4 → Instance 4 → S3://bucket/dataset4/
```

Or split single dataset:

```
Lines 1-1B   → Instance 1 → S3://bucket/part1/
Lines 1B-2B  → Instance 2 → S3://bucket/part2/
Lines 2B-3B  → Instance 3 → S3://bucket/part3/
```

## Cost Breakdown

### Example: Processing 100M Events

**Compute** (c6i.8xlarge):
- Duration: 2 hours
- Rate: $1.36/hour
- Cost: $2.72

**Storage** (500GB gp3):
- Duration: 2 hours
- Rate: $0.08/GB/month
- Cost: $0.011/hour × 2 = $0.02

**Data Transfer**:
- Download: Free (in from internet)
- Upload to S3: Free (within region)
- S3 Storage: ~50GB
- Rate: $0.023/GB/month
- Cost: $1.15/month

**S3 Requests**:
- PUTs: ~100 files
- Rate: $0.005/1000 requests
- Cost: $0.0005

**Total**: ~$2.75 one-time + $1.15/month storage

**With Spot Instance**: ~$0.82 one-time + $1.15/month storage (70% savings)

## Disaster Recovery

### Failure Scenarios

**1. Instance Terminated During Processing**
- Data lost if not yet uploaded to S3
- Solution: Regular S3 sync during processing (future enhancement)

**2. S3 Upload Failure**
- Files remain on instance
- Solution: Retry logic in code, or manual upload

**3. Out of Disk Space**
- Processing fails
- Solution: Monitor disk usage, increase volume size

**4. Network Interruption**
- Download or upload fails
- Solution: Retry with exponential backoff

### Best Practices

1. **Use Spot Instances** for cost savings (accept interruption risk)
2. **Enable S3 Versioning** to protect against overwrites
3. **Set CloudWatch Alarms** for disk/CPU/memory
4. **Tag Resources** for cost tracking
5. **Use Auto-Shutdown** to avoid idle charges
6. **Keep Logs** in S3 for debugging
7. **Test with Small Dataset** first

## Monitoring Dashboard

```
┌─────────────────────────────────────────────────────────┐
│                  CloudWatch Dashboard                    │
│                                                          │
│  ┌──────────────────────┐  ┌──────────────────────┐    │
│  │   CPU Utilization    │  │   Memory Usage       │    │
│  │   ▂▃▅▆▇█████████     │  │   ▃▄▅▆▇████████      │    │
│  │   85%                │  │   60%                │    │
│  └──────────────────────┘  └──────────────────────┘    │
│                                                          │
│  ┌──────────────────────┐  ┌──────────────────────┐    │
│  │   Disk I/O           │  │   Network            │    │
│  │   ▃▄▆▇██████▇▆▄      │  │   ▂▃▄▅▆▇████▆▅▄▃     │    │
│  │   1500 IOPS          │  │   250 Mbps           │    │
│  └──────────────────────┘  └──────────────────────┘    │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │   S3 Operations                                   │  │
│  │   PUTs: 42 | GETs: 5 | Total: 2.3 GB uploaded   │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

## Security Architecture

```
┌─────────────────────────────────────────────────┐
│                  Security Layers                 │
│                                                  │
│  1. Network Security                            │
│     └─ Security Groups (Firewall)               │
│     └─ VPC Isolation                            │
│                                                  │
│  2. Access Control                              │
│     └─ IAM Roles (No access keys!)              │
│     └─ S3 Bucket Policies                       │
│     └─ Resource Tags                            │
│                                                  │
│  3. Data Protection                             │
│     └─ S3 Server-Side Encryption                │
│     └─ EBS Volume Encryption                    │
│     └─ HTTPS for data transfer                  │
│                                                  │
│  4. Audit & Compliance                          │
│     └─ CloudTrail Logging                       │
│     └─ S3 Access Logs                           │
│     └─ VPC Flow Logs                            │
│                                                  │
└──────────────────────────────────────────────────┘
```

## Deployment Methods Comparison

### 1. CloudFormation (Infrastructure as Code)
```
User → Parameters → CloudFormation → Stack Creation
                                          ↓
                                    All Resources
                                    Created & Configured
```

**Pros**:
- Repeatable
- Version controlled
- Automatic rollback
- Easy cleanup

**Cons**:
- YAML syntax to learn
- Less flexible for ad-hoc changes

### 2. Manual Deployment
```
User → Launch Instance → SSH → Download Script → Execute
```

**Pros**:
- Full control
- Easy debugging
- No templates needed

**Cons**:
- Manual steps
- Error-prone
- Not repeatable

### 3. User Data (Fully Automated)
```
User → Launch with User Data → Instance Self-Configures
```

**Pros**:
- Zero-touch deployment
- Fast setup
- Works with auto-scaling

**Cons**:
- Hard to debug
- No interactive input
- Requires pre-configuration

## Summary

This AWS architecture provides:
- ✅ Scalable compute (EC2)
- ✅ Durable storage (S3)
- ✅ Secure access (IAM)
- ✅ Cost-effective (Spot instances)
- ✅ Automated deployment (CloudFormation)
- ✅ Production-ready (monitoring, logging)

Perfect for processing large Nostr datasets in the cloud! 🚀

