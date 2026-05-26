# Rust-Red Security

This document describes the security posture of Rust-Red, default settings,
and how to configure security for your deployment.

## Default Security Posture

Rust-Red is configured **secure by default**. Out of the box:

- **Security headers** are enabled (CSP, X-Frame-Options, X-Content-Type-Options, etc.)
- **CORS** is restricted to same-origin only (no cross-origin access)
- **Rate limiting** is active at 300 requests/minute per client IP
- **Flow payload size** is capped at 10 MB
- **Admin API** binds to `127.0.0.1` (localhost only) by default

## Security Configuration

All security settings are in the `[security]` section of `rust-red.toml`:

```toml
[security]
# Require authentication for admin API access.
# Currently advisory (logged but not enforced).
# Future versions will implement HTTP Basic/Bearer auth.
require_auth = true

# Maximum size for flow JSON payloads in bytes.
# Prevents denial-of-service via oversized payloads.
# Default: 10485760 (10 MB)
max_flow_size = 10485760

# Rate limit: requests per minute per client IP.
# Protects against brute force and basic DoS.
# Set to 0 to disable rate limiting.
# Default: 300
rate_limit_rpm = 300

# Allowed CORS origins for the admin API.
# Empty array (default) = same-origin only (most secure).
# ["*"] = allow all origins (NOT recommended for production).
# ["http://localhost:3000"] = allow specific origin(s).
cors_origins = []

# Enable/disable security response headers.
# Default: true
security_headers = true
```

## Authentication

**Current status:** Authentication is not yet enforced. The admin API accepts
all requests. This is a known limitation being addressed in a future release.

**Mitigation:** Bind the admin API to localhost (`127.0.0.1`) and use a
reverse proxy (nginx, Caddy, etc.) with authentication in front of Rust-Red.

### Planned authentication features:
- HTTP Basic authentication with configurable credentials
- Token-based API authentication
- Session management for the editor UI

## Network Binding

By default, Rust-Red binds to `127.0.0.1:1888`, making it accessible only
from the local machine. This is intentional for security.

To expose Rust-Red to a network (do so with caution):

```toml
[ui-host]
host = "0.0.0.0"  # Listen on all interfaces
port = 1888
```

**Warning:** Only bind to `0.0.0.0` if you have a reverse proxy with
authentication in front, or if the network is trusted.

## Security Headers

When `security_headers = true` (default), the following headers are added
to all HTTP responses:

| Header | Value | Purpose |
|--------|-------|---------|
| X-Content-Type-Options | nosniff | Prevent MIME-type sniffing |
| X-Frame-Options | DENY | Prevent clickjacking |
| X-XSS-Protection | 0 | Disable legacy XSS filter (CSP preferred) |
| Referrer-Policy | strict-origin-when-cross-origin | Limit referrer leakage |
| Content-Security-Policy | (see below) | Control resource loading |
| Permissions-Policy | camera=(), microphone=(), geolocation=() | Disable unnecessary APIs |

### Content Security Policy

The CSP is configured for Node-RED editor compatibility:
```
default-src 'self';
script-src 'self' 'unsafe-inline' 'unsafe-eval';
style-src 'self' 'unsafe-inline';
img-src 'self' data: blob:;
font-src 'self';
connect-src 'self' ws: wss:;
worker-src 'self' blob:;
frame-ancestors 'none';
```

The `unsafe-inline` and `unsafe-eval` directives are required for the
Node-RED editor frontend. For production deployments behind a reverse proxy,
consider tightening the CSP.

## Rate Limiting

Rate limiting protects the admin API from abuse. The default is 300 requests
per minute per client IP.

Rate-limited requests receive a `429 Too Many Requests` response with a
`Retry-After: 60` header.

## Known Security Considerations

### Exec Node
The `exec` node allows flows to execute arbitrary shell commands. This is
a core Node-RED feature but presents a significant security risk. An attacker
who can deploy flows can achieve **Remote Code Execution (RCE)**.

Mitigations:
- Restrict admin API access (bind to localhost, use a reverse proxy)
- Audit deployed flows for exec node usage
- Future versions will support disabling the exec node via configuration

### Function Node
The `function` node executes JavaScript (via QuickJS). While sandboxed,
arbitrary code execution within the JS runtime is possible by design.

### WebSocket Endpoint
The `/comms` WebSocket endpoint is used by the editor UI. It currently
accepts connections without authentication. The WebSocket sends a
`{"auth": "required"}` message for Node-RED compatibility but does not
enforce authentication.

### No TLS Termination
Rust-Red does not include built-in TLS/HTTPS support. Use a reverse proxy
(nginx, Caddy, Traefik, etc.) for TLS termination. The security headers
module does not add `Strict-Transport-Security` since it should be set at
the reverse proxy level.

## Reverse Proxy Setup

For production, run Rust-Red behind a reverse proxy:

```
Internet -> Reverse Proxy (TLS, Auth) -> Rust-Red (localhost:1888)
```

Example nginx configuration:

```nginx
server {
    listen 443 ssl http2;
    server_name rust-red.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    # Auth
    auth_basic "Rust-Red Admin";
    auth_basic_user_file /path/to/.htpasswd;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    location / {
        proxy_pass http://127.0.0.1:1888;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

## Reporting Security Issues

If you discover a security vulnerability in Rust-Red, please report it
responsibly by opening a GitHub issue at:
https://github.com/dzulfikar08/rust-red/issues

Please do not publicly disclose security vulnerabilities before they have
been addressed.
