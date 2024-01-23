/// The X509SVIDRequest message conveys parameters for requesting an X.509-SVID.
/// There are currently no request parameters.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct X509svidRequest {}
/// The X509SVIDResponse message carries X.509-SVIDs and related information,
/// including a set of global CRLs and a list of bundles the workload may use
/// for federating with foreign trust domains.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct X509svidResponse {
    /// Required. A list of X509SVID messages, each of which includes a single
    /// X.509-SVID, its private key, and the bundle for the trust domain.
    #[prost(message, repeated, tag = "1")]
    pub svids: ::prost::alloc::vec::Vec<X509svid>,
    /// Optional. ASN.1 DER encoded certificate revocation lists.
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub crl: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// Optional. CA certificate bundles belonging to foreign trust domains that
    /// the workload should trust, keyed by the SPIFFE ID of the foreign trust
    /// domain. Bundles are ASN.1 DER encoded.
    #[prost(map = "string, bytes", tag = "3")]
    pub federated_bundles: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::vec::Vec<u8>,
    >,
}
/// The X509SVID message carries a single SVID and all associated information,
/// including the X.509 bundle for the trust domain.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct X509svid {
    /// Required. The SPIFFE ID of the SVID in this entry
    #[prost(string, tag = "1")]
    pub spiffe_id: ::prost::alloc::string::String,
    /// Required. ASN.1 DER encoded certificate chain. MAY include
    /// intermediates, the leaf certificate (or SVID itself) MUST come first.
    #[prost(bytes = "vec", tag = "2")]
    pub x509_svid: ::prost::alloc::vec::Vec<u8>,
    /// Required. ASN.1 DER encoded PKCS#8 private key. MUST be unencrypted.
    #[prost(bytes = "vec", tag = "3")]
    pub x509_svid_key: ::prost::alloc::vec::Vec<u8>,
    /// Required. ASN.1 DER encoded X.509 bundle for the trust domain.
    #[prost(bytes = "vec", tag = "4")]
    pub bundle: ::prost::alloc::vec::Vec<u8>,
}
/// Generated client implementations.
pub mod spiffe_workload_api_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct SpiffeWorkloadApiClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl<T> SpiffeWorkloadApiClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> SpiffeWorkloadApiClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            SpiffeWorkloadApiClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Fetch X.509-SVIDs for all SPIFFE identities the workload is entitled to,
        /// as well as related information like trust bundles and CRLs. As this
        /// information changes, subsequent messages will be streamed from the
        /// server.
        pub async fn fetch_x509svid(
            &mut self,
            request: impl tonic::IntoRequest<super::X509svidRequest>,
        ) -> std::result::Result<
            tonic::Response<tonic::codec::Streaming<super::X509svidResponse>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/SpiffeWorkloadAPI/FetchX509SVID",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("SpiffeWorkloadAPI", "FetchX509SVID"));
            self.inner.server_streaming(req, path, codec).await
        }
    }
}