# vnotify Deep Exploration — Index

A technical exploration of the Rust library `vnotify` (version 0.1.0, by the Deno authors), installed at `/Users/darkvoid/GitSources/src.s3/vnotify`. It is a cache for Amazon Simple Storage Service (S3) objects plus a change signal that lives in the same bucket: a fixed-size keyspace of marker objects works as a hash map to flag bucket-wide changes.

- [00-overview.md](./00-overview.md) — What vnotify is: a read-through cache plus a change signal made of S3 marker objects, and the map of the whole library.
- [01-one-headobject-signal.md](./01-one-headobject-signal.md) — The one `HeadObject` detection idea: a global marker flags any change.
- [02-keyspace-as-hashmap.md](./02-keyspace-as-hashmap.md) — The fixed-size S3 keyspace as a hash map of shard markers.
- [03-refresh-worker-flow.md](./03-refresh-worker-flow.md) — The refresh loop: `HeadObject`, then `ListObjectsV2`, then revalidation and fencing.
- [04-detection-math.md](./04-detection-math.md) — The change-detection math: hashing, sharding, ETag uniqueness, and false evictions.
