#pragma once

#include <grpcpp/grpcpp.h>
#include "analytics.grpc.pb.h"
#include <absl/container/flat_hash_map.h>

using grpc::ServerCompletionQueue;
using grpc::ServerContext;
using grpc::ServerAsyncResponseWriter;
using grpc::Status;

class CallData {
 public:
  CallData(AnalyticsService::AsyncService* service, ServerCompletionQueue* cq);
  void Proceed();

 private:
  void ExecuteLogic();

  AnalyticsService::AsyncService* service_;
  ServerCompletionQueue* cq_;
  ServerContext ctx_;
  AnalyticsRequest request_;
  AggregateResult reply_;
  ServerAsyncResponseWriter<AggregateResult> responder_;
  enum CallStatus { CREATE, PROCESS, FINISH };
  CallStatus status_;
};
