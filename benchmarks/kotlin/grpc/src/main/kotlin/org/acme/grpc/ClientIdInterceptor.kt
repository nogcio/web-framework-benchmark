package org.acme.grpc

import io.grpc.*

class ClientIdInterceptor : ServerInterceptor {
    companion object {
        val CLIENT_ID_CTX_KEY: Context.Key<String> = Context.key("x-client-id")
        val CLIENT_ID_HEADER_KEY: Metadata.Key<String> = Metadata.Key.of("x-client-id", Metadata.ASCII_STRING_MARSHALLER)
    }

    override fun <ReqT, RespT> interceptCall(
        call: ServerCall<ReqT, RespT>,
        headers: Metadata,
        next: ServerCallHandler<ReqT, RespT>
    ): ServerCall.Listener<ReqT> {
        val clientId = headers.get(CLIENT_ID_HEADER_KEY) ?: ""
        val context = Context.current().withValue(CLIENT_ID_CTX_KEY, clientId)
        return Contexts.interceptCall(context, call, headers, next)
    }
}
