# Eduxal Authentication Service

> Stateless, phone-based OTP authentication service built with **Rust**, **Axum**, and **AWS Lambda**. Verifies identities via a 6-digit OTP delivered over the **WhatsApp Business API**, issues **PASETO v4-local** tokens, and persists state in **DynamoDB**.

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Authentication Flows](#authentication-flows)
  - [New User Registration](#new-user-registration)
  - [Returning User Login](#returning-user-login)
  - [Token Refresh](#token-refresh)
  - [Phone Number Change](#phone-number-change)
- [Token System](#token-system)
- [API Reference](#api-reference)
  - [POST /auth/login](#post-authlogin)
  - [POST /auth/verify](#post-authverify)
  - [POST /auth/setup](#post-authsetup)
  - [GET /auth/refresh](#get-authrefresh)
  - [GET /user](#get-user)
  - [PATCH /user/rename](#patch-userrename)
  - [POST /user/change-phone](#post-userchange-phone)
  - [PATCH /user/confirm-change-phone](#patch-userconfirm-change-phone)
  - [GET /sessions](#get-sessions)
  - [DELETE /sessions/{id}](#delete-sessionsid)
  - [GET /health](#get-health)
- [Error Reference](#error-reference)
- [Database Schema](#database-schema)
- [Configuration](#configuration)
- [Development](#development)
- [Deployment](#deployment)
- [Known Behaviors & Security Considerations](#known-behaviors--security-considerations)

---

## Overview

The Eduxal Authentication Service handles the full identity lifecycle for the Eduxal platform:

- **Passwordless login** via WhatsApp OTP (one-time password)
- **Account registration** with name and device capture
- **Session management** (list and revoke individual sessions)
- **Profile mutations** (rename, change phone number)
- **Token rotation** via a dedicated refresh endpoint

There are no passwords. A user proves ownership of a phone number by entering a 6-digit code sent to them on WhatsApp. New users are detected automatically during the verify step and routed through a one-time setup flow.

---

## Architecture

```
Client
  │
  ▼
API Gateway HTTP API (auth.eduxal.com)
  │  CORS: all origins, GET/POST/PUT/DELETE/OPTIONS
  │  Custom domain via ACM certificate + Cloudflare CNAME
  ▼
AWS Lambda  (eduxal-authentication)
  ├── Runtime:    provided.al2023 (Rust, cargo-lambda)
  ├── Arch:       arm64
  ├── Memory:     256 MB
  ├── Timeout:    29 seconds
  └── Tracing:    AWS X-Ray (Active)
       │
       ├── DynamoDB ──── eduxal-users         (user records)
       │            ├─── eduxal-sessions      (session records)
       │            └─── eduxal-verifications (OTP records)
       │
       └── WhatsApp Business API (Facebook Graph API v21.0)
                         OTP delivery via "auth_code" template
```

The Lambda function is a single Axum router deployed as a catch-all proxy behind API Gateway. All three DynamoDB tables are granted full CRUD access via IAM `DynamoDBCrudPolicy`.

---

## Authentication Flows

### New User Registration

```
1.  POST /auth/login       { phone }
      → OTP sent to phone via WhatsApp
      ← 200 { phone, created, ttl }

2.  POST /auth/verify      { phone, code, session: null, device }
      → OTP validated, no existing user found
      ← 200 { token }          ← short-lived Setup token (15 min)

3.  POST /auth/setup       Authorization: Bearer <setup_token>
                           { name, device }
      → User record created, session opened
      ← 200 { session, access_token, refresh_token, user }
```

### Returning User Login

```
1.  POST /auth/login       { phone }
      → OTP sent to phone via WhatsApp
      ← 200 { phone, created, ttl }

2.  POST /auth/verify      { phone, code, session, device }
      → OTP validated, existing user found
      → Session created (or reused if session ID provided and owned by this user)
      ← 200 { session, access_token, refresh_token, user }
```

### Token Refresh

```
GET /auth/refresh          Authorization: Bearer <refresh_token>
  → Session validated, fresh token pair issued
  ← 200 { session, access_token, refresh_token, user }
```

### Phone Number Change

```
1.  POST /user/change-phone           Authorization: Bearer <access_token>
                                      { phone }
      → OTP sent to NEW phone via WhatsApp
      ← 200 { phone, created, ttl }

2.  PATCH /user/confirm-change-phone  Authorization: Bearer <access_token>
                                      { phone, code }
      → OTP validated, phone field updated on user record
      ← 200 { ...updated User }
```

---

## Token System

All tokens are **PASETO v4-local** (symmetric encryption with `PASETO_PASSWORD`). The encrypted payload contains:

| Field | Description |
|---|---|
| `id` | Unique token ID (MongoDB ObjectId) |
| `subject` | Either a user `Id` or a `Phone`, depending on purpose |
| `session` | Session ID (only present on Access and Refresh tokens) |
| `purpose` | Numeric discriminant: `1` = Access, `2` = Refresh, `3` = Setup |
| `created` | Issuance timestamp |
| `expires` | Expiry timestamp |

### Token Types

| Token | Purpose Code | TTL | Subject Type | Used On |
|---|---|---|---|---|
| **Access** | `1` | 2 days | User `Id` | All authenticated endpoints |
| **Refresh** | `2` | 52 days | User `Id` | `GET /auth/refresh` only |
| **Setup** | `3` | 15 minutes | `Phone` | `POST /auth/setup` only |

### Token Extraction (Axum Extractor)

Every protected endpoint declares a `Token<P>` extractor in the handler signature. Axum runs this **before** the handler body, so any token failure short-circuits the request:

1. Read `Authorization` header → missing or non-`Bearer ` prefix → **401**
2. Strip `Bearer ` prefix, attempt PASETO v4-local decryption → failure → **400**
3. Check `expires < now()` → **401**
4. Check `purpose == P::KIND` → mismatch → **403**

---

## API Reference

**Base URL:** `https://auth.eduxal.com`

**Common error body shape** (for all service-layer errors):
```json
{
  "message": "<human-readable description>"
}
```

> ⚠️ Errors thrown by Axum's extractor layer (JSON parse failures, missing `Content-Type`) use **plain-text** bodies, not the `{ "message" }` shape.

---

### POST /auth/login

Sends a 6-digit OTP to the given phone number via WhatsApp. This is always the first step in any authentication or login flow.

**Authentication:** None required.

#### Request

```
POST /auth/login
Content-Type: application/json
```

```json
{
  "phone": "+254759762268"
}
```

| Field | Type | Required | Constraints |
|---|---|---|---|
| `phone` | string | ✅ | E.164 format. Must start with `+`. Must pass `phone-number-verifier` validation. |

#### Response — `200 OK`

```json
{
  "phone":   "+254759762268",
  "created": "2025-04-26T07:30:00Z",
  "ttl":     "2025-04-26T07:45:00Z"
}
```

| Field | Description |
|---|---|
| `phone` | The phone number the OTP was sent to. |
| `created` | UTC timestamp when the OTP was generated. |
| `ttl` | UTC timestamp when the OTP expires (always `created + 15 minutes`). |

> **The OTP code is never present in the response.** It is delivered exclusively via WhatsApp.

#### Errors

| Status | Message | Cause |
|---|---|---|
| `400` | *(Axum rejection — plain text)* | Body is not valid JSON, or `Content-Type` is not `application/json`. |
| `422` | *(Axum rejection — plain text)* | `phone` field is missing or fails E.164 validation. |
| `429` | `too many requests, Please slow down` | A verification code was already sent to this phone within the past **60 seconds**. |
| `500` | `internal server error` | WhatsApp API call failed, or DynamoDB read/write failed. |

#### Notes

- The 60-second rate limit is keyed on the **phone number**, not on the client IP.
- A new successful call **immediately overwrites** any previous OTP for that phone; prior codes can no longer be verified.
- The WhatsApp delivery happens **before** the record is persisted to DynamoDB. If the DynamoDB write fails, the user will have received a code that cannot be verified (they must wait 60 seconds and retry).
- Phone numbers must be on WhatsApp — there is no SMS fallback.

---

### POST /auth/verify

Validates the OTP received via WhatsApp. The response shape differs based on whether the phone number belongs to an existing user or not.

**Authentication:** None required.

#### Request

```
POST /auth/verify
Content-Type: application/json
```

```json
{
  "phone":   "+254759762268",
  "code":    "136483",
  "session": "69ed056664bc5d80849a9322",
  "device":  "Ubuntu Linux, Bruno"
}
```

| Field | Type | Required | Constraints |
|---|---|---|---|
| `phone` | string | ✅ | E.164 format. |
| `code` | string | ✅ | The 6-digit OTP received on WhatsApp. |
| `session` | string \| null | ❌ | 24-char hex MongoDB ObjectId. If provided, the server will attempt to reuse this session ID. Send `null` or omit if no existing session. |
| `device` | string | ✅ | Human-readable device identifier. Stored on the new session record. Ignored for new (unregistered) users. |

#### Response — `200 OK` (Existing User)

```json
{
  "session":       "69ed056664bc5d80849a9322",
  "access_token":  "v4.local.ukuGtEfqElk6...",
  "refresh_token": "v4.local.rPWuvV7iMHrS...",
  "user": {
    "id":       "507f1f77bcf86cd799439011",
    "phone":    "+254759762268",
    "name":     "Abdihakim Osman",
    "level":    "Normal",
    "status":   "Active",
    "profiled": true,
    "created":  "2025-01-15T10:30:00Z"
  }
}
```

#### Response — `200 OK` (New / Unregistered User)

```json
{
  "token": "v4.local.zXZ3HE1F7aR5..."
}
```

The `token` field is a **Setup token** (PASETO v4-local, KIND=3, valid 15 minutes). It must be passed to `POST /auth/setup` to complete account registration.

> **Distinguishing the two shapes:** There is no `type` discriminant field. If the response has a `token` key, the user is new. If it has `session`, `access_token`, `refresh_token`, and `user`, the user is registered.

#### Errors

| Status | Message | Cause |
|---|---|---|
| `400` | `invalid verification code` | The submitted `code` does not match the stored OTP. |
| `400` | `invalid phone number...` | `phone` fails E.164 validation. |
| `400` | `invalid id` | `session` field is present but not a valid 24-hex ObjectId. |
| `403` | `permission denied` | The user account for this phone is **Blocked**. |
| `404` | `verification code not found` | `POST /auth/login` was never called for this phone, or the record has been evicted from DynamoDB. |
| `500` | `internal server error` | Any DynamoDB operation failed. |

#### Session Reuse Logic

When `session` is provided, the server attempts to reuse it:
1. Fetches the session record by ID.
2. If it exists **and** `session.user == current user`, the session ID is reused (the session's `ttl` and `refresh` are refreshed).
3. In any other case (not found, wrong owner), a brand-new session is created.

Always read `session` from the response — it may differ from what you sent.

#### Notes

- Blocked users are checked **twice** (before and after OTP validation) to close a potential race window.
- The verification record is **not deleted** after a successful verify. The same code can technically be reused until the DynamoDB TTL evicts the record (up to 48 hours after `ttl`).
- For new users, `device` and `session` are silently ignored — no session is created.

---

### POST /auth/setup

Completes account registration for a newly verified phone number. Requires the **Setup token** returned by `POST /auth/verify` when the phone was unrecognized.

**Authentication:** `Authorization: Bearer <setup_token>` (Setup token — KIND=3)

#### Request

```
POST /auth/setup
Authorization: Bearer v4.local.zXZ3HE1F7aR5...
Content-Type: application/json
```

```json
{
  "name":   "Abdihakim Osman",
  "device": "Ubuntu Linux, Bruno"
}
```

| Field | Type | Required | Constraints |
|---|---|---|---|
| `name` | string | ✅ | Display name. No length or character validation enforced at the API layer. |
| `device` | string | ✅ | Human-readable device identifier. Stored on the session. |

#### Response — `200 OK`

```json
{
  "session":       "69ed056664bc5d80849a9322",
  "access_token":  "v4.local.ukuGtEfqElk6...",
  "refresh_token": "v4.local.rPWuvV7iMHrS...",
  "user": {
    "id":       "507f1f77bcf86cd799439011",
    "phone":    "+254759762268",
    "name":     "Abdihakim Osman",
    "level":    "Normal",
    "status":   "Active",
    "profiled": false,
    "created":  "2025-04-26T07:55:00Z"
  }
}
```

New users always have `level: "Normal"`, `status: "Active"`, and `profiled: false`.

#### Errors

| Status | Message | Cause |
|---|---|---|
| `400` | `invalid token` | Bearer token cannot be decrypted or is malformed. |
| `401` | `unauthorized` | `Authorization` header missing, wrong prefix, or Setup token is expired (> 15 min old). |
| `403` | `permission denied` | Token is valid but is an Access or Refresh token (not a Setup token). Also returned if the user is **Blocked**. |
| `422` | *(Axum rejection — plain text)* | `name` or `device` is missing from the body. |
| `500` | `internal server error` | DynamoDB read or write failed. |

#### Notes

- The **phone number is taken entirely from the Setup token**, not from the request body. The body only provides the display name and device label.
- If the phone number already has a user record (e.g., the user called this endpoint twice), the user will be **renamed** to the provided `name` and a fresh session will be created. This endpoint is **not idempotent** for existing users.
- Calling this endpoint for a Blocked user will **commit the rename** to DynamoDB before the 403 is returned — the name is updated even though the response is an error.
- The Setup token has a hard 15-minute TTL. If it expires, the user must restart from `POST /auth/login`.

---

### GET /auth/refresh

Exchanges a valid Refresh token for a fresh Access token and Refresh token pair. The session ID remains the same.

**Authentication:** `Authorization: Bearer <refresh_token>` (Refresh token — KIND=2)

#### Request

```
GET /auth/refresh
Authorization: Bearer v4.local.rPWuvV7iMHrS...
```

No request body.

#### Response — `200 OK`

```json
{
  "session":       "69ed056664bc5d80849a9322",
  "access_token":  "v4.local.<new_access_token>",
  "refresh_token": "v4.local.<new_refresh_token>",
  "user": {
    "id":       "507f1f77bcf86cd799439011",
    "phone":    "+254759762268",
    "name":     "Abdihakim Osman",
    "level":    "Normal",
    "status":   "Active",
    "profiled": true,
    "created":  "2025-01-15T10:30:00Z"
  }
}
```

Both `access_token` and `refresh_token` are **newly generated** on every call. The `session` ID in the response is always the same one embedded in the Refresh token.

#### Errors

| Status | Message | Cause |
|---|---|---|
| `400` | `invalid token` | Refresh token cannot be decrypted or is malformed. |
| `401` | `unauthorized` | `Authorization` header missing, wrong prefix, or Refresh token is expired (> 52 days old). |
| `403` | `permission denied` | Token is an Access or Setup token (not a Refresh token). Also returned if the session document no longer exists (e.g., it was explicitly deleted). Also returned if the user is **Blocked**. |
| `404` | `user not found` | The user record referenced by the token was deleted from DynamoDB. |
| `500` | `internal server error` | DynamoDB read or write failed. |

#### Notes

- A `401` signals **session expiry** — the refresh token has expired and the user must re-authenticate from scratch.
- A `403` (when the session is missing) signals **session revocation** — the session was explicitly deleted via `DELETE /sessions/{id}`. Clear client credentials and redirect to login.
- A `404` indicates the user account was deleted. Do not retry.
- The server updates `session.refresh` with the new refresh token's ID on each successful call. However, the endpoint **does not validate** that the presented token's ID matches `session.refresh`, so old refresh tokens technically remain usable until their 52-day PASETO expiry. Always replace the stored refresh token with the one from the response.
- Sessions have a 100-day DynamoDB TTL, which is longer than the 52-day refresh token TTL. A session document may still exist after the refresh token has expired, but refresh attempts will return `401` (token expired at PASETO level, before any DB access).

---

### GET /user

Returns the full profile of the currently authenticated user.

**Authentication:** `Authorization: Bearer <access_token>` (Access token — KIND=1)

#### Request

```
GET /user
Authorization: Bearer v4.local.ukuGtEfqElk6...
```

No request body.

#### Response — `200 OK`

```json
{
  "id":       "507f1f77bcf86cd799439011",
  "phone":    "+254759762268",
  "name":     "Abdihakim Osman",
  "level":    "Normal",
  "status":   "Active",
  "profiled": true,
  "created":  "2025-01-15T10:30:00Z"
}
```

| Field | Type | Description |
|---|---|---|
| `id` | string | 24-char hex MongoDB ObjectId — unique, immutable user identifier. |
| `phone` | string | Current phone number in E.164 format. |
| `name` | string | Display name. |
| `level` | string | `"Normal"` \| `"System"` \| `"Super"` — privilege tier. |
| `status` | string | `"Active"` \| `"Invited"` \| `"Blocked"` \| `"Deleted"` |
| `profiled` | boolean | Whether the user has completed their extended profile. |
| `created` | string | RFC 3339 UTC timestamp. Second-level precision (sub-seconds are not stored). |

#### Errors

| Status | Message | Cause |
|---|---|---|
| `400` | `invalid token` | Access token cannot be decrypted. |
| `401` | `unauthorized` | `Authorization` header missing, wrong prefix, or token expired. |
| `403` | `permission denied` | Token is a Refresh or Setup token. |
| `404` | `user not found` | User record does not exist in DynamoDB (account deleted after token issuance). |
| `500` | `internal server error` | DynamoDB read failed or record is corrupt. |

#### Notes

- A `200` response does **not** guarantee the user is active. `status` may be `"Blocked"` or `"Deleted"`. Always inspect the `status` field in your application logic.
- Timestamps are stored as Unix epoch integers in DynamoDB (second precision). Sub-second components are permanently discarded.

---

### PATCH /user/rename

Updates the authenticated user's display name. Returns the full updated user record.

**Authentication:** `Authorization: Bearer <access_token>` (Access token — KIND=1)

#### Request

```
PATCH /user/rename
Authorization: Bearer v4.local.ukuGtEfqElk6...
Content-Type: application/json
```

```json
{
  "name": "Alagha"
}
```

| Field | Type | Required | Constraints |
|---|---|---|---|
| `name` | string | ✅ | No length or character validation at the API layer. Empty strings are accepted. |

#### Response — `200 OK`

Full updated `User` object (same shape as `GET /user`).

```json
{
  "id":       "507f1f77bcf86cd799439011",
  "phone":    "+254759762268",
  "name":     "Alagha",
  "level":    "Normal",
  "status":   "Active",
  "profiled": true,
  "created":  "2025-01-15T10:30:00Z"
}
```

The response is the **complete post-update record** from DynamoDB. There is no need to call `GET /user` afterwards.

#### Errors

| Status | Message | Cause |
|---|---|---|
| `400` | `invalid token` | Access token cannot be decrypted. |
| `401` | `unauthorized` | Header missing, wrong prefix, or token expired. |
| `403` | `permission denied` | Token is a Refresh or Setup token. |
| `404` | `user not found` | User record not found after the update (extremely rare — see notes). |
| `415` | *(Axum rejection)* | `Content-Type` is not `application/json`. |
| `422` | *(Axum rejection)* | `name` field is missing or not a JSON string. |
| `500` | `internal server error` | DynamoDB update failed or record is corrupt. |

#### Notes

- There is **no "same name" guard** — sending the current name succeeds and writes to DynamoDB every time.
- Name validation (length limits, character restrictions) must be enforced **client-side** or at the API gateway. The server accepts empty strings and arbitrarily long values.
- DynamoDB's `UpdateItem` is an **upsert**. If the user record is deleted between token issuance and this call, DynamoDB will **create a partial ghost record** with only `id` and `name`. The subsequent deserialization of this record will fail (`500`). The ghost record will remain in DynamoDB and corrupt future reads. Always verify the user exists with `GET /user` before calling rename if your application allows account deletion.

---

### POST /user/change-phone

Initiates a phone number change by sending a 6-digit OTP to the **new** phone number via WhatsApp. The change is not committed until confirmed via `PATCH /user/confirm-change-phone`.

**Authentication:** `Authorization: Bearer <access_token>` (Access token — KIND=1)

#### Request

```
POST /user/change-phone
Authorization: Bearer v4.local.ukuGtEfqElk6...
Content-Type: application/json
```

```json
{
  "phone": "+254759766444"
}
```

| Field | Type | Required | Constraints |
|---|---|---|---|
| `phone` | string | ✅ | The **new** phone number in E.164 format. |

#### Response — `200 OK`

```json
{
  "phone":   "+254759766444",
  "created": "2025-04-26T07:50:00Z",
  "ttl":     "2025-04-26T08:05:00Z"
}
```

Same shape as the `POST /auth/login` response. The OTP code is not in the response.

#### Errors

| Status | Message | Cause |
|---|---|---|
| `400` | `invalid token` | Access token cannot be decrypted. |
| `400` | `nothing new to update` | The submitted phone number is the same as the user's **current** phone number. |
| `400` | `invalid phone number...` | `phone` fails E.164 validation. |
| `401` | `unauthorized` | Header missing, wrong prefix, or token expired. |
| `403` | `permission denied` | Token is a Refresh or Setup token. |
| `404` | `user not found` | User record does not exist in DynamoDB. |
| `429` | `too many requests, Please slow down` | A verification code was sent to this **target phone** within the past 60 seconds. |
| `500` | `internal server error` | WhatsApp delivery failed or DynamoDB write failed. |

#### Notes

- The rate limit is applied to the **target phone number**, not the user ID. A different user can cause you to be rate-limited on a number you are trying to claim.
- Like `POST /auth/login`, the WhatsApp send happens **before** the DynamoDB persist. A failed persist means the user received an OTP that can never be verified.
- The new phone number is **not** reserved during the pending confirmation window. Another user could register with it.
- The `ttl` in the response is informational — the confirm endpoint **does not enforce** it at the application level.

---

### PATCH /user/confirm-change-phone

Completes a pending phone number change by validating the OTP sent to the new phone number. Commits the new phone to the user's record.

**Authentication:** `Authorization: Bearer <access_token>` (Access token — KIND=1)

#### Request

```
PATCH /user/confirm-change-phone
Authorization: Bearer v4.local.ukuGtEfqElk6...
Content-Type: application/json
```

```json
{
  "phone": "+254759766444",
  "code":  "861877"
}
```

| Field | Type | Required | Constraints |
|---|---|---|---|
| `phone` | string | ✅ | The **new** phone number (same value used in `change-phone`). E.164 format. |
| `code` | string | ✅ | The 6-digit OTP delivered to the new phone via WhatsApp. Leading zeros are significant — pass as a string (e.g. `"048392"`, not `48392`). |

#### Response — `200 OK`

Full updated `User` object (same shape as `GET /user`), with `phone` reflecting the new number.

```json
{
  "id":       "507f1f77bcf86cd799439011",
  "phone":    "+254759766444",
  "name":     "Abdihakim Osman",
  "level":    "Normal",
  "status":   "Active",
  "profiled": true,
  "created":  "2025-01-15T10:30:00Z"
}
```

#### Errors

| Status | Message | Cause |
|---|---|---|
| `400` | `invalid token` | Access token cannot be decrypted. |
| `400` | `invalid verification code` | The submitted `code` does not match the stored OTP. |
| `400` | `invalid phone number...` | `phone` fails E.164 validation. |
| `401` | `unauthorized` | Header missing, wrong prefix, or token expired. |
| `403` | `permission denied` | Token is a Refresh or Setup token. |
| `404` | `verification code not found` | No OTP record exists for this phone number — `POST /user/change-phone` was not called first, or the DynamoDB TTL has evicted the record. |
| `404` | `user not found` | User record not found during the update (account deleted between calls). |
| `500` | `internal server error` | DynamoDB read or write failed. |

#### Notes

- The user identity is taken **from the token**, not from the request body. The `phone` field in the body only selects which verification record to validate. Any user with a valid access token can confirm a phone change if they know a valid OTP for that number, regardless of who initiated the change.
- The verification record is **not deleted** after a successful confirmation. The same code can be replayed until the DynamoDB TTL evicts the record.
- There is **no phone uniqueness enforcement**. Two users can end up with the same phone number after separate change-phone flows.
- There is **no brute-force protection** on this endpoint — failed code attempts are not rate-limited or counted.
- The access token must remain valid for the entire duration of the two-step flow. If it expires between `change-phone` and `confirm-change-phone`, obtain a new one via `GET /auth/refresh` first.

---

### GET /sessions

Lists all active sessions for the currently authenticated user.

**Authentication:** `Authorization: Bearer <access_token>` (Access token — KIND=1)

#### Request

```
GET /sessions
Authorization: Bearer v4.local.ukuGtEfqElk6...
```

No request body.

#### Response — `200 OK`

A JSON array of `Session` objects. Returns `[]` (empty array) if no sessions exist — this is **not an error**.

```json
[
  {
    "id":      "69ed056664bc5d80849a9322",
    "user":    "507f1f77bcf86cd799439011",
    "refresh": "507f1f77bcf86cd799439013",
    "device":  "Ubuntu Linux, Bruno",
    "created": "2025-04-01T10:00:00Z",
    "ttl":     "2025-07-10T10:00:00Z"
  }
]
```

| Field | Description |
|---|---|
| `id` | Session primary key (24-char hex ObjectId). Use this to delete the session. |
| `user` | Owner's user ID — always matches the token subject. |
| `refresh` | ID of the most recently issued Refresh token for this session. Treat as opaque. |
| `device` | Human-readable device label supplied at login or setup. |
| `created` | When the session was originally opened. |
| `ttl` | Session expiry hint (`created + 100 days`). DynamoDB TTL cleanup is eventually consistent. |

#### Errors

| Status | Message | Cause |
|---|---|---|
| `400` | `invalid token` | Access token cannot be decrypted. |
| `401` | `unauthorized` | Header missing, wrong prefix, or token expired. |
| `403` | `permission denied` | Token is a Refresh or Setup token. |
| `500` | `internal server error` | DynamoDB query failed. |

#### Notes

- The response is a **top-level JSON array**, not a wrapped object like `{ "sessions": [...] }`.
- The session used to authenticate **this request** is included in the list. Identify the current session by comparing IDs if you need to mark it in the UI.
- Sessions whose `ttl` has passed may still appear if DynamoDB TTL has not yet evicted them. Do not treat a stale `ttl` as meaning the session is already gone.

---

### DELETE /sessions/{id}

Deletes a specific session, immediately preventing any future token refresh for that session. This is the primary "log out of a device" mechanism.

**Authentication:** `Authorization: Bearer <access_token>` (Access token — KIND=1)

#### Request

```
DELETE /sessions/69ed056664bc5d80849a9322
Authorization: Bearer v4.local.ukuGtEfqElk6...
```

No request body.

#### Response — `200 OK`

```json
null
```

`200 null` is returned whether the session was deleted **or** the session did not exist (the operation is idempotent).

#### Errors

| Status | Message | Cause |
|---|---|---|
| `400` | `invalid id` | Path segment `{id}` is not a valid 24-char hex ObjectId. |
| `400` | `invalid token` | Access token cannot be decrypted. |
| `401` | `unauthorized` | Header missing, wrong prefix, or token expired. |
| `403` | `permission denied` | Token is a Refresh or Setup token. Also returned if the session **exists** but belongs to a different user. |
| `500` | `internal server error` | DynamoDB read or write failed. |

#### Notes

- `403` (ownership mismatch) only fires when the session **exists** and belongs to a different user. A non-existent session ID always returns `200`.
- Deleting your **own active session** succeeds immediately. The current access token remains structurally valid until its 2-day PASETO expiry (it is not validated against the session on every request), but all subsequent `GET /auth/refresh` calls for that session will return `403`. Discard both tokens after deleting your own session.
- The operation requires **two DynamoDB calls** — a `GetItem` (to check ownership) followed by a `DeleteItem`. This is intentional to enforce the ownership constraint.
- The `{id}` in the URL is the **session's own `id` field**, not the `refresh` field also visible in `GET /sessions`.

---

### GET /health

Simple health check endpoint. No authentication required.

#### Request

```
GET /health
```

#### Response — `200 OK`

```json
{
  "status": "Ok"
}
```

---

## Error Reference

Complete table of all application-level errors, their HTTP status codes, and trigger conditions:

| HTTP Status | `message` | Error Variant | Trigger |
|---|---|---|---|
| `400` | `verification code not found` | `VerificationCodeNotFound` | No OTP record in DynamoDB for the given phone. |
| `400` | `invalid verification code` | `InvalidVerificationCode` | OTP code in request does not match stored code. |
| `400` | `invalid phone number. Make sure to include country code` | `InvalidPhoneNumber` | Phone fails E.164 validation (no `+` prefix, or invalid structure). |
| `400` | `invalid id` | `InvalidId` | A path/body ID is not a valid 24-char hex MongoDB ObjectId. |
| `400` | `invalid token` | `InvalidToken` | PASETO token cannot be decrypted or parsed. |
| `400` | `nothing new to update` | `UptoDate` | `change-phone` received the same phone number as the current one. |
| `401` | `unauthorized` | `Unauthorized` | Missing `Authorization` header, wrong prefix, or expired token. |
| `403` | `permission denied` | `Forbidden` | Wrong token type, blocked user, missing session (on refresh), or cross-user session delete. |
| `404` | `verification code not found` | `VerificationCodeNotFound` | No OTP record exists for the phone (login not called first, or record evicted). |
| `404` | `user not found` | `UserNotFound` | User record does not exist in DynamoDB. |
| `404` | `invalid session` | `InvalidSession` | Session ID does not resolve to a session record. |
| `409` | `record already exists` | `RecordAlreadyExists` | A conflicting record already exists in the database. |
| `429` | `too many requests, Please slow down` | `SlowDown` | OTP requested within the 60-second rate-limit window for a phone number. |
| `500` | `internal server error` | `InternalServerError` | Database error, WhatsApp API error, or data corruption. Details are logged server-side only. |

> Errors thrown at the **Axum extractor layer** (before the handler runs) use plain-text bodies, not the `{ "message" }` envelope. These include `Content-Type: application/json` enforcement (`415`), JSON parse failures (`400`), and missing required fields (`422`).

---

## Database Schema

### Table: `eduxal-users`

Primary key: `id` (String — 24-char hex ObjectId)
GSI: `phone-index` on `phone` (String)

| Attribute | DynamoDB Type | Description |
|---|---|---|
| `id` | S | Unique user ID (ObjectId hex). Partition key. |
| `phone` | S | E.164 phone number. Indexed via `phone-index` GSI. |
| `name` | S | Display name. |
| `level` | S | `"Normal"` \| `"System"` \| `"Super"` |
| `status` | S | `"Active"` \| `"Invited"` \| `"Blocked"` \| `"Deleted"` |
| `profiled` | BOOL | Whether the user has completed extended profile setup. |
| `created` | N | Unix timestamp (seconds) of account creation. |

### Table: `eduxal-sessions`

Primary key: `id` (String — 24-char hex ObjectId)
GSI: `user-index` on `user` (String)

| Attribute | DynamoDB Type | Description |
|---|---|---|
| `id` | S | Unique session ID. Partition key. |
| `user` | S | Owning user's ID. Indexed via `user-index` GSI. |
| `refresh` | S | ID of the most recent Refresh token issued for this session. |
| `device` | S | Human-readable device label. |
| `created` | N | Unix timestamp (seconds) of session creation. |
| `ttl` | N | Unix timestamp (seconds) of session expiry (created + 100 days). Drives DynamoDB TTL. |

### Table: `eduxal-verifications`

Primary key: `phone` (String — E.164)

| Attribute | DynamoDB Type | Description |
|---|---|---|
| `phone` | S | Phone number. Partition key. Only one pending OTP per phone at a time. |
| `code` | S | 6-digit OTP string (zero-padded). |
| `created` | N | Unix timestamp (seconds) of OTP generation. |
| `ttl` | N | Unix timestamp (seconds) of OTP expiry (created + 15 minutes). Drives DynamoDB TTL. |

---

## Configuration

### Compile-time Environment Variables

Read from a `.env` file via `build.rs` at compile time. **Baked into the binary** — cannot be changed without a rebuild.

| Variable | Required | Description |
|---|---|---|
| `PASETO_PASSWORD` | ✅ | Symmetric key for PASETO v4-local token encryption/decryption. Must be exactly 32 bytes. |
| `WHATSAPP_TOKEN` | ✅ | Meta/Facebook Graph API bearer token for the WhatsApp Business account. Used for all OTP deliveries. |

> ⚠️ Because these are compile-time constants (`env!` macro), **rotating either key requires a full rebuild and redeployment**. The `WHATSAPP_TOKEN` is embedded in the binary, not read from Lambda environment variables at runtime.

### Runtime Environment Variables (Lambda)

Set via the SAM `template.yaml`. Used by the AWS SDK for DynamoDB configuration.

| Variable | Description |
|---|---|
| `USERS_TABLE` | DynamoDB users table name (imported from CloudFormation). |
| `VERIFICATIONS_TABLE` | DynamoDB verifications table name. |
| `SESSIONS_TABLE` | DynamoDB sessions table name. |
| `RUST_LOG` | Tracing log level (default: `info`). |
| `AWS_REGION` | Set automatically by the Lambda runtime. |

> **Note:** The table names in the runtime environment variables (`USERS_TABLE`, etc.) are currently **not read by the application code**. The Rust source hardcodes table names as `"eduxal-users"`, `"eduxal-sessions"`, and `"eduxal-verifications"`. The environment variables exist in the template for reference but have no effect at runtime.

### WhatsApp Business Setup

The OTP is sent via the Facebook Graph API using a pre-approved template:
- **API version:** `v21.0`
- **Phone number ID:** `960426547146856`
- **Template name:** `auth_code`
- **Language:** `en`

The template must be approved by Meta before use. It uses two components:
1. A `body` component with the OTP as a text variable.
2. A `button` component (`url` sub-type, index 0) with the OTP as the URL suffix.

---

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2024)
- [cargo-lambda](https://www.cargo-lambda.info/) — for building and local testing
- [AWS SAM CLI](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/install-sam-cli.html)
- An AWS account with DynamoDB tables created

### Local Setup

1. Clone the repository.

2. Create a `.env` file in the project root:
   ```
   PASETO_PASSWORD=<32-byte-hex-or-base64-key>
   WHATSAPP_TOKEN=<your-meta-graph-api-token>
   ```

3. Build the project:
   ```bash
   cargo build
   ```

4. Run locally with cargo-lambda:
   ```bash
   cargo lambda watch
   ```

   The Lambda will be available at `http://localhost:9000`.

5. Invoke a function locally:
   ```bash
   cargo lambda invoke --data-ascii '{"httpMethod":"GET","path":"/health"}'
   ```

### Running Tests

```bash
cargo test
```

---

## Deployment

The service is deployed via **AWS SAM**.

### First-Time Deployment

```bash
sam build
sam deploy --guided
```

During guided deployment you will be prompted for:
- `AcmCertificateArn` — the ARN of an ACM certificate covering `auth.eduxal.com` (must be in `us-east-1` for API Gateway).

### Subsequent Deployments

```bash
sam build && sam deploy
```

Configuration is persisted in `samconfig.toml`.

### CI/CD

A GitHub Actions workflow (`.github/workflows/`) handles automated deployments. OIDC-based AWS authentication is configured in `github-oidc.yml` — no long-lived AWS credentials are stored in GitHub Secrets.

### DNS

After deployment, set a **Cloudflare CNAME** record for `auth.eduxal.com` pointing to the value of the `AuthApiDomainName` CloudFormation output (the API Gateway regional domain name).

---

## Known Behaviors & Security Considerations

The following are notable behaviors identified through code analysis. They are documented here for transparency and to inform future development decisions.

### OTP Replay — Verification Records Are Not Deleted

After a successful `POST /auth/verify` or `PATCH /user/confirm-change-phone`, the OTP record in `eduxal-verifications` is **not deleted**. The same code remains valid until DynamoDB's TTL mechanism evicts the record, which can take up to 48 hours after the `ttl` timestamp. Within this window, a valid code can be reused to open multiple sessions or change a phone number multiple times.

**Mitigation:** Delete the verification record immediately after successful validation.

### No Brute-Force Protection on Confirm Endpoints

`POST /auth/verify` and `PATCH /user/confirm-change-phone` have no rate limiting on failed code attempts. An attacker who knows a target phone number can enumerate all 1,000,000 possible 6-digit codes without restriction.

**Mitigation:** Add a per-phone failed-attempt counter with a lockout threshold.

### Refresh Token Rotation Is Not Strict

`GET /auth/refresh` updates `session.refresh` with the new token's ID, but it never validates that the presented token's ID matches the stored value. Old refresh tokens remain usable until their 52-day PASETO expiry, even after rotation. A stolen refresh token cannot be invalidated by the token holder simply rotating it.

**Mitigation:** Add a check that `token.id == session.refresh` and return `403` if it does not match, implementing true single-use rotation.

### Phone Uniqueness Not Enforced at the DB Level

`PATCH /user/confirm-change-phone` can assign a phone number to a user without checking whether that phone is already in use by another account. Two users can share the same phone number, causing non-deterministic results in the phone-index GSI lookups used by login flows.

**Mitigation:** Add a conditional check (or a DynamoDB condition expression) before committing the phone update.

### `POST /auth/setup` Renames Blocked Users

When `POST /auth/setup` is called for a phone number that maps to an existing blocked user, the `name` field is **updated in DynamoDB before the blocked-user check fires**. The endpoint correctly returns `403`, but the side effect (the rename) is committed.

### DynamoDB `UpdateItem` Is an Upsert

`PATCH /user/rename` uses `UpdateItem` without a condition expression. If the user record has been deleted between token issuance and the rename call, DynamoDB will create a partial record containing only `id` and `name`. Subsequent reads of this partial record will fail with `500` due to missing required fields. The ghost record persists indefinitely.

**Mitigation:** Add `ConditionExpression: attribute_exists(id)` to the `UpdateItem` call.

### `WHATSAPP_TOKEN` Is a Compile-Time Constant

The WhatsApp bearer token is baked into the binary via `env!()`. Rotating it requires a full rebuild and redeployment.

**Mitigation:** Move to a runtime secret (AWS Secrets Manager or SSM Parameter Store).

### Send-Before-Persist in OTP Flow

OTP delivery via WhatsApp happens before the verification record is written to DynamoDB (in both `POST /auth/login` and `POST /user/change-phone`). A DynamoDB write failure after WhatsApp delivery leaves the user with a code that cannot be verified. The user must wait 60 seconds and retry.

---

*Service version: 0.1.0 · Rust edition 2024 · Deployed on AWS Lambda (arm64, provided.al2023)*
```

Now let me write the cleaned-up documentation HTML: