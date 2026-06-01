---
title: Extending mirage
prev: 07-frameworks.md
---

# Extending mirage

Adding custom resource types.

## Resource Interface

**Source:** `python/mirage/resource/base.py`

To create a custom resource, implement the `Resource` interface:

```python
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Optional

@dataclass
class FileStat:
    size: int
    modified: float
    is_directory: bool
    is_file: bool

@dataclass
class DirEntry:
    name: str
    is_directory: bool
    is_file: bool

class Resource(ABC):
    """Abstract base class for all resources."""
    
    @abstractmethod
    async def read(self, path: str) -> bytes:
        """Read file contents.
        
        Args:
            path: Relative path within resource
            
        Returns:
            File contents as bytes
            
        Raises:
            FileNotFoundError: If path doesn't exist
        """
        pass
    
    @abstractmethod
    async def write(self, path: str, data: bytes) -> None:
        """Write file contents.
        
        Args:
            path: Relative path within resource
            data: Bytes to write
            
        Raises:
            PermissionError: If write not allowed
        """
        pass
    
    @abstractmethod
    async def list(self, path: str) -> list[DirEntry]:
        """List directory contents.
        
        Args:
            path: Directory path
            
        Returns:
            List of directory entries
            
        Raises:
            FileNotFoundError: If path doesn't exist
            NotADirectoryError: If path is a file
        """
        pass
    
    @abstractmethod
    async def stat(self, path: str) -> FileStat:
        """Get file statistics.
        
        Args:
            path: File or directory path
            
        Returns:
            File statistics
        """
        pass
```

## Custom Resource Example

Let's create a `WeatherResource` that exposes weather data as files:

```python
# my_resources/weather.py
import json
import aiohttp
from mirage.resource import Resource, FileStat, DirEntry

class WeatherResource(Resource):
    """Resource that fetches weather data."""
    
    def __init__(self, api_key: str, default_cities: list[str] = None):
        self.api_key = api_key
        self.default_cities = default_cities or ['London', 'NYC', 'Tokyo']
        self.base_url = 'https://api.weatherapi.com/v1'
    
    async def read(self, path: str) -> bytes:
        """Read weather data for a city.
        
        Path format: /{city}.json
        Example: /london.json
        """
        city = path.strip('/').replace('.json', '')
        
        async with aiohttp.ClientSession() as session:
            url = f'{self.base_url}/current.json'
            params = {
                'key': self.api_key,
                'q': city,
            }
            async with session.get(url, params=params) as resp:
                data = await resp.json()
        
        return json.dumps(data, indent=2).encode('utf-8')
    
    async def write(self, path: str, data: bytes) -> None:
        """Weather data is read-only."""
        raise PermissionError("Weather resource is read-only")
    
    async def list(self, path: str) -> list[DirEntry]:
        """List available cities."""
        return [
            DirEntry(
                name=f"{city}.json",
                is_file=True,
                is_directory=False
            )
            for city in self.default_cities
        ]
    
    async def stat(self, path: str) -> FileStat:
        """Get file statistics."""
        return FileStat(
            size=0,  # Unknown until read
            modified=time.time(),
            is_file=True,
            is_directory=False,
        )
```

**Usage:**

```python
from mirage import Workspace
from my_resources.weather import WeatherResource

ws = Workspace({
    '/weather': WeatherResource(api_key='...'),
})

# Use weather data
await ws.execute('cat /weather/london.json')
await ws.execute('ls /weather')
```

## Advanced Resource

**Aha:** Resources can support custom commands via command override:

```python
class ParquetResource(Resource):
    """Resource with custom cat command for Parquet files."""
    
    async def read(self, path: str) -> bytes:
        # Default: return raw bytes
        return await self._read_raw(path)
    
    async def custom_cat(self, path: str, format: str = 'json') -> str:
        """Custom cat that renders Parquet as readable format."""
        import pandas as pd
        
        data = await self._read_raw(path)
        df = pd.read_parquet(io.BytesIO(data))
        
        if format == 'json':
            return df.to_json(orient='records', indent=2)
        elif format == 'csv':
            return df.to_csv(index=False)
        else:
            return str(df)

# Register custom command
ws = Workspace({
    '/data': ParquetResource(...),
})

# Override cat for parquet files
ws.command('cat', {
    'resource': '/data',
    'filetype': 'parquet',
    'handler': ParquetResource.custom_cat,
})
```

## Testing Custom Resources

```python
# tests/test_weather.py
import pytest
from my_resources.weather import WeatherResource

@pytest.mark.asyncio
async def test_weather_read():
    resource = WeatherResource(api_key='test-key')
    
    # Mock the API call
    with aioresponses() as m:
        m.get(
            'https://api.weatherapi.com/v1/current.json',
            payload={'temp_c': 20, 'condition': 'Sunny'}
        )
        
        data = await resource.read('/london.json')
        result = json.loads(data)
    
    assert result['temp_c'] == 20

@pytest.mark.asyncio
async def test_weather_list():
    resource = WeatherResource(api_key='test-key')
    entries = await resource.list('/')
    
    assert len(entries) == 3
    assert entries[0].name == 'London.json'
```

## Publishing Resources

Share your resource with the community:

```bash
# Create package
mkdir mirage-weather-resource
cd mirage-weather-resource

# Structure
# mirage_weather/
#   __init__.py
#   resource.py
# pyproject.toml
# README.md

# pyproject.toml
[project]
name = "mirage-weather-resource"
version = "0.1.0"
dependencies = [
    "mirage-ai>=0.0.2",
    "aiohttp>=3.13",
]

# Publish
pip build
pip publish
```

## Resource Checklist

Before publishing a custom resource:

- [ ] Implements all `Resource` abstract methods
- [ ] Handles errors gracefully (FileNotFoundError, PermissionError)
- [ ] Supports async/await
- [ ] Includes tests
- [ ] Has documentation
- [ ] Handles edge cases (empty paths, special characters)
- [ ] Implements caching if appropriate
- [ ] Follows mirage conventions

## Example Resources to Build

| Resource | Description | Complexity |
|----------|-------------|------------|
| **JiraResource** | Jira issues as files | Medium |
| **NotionResource** | Notion pages as files | Medium |
| **LinearResource** | Linear issues as files | Medium |
| **ConfluenceResource** | Confluence pages as files | Medium |
| **DropboxResource** | Dropbox files | Medium |
| **AzureBlobResource** | Azure Blob Storage | Low |
| **SFTPResource** | SFTP server | Medium |
| **WebDAVResource** | WebDAV server | Low |

## Summary

Extending mirage is straightforward:

1. Implement the `Resource` interface
2. Handle filesystem operations
3. Register with workspace
4. Test thoroughly
5. Share with community

**Aha:** The Resource interface is intentionally simple — just 4 methods to implement any backend as files.
