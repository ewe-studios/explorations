# Resources

Complete reference for all 50+ built-in resource types.

## Resource Categories

| Category | Count | Resources |
|----------|-------|-----------|
| **Core Storage** | 3 | RAM, Disk, File |
| **Cloud Object Storage** | 15 | S3, GCS, R2, OCI, Azure, etc. |
| **Google Workspace** | 4 | Drive, Docs, Sheets, Slides |
| **Communication** | 6 | Slack, Gmail, Discord, Trello, etc. |
| **Databases** | 4 | Redis, Postgres, MongoDB, etc. |
| **Dev/CI** | 5 | GitHub, GitHub CI, Dify, etc. |
| **HuggingFace** | 4 | Datasets, Models, Spaces, Buckets |
| **Remote** | 2 | SSH, Email |
| **Other** | 8 | Notion, Linear, Langfuse, etc. |

**Total: 50+ resources**

## Core Storage

### RAMResource
In-memory temporary storage.

```python
from mirage.resource import RAMResource

ws = Workspace({'/tmp': RAMResource()})
```

### DiskResource
Local filesystem access.

```python
from mirage.resource import DiskResource

ws = Workspace({'/local': DiskResource('/home/user/data')})
```

### FileResource
Generic file access.

```python
from mirage.resource import FileResource

ws = Workspace({'/data': FileResource('/path/to/files')})
```

## Cloud Object Storage (15 resources)

### S3Resource
AWS S3 buckets.

```python
from mirage.resource import S3Resource

ws = Workspace({
    '/s3': S3Resource(
        bucket='my-bucket',
        region='us-east-1',
        access_key_id='...',
        secret_access_key='...',
    ),
})
```

### GCSResource
Google Cloud Storage.

```python
from mirage.resource import GCSResource

ws = Workspace({
    '/gcs': GCSResource(
        bucket='my-bucket',
        project='my-project',
    ),
})
```

### R2Resource
Cloudflare R2.

```python
from mirage.resource import R2Resource

ws = Workspace({
    '/r2': R2Resource(
        account_id='...',
        access_key_id='...',
        secret_access_key='...',
        bucket='my-bucket',
    ),
})
```

### OCIDataResource
Oracle Cloud Infrastructure.

```python
from mirage.resource import OCIDataResource

ws = Workspace({
    '/oci': OCIDataResource(
        namespace='...',
        bucket='my-bucket',
        region='us-ashburn-1',
    ),
})
```

### MinioResource
MinIO object storage.

```python
from mirage.resource import MinioResource

ws = Workspace({
    '/minio': MinioResource(
        endpoint='localhost:9000',
        access_key='...',
        secret_key='...',
        bucket='my-bucket',
    ),
})
```

### WasabiResource
Wasabi cloud storage.

```python
from mirage.resource import WasabiResource

ws = Workspace({
    '/wasabi': WasabiResource(
        access_key='...',
        secret_key='...',
        bucket='my-bucket',
    ),
})
```

### BackblazeResource
Backblaze B2.

```python
from mirage.resource import BackblazeResource

ws = Workspace({
    '/backblaze': BackblazeResource(
        application_key_id='...',
        application_key='...',
        bucket='my-bucket',
    ),
})
```

### DigitalOceanResource
DigitalOcean Spaces.

```python
from mirage.resource import DigitalOceanResource

ws = Workspace({
    '/digitalocean': DigitalOceanResource(
        region='nyc3',
        access_key='...',
        secret_key='...',
        bucket='my-bucket',
    ),
})
```

### ScalewayResource
Scaleway Object Storage.

```python
from mirage.resource import ScalewayResource

ws = Workspace({
    '/scaleway': ScalewayResource(
        access_key='...',
        secret_key='...',
        bucket='my-bucket',
        region='fr-par',
    ),
})
```

### SupabaseResource
Supabase Storage.

```python
from mirage.resource import SupabaseResource

ws = Workspace({
    '/supabase': SupabaseResource(
        url='https://...supabase.co',
        key='...',
        bucket='my-bucket',
    ),
})
```

### CEPHResource
CEPH object storage.

```python
from mirage.resource import CEPHResource

ws = Workspace({
    '/ceph': CEPHResource(
        endpoint='...',
        access_key='...',
        secret_key='...',
        bucket='my-bucket',
    ),
})
```

### AliyunResource
Alibaba Cloud OSS.

```python
from mirage.resource import AliyunResource

ws = Workspace({
    '/aliyun': AliyunResource(
        access_key_id='...',
        access_key_secret='...',
        bucket='my-bucket',
        endpoint='...',
    ),
})
```

### TencentResource
Tencent Cloud COS.

```python
from mirage.resource import TencentResource

ws = Workspace({
    '/tencent': TencentResource(
        secret_id='...',
        secret_key='...',
        bucket='my-bucket',
        region='ap-beijing',
    ),
})
```

### QingstorResource
QingStor object storage.

```python
from mirage.resource import QingstorResource

ws = Workspace({
    '/qingstor': QingstorResource(
        access_key='...',
        secret_key='...',
        bucket='my-bucket',
        zone='...',
    ),
})
```

