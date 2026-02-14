use crate::exec_domain::ExecDomain;
use crate::ssh::{SshBackend, SshDomain};
use crate::tls::{TlsDomainClient, TlsDomainServer};
use crate::unix::UnixDomain;
use phaedra_dynamic::{FromDynamic, ToDynamic};

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct DomainConfig {
    #[dynamic(default)]
    pub exec_domains: Vec<ExecDomain>,
    #[dynamic(default = "UnixDomain::default_unix_domains")]
    pub unix_domains: Vec<UnixDomain>,
    #[dynamic(default)]
    pub ssh_domains: Option<Vec<SshDomain>>,
    #[dynamic(default)]
    pub ssh_backend: SshBackend,
    #[dynamic(default)]
    pub tls_servers: Vec<TlsDomainServer>,
    #[dynamic(default)]
    pub tls_clients: Vec<TlsDomainClient>,
    #[dynamic(default = "default_true")]
    pub mux_enable_ssh_agent: bool,
    #[dynamic(default)]
    pub default_ssh_auth_sock: Option<String>,
    #[dynamic(default = "default_mux_env_remove")]
    pub mux_env_remove: Vec<String>,
    #[dynamic(default)]
    pub default_domain: Option<String>,
    #[dynamic(default)]
    pub default_mux_server_domain: Option<String>,
}

impl Default for DomainConfig {
    fn default() -> Self {
        Self {
            exec_domains: Vec::new(),
            unix_domains: UnixDomain::default_unix_domains(),
            ssh_domains: None,
            ssh_backend: SshBackend::default(),
            tls_servers: Vec::new(),
            tls_clients: Vec::new(),
            mux_enable_ssh_agent: default_true(),
            default_ssh_auth_sock: None,
            mux_env_remove: default_mux_env_remove(),
            default_domain: None,
            default_mux_server_domain: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_mux_env_remove() -> Vec<String> {
    vec![
        "SSH_AUTH_SOCK".to_string(),
        "SSH_CLIENT".to_string(),
        "SSH_CONNECTION".to_string(),
    ]
}
