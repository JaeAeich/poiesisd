# PoiesisD

A [GA4GH][ga4gh]-compliant
[Task Execution Service (TES)][tes] running on Docker.

> [!NOTE]
> PoiesisD is a lightweight, development-focused TES.
> For production deployments, see [Poiesis][poiesis].

## Quick Start

```sh
docker compose up
```

That's it. Everything is running:

| Service | URL |
| --- | --- |
| Web UI | <http://localhost:8080> |
| TES API | <http://localhost:8080/ga4gh/tes/v1> |

## Features

- **TES v1.1** — create, list, and monitor tasks
- **Docker executor** — runs task containers locally
- **S3 filer** — stage inputs/outputs via S3-compatible storage
- **Built-in UI** — task explorer, live logs, dark/light theme
- **Single binary** — everything embedded, zero external deps

> [!TIP]
> For production use, Kubernetes support, and full docs,
> check out [Poiesis][poiesis].

[ga4gh]: https://www.ga4gh.org/
[tes]: https://www.ga4gh.org/product/task-execution-service-tes/
[poiesis]: https://github.com/jaeaeich/poiesis
