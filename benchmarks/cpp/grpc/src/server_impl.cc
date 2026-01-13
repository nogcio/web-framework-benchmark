#include "server_impl.h"
#include "call_data.h"

#include <iostream>
#include <string>
#include <cstdlib>

#include <grpcpp/health_check_service_interface.h>
#include <grpcpp/ext/proto_server_reflection_plugin.h>

using grpc::ServerBuilder;

// Configuration matching other languages
constexpr int kStreamWindowSize = 1 * 1024 * 1024;      // 1MB
constexpr int kConnectionWindowSize = 10 * 1024 * 1024; // 10MB
constexpr int kMaxConcurrentStreams = 256;

ServerImpl::~ServerImpl() {
  server_->Shutdown();
  for (const auto& cq : cqs_) {
    cq->Shutdown();
  }
}

void ServerImpl::Run() {
  grpc::reflection::InitProtoReflectionServerBuilderPlugin();
  
  const char* port_env = std::getenv("PORT");
  std::string port = port_env ? port_env : "8080";
  std::string server_address = "0.0.0.0:" + port;

  ServerBuilder builder;
  builder.AddListeningPort(server_address, grpc::InsecureServerCredentials());
  builder.RegisterService(&service_);
  
  // Performance settings
  builder.AddChannelArgument("grpc.max_concurrent_streams", kMaxConcurrentStreams);
  builder.AddChannelArgument("grpc.http2.stream_window_size", kStreamWindowSize);
  builder.AddChannelArgument("grpc.http2.connection_window_size", kConnectionWindowSize);
  // Reuse port is generally good practice in containerized envs
  builder.AddChannelArgument("grpc.so_reuseport", 1);
  
  int num_cpus = std::thread::hardware_concurrency();
  if (num_cpus == 0) num_cpus = 1;
  
  std::cout << "Starting Async Server on " << server_address << " with " << num_cpus << " threads/CQs..." << std::endl;

  // Create one CompletionQueue per CPU core
  for (int i = 0; i < num_cpus; i++) {
    cqs_.emplace_back(builder.AddCompletionQueue());
  }

  // Health check setup (standard)
  grpc::EnableDefaultHealthCheckService(true);

  server_ = builder.BuildAndStart();
  if (!server_) {
      std::cerr << "Failed to start server" << std::endl;
      return;
  }
  
  if (auto* health_service = server_->GetHealthCheckService()) {
      health_service->SetServingStatus("", true);
      health_service->SetServingStatus("AnalyticsService", true);
  }

  // Spawn threads to drive the CQs
  std::vector<std::thread> threads;
  for (auto& cq : cqs_) {
    threads.emplace_back(&ServerImpl::HandleRpcs, this, cq.get());
  }

  for (auto& t : threads) {
    if (t.joinable()) t.join();
  }
}

void ServerImpl::HandleRpcs(ServerCompletionQueue* cq) {
  new CallData(&service_, cq);
  void* tag;
  bool ok;
  while (cq->Next(&tag, &ok)) {
    if (ok) {
      static_cast<CallData*>(tag)->Proceed();
    } else {
      // If !ok, it usually implies the CQ is shutting down.
      // In a real server we might need to cleanup the tag if it wasn't deleted.
      // But for benchmark simply exit loop.
      // delete static_cast<CallData*>(tag); 
    }
  }
}
