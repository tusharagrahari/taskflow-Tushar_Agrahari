# TaskFlow — Backend API

A REST API for task management with authentication, projects, and tasks. Built as a backend engineering take-home assignment.

**Language note:** The spec lists Go as preferred. I used **Rust** — it's the language I work in daily at NPCI.

---

## 1. Overview

TaskFlow is a REST API that lets users register, log in, create projects, add tasks to projects, and assign tasks to team members.

| Layer | Choice |
|---|---|
| Language | Rust |
| Web Framework | Axum 0.8 |
| Database | PostgreSQL 16 |
| DB Driver | sqlx 0.8 (compile-time query verification) |
| Auth | JWT — 24hr expiry, HS256 |
| Password Hashing | bcrypt cost 12 |
| Logging | tracing + tracing-subscriber (structured JSON) |
| Containerization | Docker + docker-compose (multi-stage build) |

---

## 2. Architecture Decisions

### Error handling — single `AppError` enum

All errors in the codebase flow through one enum that implements `IntoResponse`. Each variant maps to a specific HTTP status and JSON shape. This means handlers never touch `StatusCode` directly for errors — they just return `Err(AppError::NotFound)` and the shape is guaranteed.

Postgres error codes are mapped at the boundary: `23505` (unique violation) → 409, `23503` (FK violation) → 400, `RowNotFound` → 404. Internal errors are logged server-side but never leaked to the client.

### Auth middleware as an Axum extractor

`AuthUser` implements `FromRequestParts<AppState>`. Any handler that declares `auth: AuthUser` in its signature automatically requires a valid JWT — no decorator, no middleware chain to wire up. If the token is missing or invalid, Axum returns 401 before the handler runs.

The extractor reads `jwt_secret` directly from `AppState` (loaded once at startup), not from `std::env::var` on every request.

### sqlx compile-time query verification

sqlx checks SQL queries against a live database at compile time. For Docker builds (no database available), `SQLX_OFFLINE=true` is set and a `.sqlx/` directory with cached query metadata is committed to the repo. This gives compile-time safety without requiring a database during `docker build`.

### Migrations run on startup

`sqlx::migrate!()` runs in `main.rs` before the server binds to a port. No separate migration container, no manual step. `docker compose up` → migrations apply → server starts. Down migrations exist for every file.

### Multi-stage Dockerfile with rustls

Builder stage compiles the release binary. Runtime stage is `debian:bookworm-slim` with just the binary copied over. sqlx uses the `tls-rustls` feature (pure Rust TLS) — no OpenSSL dependency, no extra packages needed in the runtime image.

### Pagination — offset-based

List endpoints (`GET /projects`, `GET /projects/:id/tasks`) support `?page=&limit=` with a default of `page=1, limit=20`. Limit is clamped to 100. Response envelope: `{data, total, page, limit}`. Two queries per request (COUNT + SELECT) — clean and readable at this scale.

### COALESCE for PATCH updates

PATCH handlers use `SET field = COALESCE($1, field)` — a single query that only updates fields that are non-null. Tradeoff: can't distinguish "field not sent" from "field explicitly set to null". For nullable fields like `description`, sending `null` does not clear it.

### What I intentionally left out

- **Refresh tokens** — single 24hr JWT. In production: short-lived access tokens + refresh token rotation.
- **Rate limiting** — no brute-force protection on auth endpoints. Would add `tower-governor`.
- **Soft deletes** — all deletes are hard (permanent). Production would add `deleted_at` column for audit trail.
- **Stats endpoint** — `GET /projects/:id/stats` is a bonus item, not implemented.
- **Integration tests** — no automated tests. The Bruno collection covers manual verification.

---

## 3. Running Locally

**Prerequisites:** Docker and Docker Compose. Nothing else required.

```bash
git clone https://github.com/tusharagrahari/taskflow-Tushar_Agrahari.git
cd taskflow-Tushar_Agrahari
cp .env.example .env
docker compose up --build
```

API is available at `http://localhost:8080`.

On first run:
- PostgreSQL starts and passes its health check
- Migrations run automatically
- Seed data is loaded (1 user, 1 project, 3 tasks)
- Server starts and logs `Server running on 0.0.0.0:8080`

To stop:
```bash
docker compose down
```

To reset the database:
```bash
docker compose down -v   # -v removes the named volume
docker compose up --build
```

---

## 4. Running Migrations

Migrations run **automatically on container startup**. No manual step required.

If you want to run them manually outside Docker:
```bash
cd backend
DATABASE_URL=postgres://taskflow:taskflow_secret@localhost:5432/taskflow cargo sqlx migrate run
```

---

## 5. Test Credentials

Seeded automatically on first run:

```
Email:    test@example.com
Password: password123
```

---

## 6. API Reference

