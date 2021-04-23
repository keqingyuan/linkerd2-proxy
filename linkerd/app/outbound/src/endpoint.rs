use crate::{
    http::SkipHttpDetection,
    logical::{Concrete, Logical, LogicalAddr},
    tcp::opaque_transport,
    Accept, Outbound,
};
use linkerd_app_core::{
    metrics, profiles,
    proxy::{api_resolve::Metadata, http, resolve::map_endpoint::MapEndpoint},
    svc::{self, Param},
    tls,
    transport::{self, Remote, ServerAddr},
    transport_header, Addr, Conditional, Error,
};
use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct Endpoint<P> {
    pub addr: Remote<ServerAddr>,
    pub tls: tls::ConditionalClientTls,
    pub metadata: Metadata,
    pub logical_addr: Addr,
    pub protocol: P,
}

/// An endpoint from a profile endpoint override.
///
/// This has to be a type, rather than a tuple, so that we can implement
/// `Param<SkipHttpDetection>` for it.
#[derive(Clone, Debug)]
pub struct ProfileOverride {
    endpoint: Endpoint<()>,
    opaque_protocol: bool,
}
#[derive(Copy, Clone)]
pub struct FromMetadata {
    pub identity_disabled: bool,
}

pub type OrOverride<S, E> = svc::stack::ResultService<svc::Either<S, E>>;

impl<E> Outbound<E> {
    pub fn push_into_endpoint<P, T>(
        self,
    ) -> Outbound<impl svc::NewService<T, Service = E::Service> + Clone>
    where
        Endpoint<P>: From<(tls::NoClientTls, T)>,
        E: svc::NewService<Endpoint<P>> + Clone,
    {
        let Self {
            config,
            runtime,
            stack: endpoint,
        } = self;
        let identity_disabled = runtime.identity.is_none();
        let no_tls_reason = if identity_disabled {
            tls::NoClientTls::Disabled
        } else {
            tls::NoClientTls::NotProvidedByServiceDiscovery
        };
        let stack =
            svc::stack(endpoint).push_map_target(move |t| Endpoint::<P>::from((no_tls_reason, t)));
        Outbound {
            config,
            runtime,
            stack,
        }
    }

    /// Pushes a layer that checks if a discovered service profile contains an
    /// endpoint override, and forwards directly to that endpoint (bypassing the
    /// current stack) if one exists.
    pub fn push_endpoint_override<T, O, R>(
        self,
        ep_override: O,
    ) -> Outbound<
        impl svc::NewService<
                (Option<profiles::Receiver>, T),
                Service = OrOverride<E::Service, O::Service>,
            > + Clone,
    >
    where
        E: svc::NewService<(Option<profiles::Receiver>, T)> + Clone,
        E::Service: svc::Service<R, Error = Error>,
        <E::Service as svc::Service<R>>::Future: Send,
        O: svc::NewService<ProfileOverride> + Clone,
        O::Service:
            svc::Service<R, Response = <E::Service as svc::Service<R>>::Response, Error = Error>,
        <O::Service as svc::Service<R>>::Future: Send,
        T: std::fmt::Debug,
    {
        let Self {
            config,
            runtime,
            stack: no_override,
        } = self;
        let identity_disabled = runtime.identity.is_none();
        let stack = no_override.push_switch(
            move |(profile, target): (Option<profiles::Receiver>, T)| -> Result<_, Error> {
                let rx = match profile {
                    Some(profile) => profile,
                    None => return Ok(svc::Either::A((None, target))),
                };
                {
                let profile = rx.borrow();
                if let Some((addr, metadata)) = profile.endpoint.clone() {
                    tracing::debug!(%addr, ?metadata, ?target, "Using endpoint from profile override");
                    let tls = if identity_disabled {
                        tls::ConditionalClientTls::None(tls::NoClientTls::Disabled)
                    } else {
                        FromMetadata::client_tls(&metadata, tls::NoClientTls::NotProvidedByServiceDiscovery)
                    };
                    let logical_addr = profile.addr.clone().map(|LogicalAddr(addr)| Addr::from(addr)).unwrap_or_else(|| Addr::from(addr));
                    let endpoint = Endpoint {
                        addr: Remote(ServerAddr(addr)),
                        tls,
                        metadata,
                        logical_addr,
                        protocol: (),
                    };
                    let opaque_protocol = profile.opaque_protocol;
                    return Ok(svc::Either::B(ProfileOverride { endpoint, opaque_protocol }));
                }
            };
                Ok(svc::Either::A((Some(rx), target)))
            },
            ep_override,
        );
        Outbound {
            config,
            runtime,
            stack,
        }
    }
}

// === impl Endpoint ===

impl<P> Endpoint<P> {
    pub fn no_tls(reason: tls::NoClientTls) -> impl Fn(Accept<P>) -> Self {
        move |accept| Self::from((reason, accept))
    }
}

