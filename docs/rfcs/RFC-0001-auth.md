# RFC-0001: Authentication & Authorization

**Status**: Draft
**Author**: Youssef Tarek
**Created**: 2026-02-25

## Summary

iDB ships a unified auth layer that handles authentication and authorization
natively, while remaining fully interoperable with external auth providers via
JWT. The system can act as both a **token issuer** (identity provider) and a
**token consumer**, allowing it to slot into any stack.

## Motivation

Every application rebuilds the same auth plumbing: user tables, session
management, role checks, permission guards scattered across API routes. This
is tedious, error-prone, and creates a gap between "who the user is" and
"what they can access" that lives in application code rather than the data
layer.

iDB's entity-graph model makes this solvable at the database level:

- **Users are entities.** They live in the graph alongside everything else.
- **Permissions are graph paths.** "Can user X see product Y?" is a traversal,
  not custom code.
- **The database enforces access on every query.** Developers can't forget a
  WHERE clause.

## Design Principles

1. **Native-first, external-friendly.** Built-in auth works out of the box.
   External JWT works with zero friction. Mix and match freely.
2. **Auth is data.** User, Session, Account are regular entities that
   participate in the graph, support reactive subscriptions, and follow the
   same schema rules as everything else.
3. **Permissions are iQL.** Access policies are expressed as graph traversals
   and predicates, not a separate DSL.
4. **Zero auth code in apps.** Connection carries identity. Every query is
   automatically scoped. Developers define policies once in the schema.
5. **iDB-issued JWTs work everywhere.** The tokens iDB mints are standard
   JWTs consumable by any external service (REST APIs, file storage, legacy
   Postgres, etc).

## Architecture

### Three Modes

```
Mode A: Full Native
  iDB handles authn + authz + token issuance
  Best for: greenfield projects, "one system" simplicity

Mode B: External Authn, Native Authz
  JWT from Clerk/Auth0/Supabase/Better Auth → iDB enforces policies
  Best for: existing auth stack, adding iDB to current infra

Mode C: External Authn + External Authz
  JWT in → delegate policy checks to OPA/Cerbos/webhook
  Best for: enterprise with centralized policy engines
```

All three modes can coexist per-tenant or per-entity type.

### 1. Native Authentication

#### Built-in Auth Entities

```python
# These ship with iDB, opt-in via `auth { native: true }`

define entity User @auth {
  email: string @unique
  password_hash: string @hidden    # never returned in queries
  name: string
  avatar: string?
  email_verified: bool
  created_at: timestamp
  updated_at: timestamp

  sessions: -> Session[]
  accounts: -> Account[]           # OAuth linked accounts
  memberships: -> OrgMember[]

  # User data is always scoped to itself unless admin
  @allow(read, where: caller.id == id or caller.role == "admin")
  @allow(write, where: caller.id == id)
}

define entity Session @auth {
  token: string @unique @hidden
  refresh_token: string @unique @hidden
  expires_at: timestamp
  ip_address: string?
  user_agent: string?
  belongs_to: -> User

  @allow(read, where: caller.id == belongs_to.id)
  @allow(delete, where: caller.id == belongs_to.id)
}

define entity Account @auth {
  provider: enum(google, github, apple, microsoft, custom)
  provider_account_id: string
  access_token: string @hidden
  refresh_token: string? @hidden
  belongs_to: -> User

  @allow(read, where: caller.id == belongs_to.id)
}

define entity Org @auth {
  name: string
  slug: string @unique
  members: -> OrgMember[]
}

define entity OrgMember @auth {
  role: enum(owner, admin, member, viewer)
  user: -> User
  org: -> Org
}
```

#### Auth Endpoints

When native auth is enabled, iDB exposes standard REST endpoints:

```
POST /auth/register          # email + password
POST /auth/login             # email + password → JWT + refresh token
POST /auth/refresh           # refresh token → new JWT
POST /auth/logout            # invalidate session
POST /auth/verify-email      # email verification flow
POST /auth/forgot-password   # password reset flow
POST /auth/oauth/:provider   # OAuth redirect
GET  /auth/oauth/:provider/callback

GET  /.well-known/jwks.json  # public keys for external verification
```

#### Password Handling

- Argon2id for hashing (memory-hard, side-channel resistant)
- Password field is `@hidden` — never included in query results, never
  leaves the auth module
- Rate limiting on login attempts (configurable per-tenant)

### 2. JWT Issuance

When iDB authenticates a user (native or OAuth), it issues a standard JWT:

