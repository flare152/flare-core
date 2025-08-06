//! TLS 证书统一处理模块

use anyhow::{Context, Result};
use quinn::{ClientConfig, ServerConfig, TransportConfig, VarInt};
use rustls::{RootCertStore};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::ServerConfig as RustlsServerConfig;
use std::sync::Arc;
use std::time::Duration;
use std::{fs::File, io::BufReader, path::Path};
use tracing::{debug, info};
use rustls_pemfile;

/// 默认路径常量
const DEFAULT_CERT_PATH: &str = "certs/server.crt";
const DEFAULT_KEY_PATH: &str = "certs/server.key";
const DEFAULT_CLIENT_CERT: &str = "certs/client.crt";

/// 初始化加密提供程序
pub fn init_crypto() -> Result<()> {
    if rustls::crypto::ring::default_provider().install_default().is_err() {
        debug!("Crypto provider already installed");
    }
    Ok(())
}

/// ALPN 协议标识
pub const ALPN_QUIC_HTTP: &[&str] = &["hq-29", "flare-core"];

/// 加载证书列表
fn load_certs<P: AsRef<Path>>(path: P) -> Result<Vec<CertificateDer<'static>>> {
    let certfile = File::open(&path)
        .with_context(|| format!("cannot open certificate file: {:?}", path.as_ref()))?;
    let mut reader = BufReader::new(certfile);
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to parse certificates")?;

    Ok(certs.into_iter().map(|c| CertificateDer::from(c)).collect())
}

/// 加载私钥（PKCS8优先）
fn load_key<P: AsRef<Path>>(path: P) -> Result<PrivateKeyDer<'static>> {
    let path = path.as_ref();

    // 尝试 PKCS8
    let keyfile = File::open(path).context("cannot open key file")?;
    let mut reader = BufReader::new(keyfile);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to parse PKCS8 private keys")?;

    if let Some(pk) = keys.into_iter().next() {
        info!("成功加载 PKCS8 私钥");
        return Ok(PrivateKeyDer::Pkcs8(pk.into()));
    }

    // 尝试 RSA
    let keyfile = File::open(path).context("cannot open key file for RSA fallback")?;
    let mut reader = BufReader::new(keyfile);
    let keys = rustls_pemfile::rsa_private_keys(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to parse RSA private keys")?;

    if let Some(pk) = keys.into_iter().next() {
        info!("成功加载 RSA 私钥");
        return Ok(PrivateKeyDer::Pkcs1(pk.into()));
    }

    Err(anyhow::anyhow!("No valid private keys found in {:?}", path))
}

/// 创建传输层配置
pub fn create_transport_config() -> TransportConfig {
    let mut transport = TransportConfig::default();
    transport
        .keep_alive_interval(Some(Duration::from_secs(10)))
        .max_idle_timeout(Some(Duration::from_secs(60).try_into().unwrap()))
        .max_concurrent_bidi_streams(VarInt::from_u32(100))
        .max_concurrent_uni_streams(VarInt::from_u32(32))
        .initial_mtu(1200)
        .min_mtu(1200)
        .stream_receive_window(VarInt::from_u32(1_000_000))
        .receive_window(VarInt::from_u32(10_000_000));

    transport
}

/// 创建客户端配置
pub fn create_client_config<P: AsRef<Path>>(cert_path: P) -> Result<ClientConfig> {
    init_crypto()?;

    let certs = load_certs(cert_path)?;
    let mut roots = RootCertStore::empty();

    for cert in certs {
        roots.add(cert).context("failed to add certificate to root store")?;
    }

    let rustls_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let crypto = quinn::crypto::rustls::QuicClientConfig::try_from(rustls_config)?;
    Ok(ClientConfig::new(Arc::new(crypto)))
}

/// 创建服务端配置
pub fn create_server_config<C: AsRef<Path>, K: AsRef<Path>>(cert_path: C, key_path: K) -> Result<ServerConfig> {
    init_crypto()?;

    info!("加载服务端证书: {:?}", cert_path.as_ref());
    info!("加载服务端私钥: {:?}", key_path.as_ref());

    let certs = load_certs(cert_path)?;
    let key = load_key(key_path)?;

    let tls_config = RustlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    let crypto = quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)?;
    Ok(ServerConfig::with_crypto(Arc::new(crypto)))
}

/// 简化客户端配置（开发用）
pub fn create_simple_client_config(cert_path: Option<&str>) -> Result<ClientConfig> {
    let path = cert_path.unwrap_or(DEFAULT_CLIENT_CERT);
    create_client_config(path)
}

/// 简化服务端配置（开发用）
pub fn create_simple_server_config() -> Result<ServerConfig> {
    create_server_config(DEFAULT_CERT_PATH, DEFAULT_KEY_PATH)
}
