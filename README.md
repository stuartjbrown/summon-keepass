summon-keepass
==============

Simple [summon](https://cyberark.github.io/summon/) provider that allows usage of KeePass kdbx database files.

Installation
-----

Download the archive for your OS/architecture from the [latest release](../../releases) (Linux amd64 and arm64 are supported), extract it and provide the executable to summon [as a provider](https://cyberark.github.io/summon/#providers).

Alternatively there is an [install script](./install.sh) available.

Configuration
-------------

`summon-keepass` supports two configuration methods:

### Option 1: Environment Variables (Recommended for projects)

Set the following environment variables:

```bash
export SUMMON_KEEPASS_DB_PATH=/path/to/your/keepass_database_file.kdbx
export SUMMON_KEEPASS_DB_PASS="password to your keepass database"
```

This method is ideal for:
- Project-specific configuration (via `.env` files, docker-compose, etc.)
- CI/CD environments
- Containerized deployments
- Different databases per project/environment

### Option 2: Configuration File

Create a `.summon-keepass.ini` file in your `$HOME` directory:

    [keepass_db]
    path=/path/to/your/keepass_database_file.kdbx
    pass=password to your keepass database

### Configuration Priority

If both methods are configured, environment variables take precedence over the configuration file. You can also mix sources (e.g., path from environment, password from config file).

Usage
-----

`summon-keepass` uses the following syntax for secrets:

    [group/subgroup/]entry[|field]

By default the `Password` field value of the `entry` will be returned. If an alternative `field` name is being provided, the value of that field will be returned.

Here's an example of a `secrets.yml` file:

    AWS_ACCESS_KEY_ID: !var aws/iam/user/robot/access_key_id
    AWS_SECRET_ACCESS_KEY: !var aws/iam/user/robot/secret_access_key
    SOME_USER_NAME: !var account|UserName
    SSH_PRIV_KEY: !var:file ssh/some server|priv_key

Testing
-------

Integration tests validate all functionality using a containerized environment.

### Running tests with Docker:
```bash
docker build -f Dockerfile.test -t summon-keepass:test .
docker run --rm summon-keepass:test
```

### Running tests with Rust toolchain:
```bash
export HOME=$(pwd)/tests/fixtures
cargo test
```

Test database: `tests/fixtures/test-database.kdbx` (password: `test123`)

Releases
--------

This project uses [cargo-release](https://github.com/crate-ci/cargo-release) for automated version management.

**Creating a release via GitHub UI (recommended):**
1. Go to [Actions → Create Release](../../actions/workflows/release.yml)
2. Click "Run workflow" and select release type (patch/minor/major)
3. The workflow handles everything automatically

**Creating a release locally:**
```bash
cargo install cargo-release
cargo release minor --dry-run  # Preview changes
cargo release minor            # Execute release
git push && git push --tags    # Push to trigger CI
```

See [CLAUDE.md](CLAUDE.md#release-process) for detailed instructions.

Todo
----
- ~~get the KeePass DB password from an environment variable~~ ✓ Completed (supports both path and password via environment variables)
- key file authentication
- ~~error handling for incorrect config/KeePass DB file path~~ ✓ Improved (graceful error messages showing all checked configuration sources)
