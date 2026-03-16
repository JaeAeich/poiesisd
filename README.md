# PoiesisD

A [GA4GH](https://www.ga4gh.org/)-compliant [Task Execution Service (TES)](https://www.ga4gh.org/product/task-execution-service-tes/) running on Docker.

> [!NOTE]
> PoiesisD is designed as a lightweight service to simplify bioinformatics
> and analysis tool development. It is intended for development and testing
> purposes only and is not recommended for production use.

## Architecture

PoiesisD is a single-binary Rust service that accepts TES task requests over HTTP, runs them in Docker containers, and stages data through S3-compatible storage.

```
Client ──HTTP──▶ Axum API ──▶ SQLite (task queue)
                                  │
                            Worker loop
                                  │
                    ┌─────────────┼─────────────┐
                    ▼             ▼              ▼
               S3 Filer     Docker Engine    S3 Filer
             (stage inputs)  (run task)    (collect outputs)
```

**Key components:**

- **API** (`src/api/`) — Axum HTTP server exposing TES v1.1 endpoints. Tasks are validated and written to SQLite with state `QUEUED`.
- **Database** (`src/database/`) — SQLite with WAL mode. Tasks and all related data (inputs, outputs, executors, logs) are stored across normalized tables. A single atomic query claims queued tasks to avoid race conditions.
- **Worker** (`src/runner/`) — Background loop that polls for `QUEUED` tasks, transitions them through `INITIALIZING` → `RUNNING` → `COMPLETE` (or error states), and cleans up workspaces afterward.
- **Filer** (`src/filer/`) — Handles data staging between S3 and local workspaces. Supports concurrent transfers (up to 16) for both inputs and outputs. The `StorageBackend` trait allows adding new backends.
- **Docker Executor** (`src/runner/docker.rs`) — Pulls images, creates containers with computed bind mounts, captures stdout/stderr, and records exit codes. Uses Docker-outside-of-Docker (DooD) via socket mount.
- **DTOs** (`src/dto/`) — GA4GH TES data models with serde and OpenAPI schema annotations.

**Task lifecycle:**

1. `POST /ga4gh/tes/v1/tasks` — task is validated and persisted as `QUEUED`
2. Worker claims the oldest queued task (atomic `UPDATE ... RETURNING`)
3. Workspace is created, inputs are downloaded from S3
4. Executors run sequentially in Docker containers with bind-mounted workspaces
5. Outputs are uploaded to S3, logs are recorded
6. Final state is set (`COMPLETE`, `EXECUTOR_ERROR`, or `SYSTEM_ERROR`)

**Docker integration:**

PoiesisD does not run a nested Docker daemon. Instead, the host's Docker socket is mounted into the container (`/var/run/docker.sock`). Task containers are created by the host daemon, so bind mount paths must match between the PoiesisD container and the host — this is handled by the `POIESISD_WORKSPACE_ROOT` environment variable and a shared `/tmp/poiesisd` volume.
