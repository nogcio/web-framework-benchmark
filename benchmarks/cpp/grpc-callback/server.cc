#include <iostream>
#include <memory>
#include <string>
#include <thread>
#include <vector>
#include <cstdlib>

#include <grpcpp/grpcpp.h>
#include <grpcpp/health_check_service_interface.h>
#include <grpcpp/ext/proto_server_reflection_plugin.h>

#include <absl/container/flat_hash_map.h>

#include "analytics.grpc.pb.h"

using grpc::Server;
using grpc::ServerBuilder;
using grpc::ServerContext;
using grpc::Status;
using grpc::CallbackServerContext;
using grpc::ServerUnaryReactor;

// Configuration
constexpr int kStreamWindowSize = 1 * 1024 * 1024;      // 1MB
constexpr int kConnectionWindowSize = 10 * 1024 * 1024; // 10MB
constexpr int kMaxConcurrentStreams = 256;

class AnalyticsServiceImpl final : public AnalyticsService::CallbackService {
 public:
  ServerUnaryReactor* AggregateOrders(CallbackServerContext* context,
                                      const AnalyticsRequest* request,
                                      AggregateResult* reply) override {
    
    // Read Metadata
    const auto& client_metadata = context->client_metadata();
    auto it = client_metadata.find("x-client-id");
    if (it != client_metadata.end()) {
        // string_ref to string conversion
        reply->set_echoed_client_id(std::string(it->second.data(), it->second.length()));
    }

    int32_t processed = 0;
    
    // Use absl::flat_hash_map for performance
    absl::flat_hash_map<std::string, int64_t> amount_by_country;
    absl::flat_hash_map<std::string, int32_t> quantity_by_category;
    
    // Optimistic reservation
    amount_by_country.reserve(4);
    quantity_by_category.reserve(4);

    for (const auto& order : request->orders()) {
        if (order.status() == OrderStatus::COMPLETED) {
            processed++;
            int64_t order_amount = 0;
            for (const auto& item : order.items()) {
                 int64_t total = (int64_t)item.price_cents() * item.quantity();
                 order_amount += total;
                 quantity_by_category[item.category()] += item.quantity();
            }
            amount_by_country[order.country()] += order_amount;
        }
    }
    
    reply->set_processed_orders(processed);
    
    // Map transfer
    auto* amap = reply->mutable_amount_by_country();
    for (const auto& kv : amount_by_country) {
        (*amap)[kv.first] = kv.second;
    }
    auto* qmap = reply->mutable_quantity_by_category();
    for (const auto& kv : quantity_by_category) {
        (*qmap)[kv.first] = kv.second;
    }

    // Since this is a synchronous calculation (cpu bound), we can just return specialized reactor
    // If we had async IO, we would return a reactor and call Finish() later.
    // For Unary RPCs that complete immediately, this is sufficient.
    
    ServerUnaryReactor* reactor = context->DefaultReactor();
    reactor->Finish(Status::OK);
    return reactor;
  }
};

void RunServer() {
  grpc::reflection::InitProtoReflectionServerBuilderPlugin();
  
  const char* port_env = std::getenv("PORT");
  std::string port = port_env ? port_env : "8080";
  std::string server_address = "0.0.0.0:" + port;

  AnalyticsServiceImpl service;

  ServerBuilder builder;
  builder.AddListeningPort(server_address, grpc::InsecureServerCredentials());
  builder.RegisterService(&service);
  
  // Performance settings
  builder.AddChannelArgument("grpc.max_concurrent_streams", kMaxConcurrentStreams);
  builder.AddChannelArgument("grpc.http2.stream_window_size", kStreamWindowSize);
  builder.AddChannelArgument("grpc.http2.connection_window_size", kConnectionWindowSize);
  builder.AddChannelArgument("grpc.so_reuseport", 1);
  
  int num_cpus = std::thread::hardware_concurrency();
    if (num_cpus == 0) num_cpus = 1;
  // With Callback API, gRPC manages the threading model nicely.
  // We can set SyncServerOption or similar if we wanted, but default is thread pool.
  // Generally we may want to ensure enough pollers.
  
  std::cout << "Starting Callback Server on " << server_address << " with " << num_cpus << " cores detected..." << std::endl;

  // Health check setup
  grpc::EnableDefaultHealthCheckService(true);

  std::unique_ptr<Server> server(builder.BuildAndStart());
    if (!server) {
        std::cerr << "Failed to start server" << std::endl;
        return;
    }

    if (auto* health_service = server->GetHealthCheckService()) {
        health_service->SetServingStatus("", true);
        health_service->SetServingStatus("AnalyticsService", true);
    }
    
  server->Wait();
}

int main(int argc, char** argv) {
  RunServer();
  return 0;
}
