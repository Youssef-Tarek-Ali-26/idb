use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{CoreError, CoreResult, TenantId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrincipalKind {
    Anonymous,
    User(String),
    Service(String),
    System(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallerContext {
    pub principal: PrincipalKind,
    pub tenant_scope: Option<TenantId>,
    pub roles: BTreeSet<String>,
}

impl CallerContext {
    pub fn anonymous() -> Self {
        Self {
            principal: PrincipalKind::Anonymous,
            tenant_scope: None,
            roles: BTreeSet::new(),
        }
    }

    pub fn service(name: impl Into<String>, tenant_scope: Option<TenantId>) -> Self {
        Self {
            principal: PrincipalKind::Service(name.into()),
            tenant_scope,
            roles: BTreeSet::new(),
        }
    }

    pub fn system_for_tenant(tenant_id: TenantId) -> Self {
        Self {
            principal: PrincipalKind::System("internal".to_string()),
            tenant_scope: Some(tenant_id),
            roles: BTreeSet::from(["system".to_string()]),
        }
    }

    pub fn internal_unscoped() -> Self {
        Self {
            principal: PrincipalKind::System("internal".to_string()),
            tenant_scope: None,
            roles: BTreeSet::from(["system".to_string()]),
        }
    }

    pub fn can_access_tenant(&self, tenant_id: &TenantId) -> bool {
        self.tenant_scope
            .as_ref()
            .is_none_or(|scope| scope == tenant_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthAction {
    Query,
    Explain,
    Watch,
    Ingest,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    pub caller: CallerContext,
    pub action: AuthAction,
    pub tenant_id: TenantId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorizationDecision {
    Allow,
    Deny { reason: String },
}

pub trait AuthorizationProvider: Send + Sync {
    fn decide(&self, request: &AuthorizationRequest) -> AuthorizationDecision;
}

#[derive(Debug, Default)]
pub struct AllowAllAuthorizationProvider;

impl AuthorizationProvider for AllowAllAuthorizationProvider {
    fn decide(&self, _request: &AuthorizationRequest) -> AuthorizationDecision {
        AuthorizationDecision::Allow
    }
}

#[derive(Clone)]
pub struct AuthRuntime {
    provider: Arc<dyn AuthorizationProvider>,
}

impl fmt::Debug for AuthRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthRuntime").finish_non_exhaustive()
    }
}

impl Default for AuthRuntime {
    fn default() -> Self {
        Self::disabled()
    }
}

impl AuthRuntime {
    pub fn disabled() -> Self {
        Self {
            provider: Arc::new(AllowAllAuthorizationProvider),
        }
    }

    pub fn with_provider<P>(provider: P) -> Self
    where
        P: AuthorizationProvider + 'static,
    {
        Self {
            provider: Arc::new(provider),
        }
    }

    pub fn with_provider_arc(provider: Arc<dyn AuthorizationProvider>) -> Self {
        Self { provider }
    }

    pub fn authorize(
        &self,
        caller: &CallerContext,
        action: AuthAction,
        tenant_id: &TenantId,
    ) -> CoreResult<()> {
        if !caller.can_access_tenant(tenant_id) {
            return Err(CoreError::AuthorizationDenied(format!(
                "caller tenant scope does not include tenant {}",
                tenant_id.0
            )));
        }

        let request = AuthorizationRequest {
            caller: caller.clone(),
            action,
            tenant_id: tenant_id.clone(),
        };

        match self.provider.decide(&request) {
            AuthorizationDecision::Allow => Ok(()),
            AuthorizationDecision::Deny { reason } => Err(CoreError::AuthorizationDenied(reason)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{AuthAction, AuthRuntime, CallerContext, TenantId};

    #[test]
    fn disabled_runtime_allows_basic_requests() {
        let runtime = AuthRuntime::disabled();
        let caller = CallerContext::service("api", Some(TenantId("tenant_a".to_string())));
        runtime
            .authorize(
                &caller,
                AuthAction::Query,
                &TenantId("tenant_a".to_string()),
            )
            .expect("authorization");
    }

    #[test]
    fn tenant_scope_blocks_cross_tenant_access() {
        let runtime = AuthRuntime::disabled();
        let caller = CallerContext::service("api", Some(TenantId("tenant_a".to_string())));
        let err = runtime
            .authorize(
                &caller,
                AuthAction::Query,
                &TenantId("tenant_b".to_string()),
            )
            .expect_err("cross-tenant should fail");
        assert!(err.to_string().contains("tenant scope"));
    }
}
