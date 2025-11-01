# Smart Aquarium Controller — Production Container Spec

This pack hardens LXC container configs for a smart‑aquarium control plane (sensor ingestion, actuator control, alerting).
It overlays the base `LXC-CONFIGURATION-SCHEMA.json` and constrains unsafe edges for repeatable ops at scale.

See `schema/production.container.schema.json` for the overlay and `mapping/legacy_to_production.csv` for field translations.

Generated: 2025-10-31T06:28:35.456626
