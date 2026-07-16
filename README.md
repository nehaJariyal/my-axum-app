# My Axum App

A production-ready backend REST API built with **Rust**, **Axum**, **PostgreSQL**, **SQLx**, **Redis**, and **Docker**.

This project demonstrates modern backend development practices, including JWT authentication, Google OAuth, secure cookies, request validation, middleware, caching, and modular architecture.

---

## Features

* JWT Authentication
* Google OAuth 2.0 Login
* Secure HttpOnly Cookie Authentication
* CSRF Protection
* Password Hashing with Argon2
* PostgreSQL Database
* SQLx Compile-Time Checked Queries
* Redis Caching
* RESTful APIs
* Request Validation
* Request Logging Middleware
* Environment Configuration
* Docker Support
* Modular Feature-Based Architecture
* Async Runtime with Tokio

---

## Tech Stack

| Technology   | Description          |
| ------------ | -------------------- |
| Rust         | Programming Language |
| Axum         | Web Framework        |
| Tokio        | Async Runtime        |
| PostgreSQL   | Database             |
| SQLx         | Database Toolkit     |
| Redis        | Cache                |
| JWT          | Authentication       |
| Google OAuth | Social Login         |
| Argon2       | Password Hashing     |
| Serde        | Serialization        |
| Reqwest      | HTTP Client          |
| Validator    | Validation           |
| Chrono       | Date & Time          |
| UUID         | Unique Identifiers   |
| Docker       | Containerization     |

---

## Project Structure

```text
.
├── Cargo.lock
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
│
├── docker/
│   ├── aeron/
│   ├── aeron-exporter/
│   ├── aeron-viewer/
│   ├── grafana/
│   ├── pgadmin/
│   └── prometheus/
│
├── migrations/
│   ├── 001_create_users.sql
│   ├── add_password_to_users.sql
│   ├── add_avatar_url.sql
│   ├── add_oauth_to_users.sql
│   └── create_wal_entries.sql
│
├── src/
│   ├── main.rs
│   ├── router.rs
│   ├── config.rs
│   ├── db.rs
│   ├── redis.rs
│   ├── aeron.rs
│   ├── wal.rs
│   ├── error.rs
│   ├── controllers/
│   ├── middleware/
│   ├── guards/
│   ├── models/
│   ├── db/
│   └── modules/
│
├── .env.example
├── .gitignore
└── README.md
```

---

## Environment Variables

Create a `.env` file.

```env
HOST=0.0.0.0
PORT=3001

JWT_SECRET=your_jwt_secret

POSTGRES_USER=postgres
POSTGRES_PASSWORD=postgres
POSTGRES_DB=mydb

DATABASE_URL=postgres://postgres:postgres@localhost:5432/mydb

REDIS_URL=redis://127.0.0.1:6379
REDIS_PASSWORD=

GOOGLE_CLIENT_ID=your_google_client_id
GOOGLE_CLIENT_SECRET=your_google_client_secret
GOOGLE_REDIRECT_URI=http://localhost:3001/api/auth/google/callback

USERS_CACHE_TTL_SECS=300
```

> Never commit your `.env` file or any real credentials. Use `.env.example` with placeholder values.

---

## Getting Started

### Clone Repository

```bash
git clone https://github.com/YOUR_USERNAME/my-axum-app.git
cd my-axum-app
```

### Install Dependencies

```bash
cargo build
```

### Run Database Migrations

```bash
sqlx migrate run
```

### Start the Application

```bash
cargo run
```

The server will start on:

```text
http://localhost:3001
```

---

## Docker

Start services:

```bash
docker compose up -d
```

Stop services:

```bash
docker compose down
```

---

## Authentication

### Email Authentication

* Register
* Login
* Logout

### Google OAuth

```http
GET /api/auth/google
```

Callback:

```http
GET /api/auth/google/callback
```

Authentication tokens are stored using secure HttpOnly cookies.

---

## Cookie Security

* HttpOnly Cookies
* Secure Cookies
* SameSite Protection
* CSRF Protection

---

## Architecture

```text
HTTP Request
      │
      ▼
   Router
      │
      ▼
 Controller
      │
      ▼
   Service
      │
      ▼
 Repository
   │      │
   ▼      ▼
PostgreSQL Redis
```

---

## Modules

* Authentication
* Google OAuth
* User Management
* Middleware
* Database
* Redis
* Aeron Integration
* Write-Ahead Log (WAL)

---

## Development Commands

Build

```bash
cargo build
```

Run

```bash
cargo run
```

Check

```bash
cargo check
```

Format

```bash
cargo fmt
```

Lint

```bash
cargo clippy
```

Run Tests

```bash
cargo test
```

Clean

```bash
cargo clean
```

---

## Roadmap

* Refresh Token Authentication
* Email Verification
* Password Reset
* Role-Based Access Control (RBAC)
* Swagger/OpenAPI Documentation
* API Versioning
* Rate Limiting
* Unit Tests
* Integration Tests
* CI/CD Pipeline
* Kubernetes Deployment
* Monitoring with Prometheus and Grafana

---

## Contributing

1. Fork the repository.
2. Create a feature branch.
3. Commit your changes using meaningful commit messages.
4. Push your branch.
5. Open a Pull Request.

---