### DatabricksVolumeResource
Databricks Unity Catalog volumes.

```python
from mirage.resource import DatabricksVolumeResource

ws = Workspace({
    '/databricks': DatabricksVolumeResource(
        host='...',
        token='...',
        volume='/Volumes/catalog/schema/volume',
    ),
})
```

## Google Workspace (4 resources)

### GDriveResource
Google Drive.

```python
from mirage.resource import GDriveResource

ws = Workspace({
    '/drive': GDriveResource(
        credentials='...',
    ),
})

await ws.execute('ls /drive')
await ws.execute('cat /drive/documents/report.pdf')
```

### GDocsResource
Google Docs.

```python
from mirage.resource import GDocsResource

ws = Workspace({
    '/docs': GDocsResource(
        credentials='...',
    ),
})

await ws.execute('cat /docs/document-id/content.md')
```

### GSheetsResource
Google Sheets.

```python
from mirage.resource import GSheetsResource

ws = Workspace({
    '/sheets': GSheetsResource(
        credentials='...',
    ),
})

await ws.execute('cat /sheets/sheet-id/Sheet1.csv')
```

### GSlidesResource
Google Slides.

```python
from mirage.resource import GSlidesResource

ws = Workspace({
    '/slides': GSlidesResource(
        credentials='...',
    ),
})

await ws.execute('ls /slides')
```

## Communication (6 resources)

### SlackResource
Slack channels and messages.

```python
from mirage.resource import SlackResource

ws = Workspace({
    '/slack': SlackResource(token='xoxb-...'),
})

await ws.execute('ls /slack')
await ws.execute('cat /slack/general/2025-01-15.json')
await ws.execute('grep alert /slack/general/*.json')
```

### GmailResource
Gmail messages.

```python
from mirage.resource import GmailResource

ws = Workspace({
    '/gmail': GmailResource(credentials='...'),
})

await ws.execute('ls /gmail/inbox')
await ws.execute('cat /gmail/inbox/unread.json')
```

### DiscordResource
Discord channels and messages.

```python
from mirage.resource import DiscordResource

ws = Workspace({
    '/discord': DiscordResource(token='...'),
})

await ws.execute('ls /discord/server/channels')
```

### TrelloResource
Trello boards and cards.

```python
from mirage.resource import TrelloResource

ws = Workspace({
    '/trello': TrelloResource(
        api_key='...',
        token='...',
    ),
})

await ws.execute('ls /trello/boards')
await ws.execute('cat /trello/boards/board-id/cards/card-id.json')
```

### EmailResource
Generic email (IMAP/SMTP).

```python
from mirage.resource import EmailResource

ws = Workspace({
    '/email': EmailResource(
        imap_server='imap.gmail.com',
        smtp_server='smtp.gmail.com',
        username='...',
        password='...',
    ),
})
```

### NextcloudResource
Nextcloud files.

```python
from mirage.resource import NextcloudResource

ws = Workspace({
    '/nextcloud': NextcloudResource(
        url='https://...',
        username='...',
        password='...',
    ),
})
```

## Databases (4 resources)

### RedisResource
Redis keys as files.

```python
from mirage.resource import RedisResource

ws = Workspace({
    '/redis': RedisResource(host='localhost', port=6379),
})

await ws.execute('echo "value" > /redis/key.txt')
await ws.execute('cat /redis/key.txt')
```

### PostgresResource
PostgreSQL tables as directories.

```python
from mirage.resource import PostgresResource

ws = Workspace({
    '/db': PostgresResource(
        host='localhost',
        database='mydb',
        user='user',
        password='pass',
    ),
})

await ws.execute('ls /db/public')
await ws.execute('cat /db/public/users.csv')
```

### MongoDBResource
MongoDB collections.

```python
from mirage.resource import MongoDBResource

ws = Workspace({
    '/mongo': MongoDBResource(
        uri='mongodb://localhost:27017',
        database='mydb',
    ),
})
```

### NotionResource
Notion pages and databases.

```python
from mirage.resource import NotionResource

ws = Workspace({
    '/notion': NotionResource(token='...'),
})

await ws.execute('ls /notion')
await ws.execute('cat /notion/page-id/content.md')
```

## Dev/CI (5 resources)

### GitHubResource
GitHub repositories.

```python
from mirage.resource import GitHubResource

ws = Workspace({
    '/github': GitHubResource(token='ghp_...'),
})

await ws.execute('ls /github/mirage')
await ws.execute('cat /github/mirage/README.md')
```

### GitHubCIResource
GitHub Actions CI.

```python
from mirage.resource import GitHubCIResource

ws = Workspace({
    '/github-ci': GitHubCIResource(token='ghp_...'),
})

await ws.execute('ls /github-ci/runs')
await ws.execute('cat /github-ci/runs/run-id/logs.txt')
```

### DifyResource
Dify AI platform.

```python
from mirage.resource import DifyResource

ws = Workspace({
    '/dify': DifyResource(
        api_key='...',
        base_url='https://...',
    ),
})
```