```python
auth {
  jwt {
    issuer: "https://your-app.com"
    signing: env("JWT_PRIVATE_KEY")    # RSA or EdDSA
    jwks: true                          # expose /.well-known/jwks.json
    expiry: 15m
    refresh_expiry: 7d

    # Claims are graph queries — resolved at token issuance
    claims {
      user_id: caller.id
      email: caller.email
      org_id: caller -> OrgMember.org_id
      role: caller -> OrgMember.role
      brand_ids: caller -> OrgMember -> Org -> Brand.id
    }
  }
}
```

**Claims are graph traversals.** When iDB mints a token, it walks the
relationship graph to populate claims. This means:

- Your API server reads `token.org_id` without querying anything
- Your file service reads `token.role` without querying anything
- When a user's role changes in iDB, the next token refresh picks it up

#### Token Format

Standard JWT, nothing proprietary:

```json
{
  "iss": "https://your-app.com",
  "sub": "user_8x7k2m",
  "iat": 1740000000,
  "exp": 1740000900,
  "org_id": "org_norn",
  "role": "admin",
  "brand_ids": ["brand_gold", "brand_silver"]
}
```

Any service that can verify a JWT can consume this. Postgres via
`pgjwt`, your Express API, a Go microservice, a Lambda — doesn't matter.

### 3. JWT Consumption (External Auth)

For teams with existing auth, iDB accepts external JWTs:

```python
auth {
  external {
    # Verify tokens from your existing provider
    jwks_url: "https://clerk.your-app.com/.well-known/jwks.json"
    # or
    secret: env("SUPABASE_JWT_SECRET")

    # Map their claims to iDB's caller context
    claims {
      user_id: token.sub
      org_id: token.org_id
      role: token.user_metadata.role
    }
  }
}
```

The `caller` object in policies is populated from these mapped claims.
iDB doesn't care where the token came from — Clerk, Auth0, Supabase,
Better Auth, a custom Go service. Valid signature + mapped claims = works.

### 4. Authorization Policies

#### Schema-Level Policies (Native Authz)

Policies are defined on entities using `@allow` and `@deny` decorators:

```python
define entity Product {
  name: string
  price: f32
  cost: f32               # wholesale cost, sensitive
  stock: u16
  category: string
  belongs_to: -> Brand

  # Anyone authenticated can read products
  @allow(read)

  # But cost is only visible to admins and brand owners
  @deny(read, field: cost, where: caller.role not in ["admin", "owner"])

  # Only users in the same org can write
  @allow(write, where: caller -> OrgMember -> Org -> Brand -> Product == this)

  # Delete is admin-only
  @allow(delete, where: caller.role == "admin")
}
```

#### Graph-Path Policies

The most powerful pattern — permissions follow relationships:

```python
# "Users can read products belonging to brands in their org"
@allow(read, where: caller -> OrgMember -> Org -> Brand -> Product == this)

# "Users can see orders they placed"
@allow(read, where: caller -> Order == this)

# "Suppliers can see products they supply"
@allow(read, where: caller -> Supplier -> Product == this)
```

The engine compiles these into query filters. When a user runs
`Product where price < 3000`, the engine internally intersects it with
their permission path. No application code involved.

#### Policy Evaluation Order

```
1. @deny rules evaluated first (deny wins over allow)
2. @allow rules evaluated (any match = permitted)
3. No match = denied (default-deny)
```

#### External Authz (Delegation)

For teams with centralized policy engines:

```python
authz {
  # Option 1: OPA
  provider: opa {
    url: "https://opa.internal/v1/data/idb/allow"
    timeout: 50ms
  }

  # Option 2: Cerbos
  provider: cerbos {
    url: "https://cerbos.internal:3593"
  }

  # Option 3: Webhook
  provider: webhook {
    url: "https://your-api.com/authorize"
    method: POST
    headers: { "X-Service": "idb" }
  }

  # Cache decisions to avoid per-query latency
  cache: 30s
  cache_key: [caller.id, entity_type, action]
}
```

The request payload sent to external providers:

```json
{
  "caller": { "id": "user_8x7k2m", "role": "member", "org_id": "org_norn" },
  "action": "read",
  "entity_type": "Product",
  "entity_id": "prod_123",
  "fields": ["name", "price", "stock"]
}
```

### 5. Multi-Tenancy

Tenancy is built on top of auth, not separate from it:

```python
define entity Product {
  # ...
  scoped by tenant    # shorthand for: @allow(all, where: caller.org_id == org_id)
}
```

`scoped by tenant` compiles to a mandatory filter on every query. A user
in org A literally cannot see org B's data — the filter is applied at the
engine level before execution, including on the Cerebras path where it
becomes part of the PE-level predicate.

