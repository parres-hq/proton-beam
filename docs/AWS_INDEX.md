# AWS Documentation Index

Complete guide to deploying Proton Beam on AWS using CloudFormation.

## Quick Links

| Guide | Description | For |
|-------|-------------|-----|
| [Quick Start](../QUICKSTART_CLOUDFORMATION.md) | 3-step deployment | First-time users |
| [Complete Guide](../CLOUDFORMATION_GUIDE.md) | Full documentation | Detailed reference |
| [1.2TB Dataset](../DEPLOYMENT_1.2TB.md) | Specific example | Large datasets |

## Overview

Proton Beam uses **AWS CloudFormation** for automated deployment:

1. **Define** infrastructure in a template (YAML)
2. **Deploy** with one command
3. **Process** automatically on EC2
4. **Upload** results to S3
5. **Clean up** everything when done

## Architecture

```
CloudFormation Stack
├─ EC2 Instance (c6i.32xlarge, 128 cores)
├─ EBS Volume (5TB storage)
├─ IAM Role (S3 permissions)
├─ Security Group (SSH access)
└─ Auto-executes deployment script
      ↓
   Downloads & decompresses input
      ↓
   Converts to protobuf
      ↓
   Uploads to S3
      ↓
   Shuts down
```

## Features

- ✅ **Fully Automated** - Zero manual EC2 setup
- ✅ **Compression Support** - Auto-decompresses .zst, .gz, .xz files
- ✅ **S3 Integration** - Direct upload after processing
- ✅ **Cost Optimized** - Spot instances, auto-shutdown
- ✅ **Reproducible** - Same result every time
- ✅ **Clean** - Deletes all resources when done

## Files

### Scripts
- `scripts/aws-cloudformation.yaml` - CloudFormation template
- `scripts/deploy-cloudformation.sh` - Interactive deployment helper
- `scripts/aws-deploy.sh` - Instance bootstrap script (used by template)
- `scripts/iam-policy-example.json` - Example IAM policy

### Documentation
- `QUICKSTART_CLOUDFORMATION.md` - One-page quick start
- `CLOUDFORMATION_GUIDE.md` - Complete guide (7,000+ words)
- `DEPLOYMENT_1.2TB.md` - Example for 1.2TB dataset
- `docs/AWS_ARCHITECTURE.md` - Architecture diagrams
- `docs/S3_FEATURE.md` - S3 upload feature documentation
- `docs/COMPRESSION_SUPPORT.md` - Compression format details
- `docs/TESTING_AWS.md` - Testing procedures

## Quick Start

```bash
# 1. Prerequisites (once)
aws ec2 create-key-pair --key-name my-key \
  --query 'KeyMaterial' --output text > ~/.ssh/my-key.pem
chmod 400 ~/.ssh/my-key.pem

aws s3 mb s3://my-output-bucket

# 2. Deploy
cd /Users/jeff/Code/proton-beam

export INPUT_URL="https://example.com/data.jsonl.zst"
export S3_OUTPUT_BUCKET="my-output-bucket"
export KEY_NAME="my-key"
export USE_SPOT_INSTANCE="true"

./scripts/deploy-cloudformation.sh

# 3. Check results (~50 min later)
aws s3 ls s3://my-output-bucket/proton-beam-output/

# 4. Clean up
aws cloudformation delete-stack --stack-name proton-beam-*
```

## Cost Estimate

For 1.2TB dataset:
- **Processing**: $1.87 (Spot) or $5.60 (On-Demand)
- **Storage**: $10.35/month for 450GB in S3
- **Total**: Under $2 to process, $10/month to store

## Support

- **Issues**: [GitHub Issues](https://github.com/parres-hq/proton-beam/issues)
- **Questions**: See the complete guides linked above
- **Troubleshooting**: Check [CLOUDFORMATION_GUIDE.md](../CLOUDFORMATION_GUIDE.md#troubleshooting)

## Related Documentation

- [S3 Feature](S3_FEATURE.md) - Local S3 upload without AWS deployment
- [Architecture](AWS_ARCHITECTURE.md) - Detailed architecture diagrams
- [Compression](COMPRESSION_SUPPORT.md) - Supported compression formats
- [Testing](TESTING_AWS.md) - Testing procedures

---

**Start here:** [QUICKSTART_CLOUDFORMATION.md](../QUICKSTART_CLOUDFORMATION.md) 🚀

