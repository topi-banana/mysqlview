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
- DDL wizards for managing schemas: `CREATE`/`DROP DATABASE` from the home and
  database pages, plus `CREATE`/`ALTER`/`DROP TABLE` (ALTER supports
  add/drop/modify/rename column and rename table) gated behind confirmation
  dialogs for destructive actions
- CSV / SQL **export**: stream a table out as CSV or as `INSERT` statements;
  dump an entire database (DDL + data, foreign-key checks toggled off and back
  on around the dump) for backup or sharing
- CSV / SQL **import**: bulk-load rows from a CSV file or paste a multi-statement
  SQL script into a database; the wizard surfaces the failing row /
  statement index when an import stops short
- Console for executing arbitrary SQL (read or write) with results rendered as a typed data grid
- A consistent, type-safe API thanks to a shared `mysqlview-types` crate used by both backend and frontend

## Architecture

```
mysqlview/
├── types/        # Shared serde DTOs (no MySQL or Yew deps)
├── backend/      # tokio + axum + sqlx (MySQL)
└── frontend/     # yew 0.23 + tailwindcss (struct-based components)
```

- The backend pulls schema metadata from `information_schema` and `SHOW CREATE TABLE`.
- The backend rejects any identifier that does not match `^[A-Za-z0-9_$]{1,64}$` and verifies existence in `information_schema` before quoting and interpolating it. All filter values use `sqlx` parameter binding.
- DDL requests reuse the same identifier allowlist and additionally validate every column type / DEFAULT fragment against a character allowlist that rejects semicolons, backticks, comment markers, and unbalanced quotes. The exact statement the server executed is returned in the response so the UI can echo it back.
- Exports stream straight out of `sqlx::query(...).fetch(...)` into `axum::body::Body::from_stream`, so multi-million-row tables never materialise in memory. SQL outputs terminate with a `-- EXPORT COMPLETE` sentinel so truncated downloads are detectable post-hoc. Imports run statement-by-statement (multi-statement SQL is split with a quote-/comment-aware parser that also handles `#` line comments and rejects `DELIMITER` directives) and fail fast on the first error.
- The frontend issues JSON-over-HTTP requests to `/api/*`. The data grid renders MySQL values via a typed `CellValue` enum (Null/Bool/Int/Float/String/Bytes/Json) so dates, decimals, and JSON columns survive a round trip without precision loss.

### CSV conventions

The CSV import / export follows the same round-trip-safe convention as
PostgreSQL's `\copy ... CSV`:

| CellValue        | CSV cell                                                  |
|---|---|
| `NULL`           | empty unquoted cell                                       |
| Empty string     | quoted `""`                                               |
| Bytes (blob)     | `b64:<base64>` so they're distinguishable from real text  |
| JSON             | serialised JSON text (quoted per RFC-4180 if needed)      |

The output is UTF-8 without a BOM — Excel on Windows may need to be told the
encoding explicitly.

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

Build the frontend first, then build the backend with the `embedded-frontend`
feature so the `frontend/dist/` tree is baked into the binary by
`include_dir!`:

```sh
cd frontend && trunk build --release && cd ..
cargo build --release -p mysqlview-backend --features embedded-frontend

DATABASE_URI=mysql://root:pass@127.0.0.1:3306 \
  ./target/release/mysqlview-backend
```

The resulting `target/release/mysqlview-backend` is fully self-contained: no
`--frontend-dist` is needed, and the binary can be copied to any other
machine of the same target triple. The backend serves `/api/*` JSON
endpoints and every other path resolves to the embedded SPA (with index.html
as the catch-all fallback, so client-side routes survive a hard reload).

CI publishes two flavors of the binary on every push to `main` and every PR:

- `mysqlview-linux-x86_64` — dynamically linked against glibc
- `mysqlview-linux-x86_64-musl` — fully static (`x86_64-unknown-linux-musl`), suitable for `FROM scratch` containers

## Docker (scratch image)

```sh
docker build -t mysqlview .
docker run --rm -it \
  -p 127.0.0.1:3000:3000 \
  -e DATABASE_URI=mysql://root:pass@host.docker.internal:3306 \
  mysqlview
```

The resulting image is `FROM scratch` plus the static musl binary — nothing else. Ctrl-C / `docker stop` work because the backend installs explicit SIGINT/SIGTERM handlers (Linux silently drops signals delivered to PID 1 unless a handler is installed).

The image declares its own `HEALTHCHECK` that runs `mysqlview-backend --healthcheck` from inside the container — no `curl`/`wget` are needed thanks to the binary doubling as its own HTTP probe. The probe queries `/api/health`, which pings the MySQL pool, so a `healthy` status means both the HTTP server and the database are reachable.

> Keep the host-side port mapped to `127.0.0.1` (`-p 127.0.0.1:3000:3000`). The binary still has no authentication.

### Development variant (no embedding)

For iteration during backend work, the default-feature build keeps the
runtime `ServeDir` fallback and reads the dist on disk:

```sh
cd frontend && trunk build --release && cd ..
cargo run --release -p mysqlview-backend -- --frontend-dist ./frontend/dist
```

## CLI flags

| Flag | Env | Default | Purpose |
|---|---|---|---|
| `--bind` | `MYSQLVIEW_BIND` | `127.0.0.1` | IP address to listen on |
| `--port` | `MYSQLVIEW_PORT` | `3000` | Port to listen on |
| `--database-url` | `DATABASE_URI` | *(required)* | MySQL connection URI |
| `--frontend-dist` | `MYSQLVIEW_FRONTEND_DIST` | *(unset)* | Path to `frontend/dist` for static serving |
| `--max-rows` | `MYSQLVIEW_MAX_ROWS` | `1000` | Maximum rows returned by any single query |
| `--max-import-bytes` | `MYSQLVIEW_MAX_IMPORT_BYTES` | `104857600` (100 MiB) | Maximum body size accepted by the CSV / SQL import endpoints |

## Quality checks

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p mysqlview-frontend --target wasm32-unknown-unknown -- -D warnings
cargo fmt --all -- --check
```

## Future phases

- ~~Phase 2: row-level editing (INSERT/UPDATE/DELETE)~~ ✅ shipped
- ~~Phase 3: DDL wizards (CREATE/ALTER/DROP TABLE, CREATE/DROP DATABASE)~~ ✅ shipped
- ~~Phase 4: CSV / SQL import & export~~ ✅ shipped
- Phase 5: SQL editor enhancements (syntax highlighting, autocomplete), dark mode, saved queries

## License

MIT — see [LICENSE](./LICENSE).