A Bruno collection is included in the `bruno/` directory. Open it in the [Bruno desktop app](https://www.usebruno.com), select the **local** environment, run `auth/login` first (token is set automatically), then run any other request.

### Auth

#### POST /auth/register
```bash
curl -X POST http://localhost:8080/auth/register \
  -H "Content-Type: application/json" \
  -d '{"name": "Tushar", "email": "tushar@example.com", "password": "password123"}'
```
**201 Created:**
```json
{
  "token": "<jwt>",
  "user": { "id": "uuid", "name": "Tushar", "email": "tushar@example.com", "created_at": "..." }
}
```

#### POST /auth/login
```bash
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "password123"}'
```
**200 OK:** Same shape as register.

---

### Projects

All project endpoints require `Authorization: Bearer <token>`.

#### GET /projects
```bash
curl http://localhost:8080/projects -H "Authorization: Bearer <token>"
```
Returns projects the current user owns or has tasks assigned in. Supports `?page=1&limit=20`.

**200 OK:**
```json
{ "data": [{ "id": "uuid", "name": "...", "owner_id": "uuid", "created_at": "..." }], "total": 1, "page": 1, "limit": 20 }
```

#### POST /projects
```bash
curl -X POST http://localhost:8080/projects \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "My Project", "description": "Optional"}'
```
**201 Created:** Returns the created project object.

#### GET /projects/:id
```bash
curl http://localhost:8080/projects/<uuid> -H "Authorization: Bearer <token>"
```
**200 OK:** Returns project fields + embedded `tasks` array.

#### PATCH /projects/:id
Owner only. All fields optional.
```bash
curl -X PATCH http://localhost:8080/projects/<uuid> \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "New Name"}'
```
**200 OK:** Returns updated project. **403** if not owner.

#### DELETE /projects/:id
Owner only. Cascades to delete all tasks in the project.
```bash
curl -X DELETE http://localhost:8080/projects/<uuid> -H "Authorization: Bearer <token>"
```
**204 No Content.** **403** if not owner.

---

### Tasks

#### GET /projects/:id/tasks
Supports `?status=todo|in_progress|done`, `?assignee=<uuid>`, `?page=1&limit=20`.
```bash
curl "http://localhost:8080/projects/<uuid>/tasks?status=todo" \
  -H "Authorization: Bearer <token>"
```
**200 OK:**
```json
{ "data": [{ "id": "uuid", "title": "...", "status": "todo", "priority": "high", ... }], "total": 3, "page": 1, "limit": 20 }
```

#### POST /projects/:id/tasks
```bash
curl -X POST http://localhost:8080/projects/<uuid>/tasks \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"title": "Design homepage", "priority": "high", "due_date": "2026-05-01"}'
```
**201 Created:** Returns the created task. Status defaults to `todo`.

#### PATCH /tasks/:id
Project owner, task creator, or assignee only. All fields optional.
```bash
curl -X PATCH http://localhost:8080/tasks/<uuid> \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"status": "done"}'
```
**200 OK:** Returns updated task with `updated_at` refreshed. **403** if not authorized.

#### DELETE /tasks/:id
Project owner or task creator only.
```bash
curl -X DELETE http://localhost:8080/tasks/<uuid> -H "Authorization: Bearer <token>"
```
**204 No Content.** **403** if not authorized.

---

### Error Responses

All errors return `Content-Type: application/json`.

```json
// 400 — Validation
{ "error": "validation failed", "fields": { "email": "is required" } }

// 401 — Missing or invalid token
{ "error": "unauthorized" }

// 403 — Valid token, insufficient permissions
{ "error": "forbidden" }

// 404 — Resource not found
{ "error": "not found" }

// 409 — Conflict (e.g. duplicate email)
{ "error": "email already exists" }
```

---

## 7. What I'd Do With More Time

`GET /me` and `PATCH /me` are the most obvious missing endpoints. Right now there's no way to view or update your own profile — you register, get a token, and that's the last time you see your user data unless you query the DB directly. Adding these is a migration (no schema change needed, just new handlers) and about 30 lines of code, but I cut them because the spec didn't list them and I didn't want to add surface area that wasn't being evaluated.

Task comments would be the first feature addition I'd make. A `comments` table with `task_id`, `author_id`, `body`, and `created_at` is straightforward — the interesting part is the permission model. Right now only project members can see tasks; comments should inherit that same visibility. That relationship is what makes it worth thinking about rather than just bolting on.

Sorting on the task list. The current `GET /projects/:id/tasks` supports filtering by status and assignee but always returns in insertion order. In practice you want `?sort=due_date` or `?sort=priority` — priority especially, since that's how people actually triage work. I avoided dynamic ORDER BY because sqlx's compile-time query checks don't play well with runtime-constructed SQL. The clean solution is an allow-list enum for sort fields and a match statement that selects from a fixed set of prepared queries. More verbose than dynamic SQL but safe.

`GET /me/tasks` — a cross-project view of everything assigned to the current user. The query is a simple `WHERE assignee_id = $1` across tasks joined to projects, but the response shape is different from project-scoped task lists because you need to include the project name for context. It's a read-only endpoint that doesn't touch any existing logic, so it's genuinely low-risk to add. Just didn't make the cut for scope reasons.

Project archiving instead of hard delete. Right now `DELETE /projects/:id` is permanent — the project and all its tasks are gone. An `archived_at` column would let you hide projects from the default list view (`WHERE archived_at IS NULL`) while keeping the data. The complication is that every list query needs the filter, and you have to decide whether archived projects' tasks are still reachable via `GET /projects/:id/tasks`. Soft deletes sound simple but they leak into every read path.

Request IDs would make debugging significantly easier. Right now if two requests fail in the same second, the logs don't tell you which lines belong to which request. A UUID per request propagated through the tracing span would fix that.