impl<P> From<(tls::NoClientTls, Logical<P>)> for Endpoint<P> {
    fn from((reason, logical): (tls::NoClientTls, Logical<P>)) -> Self {
        match logical.profile.borrow().endpoint.clone() {
            None => Self {
                addr: Remote(ServerAddr(logical.orig_dst.into())),
                metadata: Metadata::default(),
                tls: Conditional::None(reason),
                logical_addr: logical.addr(),
                protocol: logical.protocol,
            },
            Some((addr, metadata)) => Self {
                addr: Remote(ServerAddr(addr)),
                tls: FromMetadata::client_tls(&metadata, reason),
                metadata,
                logical_addr: logical.addr(),
                protocol: logical.protocol,
            },
        }
    }
}

impl<P> From<(tls::NoClientTls, Accept<P>)> for Endpoint<P> {
    fn from((reason, accept): (tls::NoClientTls, Accept<P>)) -> Self {
        Self {
            addr: Remote(ServerAddr(accept.orig_dst.into())),
            metadata: Metadata::default(),
            tls: Conditional::None(reason),
            logical_addr: accept.orig_dst.0.into(),
            protocol: accept.protocol,
        }
    }
}

impl<P> Param<Remote<ServerAddr>> for Endpoint<P> {
    fn param(&self) -> Remote<ServerAddr> {
        self.addr
    }
}

impl<P> Param<tls::ConditionalClientTls> for Endpoint<P> {
    fn param(&self) -> tls::ConditionalClientTls {
        self.tls.clone()
    }
}

impl<P> Param<Option<opaque_transport::PortOverride>> for Endpoint<P> {
    fn param(&self) -> Option<opaque_transport::PortOverride> {
        self.metadata
            .opaque_transport_port()
            .map(opaque_transport::PortOverride)
    }
}

impl<P> Param<Option<http::AuthorityOverride>> for Endpoint<P> {
    fn param(&self) -> Option<http::AuthorityOverride> {
        self.metadata
            .authority_override()
            .cloned()
            .map(http::AuthorityOverride)
    }
}

impl<P> Param<transport::labels::Key> for Endpoint<P> {
    fn param(&self) -> transport::labels::Key {
        transport::labels::Key::OutboundConnect(self.param())
    }
}

impl<P> Param<metrics::OutboundEndpointLabels> for Endpoint<P> {
    fn param(&self) -> metrics::OutboundEndpointLabels {
        metrics::OutboundEndpointLabels {
            authority: Some(self.logical_addr.to_http_authority()),
            labels: metrics::prefix_labels("dst", self.metadata.labels().iter()),
            server_id: self.tls.clone(),
            target_addr: self.addr.into(),
        }
    }
}

impl<P> Param<metrics::EndpointLabels> for Endpoint<P> {
    fn param(&self) -> metrics::EndpointLabels {
        Param::<metrics::OutboundEndpointLabels>::param(self).into()
    }
}

impl<P: std::hash::Hash> std::hash::Hash for Endpoint<P> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.addr.hash(state);
        self.tls.hash(state);
        self.logical_addr.hash(state);
        self.protocol.hash(state);
    }
}

// === EndpointFromMetadata ===

impl FromMetadata {
    fn client_tls(metadata: &Metadata, reason: tls::NoClientTls) -> tls::ConditionalClientTls {
        // If we're transporting an opaque protocol OR we're communicating with
        // a gateway, then set an ALPN value indicating support for a transport
        // header.
        let use_transport_header =
            metadata.opaque_transport_port().is_some() || metadata.authority_override().is_some();

        metadata
            .identity()
            .cloned()
            .map(move |server_id| {
                Conditional::Some(tls::ClientTls {
                    server_id,
                    alpn: if use_transport_header {
                        Some(tls::client::AlpnProtocols(vec![
                            transport_header::PROTOCOL.into()
                        ]))
                    } else {
                        None
                    },
                })
            })
            .unwrap_or(Conditional::None(reason))
    }
}

impl<P: Copy + std::fmt::Debug> MapEndpoint<Concrete<P>, Metadata> for FromMetadata {
    type Out = Endpoint<P>;

    fn map_endpoint(
        &self,
        concrete: &Concrete<P>,
        addr: SocketAddr,
        metadata: Metadata,
    ) -> Self::Out {
        tracing::trace!(%addr, ?metadata, ?concrete, "Resolved endpoint");
        let tls = if self.identity_disabled {
            tls::ConditionalClientTls::None(tls::NoClientTls::Disabled)
        } else {
            Self::client_tls(&metadata, tls::NoClientTls::NotProvidedByServiceDiscovery)
        };
        Endpoint {
            addr: Remote(ServerAddr(addr)),
            tls,
            metadata,
            logical_addr: concrete.logical.addr(),
            protocol: concrete.logical.protocol,
        }
    }
}

// === ProfileOverride ===

// Used for skipping HTTP detection for endpoint overrides
impl svc::Param<SkipHttpDetection> for ProfileOverride {
    fn param(&self) -> SkipHttpDetection {
        SkipHttpDetection(self.opaque_protocol)
    }
}

impl From<ProfileOverride> for Endpoint<()> {
    fn from(ProfileOverride { endpoint, .. }: ProfileOverride) -> Self {
        endpoint
    }
}
