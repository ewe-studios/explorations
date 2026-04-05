---
source: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.basecamp/kamal
repository: github.com/basecamp/kamal
explored_at: 2026-04-05
focus: Asset extraction, compression, caching, CDN integration
---

# Deep Dive: Asset Handling and Compression

## Overview

This deep dive examines Kamal's asset handling system - how public assets (images, CSS, JavaScript) are extracted from containers, compressed for efficient delivery, and served with proper caching headers.

## Architecture

```mermaid
flowchart TB
    subgraph App["Application Container"]
        PublicAssets[/public/assets]
        AppCode[Application Code]
    end
    
    subgraph Extraction["Asset Extraction"]
        Extractor[Asset Extractor]
        TempDir[Temporary Directory]
    end
    
    subgraph Processing["Asset Processing"]
        Compressor[Compression]
        Fingerprint[Fingerprinting]
        Manifest[Manifest Generation]
    end
    
    subgraph Storage["Asset Storage"]
        AssetVolume[Shared Volume]
        CDN[CDN / S3]
    end
    
    subgraph Delivery["Asset Delivery"]
        Proxy[kamal-proxy]
        Cache[Cache Headers]
    end
    
    App --> Extractor
    Extractor --> TempDir
    TempDir --> Compressor
    Compressor --> Fingerprint
    Fingerprint --> Manifest
    Manifest --> AssetVolume
    AssetVolume --> Proxy
    AssetVolume --> CDN
    Proxy --> Cache
```

## Asset Extraction

### Extraction Process

```ruby
# lib/kamal/commands/assets.rb

class Kamal::Commands::Assets
  def initialize(config, role)
    @config = config
    @role = role
  end
  
  def extract
    # Create asset extraction directory
    docker :run,
      "--rm",
      "--detach",
      "--name", extraction_container_name,
      "--entrypoint", "sleep",
      config.absolute_image,
      "infinity"
    
    # Copy assets from container
    docker :cp,
      "#{extraction_container_name}:#{config.assets.path}/.",
      extraction_directory
    
    # Stop extraction container
    docker :stop, extraction_container_name
  end
  
  def extraction_directory
    # Local directory for extracted assets
    File.join(config.temp_dir, "assets", config.version)
  end
  
  def extraction_container_name
    "#{config.service}-asset-extract-#{config.version}"
  end
end
```

### Asset Path Configuration

```yaml
# deploy.yml

assets:
  # Path inside container where assets live
  path: /public/assets
  
  # Roles that serve assets (for extraction)
  roles:
    - web
  
  # Compression settings
  compression:
    # Enable gzip compression
    gzip: true
    
    # Enable zstd compression (better compression ratio)
    zstd: true
    
    # Compression level (1-9 for gzip, 1-22 for zstd)
    level: 6
  
  # Caching
  cache:
    # Cache duration in seconds
    max_age: 31536000  # 1 year
    
    # Enable immutable headers
    immutable: true
```

## Compression

### Gzip Compression

```ruby
# lib/kamal/assets/compressor.rb

module Kamal::Assets
  class Compressor
    def initialize(config = {})
      @gzip_enabled = config[:gzip] ?? true
      @zstd_enabled = config[:zstd] ?? false
      @gzip_level = config[:level] || 6
    end
    
    def compress(directory)
      assets = find_assets(directory)
      
      assets.each do |asset_path|
        compress_asset(asset_path) if should_compress?(asset_path)
      end
    end
    
    private
    
    def find_assets(directory)
      # Find all asset files
      Dir.glob(File.join(directory, "**/*")).select do |f|
        File.file?(f) && compressible_extension?(f)
      end
    end
    
    def compress_asset(path)
      # Gzip compression
      if @gzip_enabled
        gzip_path = "#{path}.gz"
        Zlib::GzipWriter.open(gzip_path, @gzip_level) do |gz|
          gz.write(File.read(path))
        end
      end
      
      # Zstd compression (if available)
      if @zstd_enabled && zstd_available?
        zstd_path = "#{path}.zst"
        system("zstd -#{@zstd_level} #{path} -o #{zstd_path}")
      end
    end
    
    def should_compress?(path)
      # Skip already compressed files
      return false if path.end_with?(".gz", ".zst")
      
      # Only compress text-based assets
      compressible_extensions = [
        ".css", ".js", ".json",
        ".html", ".xml", ".svg",
        ".txt", ".md",
        ".eot", ".otf", ".ttf", ".woff", ".woff2"
      ]
      
      compressible_extensions.any? { |ext| path.end_with?(ext) }
    end
    
    def compressible_extension?(path)
      # Check if file has a compressible extension
      compressible_extensions = %w[
        css js json html xml svg txt md
        eot otf ttf woff woff2
      ]
      
      ext = File.extname(path)[1..-1]&.downcase
      compressible_extensions.include?(ext)
    end
    
    def zstd_available?
      # Check if zstd command is available
      system("which zstd > /dev/null 2>&1")
    end
  end
end
```

