#pragma once

#include <memory>
#include <vector>
#include <thread>
#include <grpcpp/grpcpp.h>
#include "analytics.grpc.pb.h"

using grpc::Server;
using grpc::ServerCompletionQueue;

class ServerImpl final {
 public:
  ~ServerImpl();
  void Run();

 private:
  void HandleRpcs(ServerCompletionQueue* cq);

  AnalyticsService::AsyncService service_;
  std::vector<std::unique_ptr<ServerCompletionQueue>> cqs_;
  std::unique_ptr<Server> server_;
};
