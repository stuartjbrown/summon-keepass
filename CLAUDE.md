# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Critical Rules

**⚠️ ALWAYS follow these rules when working on this codebase:**

1. **ALWAYS run tests containerized** - Use Docker, never bare `cargo test`
   ```bash
   docker build -f Dockerfile.test -t summon-keepass:test .
   docker run --rm summon-keepass:test
   ```

2. **ALWAYS run tests after ANY code change** - All 26 tests must pass

3. **ALWAYS read code before modifying** - Never propose changes without understanding existing implementation

4. **ALWAYS update CHANGELOG.md** - Add changes to `[Unreleased]` section

5. **NEVER skip tests** - No exceptions for "small" changes

## Project Overview

`summon-keepass` is a Rust-based provider for [summon](https://cyberark.github.io/summon/) that enables reading secrets from KeePass (.kdbx) database files. It's a single-binary CLI tool that integrates with summon's secrets management workflow.

**Current Version:** 0.4.0
**Status:** Production-ready with comprehensive test coverage
**Rust Edition:** 2024

## Build and Development Commands

### Building
```bash
# Development build
cargo build --verbose

# Release build (optimized, symbols stripped automatically via Cargo.toml's `[profile.release] strip = true`)
cargo build --verbose --release
```

### Testing

**IMPORTANT: Always run tests for ANY code changes!**

This project has comprehensive integration tests (24 test cases) that validate all functionality. Tests MUST be run in Docker to ensure consistent environment and prevent HOME directory conflicts.

**Run tests containerized (REQUIRED):**
```bash
# Build test image
docker build -f Dockerfile.test -t summon-keepass:test .

# Run all tests
docker run --rm summon-keepass:test

# Run specific test
docker run --rm summon-keepass:test cargo test test_version_flag_short

# Shell access for debugging
docker run --rm -it summon-keepass:test /bin/bash
```

**DO NOT run tests with bare `cargo test`** - this can cause issues with HOME directory paths and test database access. Always use the Docker container.

**Test Coverage:**
- Version flag tests (2 tests)
- Basic retrieval tests (4 tests)
- Field access tests (5 tests)
- Line ending conversion (1 test)
- Error handling tests (3 tests)
- Special characters tests (2 tests)
- Configuration tests (7 tests) - environment variables, config file fallback, mixed sources, error messages

**Test Database:** `tests/fixtures/test-database.kdbx` (password: `test123`)

### Running
```bash
# Run the binary directly (requires configuration)
./target/release/summon-keepass "path/to/entry|field"

# Or from debug build
./target/debug/summon-keepass "path/to/entry|field"
```

## Architecture

### Single-File Application
The entire application logic is contained in `src/main.rs` (~182 lines). This is a straightforward CLI tool with no modular architecture.

### Configuration

**Configuration Sources (Priority Order):**
1. **Environment Variables** (highest priority):
   - `SUMMON_KEEPASS_DB_PATH` - Path to KeePass database file
   - `SUMMON_KEEPASS_DB_PASS` - Password for KeePass database

2. **Legacy INI File** (fallback):
   - `~/.summon-keepass.ini` with `[keepass_db]` section format

3. **Mixed Configuration Support**:
   - Allows partial configuration from different sources
   - Example: path from environment variable, password from config file

**Implementation:**
- `load_config()` function implements the priority order and merging logic
- `load_ini_config()` gracefully handles missing HOME, missing file, or invalid format
- `build_config_error()` generates helpful error messages showing all checked sources
- No panics on configuration errors - graceful exit with code 1 and descriptive messages

### Core Flow (src/main.rs)
1. **Version Flag Handling** (lines 28-35): Supports `-V` and `--version` flags
2. **Argument Validation** (lines 38-42): Ensures a secret path argument was provided
3. **Configuration Loading** (lines 44-51): Calls `load_config()` which:
   - Tries environment variables (`SUMMON_KEEPASS_DB_PATH`, `SUMMON_KEEPASS_DB_PASS`)
   - Falls back to `~/.summon-keepass.ini`
   - Supports mixing sources (e.g., path from env, password from file)
   - Returns helpful error messages if configuration is missing/invalid
4. **Database Access** (lines 56-59): Opens the KeePass database using the `keepass` crate with `DatabaseKey` API
5. **Secret Path Parsing** (lines 61-74): Parses the secret path format `[group/subgroup/]entry[|field]`
   - Default field is "Password" if not specified
   - Uses `|` as field separator
   - Validates path format (exits with code 2 for invalid paths)
6. **Entry Retrieval** (lines 75-88): Navigates the KeePass database tree structure using `/` separators
   - Gracefully handles missing entries and fields (exits with code 1)
7. **Output**: Writes the field value to stdout with DOS/Windows line endings converted to Unix (using `dos2unix`)

**Helper Functions (lines 96-181):**
- `load_config()` - Main configuration loader with priority order
- `load_ini_config()` - Graceful INI file loading (no panics)
- `build_config_error()` - Generates comprehensive error messages

### Dependencies (Cargo.toml)
**Runtime Dependencies:**
- `keepass` (0.8.16): KeePass database parsing (KDBX3/KDBX4 support)
- `rust-ini` (0.21.3): Configuration file parsing
- `newline-converter` (0.3.0): DOS to Unix line ending conversion

**Dev Dependencies:**
- `assert_cmd` (2.0): Command-line testing
- `predicates` (3.0): Test assertions
- `tempfile` (3.8): Temporary test directories

### Secret Path Format
The tool uses a specific syntax for identifying secrets:
- **Entry path**: `group/subgroup/entry` (uses `/` as separator)
- **Field specification**: `entry|field` (uses `|` as separator)
- **Examples**:
  - `aws/iam/user/robot/access_key_id` - Password field of nested entry
  - `account|UserName` - UserName field of root-level entry
  - `ssh/some server|priv_key` - Custom field from grouped entry

### Configuration
Requires `~/.summon-keepass.ini` with format:
```ini
[keepass_db]
path=/path/to/your/keepass_database_file.kdbx
pass=password to your keepass database
```

### Exit Codes
- `0`: Success
- `1`: No variable provided OR entry could not be retrieved
- `2`: Invalid secret path format

## Release Process

This project uses [cargo-release](https://github.com/crate-ci/cargo-release) to automate version management and releases.

### GitHub Actions Workflows

**`.github/workflows/release.yml`** - Automated Release Creation (Recommended)
- **Trigger:** Manual via GitHub UI ("Run workflow" button)
- **Purpose:** Complete end-to-end release process
- **Jobs:**
  1. `version-bump`: Runs cargo-release (version bump, CHANGELOG update, tag creation), pushes commit and tag, extracts changelog for the release version
  2. `build`: Matrix build for `linux-amd64` (native `cargo build`) and `linux-arm64` (cross-compiled via [`cross`](https://github.com/cross-rs/cross)). Symbols are stripped automatically via `strip = true` in `Cargo.toml`'s release profile. Packages each as `summon-keepass-linux-<arch>.tar.gz` and uploads as a build artifact
  3. `publish`: Downloads both architecture artifacts and creates a single GitHub Release with both binaries attached, automatically determining release vs pre-release based on version format

**`.github/workflows/rust.yml`** - Continuous Integration + Fallback
- **Trigger:** Automatic on every push and pull request
- **Purpose:**
  1. **CI Testing:** Runs integration tests in Docker on all pushes/PRs
  2. **Fallback Release:** Creates releases for manually pushed tags
- **When it creates releases:** Only when a tag matching `v*` is manually pushed via git
- **Note:** The release.yml workflow is preferred for creating releases

### Creating a Release

There are two ways to create a release:

#### Method 1: GitHub UI (Recommended)

1. **Ensure all changes are committed and documented in CHANGELOG.md under `[Unreleased]`**

2. **Navigate to Actions → Create Release:**
   - Go to https://github.com/desolat/summon-keepass/actions/workflows/release.yml
   - Click "Run workflow"
   - **Select release type from dropdown:**
     - **release** - Finalize pre-release (0.3.0-rc.2 → 0.3.0) or patch bump (0.3.0 → 0.3.1)
     - **minor** - New features (0.3.0 → 0.4.0)
     - **major** - Breaking changes (0.3.0 → 1.0.0)
     - **alpha** - Create alpha pre-release (0.3.0 → 0.3.0-alpha.1)
     - **beta** - Create beta pre-release (0.3.0 → 0.3.0-beta.1 or 0.3.0-alpha.1 → 0.3.0-beta.1)
     - **rc** - Create release candidate (0.3.0 → 0.3.0-rc.1 or 0.3.0-beta.1 → 0.3.0-rc.1)
     - **custom** - Specify exact version in the custom_version field
   - Click "Run workflow"

3. **The workflow will automatically:**
   - Run cargo-release with dry-run first (for validation)
   - Update version in `Cargo.toml` and `CHANGELOG.md`
   - Create commit: "chore: release X.Y.Z"
   - Create tag `vX.Y.Z`
   - Push commit and tag to GitHub
   - Build release binaries for `linux-amd64` and `linux-arm64`
   - Package them as `summon-keepass-linux-amd64.tar.gz` and `summon-keepass-linux-arm64.tar.gz`
   - Create GitHub Release with changelog and both binary artifacts
   - Mark as pre-release if version contains `-alpha`, `-beta`, or `-rc`

4. **Done!** The release is published at: https://github.com/desolat/summon-keepass/releases

**Typical Pre-release to Stable Workflow:**
```
0.3.0 (current stable)
  ↓ Select: "rc"
0.3.0-rc.1 (test it)
  ↓ Select: "rc" (increments to rc.2)
0.3.0-rc.2 (more testing)
  ↓ Select: "release" (finalizes)
0.3.0 (final stable release)
  ↓ Select: "minor"
0.4.0 (next version)
```

#### Method 2: Local cargo-release

**Prerequisites:**
```bash
cargo install cargo-release
```

**Release Process:**

1. **Ensure all changes are committed and documented in CHANGELOG.md under `[Unreleased]`**

2. **Perform a dry-run to preview changes:**
   ```bash
   # For patch release (0.3.0 -> 0.3.1)
   cargo release patch --dry-run

   # For minor release (0.3.0 -> 0.4.0)
   cargo release minor --dry-run

   # For major release (0.3.0 -> 1.0.0)
   cargo release major --dry-run

   # For specific version
   cargo release 0.4.0 --dry-run

   # For alpha pre-release (0.3.0 -> 0.3.1-alpha.1)
   cargo release alpha --dry-run

   # For beta pre-release (0.3.1-alpha.1 -> 0.3.1-beta.1)
   cargo release beta --dry-run

   # For release candidate (0.3.1-beta.1 -> 0.3.1-rc.1)
   cargo release rc --dry-run
   ```

3. **Review the dry-run output. cargo-release will:**
   - Update version in `Cargo.toml`
   - Update `CHANGELOG.md` (move Unreleased to versioned section with date)
   - Update changelog comparison links
   - Create a commit with message "chore: release X.Y.Z"
   - Create a git tag `vX.Y.Z`

4. **Execute the release (without --dry-run):**
   ```bash
   cargo release minor
   ```

5. **Review the commit and tag that were created:**
   ```bash
   git log -1
   git tag -l
   ```

6. **Push the commit and tag to trigger CI:**
   ```bash
   git push
   git push --tags
   ```

7. **GitHub Actions will automatically build, test, and create the GitHub release with changelog**

### Manual Release (Not Recommended)

If you need to create a release manually without cargo-release:

1. Update version in `Cargo.toml`
2. Move unreleased changes in `CHANGELOG.md` to new version header
3. Update changelog comparison links
4. Commit: `git commit -m "chore: release X.Y.Z"`
5. Tag: `git tag vX.Y.Z`
6. Push: `git push && git push origin vX.Y.Z`

### cargo-release Configuration Notes

**Important configuration details in Cargo.toml:**

1. **tag-prefix must be empty string:**
   - cargo-release's default tag-name template is `"{{prefix}}v{{version}}"`
   - Setting `tag-prefix = "v"` creates tags like `vv0.3.1-rc.1` (double "vv")
   - Correct configuration: `tag-prefix = ""` produces tags like `v0.3.1-rc.1`

2. **CHANGELOG updates only for final releases (prerelease = false):**
   - This project uses `prerelease = false` to prevent CHANGELOG clutter from pre-releases
   - **Pre-releases (alpha/beta/rc):** CHANGELOG.md stays unchanged, only Cargo.toml version updates
   - **Final releases (release/minor/major):** CHANGELOG.md `[Unreleased]` becomes `[0.3.0]`
   - **GitHub Releases for pre-releases:** Automatically use `[Unreleased]` content as changelog
   - This keeps one clean changelog section that gets finalized when you do the official release

3. **Pre-release levels work on the current version:**
   - `cargo release alpha` from `0.3.0` → `0.3.0-alpha.1` (NOT 0.3.1-alpha.1)
   - `cargo release beta` from `0.3.0-alpha.1` → `0.3.0-beta.1` (promotes alpha to beta)
   - `cargo release rc` from `0.3.0-beta.1` → `0.3.0-rc.1` (promotes beta to rc)
   - Subsequent runs increment: `0.3.0-rc.1` → `0.3.0-rc.2`
   - To release a pre-release of a NEW version, first bump version in Cargo.toml manually, then run pre-release

4. **Finalizing pre-releases uses patch (or "release"):**
   - `cargo release patch` from `0.3.0-rc.2` → `0.3.0` (removes pre-release suffix)
   - This is confusing because "patch" doesn't bump the patch version, it finalizes the current version
   - The GitHub workflow includes a "release" option which is clearer (maps to patch internally)
   - To bump to next version after pre-release, use minor/major: `0.3.0-rc.2` → `cargo release minor` → `0.4.0`

### cargo-release Configuration

The release behavior is configured in `Cargo.toml` under `[package.metadata.release]`:
- **No automatic push**: You must manually push after reviewing
- **No crates.io publish**: This is a binary-only project
- **Automatic file updates**: CHANGELOG updated with version and date
- **Consistent commit messages**: "chore: release X.Y.Z"

## Development Workflow

### Before Making Changes
1. **Always read the existing code first** - Don't propose changes without understanding current implementation
2. **Check CHANGELOG.md** - See recent changes and planned features under `[Unreleased]`
3. **Review test coverage** - Understand what tests exist for the area you're modifying

### After Making Changes
1. **ALWAYS run containerized tests:**
   ```bash
   docker build -f Dockerfile.test -t summon-keepass:test .
   docker run --rm summon-keepass:test
   ```
2. **All 17 tests must pass** before submitting changes
3. **Update CHANGELOG.md** - Add changes to `[Unreleased]` section
4. **Update version tests** - If changing version handling, verify semver validation works

### Dependency Updates
When updating dependencies:
1. Use `cargo update --dry-run` to preview changes
2. Use `cargo search <crate>` to find latest versions
3. Update `Cargo.toml` with new versions
4. **Run full test suite** to catch breaking changes
5. Update dependency versions in CLAUDE.md Architecture section
6. Document any API changes required in CHANGELOG.md

## Known Issues and Todo Items

**Completed:**
- ✅ Tests (comprehensive 24-test suite with Docker integration)
- ✅ Version flag support
- ✅ CI/CD pipeline with automated releases
- ✅ cargo-release integration
- ✅ Error handling improvements (graceful exit codes)
- ✅ Environment variable configuration (SUMMON_KEEPASS_DB_PATH, SUMMON_KEEPASS_DB_PASS)
- ✅ Improved error handling with helpful messages showing all checked configuration sources

**Outstanding (from README.md):**
- Key file authentication not yet supported

## Recent Updates (v0.4.0)

**New Features:**
- **Environment Variable Configuration:** Added support for `SUMMON_KEEPASS_DB_PATH` and `SUMMON_KEEPASS_DB_PASS` environment variables
- **Configuration Priority:** Environment variables override `~/.summon-keepass.ini` config file
- **Mixed Configuration Sources:** Support for partial configuration from different sources (e.g., path from env, password from file)
- **Improved Error Handling:** No more panics on missing configuration; graceful error messages showing all checked sources

**Dependency Updates:**
- Updated `keepass` from 0.4.9 → 0.8.16 (major version, API changes required)
- Updated `rust-ini` from 0.9.4 → 0.21.3 (major version)
- Updated `newline-converter` from 0.2.0 → 0.3.0 (minor version)
- Updated Rust edition from 2021 → 2024

**API Changes Required:**
- Updated `Database::open()` to use new `DatabaseKey` API:
  ```rust
  // Old API (0.4.9)
  Database::open(&mut file, Some(password), None)?

  // New API (0.8.16)
  let key = DatabaseKey::new().with_password(password);
  Database::open(&mut file, key).map_err(...)?
  ```
- Tree navigation no longer uses `keepass::db::NodeRef` (removed from the crate in >=0.8.17). Entries are now looked up with `db.root.group_by_path(&path_segments)` followed by `.entry_by_name(entry_name)`.

**Testing:**
- All 24 integration tests pass (17 original + 7 new configuration tests)
- No functional regressions
- Version validation tests use semver format checking (no hardcoded versions)
- New tests cover environment variables, config file fallback, mixed sources, and error handling
