package org.acme.grpc;

import io.grpc.Context;
import io.grpc.Contexts;
import io.grpc.Metadata;
import io.grpc.ServerCall;
import io.grpc.ServerCallHandler;
import io.grpc.ServerInterceptor;
import io.quarkus.grpc.GlobalInterceptor;
import jakarta.inject.Singleton;

@Singleton
@GlobalInterceptor
public class HeaderInterceptor implements ServerInterceptor {
    public static final Context.Key<String> CLIENT_ID_CTX_KEY = Context.key("x-client-id");
    public static final Metadata.Key<String> CLIENT_ID_HEADER_KEY = Metadata.Key.of("x-client-id", Metadata.ASCII_STRING_MARSHALLER);

    @Override
    public <ReqT, RespT> ServerCall.Listener<ReqT> interceptCall(ServerCall<ReqT, RespT> call, Metadata headers, ServerCallHandler<ReqT, RespT> next) {
        String clientId = headers.get(CLIENT_ID_HEADER_KEY);
        if (clientId == null) {
            clientId = "";
        }
        Context ctx = Context.current().withValue(CLIENT_ID_CTX_KEY, clientId);
        return Contexts.interceptCall(ctx, call, headers, next);
    }
}