### 6. Reactive Auth

Since auth entities are regular iDB entities, they support reactive queries:

```python
# Live session monitoring
watch Session where belongs_to == User("admin_user")
# → get notified when admin logs in/out

# Live permission changes
watch OrgMember where org == Org("norn")
# → get notified when someone joins/leaves/changes role

# Live low-stock alerts scoped to your permissions
watch Product where stock < 10
# → only returns products you're authorized to see
```

### 7. OAuth Provider Support

Native auth supports standard OAuth2/OIDC providers:

```python
auth {
  native: true

  providers {
    google {
      client_id: env("GOOGLE_CLIENT_ID")
      client_secret: env("GOOGLE_CLIENT_SECRET")
      scopes: ["email", "profile"]
    }

    github {
      client_id: env("GITHUB_CLIENT_ID")
      client_secret: env("GITHUB_CLIENT_SECRET")
      scopes: ["user:email"]
    }

    # Custom OIDC provider
    custom_sso {
      type: oidc
      issuer: "https://sso.corp.com"
      client_id: env("SSO_CLIENT_ID")
      client_secret: env("SSO_CLIENT_SECRET")
    }
  }
}
```

OAuth accounts are linked to User entities via the Account relationship.
A user can have multiple linked providers.

## Interop Matrix

| Your Stack | iDB Mode | How It Works |
|---|---|---|
| Greenfield, no auth yet | Native authn + authz | iDB does everything, issues JWTs for your API too |
| Clerk/Auth0 + REST API | External JWT + native authz | Bring your JWT, iDB enforces policies |
| Supabase + Postgres | External JWT + native authz | Same JWT works for both Postgres RLS and iDB policies |
| Enterprise + OPA | External JWT + external authz | iDB delegates to your policy engine |
| Migrating from Postgres | External JWT now, native later | Start with existing auth, migrate incrementally |
| Microservices | Native authn, iDB issues JWTs | All services verify against iDB's JWKS endpoint |

## Performance Considerations

### Policy Evaluation

- Schema-level `@allow/@deny` compiles to query predicates at parse time —
  zero runtime overhead beyond the predicate evaluation itself
- Graph-path policies compile to join filters, evaluated during the normal
  query execution path
- On the Cerebras path, tenant scoping adds ~2 cycles per record per PE
  (included in the predicate evaluation)

### Token Verification

- JWKS keys are cached in-memory, refreshed on rotation
- JWT signature verification: ~50us (EdDSA) or ~200us (RS256)
- External authz webhook adds network latency — mitigated by caching

### Caching

- External authz decisions cached by (caller, entity_type, action) tuple
- Cache TTL configurable, default 30s
- Cache invalidated on permission-related entity changes (OrgMember
  create/update/delete triggers cache bust)

## Migration Path

### From Postgres + Supabase Auth

```python
# 1. Point iDB at your existing Supabase JWT
auth {
  external {
    secret: env("SUPABASE_JWT_SECRET")
    claims { user_id: token.sub, role: token.role }
  }
}

# 2. Later, enable native auth alongside
auth {
  native: true           # iDB now manages users too
  external { ... }       # still accepts Supabase JWTs during migration
}

# 3. Eventually, drop external
auth {
  native: true
}
```

### From Better Auth

Better Auth already uses a database as storage. The migration is:
1. Import User/Session/Account tables as iDB entities
2. Enable native auth
3. Point your app's auth client at iDB's endpoints instead

## Open Questions

1. **Token revocation**: JWTs are stateless. Do we need a revocation list
   for immediate logout, or is short expiry + refresh token rotation enough?
2. **API keys**: Should iDB support long-lived API keys for service-to-service
   auth, or only JWTs?
3. **Rate limiting**: Per-user or per-tenant rate limits on auth endpoints?
   Configurable or opinionated defaults?
4. **Passkeys/WebAuthn**: Support as a first-class auth method alongside
   password and OAuth?
5. **Field-level encryption**: Should `@hidden` fields be encrypted at rest,
   or is access control sufficient?

## References

- [Better Auth](https://www.better-auth.com/) — Auth framework that uses
  the app's database as storage
- [Supabase Auth + RLS](https://supabase.com/docs/guides/auth) — JWT-based
  auth with Postgres row-level security
- [ZenStack](https://zenstack.dev/) — Access policies on Prisma schemas
- [Cerbos](https://www.cerbos.dev/) — External authorization engine
- [OPA](https://www.openpolicyagent.org/) — Open Policy Agent