### DevResource
Local development environment.

```python
from mirage.resource import DevResource

ws = Workspace({
    '/dev': DevResource(
        project_path='/path/to/project',
    ),
})
```

### LangfuseResource
Langfuse tracing.

```python
from mirage.resource import LangfuseResource

ws = Workspace({
    '/langfuse': LangfuseResource(
        public_key='...',
        secret_key='...',
        host='https://...',
    ),
})
```

## HuggingFace (4 resources)

### HFDatasetsResource
HuggingFace datasets.

```python
from mirage.resource import HFDatasetsResource

ws = Workspace({
    '/hf/datasets': HFDatasetsResource(token='...'),
})

await ws.execute('ls /hf/datasets')
await ws.execute('cat /hf/datasets/dataset-name/train.csv')
```

### HFModelsResource
HuggingFace models.

```python
from mirage.resource import HFModelsResource

ws = Workspace({
    '/hf/models': HFModelsResource(token='...'),
})

await ws.execute('ls /hf/models')
await ws.execute('cat /hf/models/model-name/README.md')
```

### HFSpacesResource
HuggingFace Spaces.

```python
from mirage.resource import HFSpacesResource

ws = Workspace({
    '/hf/spaces': HFSpacesResource(token='...'),
})
```

### HFBucketsResource
HuggingFace Storage Buckets.

```python
from mirage.resource import HFBucketsResource

ws = Workspace({
    '/hf/buckets': HFBucketsResource(token='...'),
})
```

## Remote (2 resources)

### SSHResource
Remote SSH hosts.

```python
from mirage.resource import SSHResource

ws = Workspace({
    '/remote': SSHResource(
        host='server.example.com',
        user='admin',
        key_path='~/.ssh/id_rsa',
    ),
})

await ws.execute('ls /remote/home/admin')
```

## Other Resources (4 resources)

### LinearResource
Linear issue tracking.

```python
from mirage.resource import LinearResource

ws = Workspace({
    '/linear': LinearResource(api_key='...'),
})

await ws.execute('ls /linear/issues')
await ws.execute('cat /linear/issues/TEAM-123.json')
```

### FileTypeResource
File type detection.

```python
from mirage.resource import FileTypeResource

ws = Workspace({
    '/types': FileTypeResource(),
})
```

### JQResource
JQ JSON processing.

```python
from mirage.resource import JQResource

ws = Workspace({
    '/jq': JQResource(),
})
```

### SecretsResource
Secret management.

```python
from mirage.resource import SecretsResource

ws = Workspace({
    '/secrets': SecretsResource(backend='...'),
})
```

## Resource Comparison Matrix

| Category | Read | Write | List | Best For |
|----------|------|-------|------|----------|
| **Core Storage** | | | | |
| RAM | ✅ | ✅ | ✅ | Temp data |
| Disk | ✅ | ✅ | ✅ | Local files |
| File | ✅ | ✅ | ✅ | Generic files |
| **Cloud Storage** | | | | |
| S3/GCS/R2 | ✅ | ✅ | ✅ | Object storage |
| Wasabi/Backblaze | ✅ | ✅ | ✅ | S3-compatible |
| **Google Workspace** | | | | |
| Drive | ✅ | ✅ | ✅ | File storage |
| Docs/Sheets/Slides | ✅ | ⚠️ | ✅ | Documents |
| **Communication** | | | | |
| Slack/Discord | ✅ | ⚠️ | ✅ | Messages |
| Gmail/Email | ✅ | ⚠️ | ✅ | Email |
| **Databases** | | | | |
| Redis | ✅ | ✅ | ✅ | Cache |
| Postgres | ✅ | ⚠️ | ✅ | SQL data |
| MongoDB | ✅ | ✅ | ✅ | Documents |
| **Dev/CI** | | | | |
| GitHub | ✅ | ⚠️ | ✅ | Repos |
| Dify | ✅ | ✅ | ✅ | AI apps |

## Resource Registry

All resources are registered in `python/mirage/resource/registry.py`:

```python
# python/mirage/resource/registry.py
RESOURCE_REGISTRY = {
    'ram': RAMResource,
    'disk': DiskResource,
    's3': S3Resource,
    'gcs': GCSResource,
    'r2': R2Resource,
    'gdrive': GDriveResource,
    'gdocs': GDocsResource,
    'gsheets': GSheetsResource,
    'gslides': GSlidesResource,
    'slack': SlackResource,
    'gmail': GmailResource,
    'discord': DiscordResource,
    'github': GitHubResource,
    'redis': RedisResource,
    'postgres': PostgresResource,
    'mongodb': MongoDBResource,
    'notion': NotionResource,
    'linear': LinearResource,
    # ... 30+ more
}

async def create_resource(name: str, config: dict) -> Resource:
    """Create resource by name."""
    resource_class = RESOURCE_REGISTRY[name]
    return resource_class(**config)
```

**Aha:** Resources are dynamically loaded from the registry, enabling plugin architecture.

## Next Steps

Continue to [Commands →](05-commands.html) for command reference.
