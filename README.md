# mysqlview

A local-only, OSS-quality MySQL WebUI written in Rust. Backed by [Axum](https://github.com/tokio-rs/axum) and [sqlx](https://github.com/launchbadge/sqlx), with a [Yew](https://yew.rs/) + [Tailwind CSS](https://tailwindcss.com/) frontend bundled into a single binary.

> ⚠️ **Security warning — local development only.** This tool ships **without authentication**. It binds to `127.0.0.1` by default and is intended exclusively for use against your own development databases. **Do not** expose it to the public internet, shared networks, or any environment you do not fully trust. The `DATABASE_URI` is passed via environment variable and contains credentials in plaintext.

## Features

- Browse databases and tables on any MySQL server reachable via `DATABASE_URI`
- Inspect table structure: columns, indexes, foreign keys, and the original `CREATE TABLE` statement
- Paginated, sortable, and filterable row browsing for any table
- Row-level editing for any table with a primary key or a NOT NULL UNIQUE index:
  modal-based form editor supports adding, updating, and deleting rows with NULL
  toggles per column. Tables without an identifying key surface a banner and
  the editing UI is disabled
- Console for executing arbitrary SQL (read or write) with results rendered as a typed data grid
- A consistent, type-safe API thanks to a shared `mysqlview-types` crate used by both backend and frontend

DDL operations and import/export remain **out of scope** and are planned for future phases.

## Architecture

```
mysqlview/
├── types/        # Shared serde DTOs (no MySQL or Yew deps)
├── backend/      # tokio + axum + sqlx (MySQL)
└── frontend/     # yew 0.23 + tailwindcss (struct-based components)
```

- The backend pulls schema metadata from `information_schema` and `SHOW CREATE TABLE`.
- The backend rejects any identifier that does not match `^[A-Za-z0-9_$]{1,64}$` and verifies existence in `information_schema` before quoting and interpolating it. All filter values use `sqlx` parameter binding.
- The frontend issues JSON-over-HTTP requests to `/api/*`. The data grid renders MySQL values via a typed `CellValue` enum (Null/Bool/Int/Float/String/Bytes/Json) so dates, decimals, and JSON columns survive a round trip without precision loss.

## Quickstart (development)

Requirements:

- Rust ≥ 1.84 (workspace uses edition 2024)
- `trunk` for serving the frontend: `cargo install trunk`
- The `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- A reachable MySQL server (use Docker: `docker run --name mysql-dev -e MYSQL_ROOT_PASSWORD=pass -p 3306:3306 -d mysql:8`)

In one shell, start the backend:

```sh
DATABASE_URI=mysql://root:pass@127.0.0.1:3306 cargo run -p mysqlview-backend
```

In another shell, serve the frontend with a proxy to the backend:

```sh
cd frontend
trunk serve --proxy-backend=http://127.0.0.1:3000/api
```

Then open <http://127.0.0.1:8080>.

## Production build (single binary)

```sh
cd frontend && trunk build --release
cargo run --release -p mysqlview-backend -- --frontend-dist ./frontend/dist
```

The backend will serve both `/api/*` JSON endpoints and the static frontend on the same port (default `3000`).

## CLI flags

| Flag | Env | Default | Purpose |
|---|---|---|---|
| `--bind` | `MYSQLVIEW_BIND` | `127.0.0.1` | IP address to listen on |
| `--port` | `MYSQLVIEW_PORT` | `3000` | Port to listen on |
| `--database-url` | `DATABASE_URI` | *(required)* | MySQL connection URI |
| `--frontend-dist` | `MYSQLVIEW_FRONTEND_DIST` | *(unset)* | Path to `frontend/dist` for static serving |
| `--max-rows` | `MYSQLVIEW_MAX_ROWS` | `1000` | Maximum rows returned by any single query |

## Quality checks

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p mysqlview-frontend --target wasm32-unknown-unknown -- -D warnings
cargo fmt --all -- --check
```

## Future phases

- Phase 2: row-level editing (INSERT/UPDATE/DELETE)
- Phase 3: DDL wizards (CREATE/ALTER/DROP TABLE, CREATE/DROP DATABASE)
- Phase 4: CSV / SQL import & export
- Phase 5: SQL editor enhancements (syntax highlighting, autocomplete), dark mode, saved queries

## License

MIT — see [LICENSE](./LICENSE).
