# Reasoning: tendhost-pkg Implementation Plan

## Current State Analysis

### What Exists
- **Skeleton structure**: `lib.rs`, `traits.rs`, `apt.rs`, `dnf.rs`, `docker.rs` files exist
- **Trait definition**: `PackageManager` trait with basic methods
- **Type definitions**: `UpgradablePackage`, `UpdateResult` structs

### Dependencies Available
From `Cargo.toml`:
- `tokio` - Async runtime
- `async-trait` - Async trait support
- `thiserror` - Error types
- `serde/serde_json` - Serialization
- `tracing` - Structured logging
- `tendhost-exec` - Remote executor trait (dependency)

### Design Decisions

#### 1. Package Manager Implementations
Three implementations needed:
- **`AptManager`**: Debian/Ubuntu (apt, apt-get)
- **`DnfManager`**: CentOS/Fedora/RHEL (dnf, yum fallback)
- **`DockerComposeManager`**: Docker Compose stacks

#### 2. Integration with tendhost-exec
Each package manager needs a `RemoteExecutor` to run commands:
- Constructor takes `Arc<dyn RemoteExecutor>`
- Commands executed via executor (SSH or local)
- Consistent error handling across all managers

#### 3. Command Parsing
Need to parse output from various commands:
- `apt list --upgradable` - Parse package names and versions
- `dnf check-update` - Parse update list
- `docker compose pull` / `up` - Manage compose stacks

#### 4. Error Handling Strategy
Package manager specific errors:
- Package not found
- Repository unavailable
- Lock file conflicts (another apt/dnf running)
- Insufficient permissions

#### 5. Docker Compose Support
Two versions to support:
- `docker-compose` (v1, Python)
- `docker compose` (v2, Go plugin)

Auto-detect which is available.

## Implementation Phases

### Phase 1: Error Types
Define comprehensive error enum for package operations.

### Phase 2: Trait Enhancement
Enhance `PackageManager` trait with:
- Better return types
- Dry-run support
- Reboot detection

### Phase 3: Apt Implementation
Implement Debian/Ubuntu package manager.

### Phase 4: Dnf Implementation
Implement CentOS/Fedora package manager.

### Phase 5: Docker Compose Implementation
Implement Docker Compose stack manager.

### Phase 6: Testing
- Unit tests with mock executor
- Integration tests (optional)

## Risk Areas

1. **Command output parsing** - Format varies by distro version
2. **Concurrent operations** - Lock files may block
3. **Privilege escalation** - Need `sudo` for most operations
4. **Docker compose detection** - Version differences

## Next Steps

See `01-implementation-plan.md` for detailed task breakdown.
