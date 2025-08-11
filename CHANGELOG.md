# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.1] - 2025-08-11

### Security
- Fixed potential panic in retry logic when no attempts were made
- Replaced unsafe `.unwrap()` calls with proper error handling in `utils.rs`
- Added fallback error for edge cases in retry strategies

### Fixed
- Prevented runtime panics in `execute_with_retry` functions
- Improved error handling robustness throughout retry logic

## [0.4.0] - Previous Release

### Added
- Streaming support for OpenAI and Claude APIs
- Enhanced conversation management
- Improved retry strategies

### Changed
- Updated dependencies
- Enhanced API compatibility