### Compression Comparison

```ruby
# Benchmark compression algorithms

require "zlib"
require "zstd-ruby"

class CompressionBenchmark
  def self.compare(file_path)
    content = File.read(file_path)
    original_size = content.bytesize
    
    # Gzip
    gzip_start = Time.now
    gzip_content = Zlib::Deflate.deflate(content, Zlib::BEST_COMPRESSION)
    gzip_time = Time.now - gzip_start
    gzip_size = gzip_content.bytesize
    
    # Zstd
    zstd_start = Time.now
    zstd_content = Zstd.compress(content, 19)
    zstd_time = Time.now - zstd_start
    zstd_size = zstd_content.bytesize
    
    puts "File: #{file_path}"
    puts "Original: #{original_size} bytes"
    puts "Gzip: #{gzip_size} bytes (#{compression_ratio(original_size, gzip_size)}%) - #{gzip_time.round(3)}s"
    puts "Zstd: #{zstd_size} bytes (#{compression_ratio(original_size, zstd_size)}%) - #{zstd_time.round(3)}s"
  end
  
  def self.compression_ratio(original, compressed)
    ((1 - compressed.to_f / original) * 100).round(1)
  end
end

# Example results:
# File: application.js
# Original: 500000 bytes
# Gzip: 150000 bytes (70.0%) - 0.05s
# Zstd: 120000 bytes (76.0%) - 0.08s
```

## Fingerprinting

### Asset Fingerprinting

```ruby
# lib/kamal/assets/fingerprinter.rb

module Kamal::Assets
  class Fingerprinter
    def fingerprint(directory)
      assets = find_assets(directory)
      
      assets.each do |asset_path|
        # Generate fingerprint from content hash
        fingerprint = generate_fingerprint(asset_path)
        
        # Rename file with fingerprint
        rename_with_fingerprint(asset_path, fingerprint)
      end
      
      # Generate manifest
      generate_manifest(directory, assets)
    end
    
    private
    
    def generate_fingerprint(path)
      content = File.read(path)
      hash = Digest::MD5.hexdigest(content)
      
      # Use first 8 characters of hash
      hash.first(8)
    end
    
    def rename_with_fingerprint(path, fingerprint)
      dirname = File.dirname(path)
      basename = File.basename(path, File.extname(path))
      extname = File.extname(path)
      
      # New filename: name-fingerprint.ext
      new_name = "#{basename}-#{fingerprint}#{extname}"
      new_path = File.join(dirname, new_name)
      
      File.rename(path, new_path) unless path == new_path
    end
    
    def generate_manifest(directory, assets)
      manifest = assets.each_with_object({}) do |path, result|
        # Map logical name to fingerprinted name
        logical_name = path.relative_path_from(directory).to_s
        fingerprinted_name = File.basename(path)
        result[logical_name] = fingerprinted_name
      end
      
      # Write manifest file
      manifest_path = File.join(directory, "manifest.json")
      File.write(manifest_path, JSON.pretty_generate(manifest))
    end
  end
end
```

## Asset Serving

### Proxy Asset Serving

