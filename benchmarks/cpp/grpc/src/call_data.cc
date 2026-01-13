#include "call_data.h"
#include <iostream>

CallData::CallData(AnalyticsService::AsyncService* service, ServerCompletionQueue* cq)
    : service_(service), cq_(cq), responder_(&ctx_), status_(CREATE) {
  Proceed();
}

void CallData::Proceed() {
  if (status_ == CREATE) {
    status_ = PROCESS;
    // Request the next call from the system.
    service_->RequestAggregateOrders(&ctx_, &request_, &responder_, cq_, cq_,
                                     this);
  } else if (status_ == PROCESS) {
    // Spawn a new CallData instance to serve new clients while we process this one.
    new CallData(service_, cq_);

    // The actual business logic
    ExecuteLogic();
    
    status_ = FINISH;
    responder_.Finish(reply_, Status::OK, this);
  } else {
    // STATUS == FINISH
    // The RPC is done, delete ourselves.
    // GPR_ASSERT(status_ == FINISH);
    delete this;
  }
}

void CallData::ExecuteLogic() {
    // Read Metadata
    const auto& client_metadata = ctx_.client_metadata();
    auto it = client_metadata.find("x-client-id");
    if (it != client_metadata.end()) {
        // string_ref to string conversion
        reply_.set_echoed_client_id(std::string(it->second.data(), it->second.length()));
    }

    int32_t processed = 0;
    
    // Use absl::flat_hash_map for performance
    absl::flat_hash_map<std::string, int64_t> amount_by_country;
    absl::flat_hash_map<std::string, int32_t> quantity_by_category;
    
    // Optimistic reservation
    amount_by_country.reserve(4);
    quantity_by_category.reserve(4);

    for (const auto& order : request_.orders()) {
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
    
    reply_.set_processed_orders(processed);
    
    // Map transfer
    auto* amap = reply_.mutable_amount_by_country();
    for (const auto& kv : amount_by_country) {
        (*amap)[kv.first] = kv.second;
    }
    auto* qmap = reply_.mutable_quantity_by_category();
    for (const auto& kv : quantity_by_category) {
        (*qmap)[kv.first] = kv.second;
    }
}
