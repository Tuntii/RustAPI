use super::types::RustApi;
use crate::error::Result;
use crate::server::Server;

impl RustApi {
    pub async fn run(mut self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.prepare_for_serve(addr).await;

        let server = Server::new(self.router, self.layers, self.interceptors);
        server.run(addr).await
    }

    /// Run the server with graceful shutdown signal
    pub async fn run_with_shutdown<F>(
        mut self,
        addr: impl AsRef<str>,
        signal: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        self.prepare_for_serve(addr.as_ref()).await;

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
        let server = Server::new(self.router, self.layers, self.interceptors);
        server.run_with_shutdown(addr.as_ref(), signal).await?;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        Ok(())
    }

    /// Run HTTP/3 with TLS certificates and a graceful shutdown signal.
    #[cfg(feature = "http3")]
    pub async fn run_http3_with_shutdown<F>(
        mut self,
        config: crate::http3::Http3Config,
        signal: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        use std::sync::Arc;

        let addr = config.socket_addr();
        self.prepare_for_serve(&addr).await;

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
        let server = crate::http3::Http3Server::new(
            &config,
            Arc::new(self.router.clone()),
            Arc::new(self.layers.clone()),
            Arc::new(self.interceptors.clone()),
        )
        .await?;

        server.run_with_shutdown(signal).await?;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        Ok(())
    }

    #[cfg(feature = "http3")]
    pub async fn run_http3(
        mut self,
        config: crate::http3::Http3Config,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::Arc;

        let addr = config.socket_addr();
        self.prepare_for_serve(&addr).await;

        let server = crate::http3::Http3Server::new(
            &config,
            Arc::new(self.router.clone()),
            Arc::new(self.layers.clone()),
            Arc::new(self.interceptors.clone()),
        )
        .await?;

        server.run().await
    }

    /// Run HTTP/3 (self-signed) with a graceful shutdown signal.
    #[cfg(feature = "http3-dev")]
    pub async fn run_http3_dev_with_shutdown<F>(
        mut self,
        addr: &str,
        signal: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        use std::sync::Arc;

        self.prepare_for_serve(addr).await;

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
        let server = crate::http3::Http3Server::new_with_self_signed(
            addr,
            Arc::new(self.router.clone()),
            Arc::new(self.layers.clone()),
            Arc::new(self.interceptors.clone()),
        )
        .await?;

        server.run_with_shutdown(signal).await?;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        Ok(())
    }

    #[cfg(feature = "http3-dev")]
    pub async fn run_http3_dev(
        mut self,
        addr: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::Arc;

        self.prepare_for_serve(addr).await;

        let server = crate::http3::Http3Server::new_with_self_signed(
            addr,
            Arc::new(self.router.clone()),
            Arc::new(self.layers.clone()),
            Arc::new(self.interceptors.clone()),
        )
        .await?;

        server.run().await
    }

    /// Configure HTTP/3 support for `run_http3` and `run_dual_stack`.
    #[cfg(feature = "http3")]
    pub fn with_http3(mut self, cert_path: impl Into<String>, key_path: impl Into<String>) -> Self {
        self.http3_config = Some(crate::http3::Http3Config::new(cert_path, key_path));
        self
    }

    /// Run HTTP/1.1 and HTTP/3 together with a graceful shutdown signal.
    #[cfg(feature = "http3")]
    pub async fn run_dual_stack_with_shutdown<F>(
        mut self,
        http_addr: &str,
        signal: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        use std::sync::Arc;

        let mut config = self
            .http3_config
            .take()
            .ok_or("HTTP/3 config not set. Use .with_http3(...)")?;

        let http_socket: std::net::SocketAddr = http_addr.parse()?;
        config.bind_addr = if http_socket.ip().is_ipv6() {
            format!("[{}]", http_socket.ip())
        } else {
            http_socket.ip().to_string()
        };
        config.port = http_socket.port();
        let http_addr = http_socket.to_string();

        self.prepare_for_serve(&http_addr).await;

        let shutdown_hooks = std::mem::take(&mut self.lifecycle_hooks.on_shutdown);
        let router = Arc::new(self.router);
        let layers = Arc::new(self.layers);
        let interceptors = Arc::new(self.interceptors);

        let http1_server =
            Server::from_shared(router.clone(), layers.clone(), interceptors.clone());
        let http3_server =
            crate::http3::Http3Server::new(&config, router, layers, interceptors).await?;

        tracing::info!(
            http1_addr = %http_addr,
            http3_addr = %config.socket_addr(),
            "Starting dual-stack HTTP/1.1 + HTTP/3 servers"
        );

        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let notify_for_signal = notify.clone();
        tokio::spawn(async move {
            signal.await;
            notify_for_signal.notify_waiters();
        });
        let wait_for_shutdown = {
            let notify = notify.clone();
            async move {
                notify.notified().await;
            }
        };
        let wait_for_shutdown_http3 = async move {
            notify.notified().await;
        };

        tokio::try_join!(
            http1_server.run_with_shutdown(&http_addr, wait_for_shutdown),
            http3_server.run_with_shutdown(wait_for_shutdown_http3),
        )?;
        Self::run_shutdown_hooks(shutdown_hooks).await;
        Ok(())
    }

    #[cfg(feature = "http3")]
    pub async fn run_dual_stack(
        mut self,
        http_addr: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::sync::Arc;

        let mut config = self
            .http3_config
            .take()
            .ok_or("HTTP/3 config not set. Use .with_http3(...)")?;

        let http_socket: std::net::SocketAddr = http_addr.parse()?;
        config.bind_addr = if http_socket.ip().is_ipv6() {
            format!("[{}]", http_socket.ip())
        } else {
            http_socket.ip().to_string()
        };
        config.port = http_socket.port();
        let http_addr = http_socket.to_string();

        self.prepare_for_serve(&http_addr).await;

        let router = Arc::new(self.router);
        let layers = Arc::new(self.layers);
        let interceptors = Arc::new(self.interceptors);

        let http1_server =
            Server::from_shared(router.clone(), layers.clone(), interceptors.clone());
        let http3_server =
            crate::http3::Http3Server::new(&config, router, layers, interceptors).await?;

        tracing::info!(
            http1_addr = %http_addr,
            http3_addr = %config.socket_addr(),
            "Starting dual-stack HTTP/1.1 + HTTP/3 servers"
        );

        tokio::try_join!(http1_server.run(&http_addr), http3_server.run(),)?;
        Ok(())
    }
}