```ruby
# kamal-proxy/lib/kamal/proxy/assets.rb

module Kamal::Proxy
  class AssetServer
    def initialize(asset_path:, cache_control:)
      @asset_path = asset_path
      @cache_control = cache_control
    end
    
    def call(env)
      path = env["PATH_INFO"]
      
      # Check if file exists
      file_path = File.join(@asset_path, path)
      
      unless File.file?(file_path)
        return [404, {}, ["Not Found"]]
      end
      
      # Determine content type
      content_type = Rack::Mime.mime_type(File.extname(path))
      
      # Build headers
      headers = {
        "Content-Type" => content_type,
        "Content-Length" => File.size(file_path).to_s
      }
      
      # Add cache headers
      if fingerprinted?(path)
        headers.merge!(cache_headers)
      end
      
      # Check for compressed version
      compressed = find_compressed(file_path, env["HTTP_ACCEPT_ENCODING"])
      
      if compressed
        headers["Content-Encoding"] = compressed[:encoding]
        file_path = compressed[:path]
      end
      
      # Serve file
      [200, headers, [File.read(file_path)]]
    end
    
    private
    
    def fingerprinted?(path)
      # Check if filename contains fingerprint (8 char hash)
      basename = File.basename(path, File.extname(path))
      basename =~ /-[a-f0-9]{8}$/
    end
    
    def cache_headers
      {
        "Cache-Control" => "public, max-age=31536000, immutable",
        "ETag" => "\"#{File.mtime(@asset_path).to_i}\""
      }
    end
    
    def find_compressed(file_path, accept_encoding)
      return nil unless accept_encoding
      
      # Check for zstd first (better compression)
      if accept_encoding.include?("zstd")
        zstd_path = "#{file_path}.zst"
        if File.file?(zstd_path)
          return { path: zstd_path, encoding: "zstd" }
        end
      end
      
      # Fall back to gzip
      if accept_encoding.include?("gzip")
        gzip_path = "#{file_path}.gz"
        if File.file?(gzip_path)
          return { path: gzip_path, encoding: "gzip" }
        end
      end
      
      nil
    end
  end
end
```

### CDN Integration

```ruby
# lib/kamal/assets/cdn.rb

module Kamal::Assets
  class CDN
    def initialize(config)
      @provider = config[:provider]
      @bucket = config[:bucket]
      @region = config[:region]
      @prefix = config[:prefix] || "assets"
    end
    
    def upload(directory)
      assets = find_assets(directory)
      
      assets.each do |asset_path|
        upload_asset(asset_path)
      end
    end
    
    private
    
    def upload_asset(path)
      key = "#{@prefix}/#{File.basename(path)}"
      
      case @provider
      when "s3"
        upload_to_s3(path, key)
      when "gcs"
        upload_to_gcs(path, key)
      when "cloudflare"
        upload_to_cloudflare(path, key)
      end
    end
    
    def upload_to_s3(local_path, key)
      `aws s3 cp #{local_path} s3://#{@bucket}/#{key} \\
        --region #{@region} \\
        --cache-control "public, max-age=31536000, immutable" \\
        --content-type "#{content_type(local_path)}"`
    end
    
    def content_type(path)
      Rack::Mime.mime_type(File.extname(path))
    end
  end
end
```

## Configuration

### Complete Asset Configuration

```yaml
# deploy.yml

service: myapp
image: myorg/myapp

servers:
  - 192.168.0.1
  - 192.168.0.2

# Asset configuration
assets:
  # Path inside container
  path: /public/assets
  
  # Roles to extract assets from
  roles:
    - web
  
  # Compression
  compression:
    gzip: true
    zstd: true
    level: 6
  
  # Local serving
  serve_from_proxy: true
  
  # CDN upload
  cdn:
    provider: s3
    bucket: myapp-assets
    region: us-east-1
    prefix: production
    upload: true  # Upload on deploy
  
  # Caching
  cache:
    max_age: 31536000  # 1 year
    immutable: true
```

## Conclusion

Kamal's asset handling provides:

1. **Automatic Extraction**: Assets copied from containers
2. **Compression**: Gzip and Zstd compression
3. **Fingerprinting**: Content-based cache busting
4. **CDN Integration**: S3, GCS, Cloudflare upload
5. **Cache Headers**: Proper caching for fingerprinted assets
6. **Content Negotiation**: Serve compressed versions based on Accept-Encoding
