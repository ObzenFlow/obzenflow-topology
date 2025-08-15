# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-01-14

### Added
- Initial release of obzenflow-topology as a standalone crate
- Core topology data structures (Topology, DirectedEdge) 
- Stage types (StageId, StageType, StageInfo, StageMetadata)
- Topology builder with fluent API
- Validation utilities (connectivity analysis, strongly connected components)
- Support for directed graphs with cycles (feedback loops, retry patterns)
- WASM compatibility for browser environments
- Comprehensive test suite

### Design Decisions
- Multi-stage cycles are supported to enable feedback loops and retry patterns
- Self-cycles (stage connecting to itself) are explicitly forbidden
- Uses ULID for stage identifiers to ensure global uniqueness

### Notes
- Extracted from the ObzenFlow project to enable reuse across backend and frontend
- Dual licensed under MIT OR Apache-2.0